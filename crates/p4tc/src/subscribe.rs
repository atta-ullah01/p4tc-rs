use crate::error::{Error, Result};
use crate::ffi_util::to_cstring;
use crate::table::parse;
use crate::types::Phase;
use crate::TableEntry;

/// Holds the user closure with pipeline/table for the trampoline.
struct CbState {
    cb: Box<dyn FnMut(&[TableEntry], Phase) + Send + 'static>,
    pipeline: String,
    table: String,
}

/// Background table event subscription.
///
/// The C library spawns an internal thread via
/// `p4tc_subscribe_resp_handle` to dispatch events.
///
/// Created via [`Context::subscribe`](crate::Context::subscribe).
pub struct Subscription {
    ctx: *mut p4tc_sys::p4tc_runt_ctx,
    sub_id: i32,
    state_ptr: *mut CbState,
    cookie_ptr: *mut u64,
}

// Safety: the C library serialises access internally.
unsafe impl Send for Subscription {}
unsafe impl Sync for Subscription {}

impl Subscription {
    /// Returns true if the subscription is active.
    pub fn active(&self) -> bool {
        self.sub_id >= 0
    }

    /// Cancel the subscription.
    pub fn stop(&mut self) {
        if self.sub_id >= 0 {
            unsafe { p4tc_sys::p4tc_unsubscribe(self.ctx, self.sub_id) };
            self.sub_id = -1;
        }
    }

    /// Same as `stop()`, consuming self.
    pub fn join(mut self) {
        self.stop();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.stop();
        if !self.cookie_ptr.is_null() {
            unsafe { let _ = Box::from_raw(self.cookie_ptr); }
            self.cookie_ptr = std::ptr::null_mut();
        }
        if !self.state_ptr.is_null() {
            unsafe { let _ = Box::from_raw(self.state_ptr); }
            self.state_ptr = std::ptr::null_mut();
        }
    }
}

pub(crate) fn spawn<F>(
    ctx: *mut p4tc_sys::p4tc_runt_ctx,
    pipeline: &str,
    table: &str,
    filter: Option<&str>,
    callback: F,
) -> Result<Subscription>
where
    F: FnMut(&[TableEntry], Phase) + Send + 'static,
{
    let c_pipe = to_cstring(pipeline, "pipeline")?;
    let c_table = to_cstring(table, "table")?;
    let c_filter = filter.map(|f| to_cstring(f, "filter")).transpose()?;

    // Package the closure with pipeline/table for the trampoline.
    let state = Box::new(CbState {
        cb: Box::new(callback),
        pipeline: pipeline.to_owned(),
        table: table.to_owned(),
    });
    let state_ptr = Box::into_raw(state);

    // Cookie points to the state, which the trampoline dereferences.
    let cookie_ptr = Box::into_raw(Box::new(state_ptr as u64));

    unsafe extern "C" fn trampoline(
        obj_ptr: *const p4tc_sys::p4tc_obj,
        _ctx: *mut p4tc_sys::p4tc_runt_ctx,
        _cookie: *mut u64,
        phase_val: libc::c_int,
    ) -> libc::c_int {
        let phase = Phase::from_raw(phase_val);
        match phase {
            Phase::Sot | Phase::Mot => {
                if !obj_ptr.is_null() && !_cookie.is_null() {
                    let state_addr = unsafe { *_cookie };
                    let ptr = state_addr as *mut CbState;
                    if !ptr.is_null() {
                        let state = unsafe { &mut *ptr };
                        let entries = unsafe {
                            parse::parse_obj(obj_ptr, &state.pipeline, &state.table)
                        };
                        (state.cb)(&entries, phase);
                    }
                }
                0
            }
            _ => 0,
        }
    }

    // Register the subscription.
    let obj = unsafe { p4tc_sys::p4tc_obj_create(c_pipe.as_ptr(), p4tc_sys::P4TC_OBJ_TABLE) };
    if obj.is_null() {
        unsafe {
            let _ = Box::from_raw(cookie_ptr);
            let _ = Box::from_raw(state_ptr);
        }
        return Err(Error::Object { msg: "p4tc_obj_create failed for subscribe".into() });
    }

    unsafe { p4tc_sys::p4tc_obj_objname_set(obj, c_table.as_ptr()) };
    if let Some(ref c_f) = c_filter {
        unsafe { p4tc_sys::p4tc_obj_filter_set(obj, c_f.as_ptr()) };
    }

    let sub_id = unsafe {
        p4tc_sys::p4tc_subscribe(
            ctx,
            obj as *const _,
            0,
            Some(trampoline),
            cookie_ptr,
        )
    };
    unsafe { p4tc_sys::p4tc_obj_destroy(obj) };

    if sub_id < 0 {
        unsafe {
            let _ = Box::from_raw(cookie_ptr);
            let _ = Box::from_raw(state_ptr);
        }
        return Err(Error::Subscribe(std::io::Error::last_os_error()));
    }

    // Spawns an internal thread in the library; returns immediately.
    unsafe { p4tc_sys::p4tc_subscribe_resp_handle(ctx, sub_id) };

    Ok(Subscription {
        ctx,
        sub_id,
        state_ptr,
        cookie_ptr,
    })
}
