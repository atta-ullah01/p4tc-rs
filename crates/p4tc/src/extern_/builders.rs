use crate::error::{Error, Result};
use crate::extern_::{ExtObjHandle, ExternEntry, parse_ext_obj};
use crate::types::{MsgFlags, Phase};
use crate::Context;
use std::cell::Cell;

pub struct ExternBuilder<'a> {
    ctx: &'a Context,
    pipeline: &'a str,
    kind: &'a str,
    instance: &'a str,
    key: u32,
    params: Vec<String>,
    flags: MsgFlags,
}

impl<'a> ExternBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, kind: &'a str, instance: &'a str) -> Self {
        Self {
            ctx, pipeline, kind, instance,
            key: 0,
            params: Vec::new(),
            flags: MsgFlags::empty(),
        }
    }

    pub fn key(mut self, k: u32) -> Self {
        self.key = k;
        self
    }

    pub fn param(mut self, value: &str) -> Self {
        self.params.push(value.into());
        self
    }

    pub fn params(mut self, values: &[&str]) -> Self {
        self.params.extend(values.iter().map(|v| v.to_string()));
        self
    }

    #[cfg(feature = "schema")]
    pub fn fill_dummy_params(mut self, schema: &crate::schema::PipelineSchema) -> Self {
        if let Some(ext) = schema.get_extern(self.kind) {
            if let Some(inst) = ext.get_instance(self.instance) {
                let n = inst.param_names.len();
                if self.params.is_empty() && n > 0 {
                    self.params = vec!["0".into(); n];
                }
            }
        }
        self
    }

    fn build_obj(&self) -> Result<ExtObjHandle> {
        ExtObjHandle::new(self.pipeline, self.kind, self.instance, self.key, &self.params)
    }

    fn fire(
        &self,
        crud_fn: unsafe extern "C" fn(
            *mut p4tc_sys::p4tc_runt_ctx, *mut p4tc_sys::p4tc_obj,
            libc::c_uint, p4tc_sys::p4tc_callback, *mut u64,
        ) -> libc::c_int,
        op_name: &'static str,
    ) -> Result<()> {
        let obj = self.build_obj()?;
        let ret = unsafe {
            crud_fn(self.ctx.as_ptr(), obj.as_ptr(), self.flags.bits(), None, std::ptr::null_mut())
        };
        if ret != 0 {
            return Err(Error::Crud { op: op_name, source: std::io::Error::last_os_error() });
        }
        Ok(())
    }

    fn fire_with_cb<F: FnMut(&[ExternEntry], Phase)>(
        &self,
        crud_fn: unsafe extern "C" fn(
            *mut p4tc_sys::p4tc_runt_ctx, *mut p4tc_sys::p4tc_obj,
            libc::c_uint, p4tc_sys::p4tc_callback, *mut u64,
        ) -> libc::c_int,
        op_name: &'static str,
        cb: &mut F,
    ) -> Result<()> {
        thread_local! {
            static CB_PTR: Cell<usize> = const { Cell::new(0) };
        }

        unsafe extern "C" fn trampoline<F: FnMut(&[ExternEntry], Phase)>(
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
                                let entries = parse_ext_obj(obj_ptr);
                                unsafe { (*ptr)(&entries, phase) };
                            }
                        });
                    }
                    0
                }
                _ => 0,
            }
        }

        let obj = self.build_obj()?;
        CB_PTR.with(|cell| cell.set(cb as *mut F as usize));

        let ret = unsafe {
            crud_fn(self.ctx.as_ptr(), obj.as_ptr(), self.flags.bits(),
                    Some(trampoline::<F>), std::ptr::null_mut())
        };

        CB_PTR.with(|cell| cell.set(0));

        if ret != 0 {
            return Err(Error::Crud { op: op_name, source: std::io::Error::last_os_error() });
        }
        Ok(())
    }
}

pub struct ExternInsertBuilder<'a>(ExternBuilder<'a>);

impl<'a> ExternInsertBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, kind: &'a str, instance: &'a str) -> Self {
        Self(ExternBuilder::new(ctx, pipeline, kind, instance))
    }

    pub fn key(mut self, k: u32) -> Self { self.0 = self.0.key(k); self }
    pub fn param(mut self, value: &str) -> Self { self.0 = self.0.param(value); self }
    pub fn params(mut self, values: &[&str]) -> Self { self.0 = self.0.params(values); self }

    pub fn execute(self) -> Result<()> {
        self.0.fire(p4tc_sys::p4tc_create, "extern_insert")
    }

    pub fn execute_with<F: FnMut(&[ExternEntry], Phase)>(self, mut cb: F) -> Result<()> {
        self.0.fire_with_cb(p4tc_sys::p4tc_create, "extern_insert", &mut cb)
    }
}

pub struct ExternUpdateBuilder<'a>(ExternBuilder<'a>);

impl<'a> ExternUpdateBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, kind: &'a str, instance: &'a str) -> Self {
        Self(ExternBuilder::new(ctx, pipeline, kind, instance))
    }

    pub fn key(mut self, k: u32) -> Self { self.0 = self.0.key(k); self }
    pub fn param(mut self, value: &str) -> Self { self.0 = self.0.param(value); self }
    pub fn params(mut self, values: &[&str]) -> Self { self.0 = self.0.params(values); self }

    pub fn execute(self) -> Result<()> {
        self.0.fire(p4tc_sys::p4tc_update, "extern_update")
    }

    pub fn execute_with<F: FnMut(&[ExternEntry], Phase)>(self, mut cb: F) -> Result<()> {
        self.0.fire_with_cb(p4tc_sys::p4tc_update, "extern_update", &mut cb)
    }
}

pub struct ExternDeleteBuilder<'a>(ExternBuilder<'a>);

impl<'a> ExternDeleteBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, kind: &'a str, instance: &'a str) -> Self {
        Self(ExternBuilder::new(ctx, pipeline, kind, instance))
    }

    pub fn key(mut self, k: u32) -> Self { self.0 = self.0.key(k); self }

    #[cfg(feature = "schema")]
    pub fn schema(mut self, s: &crate::schema::PipelineSchema) -> Self {
        self.0 = self.0.fill_dummy_params(s); self
    }

    pub fn execute(self) -> Result<()> {
        self.0.fire(p4tc_sys::p4tc_del, "extern_delete")
    }

    pub fn execute_with<F: FnMut(&[ExternEntry], Phase)>(self, mut cb: F) -> Result<()> {
        self.0.fire_with_cb(p4tc_sys::p4tc_del, "extern_delete", &mut cb)
    }
}

pub struct ExternGetBuilder<'a>(ExternBuilder<'a>);

impl<'a> ExternGetBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, kind: &'a str, instance: &'a str) -> Self {
        Self(ExternBuilder::new(ctx, pipeline, kind, instance))
    }

    pub fn key(mut self, k: u32) -> Self { self.0 = self.0.key(k); self }

    #[cfg(feature = "schema")]
    pub fn schema(mut self, s: &crate::schema::PipelineSchema) -> Self {
        self.0 = self.0.fill_dummy_params(s); self
    }

    pub fn execute<F: FnMut(&[ExternEntry], Phase)>(self, mut cb: F) -> Result<()> {
        self.0.fire_with_cb(p4tc_sys::p4tc_get, "extern_get", &mut cb)
    }
}
