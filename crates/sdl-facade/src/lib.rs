//! SDL3 adapters for the platform: window, events, input and audio.
//!
//! This crate and `lantern-sdl-gpu` are the only places that call SDL. Every `unsafe` block
//! states the invariant that makes it sound. Other crates see `lantern_platform` types only.
//!
//! Ownership: `SdlContext` initialises SDL. Windows and the audio output hold a shared
//! `SdlInitGuard`, so `SDL_Quit` runs only after the last SDL object is gone, whatever order the
//! game drops things in.

mod audio;
mod context;
mod events;
mod window;

pub use audio::SdlAudioOutput;
pub use context::{SdlContext, SdlError};
pub use window::{SdlPlatform, SdlWindow};

/// Raw SDL types for `lantern-sdl-gpu`, the other crate allowed to talk to SDL directly.
pub mod raw {
    pub use sdl3_sys;
}
