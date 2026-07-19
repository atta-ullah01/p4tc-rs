use crate::error::{Error, Result};
use std::ffi::CString;

pub(crate) fn to_cstring(s: &str, what: &'static str) -> Result<CString> {
    CString::new(s).map_err(|_| Error::Object {
        msg: format!("{what} contains null byte"),
    })
}
