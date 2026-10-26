//! Lantern's core vocabulary (Umbra `src/core`).
//!
//! Everything here is plain data and small algorithms with no platform or GPU dependency, so
//! every other crate can share it and it can be unit tested on any machine.

#![forbid(unsafe_code)]

pub mod color;
pub mod debug;
pub mod handle;
pub mod input;
pub mod logging;
pub mod math;
pub mod time;

pub use glam;
