use super::entry::{Action, Param, TableEntry};
use std::ffi::CStr;

unsafe fn ptr_to_string(p: *const libc::c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

unsafe fn ptr_to_bytes(p: *const libc::c_void, len: u32) -> Vec<u8> {
    if p.is_null() || len == 0 {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(p as *const u8, len as usize) }.to_vec()
}

unsafe fn parse_param(p: *const p4tc_sys::p4tc_runt_param_attrs) -> Param {
    let mut vlen: u32 = 0;
    let vptr = unsafe { p4tc_sys::p4tc_runt_param_attrs_value_get(p, &mut vlen) };

    Param {
        name: unsafe { ptr_to_string(p4tc_sys::p4tc_runt_param_attrs_name_get(p)) },
        type_name: unsafe { ptr_to_string(p4tc_sys::p4tc_runt_param_attrs_type_name_get(p)) },
        value: unsafe { ptr_to_bytes(vptr, vlen) },
        size: vlen as usize,
    }
}

unsafe fn parse_action(a: *const p4tc_sys::p4tc_runt_act_attrs) -> Action {
    let mut params = Vec::new();
    let mut p = unsafe { p4tc_sys::p4tc_runt_act_attrs_param_first(a) };
    while !p.is_null() {
        params.push(unsafe { parse_param(p) });
        p = unsafe { p4tc_sys::p4tc_runt_act_attrs_param_next(a, p) };
    }

    Action {
        name: unsafe { ptr_to_string(p4tc_sys::p4tc_runt_act_attrs_name_get(a)) },
        index: unsafe { p4tc_sys::p4tc_runt_act_attrs_index_get(a) },
        params,
    }
}

unsafe fn parse_entry(e: *const p4tc_sys::p4tc_runt_tbl_attrs) -> TableEntry {
    let mut key_len: u32 = 0;
    let key_ptr = unsafe { p4tc_sys::p4tc_runt_tbl_attrs_key_get(e, &mut key_len) };
    let mut mask_len: u32 = 0;
    let mask_ptr = unsafe { p4tc_sys::p4tc_runt_tbl_attrs_mask_get(e, &mut mask_len) };

    let mask = if mask_ptr.is_null() || mask_len == 0 {
        None
    } else {
        Some(unsafe { ptr_to_bytes(mask_ptr, mask_len) })
    };

    let mut actions = Vec::new();
    let mut a = unsafe { p4tc_sys::p4tc_runt_tbl_attrs_act_first(e) };
    while !a.is_null() {
        actions.push(unsafe { parse_action(a) });
        a = unsafe { p4tc_sys::p4tc_runt_tbl_attrs_act_next(e, a) };
    }

    TableEntry {
        table_name: unsafe { ptr_to_string(p4tc_sys::p4tc_runt_tbl_attrs_name_get(e)) },
        priority: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_prio_get(e) },
        key: unsafe { ptr_to_bytes(key_ptr, key_len) },
        key_size: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_keysz_get(e) },
        mask,
        permissions: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_perms_get(e) },
        dynamic: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_dyn_get(e) } != 0,
        aging_ms: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_aging_get(e) },
        actions,
    }
}

pub(crate) unsafe fn parse_obj(obj: *const p4tc_sys::p4tc_obj) -> Vec<TableEntry> {
    let mut entries = Vec::new();
    let mut e = unsafe { p4tc_sys::p4tc_obj_tbl_entry_first(obj) };
    while !e.is_null() {
        entries.push(unsafe { parse_entry(e) });
        e = unsafe { p4tc_sys::p4tc_obj_tbl_entry_next(obj, e) };
    }
    entries
}
