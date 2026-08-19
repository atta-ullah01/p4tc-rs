use super::entry::{Action, DecodedValue, Param, TableEntry};
use std::collections::HashMap;
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

pub(crate) unsafe fn parse_param(p: *const p4tc_sys::p4tc_runt_param_attrs) -> Param {
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

/// Decode raw key field bytes using the schema type.
fn decode_key_field(raw: &[u8], ty: &str, bitwidth: u32) -> DecodedValue {
    match ty.to_lowercase().as_str() {
        "ipv4" if raw.len() >= 4 => {
            DecodedValue::Ipv4(format!("{}.{}.{}.{}",
                raw[0], raw[1], raw[2], raw[3]))
        }
        "ipv6" if raw.len() >= 16 => {
            let groups: Vec<String> = raw[..16].chunks(2)
                .map(|c| format!("{:02x}{:02x}", c[0], c.get(1).copied().unwrap_or(0)))
                .collect();
            DecodedValue::Ipv6(groups.join(":"))
        }
        "macaddr" if raw.len() >= 6 => {
            DecodedValue::Mac(format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5]))
        }
        "dev" if raw.len() >= 4 => {
            let mut buf = [0u8; 4];
            buf[..4].copy_from_slice(&raw[..4]);
            DecodedValue::Int(u32::from_le_bytes(buf) as u64)
        }
        _ => {
            // Generic integer, big-endian
            let byte_len = ((bitwidth + 7) / 8).max(1) as usize;
            let slice = &raw[..byte_len.min(raw.len())];
            let mut val: u64 = 0;
            for &b in slice {
                val = (val << 8) | b as u64;
            }
            DecodedValue::Int(val)
        }
    }
}

unsafe fn parse_entry(
    e: *const p4tc_sys::p4tc_runt_tbl_attrs,
    pipeline: &str,
    table: &str,
) -> TableEntry {
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

    let key_bytes = unsafe { ptr_to_bytes(key_ptr, key_len) };

    // C library returns empty table_name in callbacks; use caller's.
    let c_table_name = unsafe {
        ptr_to_string(p4tc_sys::p4tc_runt_tbl_attrs_name_get(e))
    };
    let effective_table = if c_table_name.is_empty() {
        table.to_owned()
    } else {
        c_table_name
    };

    // Decode key bytes using the schema.
    let mut key_fields = HashMap::new();
    #[cfg(feature = "schema")]
    {
        if !key_bytes.is_empty() && !pipeline.is_empty() && !effective_table.is_empty() {
            if let Some(tbl_schema) = crate::schema::get_table_schema(pipeline, &effective_table) {
                let mut offset = 0usize;
                for kf in &tbl_schema.key_fields {
                    let byte_len = ((kf.bitwidth + 7) / 8).max(1) as usize;
                    if offset + byte_len <= key_bytes.len() {
                        let chunk = &key_bytes[offset..offset + byte_len];
                        key_fields.insert(
                            kf.name.clone(),
                            decode_key_field(chunk, &kf.ty, kf.bitwidth),
                        );
                    }
                    offset += byte_len;
                }
            }
        }
    }
    if key_fields.is_empty() && !key_bytes.is_empty() {
        key_fields.insert("raw".to_owned(), DecodedValue::Raw(key_bytes.clone()));
    }

    TableEntry {
        table_name: effective_table,
        priority: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_prio_get(e) },
        key: key_bytes,
        key_size: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_keysz_get(e) },
        key_fields,
        mask,
        permissions: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_perms_get(e) },
        dynamic: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_dyn_get(e) } != 0,
        aging_ms: unsafe { p4tc_sys::p4tc_runt_tbl_attrs_aging_get(e) },
        actions,
    }
}

pub(crate) unsafe fn parse_obj(
    obj: *const p4tc_sys::p4tc_obj,
    pipeline: &str,
    table: &str,
) -> Vec<TableEntry> {
    let mut entries = Vec::new();
    let mut e = unsafe { p4tc_sys::p4tc_obj_tbl_entry_first(obj) };
    while !e.is_null() {
        entries.push(unsafe { parse_entry(e, pipeline, table) });
        e = unsafe { p4tc_sys::p4tc_obj_tbl_entry_next(obj, e) };
    }
    entries
}
