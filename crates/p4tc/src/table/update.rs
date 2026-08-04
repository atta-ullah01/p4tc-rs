use crate::error::Result;
use crate::types::{Entity, MsgFlags, Phase};
use crate::Context;
use super::obj::{self, EntryAttrs, ObjHandle};

pub struct UpdateBuilder<'a> {
    ctx: &'a Context,
    pipeline: &'a str,
    table: &'a str,
    keys: Vec<String>,
    action_path: Option<String>,
    action_params: Vec<String>,
    attrs: EntryAttrs,
    entity: Entity,
    filter: Option<String>,
    flags: MsgFlags,
}

impl<'a> UpdateBuilder<'a> {
    pub(crate) fn new(ctx: &'a Context, pipeline: &'a str, table: &'a str) -> Self {
        Self {
            ctx, pipeline, table,
            keys: Vec::new(),
            action_path: None,
            action_params: Vec::new(),
            attrs: EntryAttrs::default(),
            entity: Entity::Tc,
            filter: None,
            flags: MsgFlags::empty(),
        }
    }

    pub fn key(mut self, value: &str) -> Self {
        self.keys.push(value.into());
        self
    }

    pub fn action(mut self, path: &str) -> Self {
        self.action_path = Some(path.into());
        self
    }

    pub fn param(mut self, value: &str) -> Self {
        self.action_params.push(value.into());
        self
    }

    pub fn priority(mut self, v: u32) -> Self { self.attrs.priority = v; self }
    pub fn aging_ms(mut self, v: u32) -> Self { self.attrs.aging_ms = Some(v); self }
    pub fn profile_id(mut self, v: u32) -> Self { self.attrs.profile_id = Some(v); self }
    pub fn permissions(mut self, v: u32) -> Self { self.attrs.permissions = Some(v); self }
    pub fn dynamic(mut self, v: bool) -> Self { self.attrs.dynamic = Some(v); self }

    pub fn entity(mut self, entity: Entity) -> Self {
        self.entity = entity;
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
            let entry = obj.alloc_entry(key, self.entity as i32)?;
            obj::apply_entry_attrs(entry, &self.attrs);

            if let Some(ref path) = self.action_path {
                obj::attach_action(entry, path, &self.action_params)?;
            }
        }
        Ok(obj)
    }

    pub fn execute(self) -> Result<()> {
        let obj = self.build_obj()?;
        obj::fire_crud(p4tc_sys::p4tc_update, self.ctx, &obj,
                       self.flags.bits(), "update")
    }

    pub fn execute_with<F: FnMut(Phase)>(self, mut cb: F) -> Result<()> {
        let obj = self.build_obj()?;
        obj::fire_crud_with_cb(p4tc_sys::p4tc_update, self.ctx, &obj,
                               self.flags.bits(), "update", &mut cb)
    }
}
