use crate::error::Result;
use crate::types::{Entity, MsgFlags};
use crate::Context;
use super::obj::{self, ObjHandle};

pub struct UpdateBuilder<'a> {
    ctx: &'a Context,
    pipeline: &'a str,
    table: &'a str,
    keys: Vec<String>,
    action_path: Option<String>,
    action_params: Vec<String>,
    priority: u32,
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
            priority: 0,
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

    pub fn priority(mut self, prio: u32) -> Self {
        self.priority = prio;
        self
    }

    pub fn entity(mut self, entity: Entity) -> Self {
        self.entity = entity;
        self
    }

    pub fn filter(mut self, f: &str) -> Self {
        self.filter = Some(f.into());
        self
    }

    pub fn execute(self) -> Result<()> {
        let obj = ObjHandle::new(self.pipeline)?;
        obj.set_table(self.table)?;

        if let Some(ref f) = self.filter {
            obj.set_filter(f)?;
        }

        if !self.keys.is_empty() {
            let key = obj.make_key(&self.keys)?;
            let entry = obj.alloc_entry(key, self.entity as i32)?;

            if self.priority > 0 {
                unsafe { p4tc_sys::p4tc_runt_tbl_attrs_prio_set(entry, self.priority) };
            }
            if let Some(ref path) = self.action_path {
                obj::attach_action(entry, path, &self.action_params)?;
            }
        }

        obj::fire_crud(p4tc_sys::p4tc_update, self.ctx, &obj, self.flags.bits(), "update")
    }
}
