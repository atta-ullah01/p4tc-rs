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

    pub fn insert<'a>(&'a self, pipeline: &'a str, table: &'a str) -> crate::table::InsertBuilder<'a> {
        crate::table::InsertBuilder::new(self, pipeline, table)
    }

    pub fn update<'a>(&'a self, pipeline: &'a str, table: &'a str) -> crate::table::UpdateBuilder<'a> {
        crate::table::UpdateBuilder::new(self, pipeline, table)
    }

    pub fn delete<'a>(&'a self, pipeline: &'a str, table: &'a str) -> crate::table::DeleteBuilder<'a> {
        crate::table::DeleteBuilder::new(self, pipeline, table)
    }

    pub fn get<'a>(&'a self, pipeline: &'a str, table: &'a str) -> crate::table::GetBuilder<'a> {
        crate::table::GetBuilder::new(self, pipeline, table)
    }

    /// Dump all entries from a table. Shorthand for `get()` without keys.
    pub fn dump<'a>(&'a self, pipeline: &'a str, table: &'a str) -> crate::table::GetBuilder<'a> {
        crate::table::GetBuilder::new(self, pipeline, table)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { p4tc_sys::p4tc_runt_ctx_destroy(self.inner.as_ptr()) }
    }
}
