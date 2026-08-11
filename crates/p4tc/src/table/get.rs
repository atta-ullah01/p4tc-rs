use crate::error::{Error, Result};
use crate::types::{MsgFlags, Phase};
use crate::Context;
use super::entry::TableEntry;
use super::obj::ObjHandle;
use super::parse;

pub struct GetBuilder<'a> {
    ctx: &'a Context,
    pipeline: &'a str,
    table: &'a str,
    keys: Vec<String>,
    filter: Option<String>,
    flags: MsgFlags,
}

impl<'a> GetBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, table: &'a str) -> Self {
        Self {
            ctx, pipeline, table,
            keys: Vec::new(),
            filter: None,
            flags: MsgFlags::empty(),
        }
    }

    pub fn key(mut self, value: &str) -> Self {
        self.keys.push(value.into());
        self
    }

    pub fn filter(mut self, f: &str) -> Self {
        self.filter = Some(f.into());
        self
    }

    pub fn execute<F>(self, mut callback: F) -> Result<()>
    where
        F: FnMut(&[TableEntry], Phase),
    {
        use std::cell::Cell;

        thread_local! {
            static CB_PTR: Cell<usize> = const { Cell::new(0) };
        }

        let obj = ObjHandle::new(self.pipeline)?;
        obj.set_table(self.table)?;

        if let Some(ref f) = self.filter {
            obj.set_filter(f)?;
        }

        if !self.keys.is_empty() {
            let key = obj.make_key(&self.keys)?;
            obj.alloc_entry(key, p4tc_sys::P4TC_ENTITY_KERNEL)?;
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
                Phase::Abt => 0,
                _ => 0,
            }
        }

        CB_PTR.with(|cell| cell.set(&mut callback as *mut F as usize));

        let ret = unsafe {
            p4tc_sys::p4tc_get(
                self.ctx.as_ptr(),
                obj.as_ptr(),
                self.flags.bits(),
                Some(trampoline::<F>),
                std::ptr::null_mut(),
            )
        };

        CB_PTR.with(|cell| cell.set(0));

        if ret != 0 {
            return Err(Error::Crud {
                op: "get",
                source: std::io::Error::last_os_error(),
            });
        }

        Ok(())
    }
}
