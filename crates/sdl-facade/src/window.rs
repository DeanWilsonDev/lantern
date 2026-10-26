use std::ffi::CString;
use std::rc::Rc;

use lantern_platform::{PlatformBackend, PlatformEvent, WindowConfig, WindowMetrics};
use sdl3_sys::everything as sdl;

use crate::context::{SdlContext, SdlError, SdlInitGuard};
use crate::events;

struct WindowInner {
    raw: *mut sdl::SDL_Window,
    // Declared after `raw` is used in Drop; fields drop after `Drop::drop`, so SDL is still
    // initialised while the window is destroyed.
    _sdl: Rc<SdlInitGuard>,
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        // SAFETY: `raw` came from SDL_CreateWindow and is destroyed exactly once, here. Anything
        // that claimed the window for a GPU device holds an `SdlWindow` clone, so it has already
        // released its claim.
        unsafe { sdl::SDL_DestroyWindow(self.raw) };
    }
}

/// A shared handle to the SDL window. Cloning is cheap; the window closes when the last clone
/// (the platform, the GPU renderer) is dropped.
#[derive(Clone)]
pub struct SdlWindow {
    inner: Rc<WindowInner>,
}

impl SdlWindow {
    pub fn create(context: &SdlContext, config: &WindowConfig) -> Result<SdlWindow, SdlError> {
        let title = CString::new(config.title.replace('\0', " ")).expect("NULs were replaced");
        let mut flags = sdl::SDL_WINDOW_HIGH_PIXEL_DENSITY;
        if config.resizable {
            flags |= sdl::SDL_WINDOW_RESIZABLE;
        }
        // SAFETY: SDL is initialised (we hold its guard); `title` outlives the call, SDL copies it.
        let raw = unsafe { sdl::SDL_CreateWindow(title.as_ptr(), config.width as i32, config.height as i32, flags) };
        if raw.is_null() {
            return Err(SdlError::last("SDL_CreateWindow"));
        }
        let window = SdlWindow { inner: Rc::new(WindowInner { raw, _sdl: context.guard() }) };
        let metrics = window.metrics();
        log::info!(
            target: "platform",
            "window {}x{} points, {}x{} pixels, density {:.2}",
            metrics.logical_width, metrics.logical_height, metrics.pixel_width, metrics.pixel_height, metrics.pixel_density
        );
        Ok(window)
    }

    /// The raw window for `lantern-sdl-gpu`. Valid for as long as any clone of this `SdlWindow`
    /// is alive.
    pub fn raw(&self) -> *mut sdl::SDL_Window {
        self.inner.raw
    }

    pub fn metrics(&self) -> WindowMetrics {
        let (mut logical_width, mut logical_height, mut pixel_width, mut pixel_height) = (0, 0, 0, 0);
        // SAFETY: `raw` is a live window (see `raw`); out-pointers are valid locals.
        let (density, flags) = unsafe {
            sdl::SDL_GetWindowSize(self.inner.raw, &mut logical_width, &mut logical_height);
            sdl::SDL_GetWindowSizeInPixels(self.inner.raw, &mut pixel_width, &mut pixel_height);
            (sdl::SDL_GetWindowPixelDensity(self.inner.raw), sdl::SDL_GetWindowFlags(self.inner.raw))
        };
        WindowMetrics {
            logical_width: logical_width.max(0) as u32,
            logical_height: logical_height.max(0) as u32,
            pixel_width: pixel_width.max(0) as u32,
            pixel_height: pixel_height.max(0) as u32,
            pixel_density: if density > 0.0 { density } else { 1.0 },
            minimized: (flags & sdl::SDL_WINDOW_MINIMIZED) == sdl::SDL_WINDOW_MINIMIZED,
        }
    }

    pub fn set_title(&self, title: &str) {
        let title = CString::new(title.replace('\0', " ")).expect("NULs were replaced");
        // SAFETY: live window; SDL copies the string.
        unsafe { sdl::SDL_SetWindowTitle(self.inner.raw, title.as_ptr()) };
    }
}

struct OpenGamepad {
    raw: *mut sdl::SDL_Gamepad,
    id: sdl::SDL_JoystickID,
}

/// The SDL implementation of `PlatformBackend`: the window, the event pump, the clock and the
/// one gamepad the prototype supports.
pub struct SdlPlatform {
    window: SdlWindow,
    gamepad: Option<OpenGamepad>,
    mouse_captured: bool,
}

impl SdlPlatform {
    pub fn new(window: SdlWindow) -> SdlPlatform {
        SdlPlatform { window, gamepad: None, mouse_captured: false }
    }

    pub fn window(&self) -> &SdlWindow {
        &self.window
    }

    pub fn has_gamepad(&self) -> bool {
        self.gamepad.is_some()
    }

    fn open_gamepad(&mut self, id: sdl::SDL_JoystickID) {
        if self.gamepad.is_some() {
            return;
        }
        // SAFETY: `id` comes from a GAMEPAD_ADDED event, which is what SDL_OpenGamepad expects.
        let raw = unsafe { sdl::SDL_OpenGamepad(id) };
        if raw.is_null() {
            log::warn!(target: "input", "{}", SdlError::last("SDL_OpenGamepad"));
            return;
        }
        log::info!(target: "input", "gamepad connected");
        self.gamepad = Some(OpenGamepad { raw, id });
    }

    fn close_gamepad(&mut self, id: sdl::SDL_JoystickID) -> bool {
        match &self.gamepad {
            Some(gamepad) if gamepad.id == id => {
                // SAFETY: `raw` was opened by SDL_OpenGamepad and is closed once.
                unsafe { sdl::SDL_CloseGamepad(gamepad.raw) };
                self.gamepad = None;
                log::info!(target: "input", "gamepad disconnected");
                true
            }
            _ => false,
        }
    }
}

impl Drop for SdlPlatform {
    fn drop(&mut self) {
        if let Some(gamepad) = self.gamepad.take() {
            // SAFETY: opened by SDL_OpenGamepad, closed once; SDL is alive (we hold the window,
            // which holds the SDL guard).
            unsafe { sdl::SDL_CloseGamepad(gamepad.raw) };
        }
    }
}

impl PlatformBackend for SdlPlatform {
    fn pump_events(&mut self, out: &mut Vec<PlatformEvent>) {
        // SAFETY: SDL_Event is a plain C union; zeroed is a valid "no event" value.
        let mut event: sdl::SDL_Event = unsafe { std::mem::zeroed() };
        // SAFETY: called on the main thread with SDL initialised; `event` is a valid out-pointer.
        while unsafe { sdl::SDL_PollEvent(&mut event) } {
            match events::translate(&event) {
                events::Translated::Event(platform_event) => out.push(platform_event),
                events::Translated::GamepadAdded(id) => {
                    self.open_gamepad(id);
                    out.push(PlatformEvent::GamepadConnected);
                }
                events::Translated::GamepadRemoved(id) => {
                    if self.close_gamepad(id) {
                        out.push(PlatformEvent::GamepadDisconnected);
                    }
                }
                events::Translated::Ignored => {}
            }
        }
    }

    fn seconds_since_start(&self) -> f64 {
        // SAFETY: no preconditions once SDL is initialised.
        unsafe { sdl::SDL_GetTicksNS() as f64 / 1_000_000_000.0 }
    }

    fn window_metrics(&self) -> WindowMetrics {
        self.window.metrics()
    }

    fn set_mouse_captured(&mut self, captured: bool) {
        if captured == self.mouse_captured {
            return;
        }
        // SAFETY: live window.
        if unsafe { sdl::SDL_SetWindowRelativeMouseMode(self.window.raw(), captured) } {
            self.mouse_captured = captured;
        } else {
            log::warn!(target: "input", "{}", SdlError::last("SDL_SetWindowRelativeMouseMode"));
        }
    }
}
