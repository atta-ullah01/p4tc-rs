use crate::error::{Error, Result};
use crate::ffi_util::to_cstring;
use crate::types::{Entity, MsgFlags};
use crate::Context;
use std::ffi::CString;

pub struct InsertBuilder<'a> {
    ctx: &'a Context,
    pipeline: &'a str,
    table: &'a str,
    keys: Vec<String>,
    action_path: Option<String>,
    action_params: Vec<String>,
    priority: u32,
    entity: Entity,
    flags: MsgFlags,
}

impl<'a> InsertBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, table: &'a str) -> Self {
        Self {
            ctx,
            pipeline,
            table,
            keys: Vec::new(),
            action_path: None,
            action_params: Vec::new(),
            priority: 0,
            entity: Entity::Tc,
            flags: MsgFlags::empty(),
        }
    }

    pub fn key(mut self, value: &str) -> Self {
        self.keys.push(value.to_owned());
        self
    }

    pub fn action(mut self, path: &str) -> Self {
        self.action_path = Some(path.to_owned());
        self
    }

    pub fn param(mut self, value: &str) -> Self {
        self.action_params.push(value.to_owned());
        self
    }

    pub fn priority(mut self, prio: u32) -> Self {
        self.priority = prio;
        self
    }

    pub fn entity(mut self, entity: Entity) -> Self {
        self.entity = entity;
        self
    }

    pub fn execute(self) -> Result<()> {
        let c_pipeline = to_cstring(self.pipeline, "pipeline")?;
        let c_table = to_cstring(self.table, "table")?;

        let obj = unsafe {
            p4tc_sys::p4tc_obj_create(c_pipeline.as_ptr(), p4tc_sys::P4TC_OBJ_TABLE)
        };
        if obj.is_null() {
            return Err(Error::Object {
                msg: format!("obj_create failed for '{}'", self.pipeline),
            });
        }

        // obj_destroy cascades and frees everything attached, so we
        // make sure it runs even if something below fails.
        let result = self.build_and_send(obj, &c_table);
        unsafe { p4tc_sys::p4tc_obj_destroy(obj) };
        result
    }

    fn build_and_send(
        &self,
        obj: *mut p4tc_sys::p4tc_obj,
        c_table: &CString,
    ) -> Result<()> {
        unsafe { p4tc_sys::p4tc_obj_objname_set(obj, c_table.as_ptr()) };

        let c_keys: Vec<CString> = self
            .keys
            .iter()
            .map(|k| to_cstring(k, "key value"))
            .collect::<Result<_>>()?;
        let key_ptrs: Vec<*const libc::c_char> = c_keys.iter().map(|k| k.as_ptr()).collect();

        let raw_key = unsafe {
            p4tc_sys::p4tc_make_key(obj, c_keys.len() as i32, key_ptrs.as_ptr())
        };
        if raw_key.is_null() {
            return Err(Error::Key {
                msg: "make_key failed".to_owned(),
            });
        }

        // key ownership transfers to obj via alloc_tbl_entry
        let entry = unsafe {
            p4tc_sys::p4tc_alloc_tbl_entry(obj, raw_key, 0, self.entity as i32)
        };
        if entry.is_null() {
            return Err(Error::Entry {
                msg: format!("alloc_tbl_entry failed for '{}'", self.table),
            });
        }

        if self.priority > 0 {
            unsafe { p4tc_sys::p4tc_runt_tbl_attrs_prio_set(entry, self.priority) };
        }

        if let Some(ref path) = self.action_path {
            let c_path = to_cstring(path, "action path")?;
            let c_params: Vec<CString> = self
                .action_params
                .iter()
                .map(|p| to_cstring(p, "action param"))
                .collect::<Result<_>>()?;
            let param_ptrs: Vec<*const libc::c_char> =
                c_params.iter().map(|p| p.as_ptr()).collect();

            let act = unsafe {
                p4tc_sys::p4tc_create_runt_act(
                    entry,
                    c_path.as_ptr(),
                    c_params.len() as i32,
                    param_ptrs.as_ptr(),
                )
            };
            if act.is_null() {
                return Err(Error::Entry {
                    msg: format!("create_runt_act failed for '{path}'"),
                });
            }
        }

        let ret = unsafe {
            p4tc_sys::p4tc_create(
                self.ctx.as_ptr(),
                obj,
                self.flags.bits(),
                None,
                std::ptr::null_mut(),
            )
        };
        if ret != 0 {
            return Err(Error::Crud {
                op: "create",
                source: std::io::Error::last_os_error(),
            });
        }

        Ok(())
    }
}
