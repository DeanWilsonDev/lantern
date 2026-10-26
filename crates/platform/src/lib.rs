//! Platform vocabulary (Umbra `src/platform` and `core/window`).
//!
//! This crate says *what* a platform provides: a window with a size and pixel density, a stream
//! of events, a monotonic clock and mouse capture. `lantern-sdl-facade` says *how*, with SDL3.
//! Nothing here names SDL, so the engine loop can be tested with a fake platform.

#![forbid(unsafe_code)]

use lantern_core::input::{GamepadAxis, GamepadButton, KeyCode, MouseButton};

/// What the game asks for when it opens its window (Umbra `WindowConfig`).
#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    /// Size in logical points. On a high-DPI display the drawable has more pixels.
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig { title: "Lantern".to_string(), width: 1280, height: 720, resizable: true, vsync: true }
    }
}

/// The window's current size in both units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowMetrics {
    pub logical_width: u32,
    pub logical_height: u32,
    /// The drawable (swapchain) size in physical pixels. Zero while minimised on some platforms.
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Physical pixels per logical point (2.0 on a Retina display).
    pub pixel_density: f32,
    pub minimized: bool,
}

impl WindowMetrics {
    /// True when there is nothing to draw into: minimised or zero-sized.
    pub fn is_drawable_empty(&self) -> bool {
        self.minimized || self.pixel_width == 0 || self.pixel_height == 0
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.pixel_height == 0 { 1.0 } else { self.pixel_width as f32 / self.pixel_height as f32 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlatformEvent {
    QuitRequested,
    /// The drawable changed size. Read `WindowMetrics` for the new values.
    WindowResized,
    WindowMinimized,
    WindowRestored,
    FocusGained,
    FocusLost,
    KeyDown {
        key: KeyCode,
        repeat: bool,
    },
    KeyUp {
        key: KeyCode,
    },
    /// Relative motion in logical points (+X right, +Y down, as on screen).
    MouseMoved {
        delta_x: f32,
        delta_y: f32,
    },
    MouseButtonDown {
        button: MouseButton,
    },
    MouseButtonUp {
        button: MouseButton,
    },
    MouseWheel {
        delta_y: f32,
    },
    GamepadConnected,
    GamepadDisconnected,
    GamepadButtonDown {
        button: GamepadButton,
    },
    GamepadButtonUp {
        button: GamepadButton,
    },
    /// Sticks in -1..1 with +Y up, triggers in 0..1.
    GamepadAxisMoved {
        axis: GamepadAxis,
        value: f32,
    },
}

/// The engine loop's view of the platform. The only trait between the loop and SDL; it exists
/// so `lantern_engine::frame_loop` can be tested with a scripted clock and event list.
pub trait PlatformBackend {
    /// Appends every event that arrived since the last call.
    fn pump_events(&mut self, events: &mut Vec<PlatformEvent>);
    /// Monotonic seconds since the platform started.
    fn seconds_since_start(&self) -> f64;
    fn window_metrics(&self) -> WindowMetrics;
    /// Hides the cursor and reports relative motion (third-person camera) or releases it (menus).
    fn set_mouse_captured(&mut self, captured: bool);
}
