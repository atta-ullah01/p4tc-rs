use crate::error::{Error, Result};
use crate::ffi_util::to_cstring;
use std::ffi::CString;

/// RAII guard for a `p4tc_obj`. Calls `p4tc_obj_destroy` on drop.
pub(crate) struct ObjHandle {
    ptr: *mut p4tc_sys::p4tc_obj,
}

impl ObjHandle {
    pub fn new(pipeline: &str) -> Result<Self> {
        let c_name = to_cstring(pipeline, "pipeline")?;
        let ptr = unsafe {
            p4tc_sys::p4tc_obj_create(c_name.as_ptr(), p4tc_sys::P4TC_OBJ_TABLE)
        };
        if ptr.is_null() {
            return Err(Error::Object {
                msg: format!("obj_create failed for '{pipeline}'"),
            });
        }
        Ok(Self { ptr })
    }

    pub fn as_ptr(&self) -> *mut p4tc_sys::p4tc_obj {
        self.ptr
    }

    pub fn set_table(&self, table: &str) -> Result<()> {
        let c_table = to_cstring(table, "table")?;
        unsafe { p4tc_sys::p4tc_obj_objname_set(self.ptr, c_table.as_ptr()) };
        Ok(())
    }

    pub fn set_filter(&self, filter: &str) -> Result<()> {
        let c_filter = to_cstring(filter, "filter")?;
        unsafe { p4tc_sys::p4tc_obj_filter_set(self.ptr, c_filter.as_ptr()) };
        Ok(())
    }

    pub fn make_key(&self, values: &[String]) -> Result<*mut p4tc_sys::p4tc_key> {
        let c_vals: Vec<CString> = values
            .iter()
            .map(|v| to_cstring(v, "key value"))
            .collect::<Result<_>>()?;
        let ptrs: Vec<*const libc::c_char> = c_vals.iter().map(|c| c.as_ptr()).collect();

        let key = unsafe {
            p4tc_sys::p4tc_make_key(self.ptr, c_vals.len() as i32, ptrs.as_ptr())
        };
        if key.is_null() {
            return Err(Error::Key { msg: "make_key failed".into() });
        }
        Ok(key)
    }

    pub fn alloc_entry(
        &self,
        key: *mut p4tc_sys::p4tc_key,
        entity: i32,
    ) -> Result<*mut p4tc_sys::p4tc_runt_tbl_attrs> {
        let entry = unsafe {
            p4tc_sys::p4tc_alloc_tbl_entry(self.ptr, key, 0, entity)
        };
        if entry.is_null() {
            return Err(Error::Entry { msg: "alloc_tbl_entry failed".into() });
        }
        Ok(entry)
    }
}

impl Drop for ObjHandle {
    fn drop(&mut self) {
        unsafe { p4tc_sys::p4tc_obj_destroy(self.ptr) }
    }
}

pub(crate) fn attach_action(
    entry: *mut p4tc_sys::p4tc_runt_tbl_attrs,
    path: &str,
    params: &[String],
) -> Result<()> {
    let c_path = to_cstring(path, "action path")?;
    let c_params: Vec<CString> = params
        .iter()
        .map(|p| to_cstring(p, "action param"))
        .collect::<Result<_>>()?;
    let ptrs: Vec<*const libc::c_char> = c_params.iter().map(|c| c.as_ptr()).collect();

    let act = unsafe {
        p4tc_sys::p4tc_create_runt_act(
            entry,
            c_path.as_ptr(),
            c_params.len() as i32,
            ptrs.as_ptr(),
        )
    };
    if act.is_null() {
        return Err(Error::Entry {
            msg: format!("create_runt_act failed for '{path}'"),
        });
    }
    Ok(())
}

pub(crate) fn fire_crud(
    crud_fn: unsafe extern "C" fn(
        *mut p4tc_sys::p4tc_runt_ctx,
        *mut p4tc_sys::p4tc_obj,
        libc::c_uint,
        p4tc_sys::p4tc_callback,
        *mut u64,
    ) -> libc::c_int,
    ctx: &crate::Context,
    obj: &ObjHandle,
    flags: u32,
    op_name: &'static str,
) -> Result<()> {
    let ret = unsafe {
        crud_fn(ctx.as_ptr(), obj.as_ptr(), flags,
                None, std::ptr::null_mut())
    };
    if ret != 0 {
        return Err(Error::Crud {
            op: op_name,
            source: std::io::Error::last_os_error(),
        });
    }
    Ok(())
}

/// Fire a CRUD operation with a per-call callback.
pub(crate) fn fire_crud_with_cb<F: FnMut(crate::types::Phase)>(
    crud_fn: unsafe extern "C" fn(
        *mut p4tc_sys::p4tc_runt_ctx,
        *mut p4tc_sys::p4tc_obj,
        libc::c_uint,
        p4tc_sys::p4tc_callback,
        *mut u64,
    ) -> libc::c_int,
    ctx: &crate::Context,
    obj: &ObjHandle,
    flags: u32,
    op_name: &'static str,
    cb: &mut F,
) -> Result<()> {
    use std::cell::Cell;

    thread_local! {
        static CB_PTR: Cell<usize> = const { Cell::new(0) };
    }

    unsafe extern "C" fn trampoline<F: FnMut(crate::types::Phase)>(
        _obj: *const p4tc_sys::p4tc_obj,
        _ctx: *mut p4tc_sys::p4tc_runt_ctx,
        _cookie: *mut u64,
        phase_val: libc::c_int,
    ) -> libc::c_int {
        let phase = crate::types::Phase::from_raw(phase_val);
        CB_PTR.with(|cell| {
            let ptr = cell.get() as *mut F;
            if !ptr.is_null() {
                unsafe { (*ptr)(phase) };
            }
        });
        match phase {
            crate::types::Phase::Abt => 0,
            _ => 0,
        }
    }

    CB_PTR.with(|cell| cell.set(cb as *mut F as usize));

    let ret = unsafe {
        crud_fn(ctx.as_ptr(), obj.as_ptr(), flags,
                Some(trampoline::<F>), std::ptr::null_mut())
    };

    CB_PTR.with(|cell| cell.set(0));

    if ret != 0 {
        return Err(Error::Crud {
            op: op_name,
            source: std::io::Error::last_os_error(),
        });
    }
    Ok(())
}

#[derive(Default)]
pub(crate) struct EntryAttrs {
    pub priority: u32,
    pub aging_ms: Option<u32>,
    pub profile_id: Option<u32>,
    pub permissions: Option<u32>,
    pub dynamic: Option<bool>,
}

pub(crate) fn apply_entry_attrs(entry: *mut p4tc_sys::p4tc_runt_tbl_attrs, attrs: &EntryAttrs) {
    unsafe {
        if attrs.priority > 0 {
            p4tc_sys::p4tc_runt_tbl_attrs_prio_set(entry, attrs.priority);
        }
        if let Some(v) = attrs.aging_ms {
            p4tc_sys::p4tc_runt_tbl_attrs_aging_set(entry, v);
        }
        if let Some(v) = attrs.profile_id {
            p4tc_sys::p4tc_runt_tbl_attrs_profile_id_set(entry, v);
        }
        if let Some(v) = attrs.permissions {
            p4tc_sys::p4tc_runt_tbl_attrs_perms_set(entry, v);
        }
        if let Some(v) = attrs.dynamic {
            p4tc_sys::p4tc_runt_tbl_attrs_dyn_set(entry, v as i32);
        }
    }
}
