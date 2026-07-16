// TODO: Context::new() + Drop + builder entry points (wraps p4tc_runt_ctx)

pub struct Context {
    _inner: *mut p4tc_sys::p4tc_runt_ctx,
}
