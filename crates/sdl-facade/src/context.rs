use std::ffi::CStr;
use std::fmt;
use std::rc::Rc;

use sdl3_sys::everything as sdl;

/// An SDL call failed. Holds `SDL_GetError()` and what we were doing.
#[derive(Debug, Clone)]
pub struct SdlError {
    pub context: String,
    pub message: String,
}

impl SdlError {
    /// Captures SDL's last error message for a failed call.
    pub fn last(context: impl Into<String>) -> SdlError {
        // SAFETY: SDL_GetError returns a pointer to a NUL-terminated thread-local string that SDL
        // keeps alive until the next SDL call on this thread; we copy it before returning.
        let message = unsafe { CStr::from_ptr(sdl::SDL_GetError()) }.to_string_lossy().into_owned();
        SdlError { context: context.into(), message }
    }
}

impl fmt::Display for SdlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.context, self.message)
    }
}

impl std::error::Error for SdlError {}

/// Calls `SDL_Quit` when the last holder lets go.
pub(crate) struct SdlInitGuard;

impl Drop for SdlInitGuard {
    fn drop(&mut self) {
        log::info!(target: "platform", "shutting SDL down");
        // SAFETY: every SDL object (window, gamepad, audio stream, GPU device) holds an
        // `Rc<SdlInitGuard>`, so this runs after all of them have been destroyed.
        unsafe { sdl::SDL_Quit() };
    }
}

/// Initialised SDL video, events and gamepad support (audio is opened on demand).
pub struct SdlContext {
    pub(crate) guard: Rc<SdlInitGuard>,
}

impl SdlContext {
    pub fn init() -> Result<SdlContext, SdlError> {
        // SAFETY: SDL_SetHint copies both strings; they are static C strings.
        unsafe {
            sdl::SDL_SetHint(sdl::SDL_HINT_APP_NAME, c"Lantern".as_ptr());
        }
        let flags = sdl::SDL_INIT_VIDEO | sdl::SDL_INIT_EVENTS | sdl::SDL_INIT_GAMEPAD;
        // SAFETY: SDL_Init may be called from the main thread at any time; failure is reported
        // through the return value.
        if !unsafe { sdl::SDL_Init(flags) } {
            return Err(SdlError::last("SDL_Init"));
        }
        let guard = Rc::new(SdlInitGuard);
        log::info!(
            target: "platform",
            "SDL {} initialised, video driver: {}",
            version_string(),
            current_video_driver().unwrap_or_else(|| "none".to_string())
        );
        Ok(SdlContext { guard })
    }

    /// Opens the audio subsystem. Called by the audio output; harmless to call twice.
    pub fn init_audio(&self) -> Result<(), SdlError> {
        // SAFETY: as for SDL_Init; subsystems are reference-counted by SDL and released by SDL_Quit.
        if !unsafe { sdl::SDL_InitSubSystem(sdl::SDL_INIT_AUDIO) } {
            return Err(SdlError::last("SDL_InitSubSystem(AUDIO)"));
        }
        Ok(())
    }

    pub(crate) fn guard(&self) -> Rc<SdlInitGuard> {
        self.guard.clone()
    }
}

fn version_string() -> String {
    // SDL_GetVersion is declared safe by sdl3-sys: it has no preconditions.
    let version = sdl::SDL_GetVersion();
    format!("{}.{}.{}", version / 1_000_000, (version / 1000) % 1000, version % 1000)
}

fn current_video_driver() -> Option<String> {
    // SAFETY: returns NULL or a static NUL-terminated string owned by SDL.
    let pointer = unsafe { sdl::SDL_GetCurrentVideoDriver() };
    if pointer.is_null() {
        return None;
    }
    // SAFETY: checked non-null above; SDL keeps the string alive while video is initialised.
    Some(unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned())
}
