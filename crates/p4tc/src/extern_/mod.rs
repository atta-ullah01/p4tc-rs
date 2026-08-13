mod builders;

use crate::error::{Error, Result};
use crate::ffi_util::to_cstring;
use crate::table::Param;
use std::ffi::CString;

pub use builders::{ExternGetBuilder, ExternUpdateBuilder};

/// RAII guard for a `p4tc_obj` configured as EXTERN.
pub(crate) struct ExtObjHandle {
    ptr: *mut p4tc_sys::p4tc_obj,
}

impl ExtObjHandle {
    pub fn new(pipeline: &str, kind: &str, instance: &str, key: u32, params: &[String]) -> Result<Self> {
        let c_pipe = to_cstring(pipeline, "pipeline")?;
        let ptr = unsafe {
            p4tc_sys::p4tc_obj_create(c_pipe.as_ptr(), p4tc_sys::P4TC_OBJ_EXTERN)
        };
        if ptr.is_null() {
            return Err(Error::Object { msg: format!("obj_create failed for '{pipeline}'") });
        }

        let c_kind = to_cstring(kind, "kind")?;
        let c_inst = to_cstring(instance, "instance")?;

        let ext = if params.is_empty() {
            unsafe {
                p4tc_sys::p4tc_create_runt_ext(
                    ptr, c_kind.as_ptr(), c_inst.as_ptr(),
                    key, 0, std::ptr::null(),
                )
            }
        } else {
            let c_params: Vec<CString> = params.iter()
                .map(|p| to_cstring(p, "param"))
                .collect::<Result<_>>()?;
            let ptrs: Vec<*const libc::c_char> = c_params.iter().map(|c| c.as_ptr()).collect();
            unsafe {
                p4tc_sys::p4tc_create_runt_ext(
                    ptr, c_kind.as_ptr(), c_inst.as_ptr(),
                    key, c_params.len() as i32, ptrs.as_ptr(),
                )
            }
        };
        if ext.is_null() {
            unsafe { p4tc_sys::p4tc_obj_destroy(ptr) };
            return Err(Error::Entry { msg: format!("create_runt_ext failed for '{kind}/{instance}'") });
        }
        Ok(Self { ptr })
    }

    pub fn as_ptr(&self) -> *mut p4tc_sys::p4tc_obj {
        self.ptr
    }
}

impl Drop for ExtObjHandle {
    fn drop(&mut self) {
        unsafe { p4tc_sys::p4tc_obj_destroy(self.ptr) }
    }
}

pub(crate) fn parse_ext_obj(obj_ptr: *const p4tc_sys::p4tc_obj) -> Vec<ExternEntry> {
    let mut entries = Vec::new();
    let mut cur = unsafe { p4tc_sys::p4tc_obj_ext_first(obj_ptr) };
    while !cur.is_null() {
        entries.push(parse_ext_entry(cur));
        cur = unsafe { p4tc_sys::p4tc_obj_ext_next(obj_ptr, cur) };
    }
    entries
}

fn parse_ext_entry(ext: *const p4tc_sys::p4tc_runt_ext_attrs) -> ExternEntry {
    let kind = unsafe {
        let p = p4tc_sys::p4tc_runt_ext_attrs_kind_get(ext);
        if p.is_null() { String::new() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
    };
    let instance = unsafe {
        let p = p4tc_sys::p4tc_runt_ext_attrs_inst_get(ext);
        if p.is_null() { String::new() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
    };
    let key = unsafe { p4tc_sys::p4tc_runt_ext_attrs_key_get(ext) };
    let ext_id = unsafe { p4tc_sys::p4tc_runt_ext_attrs_ext_id_get(ext) };
    let inst_id = unsafe { p4tc_sys::p4tc_runt_ext_attrs_inst_id_get(ext) };

    let mut params = Vec::new();
    let mut pcur = unsafe { p4tc_sys::p4tc_runt_ext_attrs_param_first(ext) };
    while !pcur.is_null() {
        params.push(unsafe { crate::table::parse::parse_param(pcur) });
        pcur = unsafe { p4tc_sys::p4tc_runt_ext_attrs_param_next(ext, pcur) };
    }

    ExternEntry { kind, instance, key, ext_id, inst_id, params }
}

#[derive(Debug, Clone)]
pub struct ExternEntry {
    pub kind: String,
    pub instance: String,
    pub key: u32,
    pub ext_id: u32,
    pub inst_id: u32,
    pub params: Vec<Param>,
}
