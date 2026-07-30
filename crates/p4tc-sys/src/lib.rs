//! Raw FFI bindings to `libp4tctrl.so`.
//!
//! Exposes the C API as-is. All functions are `unsafe`.
//! Use the safe `p4tc` crate for application code.

#![allow(non_camel_case_types)]

use libc::{c_char, c_int, c_uint};



#[repr(C)]
pub struct p4tc_pipe_config { _opaque: [u8; 0] }
#[repr(C)]
pub struct p4tc_runt_ctx { _opaque: [u8; 0] }
#[repr(C)]
pub struct p4tc_obj { _opaque: [u8; 0] }
#[repr(C)]
pub struct p4tc_key { _opaque: [u8; 0] }
#[repr(C)]
pub struct p4tc_runt_tbl_attrs { _opaque: [u8; 0] }
#[repr(C)]
pub struct p4tc_runt_act_attrs { _opaque: [u8; 0] }
#[repr(C)]
pub struct p4tc_runt_ext_attrs { _opaque: [u8; 0] }
#[repr(C)]
pub struct p4tc_runt_param_attrs { _opaque: [u8; 0] }



pub type p4tc_callback = Option<
    unsafe extern "C" fn(
        obj: *const p4tc_obj,
        ctx: *mut p4tc_runt_ctx,
        cookie: *mut u64,
        trans_phase: c_int,
    ) -> c_int,
>;



pub const P4TC_TRANSPORT_NETLINK: c_int = 1;
pub const P4TC_OBJ_TABLE: c_int = 1;
pub const P4TC_OBJ_EXTERN: c_int = 2;
pub const P4TC_ENTITY_KERNEL: c_int = 1;



unsafe extern "C" {
    // init / destroy
    pub fn p4tc_init() -> c_int;
    pub fn p4tc_destroy();

    // provisioning
    pub fn p4tc_provision(pname: *const c_char, dir: *const c_char) -> *mut p4tc_pipe_config;
    pub fn p4tc_pipe_config_destroy(config: *mut p4tc_pipe_config);

    // runtime context
    pub fn p4tc_runt_ctx_create(tml_type: c_int) -> *mut p4tc_runt_ctx;
    pub fn p4tc_runt_ctx_destroy(ctx: *mut p4tc_runt_ctx);

    // object construction
    pub fn p4tc_obj_create(pname: *const c_char, obj_type: c_int) -> *mut p4tc_obj;
    pub fn p4tc_obj_destroy(obj: *mut p4tc_obj);
    pub fn p4tc_obj_objname_set(obj: *mut p4tc_obj, name: *const c_char) -> c_int;
    pub fn p4tc_obj_filter_set(obj: *mut p4tc_obj, filter: *const c_char) -> c_int;

    // key
    pub fn p4tc_make_key(obj: *mut p4tc_obj, n: c_int, kfs: *const *const c_char) -> *mut p4tc_key;

    // table entry
    pub fn p4tc_alloc_tbl_entry(
        obj: *mut p4tc_obj, key: *mut p4tc_key, flags: c_uint, entity: c_int,
    ) -> *mut p4tc_runt_tbl_attrs;
    pub fn p4tc_runt_tbl_attrs_prio_set(e: *mut p4tc_runt_tbl_attrs, v: c_uint) -> c_int;

    // action
    pub fn p4tc_create_runt_act(
        entry: *mut p4tc_runt_tbl_attrs, path: *const c_char,
        n_params: c_int, params: *const *const c_char,
    ) -> *mut p4tc_runt_act_attrs;

    // CRUD
    pub fn p4tc_create(
        ctx: *mut p4tc_runt_ctx, obj: *mut p4tc_obj,
        flags: c_uint, cb: p4tc_callback, cookie: *mut u64,
    ) -> c_int;
    pub fn p4tc_update(
        ctx: *mut p4tc_runt_ctx, obj: *mut p4tc_obj,
        flags: c_uint, cb: p4tc_callback, cookie: *mut u64,
    ) -> c_int;
    pub fn p4tc_get(
        ctx: *mut p4tc_runt_ctx, obj: *mut p4tc_obj,
        flags: c_uint, cb: p4tc_callback, cookie: *mut u64,
    ) -> c_int;
    pub fn p4tc_del(
        ctx: *mut p4tc_runt_ctx, obj: *mut p4tc_obj,
        flags: c_uint, cb: p4tc_callback, cookie: *mut u64,
    ) -> c_int;
    pub fn p4tc_resp_handle(ctx: *mut p4tc_runt_ctx) -> c_int;

    // obj iterators
    pub fn p4tc_obj_tbl_entry_first(obj: *const p4tc_obj) -> *const p4tc_runt_tbl_attrs;
    pub fn p4tc_obj_tbl_entry_next(e: *const p4tc_runt_tbl_attrs) -> *const p4tc_runt_tbl_attrs;

    // table entry getters
    pub fn p4tc_runt_tbl_attrs_name_get(e: *const p4tc_runt_tbl_attrs) -> *const c_char;
    pub fn p4tc_runt_tbl_attrs_prio_get(e: *const p4tc_runt_tbl_attrs) -> c_uint;
    pub fn p4tc_runt_tbl_attrs_key_get(e: *const p4tc_runt_tbl_attrs, len: *mut c_uint) -> *const libc::c_void;
    pub fn p4tc_runt_tbl_attrs_mask_get(e: *const p4tc_runt_tbl_attrs, len: *mut c_uint) -> *const libc::c_void;
    pub fn p4tc_runt_tbl_attrs_keysz_get(e: *const p4tc_runt_tbl_attrs) -> c_uint;
    pub fn p4tc_runt_tbl_attrs_perms_get(e: *const p4tc_runt_tbl_attrs) -> c_uint;
    pub fn p4tc_runt_tbl_attrs_dyn_get(e: *const p4tc_runt_tbl_attrs) -> c_int;
    pub fn p4tc_runt_tbl_attrs_aging_get(e: *const p4tc_runt_tbl_attrs) -> c_uint;

    // table entry → action iterators
    pub fn p4tc_runt_tbl_attrs_act_first(e: *const p4tc_runt_tbl_attrs) -> *const p4tc_runt_act_attrs;
    pub fn p4tc_runt_tbl_attrs_act_next(a: *const p4tc_runt_act_attrs) -> *const p4tc_runt_act_attrs;

    // action getters
    pub fn p4tc_runt_act_attrs_name_get(a: *const p4tc_runt_act_attrs) -> *const c_char;
    pub fn p4tc_runt_act_attrs_index_get(a: *const p4tc_runt_act_attrs) -> c_uint;

    // action → param iterators
    pub fn p4tc_runt_act_attrs_param_first(a: *const p4tc_runt_act_attrs) -> *const p4tc_runt_param_attrs;
    pub fn p4tc_runt_act_attrs_param_next(p: *const p4tc_runt_param_attrs) -> *const p4tc_runt_param_attrs;

    // param getters
    pub fn p4tc_runt_param_attrs_name_get(p: *const p4tc_runt_param_attrs) -> *const c_char;
    pub fn p4tc_runt_param_attrs_type_name_get(p: *const p4tc_runt_param_attrs) -> *const c_char;
    pub fn p4tc_runt_param_attrs_value_get(p: *const p4tc_runt_param_attrs, len: *mut c_uint) -> *const libc::c_void;
}
