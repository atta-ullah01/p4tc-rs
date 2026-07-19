use crate::error::{Error, Result};
use crate::types::Transport;
use std::ptr::NonNull;

pub struct Context {
    inner: NonNull<p4tc_sys::p4tc_runt_ctx>,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    pub fn new(transport: Transport) -> Result<Self> {
        let ptr = unsafe { p4tc_sys::p4tc_runt_ctx_create(transport as i32) };
        NonNull::new(ptr)
            .map(|inner| Self { inner })
            .ok_or_else(|| Error::Context(std::io::Error::last_os_error()))
    }

    pub(crate) fn as_ptr(&self) -> *mut p4tc_sys::p4tc_runt_ctx {
        self.inner.as_ptr()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { p4tc_sys::p4tc_runt_ctx_destroy(self.inner.as_ptr()) }
    }
}
