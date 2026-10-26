//! SDL3 audio output: one playback stream on the default device.

use std::rc::Rc;

use lantern_platform::AudioSink;
use sdl3_sys::everything as sdl;

use crate::context::{SdlContext, SdlError, SdlInitGuard};

const CHANNEL_COUNT: usize = 2;

/// An open playback stream. SDL converts our `f32` stereo to whatever the device wants and
/// follows default-device changes (for example headphones being plugged in) by itself.
pub struct SdlAudioOutput {
    stream: *mut sdl::SDL_AudioStream,
    sample_rate: u32,
    _sdl: Rc<SdlInitGuard>,
}

impl SdlAudioOutput {
    /// Opens the default playback device. Errors are for the caller to log; the game keeps
    /// running silently without audio (there may be no device at all, as in CI).
    pub fn open(context: &SdlContext, sample_rate: u32) -> Result<SdlAudioOutput, SdlError> {
        context.init_audio()?;
        let spec =
            sdl::SDL_AudioSpec { format: sdl::SDL_AUDIO_F32, channels: CHANNEL_COUNT as i32, freq: sample_rate as i32 };
        // SAFETY: the audio subsystem is initialised; `spec` outlives the call and SDL copies it.
        // No callback is registered, so SDL never calls into Rust from its audio thread.
        let stream = unsafe {
            sdl::SDL_OpenAudioDeviceStream(sdl::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &spec, None, std::ptr::null_mut())
        };
        if stream.is_null() {
            return Err(SdlError::last("SDL_OpenAudioDeviceStream"));
        }
        // SAFETY: `stream` was just opened and is valid. Devices opened through
        // SDL_OpenAudioDeviceStream start paused.
        if !unsafe { sdl::SDL_ResumeAudioStreamDevice(stream) } {
            let error = SdlError::last("SDL_ResumeAudioStreamDevice");
            // SAFETY: `stream` is valid and is not used again.
            unsafe { sdl::SDL_DestroyAudioStream(stream) };
            return Err(error);
        }
        log::info!(target: "audio", "audio output open: {sample_rate} Hz stereo f32");
        Ok(SdlAudioOutput { stream, sample_rate, _sdl: context.guard() })
    }
}

impl AudioSink for SdlAudioOutput {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn queued_frames(&self) -> usize {
        // SAFETY: `self.stream` is valid until drop.
        let queued_bytes = unsafe { sdl::SDL_GetAudioStreamQueued(self.stream) };
        queued_bytes.max(0) as usize / (CHANNEL_COUNT * std::mem::size_of::<f32>())
    }

    fn push_frames(&mut self, interleaved_stereo: &[f32]) {
        let byte_length = std::mem::size_of_val(interleaved_stereo);
        // SAFETY: `self.stream` is valid; SDL copies `byte_length` bytes from the slice before
        // returning.
        let pushed =
            unsafe { sdl::SDL_PutAudioStreamData(self.stream, interleaved_stereo.as_ptr().cast(), byte_length as i32) };
        if !pushed {
            log::warn!(target: "audio", "{}", SdlError::last("SDL_PutAudioStreamData"));
        }
    }
}

impl Drop for SdlAudioOutput {
    fn drop(&mut self) {
        // SAFETY: `self.stream` is valid and is destroyed exactly once; destroying a stream
        // opened with SDL_OpenAudioDeviceStream also closes its device. SDL is still initialised
        // because we hold `_sdl`, which drops after this body.
        unsafe { sdl::SDL_DestroyAudioStream(self.stream) };
    }
}
