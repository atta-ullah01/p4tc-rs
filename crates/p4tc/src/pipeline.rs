use crate::error::{Error, Result};
use crate::ffi_util::to_cstring;
use std::ptr::NonNull;

pub struct Pipeline {
    inner: NonNull<p4tc_sys::p4tc_pipe_config>,
    name: String,
}

impl Pipeline {
    pub fn provision(name: &str, template_dir: Option<&str>) -> Result<Self> {
        let c_name = to_cstring(name, "pipeline name")?;
        let c_dir = template_dir.map(|d| to_cstring(d, "template dir")).transpose()?;

        let ptr = unsafe {
            p4tc_sys::p4tc_provision(
                c_name.as_ptr(),
                c_dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
            )
        };

        NonNull::new(ptr)
            .map(|inner| Self { inner, name: name.to_owned() })
            .ok_or_else(|| Error::Provision {
                pipeline: name.to_owned(),
                source: std::io::Error::last_os_error(),
            })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe { p4tc_sys::p4tc_pipe_config_destroy(self.inner.as_ptr()) }
    }
}
