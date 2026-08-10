//! Safe Rust bindings for the P4TC runtime control library.

mod context;
mod error;
mod extern_;
mod ffi_util;
mod pipeline;
mod subscribe;
mod table;
mod types;

#[cfg(feature = "schema")]
mod schema;

pub use context::Context;
pub use error::{Error, Result};
pub use extern_::{ExternEntry, ExternInsertBuilder, ExternUpdateBuilder, ExternDeleteBuilder, ExternGetBuilder};
pub use pipeline::Pipeline;
pub use subscribe::Subscription;
pub use table::{Action, DeleteBuilder, GetBuilder, InsertBuilder, Param, TableEntry, UpdateBuilder};
pub use types::{Entity, MsgFlags, ObjType, Phase, Policy, Transport};

#[cfg(feature = "schema")]
pub use schema::PipelineSchema;
