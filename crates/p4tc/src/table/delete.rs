use crate::error::Result;
use crate::types::{MsgFlags, Phase};
use crate::Context;
use super::obj::{self, ObjHandle};

pub struct DeleteBuilder<'a> {
    ctx: &'a Context,
    pipeline: &'a str,
    table: &'a str,
    keys: Vec<String>,
    filter: Option<String>,
    flags: MsgFlags,
}

impl<'a> DeleteBuilder<'a> {
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

    fn build_obj(&self) -> Result<ObjHandle> {
        let obj = ObjHandle::new(self.pipeline)?;
        obj.set_table(self.table)?;

        if let Some(ref f) = self.filter {
            obj.set_filter(f)?;
        }

        if !self.keys.is_empty() {
            let key = obj.make_key(&self.keys)?;
            obj.alloc_entry(key, p4tc_sys::P4TC_ENTITY_KERNEL)?;
        }
        Ok(obj)
    }

    pub fn execute(self) -> Result<()> {
        let obj = self.build_obj()?;
        obj::fire_crud(p4tc_sys::p4tc_del, self.ctx, &obj,
                       self.flags.bits(), "delete")
    }

    pub fn execute_with<F: FnMut(Phase)>(self, mut cb: F) -> Result<()> {
        let obj = self.build_obj()?;
        obj::fire_crud_with_cb(p4tc_sys::p4tc_del, self.ctx, &obj,
                               self.flags.bits(), "delete", &mut cb)
    }
}
