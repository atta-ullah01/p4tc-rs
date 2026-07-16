// TODO: Pipeline::provision() + Drop (wraps p4tc_pipe_config)

pub struct Pipeline {
    _inner: *mut p4tc_sys::p4tc_pipe_config,
}
