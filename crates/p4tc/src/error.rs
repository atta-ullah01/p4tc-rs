use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("library init failed")]
    Init,
    #[error("provision '{pipeline}': {source}")]
    Provision { pipeline: String, source: io::Error },
    #[error("context: {0}")]
    Context(io::Error),
    #[error("object: {msg}")]
    Object { msg: String },
    #[error("key: {msg}")]
    Key { msg: String },
    #[error("entry: {msg}")]
    Entry { msg: String },
    #[error("{op}: {source}")]
    Crud { op: &'static str, source: io::Error },
    #[error("subscribe: {0}")]
    Subscribe(io::Error),
    #[error("schema: {0}")]
    Schema(String),
}

pub type Result<T> = std::result::Result<T, Error>;
