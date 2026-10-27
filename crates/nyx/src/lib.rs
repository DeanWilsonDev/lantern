#![doc = include_str!("../README.md")]

mod diagnostic;
mod error;
mod host;
mod program;
mod raw;
mod runtime;
mod value;

pub use diagnostic::{Diagnostic, Severity};
pub use error::{CallbackError, CallbackResult, Error, Result, Status};
pub use host::{CallContext, HostType, HostTypeBuilder, Receiver, PANIC_VARIANT};
pub use program::Program;
pub use runtime::Runtime;
pub use value::{Kind, Value};

pub fn capi_version() -> i32 {
    unsafe { nyx_sys::nyx_capi_version() }
}

pub const HEADER_VERSION: i32 = nyx_sys::NYX_CAPI_VERSION as i32;
