use crate::error::{Error, Result};
use crate::ffi_util::to_cstring;
use crate::table::parse;
use crate::types::Phase;
use crate::TableEntry;

use std::thread::{self, JoinHandle};

struct SendPtr(*mut p4tc_sys::p4tc_runt_ctx);
unsafe impl Send for SendPtr {}

/// Background subscription listening for kernel events on a table.
///
/// Uses the C library's `p4tc_subscribe` / `p4tc_subscribe_resp_handle` /
/// `p4tc_unsubscribe` API.  The library manages the event loop
/// internally via epoll.
///
/// Created via [`Context::subscribe`](crate::Context::subscribe).
/// Call [`stop`](Self::stop) or drop to cancel.
pub struct Subscription {
    ctx: SendPtr,
    sub_id: i32,
    thread: Option<JoinHandle<()>>,
}

// Safety: the C library serialises access internally.
unsafe impl Send for Subscription {}
unsafe impl Sync for Subscription {}

impl Subscription {
    /// Returns true if the background thread is still running.
    pub fn active(&self) -> bool {
        self.thread.as_ref().map_or(false, |t| !t.is_finished())
    }

    /// Cancel the subscription and join the library's internal thread.
    pub fn stop(&mut self) {
        if self.sub_id >= 0 {
            unsafe { p4tc_sys::p4tc_unsubscribe(self.ctx.0, self.sub_id) };
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
            self.sub_id = -1;
        }
    }

    /// Cancel and wait for the background thread to exit.
    pub fn join(mut self) {
        self.stop();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) fn spawn<F>(
    ctx: *mut p4tc_sys::p4tc_runt_ctx,
    pipeline: &str,
    table: &str,
    filter: Option<&str>,
    mut callback: F,
) -> Result<Subscription>
where
    F: FnMut(&[TableEntry], Phase) + Send + 'static,
{
    use std::cell::Cell;

    let c_pipe = to_cstring(pipeline, "pipeline")?;
    let c_table = to_cstring(table, "table")?;
    let c_filter = filter.map(|f| to_cstring(f, "filter")).transpose()?;

    // Build the callback trampoline.
    thread_local! {
        static CB_PTR: Cell<usize> = const { Cell::new(0) };
    }

    unsafe extern "C" fn trampoline<F: FnMut(&[TableEntry], Phase)>(
        obj_ptr: *const p4tc_sys::p4tc_obj,
        _ctx: *mut p4tc_sys::p4tc_runt_ctx,
        _cookie: *mut u64,
        phase_val: libc::c_int,
    ) -> libc::c_int {
        let phase = Phase::from_raw(phase_val);
        match phase {
            Phase::Sot | Phase::Mot => {
                if !obj_ptr.is_null() {
                    CB_PTR.with(|cell| {
                        let ptr = cell.get() as *mut F;
                        if !ptr.is_null() {
                            let entries = unsafe { parse::parse_obj(obj_ptr) };
                            unsafe { (*ptr)(&entries, phase) };
                        }
                    });
                }
                0
            }
            _ => 0,
        }
    }

    // Register the subscription (non-blocking).
    let obj = unsafe { p4tc_sys::p4tc_obj_create(c_pipe.as_ptr(), p4tc_sys::P4TC_OBJ_TABLE) };
    if obj.is_null() {
        return Err(Error::Object { msg: "p4tc_obj_create failed for subscribe".into() });
    }

    unsafe { p4tc_sys::p4tc_obj_objname_set(obj, c_table.as_ptr()) };
    if let Some(ref c_f) = c_filter {
        unsafe { p4tc_sys::p4tc_obj_filter_set(obj, c_f.as_ptr()) };
    }

    let mut cookie: u64 = 0;
    let sub_id = unsafe {
        p4tc_sys::p4tc_subscribe(
            ctx,
            obj as *const _,
            0,
            Some(trampoline::<F>),
            &mut cookie,
        )
    };
    unsafe { p4tc_sys::p4tc_obj_destroy(obj) };

    if sub_id < 0 {
        return Err(Error::Subscribe(std::io::Error::last_os_error()));
    }

    // p4tc_subscribe_resp_handle blocks, so run it in a thread.
    // Cast ctx to usize to cross the thread boundary (raw pointers aren't Send).
    let ctx_addr = ctx as usize;
    let sid = sub_id;
    let thread = thread::spawn(move || {
        CB_PTR.with(|cell| cell.set(&mut callback as *mut F as usize));
        let ctx_ptr = ctx_addr as *mut p4tc_sys::p4tc_runt_ctx;
        unsafe { p4tc_sys::p4tc_subscribe_resp_handle(ctx_ptr, sid) };
        CB_PTR.with(|cell| cell.set(0));
    });

    Ok(Subscription {
        ctx: SendPtr(ctx),
        sub_id,
        thread: Some(thread),
    })
}
