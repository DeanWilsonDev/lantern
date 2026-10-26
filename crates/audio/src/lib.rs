//! Audio without a device (decision D-006).
//!
//! `Mixer` turns playing sounds into interleaved stereo `f32` samples. The game asks it for
//! however many frames the device queue needs (`lantern_sdl_facade::SdlAudioOutput` does the
//! asking), so this crate never touches SDL and is fully unit-tested.
//!
//! Gameplay and scripts refer to sounds by `SoundHandle` or by name through `SoundBank`, never
//! by SDL type.

#![forbid(unsafe_code)]

pub mod mixer;
pub mod wav;

pub use mixer::{Mixer, SoundBank, SoundHandle, Volumes};
pub use wav::{SoundData, WavError};
