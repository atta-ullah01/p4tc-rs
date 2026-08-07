use crate::error::Result;
use crate::ffi_util::to_cstring;
use crate::table::parse;
use crate::types::Phase;
use crate::TableEntry;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Background subscription listening for kernel events on a table.
///
/// Created via [`Context::subscribe`].  Call [`stop`] or drop to end.
pub struct Subscription {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Subscription {
    pub fn active(&self) -> bool {
        !self.stop.load(Ordering::Relaxed)
            && self.thread.as_ref().map_or(false, |t| !t.is_finished())
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn join(mut self) {
        self.stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) fn spawn<F>(
    pipeline: &str,
    table: &str,
    filter: Option<&str>,
    callback: F,
) -> Result<Subscription>
where
    F: FnMut(&[TableEntry], Phase) + Send + 'static,
{
    let pipeline = pipeline.to_owned();
    let table = table.to_owned();
    let filter = filter.map(|f| f.to_owned());
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);

    let thread = thread::spawn(move || {
        run_subscriber(pipeline, table, filter, callback, stop2);
    });

    Ok(Subscription {
        stop,
        thread: Some(thread),
    })
}

fn run_subscriber<F>(
    pipeline: String,
    table: String,
    filter: Option<String>,
    mut callback: F,
    stop: Arc<AtomicBool>,
) where
    F: FnMut(&[TableEntry], Phase),
{
    use std::cell::Cell;

    let sub_ctx = unsafe {
        p4tc_sys::p4tc_runt_ctx_create(p4tc_sys::P4TC_TRANSPORT_NETLINK)
    };
    if sub_ctx.is_null() {
        return;
    }

    let c_pipe = match to_cstring(&pipeline, "pipeline") {
        Ok(c) => c,
        Err(_) => { unsafe { p4tc_sys::p4tc_runt_ctx_destroy(sub_ctx) }; return; }
    };
    let c_table = match to_cstring(&table, "table") {
        Ok(c) => c,
        Err(_) => { unsafe { p4tc_sys::p4tc_runt_ctx_destroy(sub_ctx) }; return; }
    };
    let c_filter = filter.as_deref().and_then(|f| to_cstring(f, "filter").ok());

    thread_local! {
        static CB_PTR: Cell<usize> = const { Cell::new(0) };
        static STOP_PTR: Cell<usize> = const { Cell::new(0) };
    }

    unsafe extern "C" fn trampoline<F: FnMut(&[TableEntry], Phase)>(
        obj_ptr: *const p4tc_sys::p4tc_obj,
        _ctx: *mut p4tc_sys::p4tc_runt_ctx,
        _cookie: *mut u64,
        phase_val: libc::c_int,
    ) -> libc::c_int {
        let should_stop = STOP_PTR.with(|cell| {
            let ptr = cell.get() as *const AtomicBool;
            if !ptr.is_null() {
                unsafe { (*ptr).load(Ordering::Relaxed) }
            } else {
                false
            }
        });
        if should_stop {
            return -1;
        }

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
            Phase::Abt => -1,
            _ => 0,
        }
    }

    CB_PTR.with(|cell| cell.set(&mut callback as *mut F as usize));
    STOP_PTR.with(|cell| cell.set(Arc::as_ptr(&stop) as usize));

    while !stop.load(Ordering::Relaxed) {
        let obj = unsafe { p4tc_sys::p4tc_obj_create(c_pipe.as_ptr(), p4tc_sys::P4TC_OBJ_TABLE) };
        if obj.is_null() {
            break;
        }
        unsafe { p4tc_sys::p4tc_obj_objname_set(obj, c_table.as_ptr()) };
        if let Some(ref c_f) = c_filter {
            unsafe { p4tc_sys::p4tc_obj_filter_set(obj, c_f.as_ptr()) };
        }

        unsafe {
            p4tc_sys::p4tc_subscribe(
                sub_ctx,
                obj as *const _,
                0,
                Some(trampoline::<F>),
                std::ptr::null_mut(),
            );
            p4tc_sys::p4tc_obj_destroy(obj);
        }
    }

    CB_PTR.with(|cell| cell.set(0));
    STOP_PTR.with(|cell| cell.set(0));
    unsafe { p4tc_sys::p4tc_runt_ctx_destroy(sub_ctx) };
}
