//! Lantern's engine layer (Umbra `src/engine`): the frame loop, input routing, the world of
//! entities and transforms, scenes, the game state machine and render extraction.
//!
//! The engine owns no platform, GPU, audio or script objects. The game executable constructs
//! those and hands the loop a `Game` to call back into.

#![forbid(unsafe_code)]

pub mod audio;
pub mod extract;
pub mod frame_loop;
pub mod input;
pub mod world;
