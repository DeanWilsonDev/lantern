//! Debug HUD and debug drawing.
//!
//! - `DebugHud` draws a `DebugValues` list as a text panel, grouped by the first path segment
//!   (`"Player/Velocity"` goes under "Player").
//! - `debug_draw` adds wireframe shapes to a `RenderWorld`'s line list: capsules, triangle
//!   edges and axes. Lines are depth-tested, so they read correctly in the scene.
//!
//! Neither knows about gameplay or physics types; callers pass plain geometry and values.

#![forbid(unsafe_code)]

pub mod debug_draw;
mod hud;

pub use hud::DebugHud;
