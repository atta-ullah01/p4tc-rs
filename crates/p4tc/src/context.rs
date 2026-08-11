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

    /// Flush all entries from a table.
    pub fn flush<'a>(&'a self, pipeline: &'a str, table: &'a str) -> crate::table::DeleteBuilder<'a> {
        crate::table::DeleteBuilder::new(self, pipeline, table)
    }

    pub fn extern_insert<'a>(&'a self, pipeline: &'a str, kind: &'a str, instance: &'a str) -> crate::extern_::ExternInsertBuilder<'a> {
        crate::extern_::ExternInsertBuilder::new(self, pipeline, kind, instance)
    }

    pub fn extern_update<'a>(&'a self, pipeline: &'a str, kind: &'a str, instance: &'a str) -> crate::extern_::ExternUpdateBuilder<'a> {
        crate::extern_::ExternUpdateBuilder::new(self, pipeline, kind, instance)
    }

    pub fn extern_delete<'a>(&'a self, pipeline: &'a str, kind: &'a str, instance: &'a str) -> crate::extern_::ExternDeleteBuilder<'a> {
        crate::extern_::ExternDeleteBuilder::new(self, pipeline, kind, instance)
    }

    pub fn extern_get<'a>(&'a self, pipeline: &'a str, kind: &'a str, instance: &'a str) -> crate::extern_::ExternGetBuilder<'a> {
        crate::extern_::ExternGetBuilder::new(self, pipeline, kind, instance)
    }

    /// Subscribe to real-time events on a table.
    pub fn subscribe<F>(
        &self, pipeline: &str, table: &str, callback: F,
    ) -> crate::error::Result<crate::subscribe::Subscription>
    where
        F: FnMut(&[crate::table::TableEntry], crate::types::Phase) + Send + 'static,
    {
        crate::subscribe::spawn(self.as_ptr(), pipeline, table, None, callback)
    }

    /// Subscribe with a filter expression.
    pub fn subscribe_filtered<F>(
        &self, pipeline: &str, table: &str, filter: &str, callback: F,
    ) -> crate::error::Result<crate::subscribe::Subscription>
    where
        F: FnMut(&[crate::table::TableEntry], crate::types::Phase) + Send + 'static,
    {
        crate::subscribe::spawn(self.as_ptr(), pipeline, table, Some(filter), callback)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { p4tc_sys::p4tc_runt_ctx_destroy(self.inner.as_ptr()) }
    }
}
