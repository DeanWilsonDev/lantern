//! The world editor's Rust backend.
//!
//! [`EditorBackend`] owns one [`EditorSession`] (all editor logic, from `lantern-editor-core`)
//! and an off-screen [`GpuRenderer`] that draws the viewport into memory. The front end
//! (Penumbra, Chaos, Cosmos and Nyx, in `apps/world-editor`) reaches it through the C API in
//! `ffi.rs` / `include/lantern_editor.h`:
//!
//! ```text
//! Nyx/Chaos panels ──host functions──► C++ shell ──C API──► EditorBackend ──► EditorSession
//!                                          │                       │
//!   viewport widget ◄──RGBA pixels─────────┘                       └──► GpuRenderer (off-screen)
//! ```
//!
//! The GPU is opened lazily on the first `render`, so everything else (opening, editing,
//! saving, the panels) works and is testable without a GPU.

mod ffi;
mod ffi_tools;
pub mod play;
pub mod preview;
mod resources;

use std::path::{Path, PathBuf};
use std::time::Instant;

use glam::Vec2;
use lantern_editor_core::{EditorPreferences, EditorSession, LogLevel, Palette, build_viewport};
use lantern_renderer_3d::RenderWorld;
use lantern_sdl_gpu::{FrameReport, GpuOptions, GpuRenderer};
use lantern_world_format::WorldFile;

pub use ffi::*;
pub use ffi_tools::*;
use play::PlaySession;
use preview::PreviewState;
use resources::GpuViewportResources;

/// Numbers the status bar and the stats overlay show.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderStats {
    pub last_render_milliseconds: f32,
    pub frames_per_second: f32,
    pub draws_submitted: u32,
    pub draws_culled: u32,
    pub triangles: u64,
    pub lines: u32,
    last_render_at: Option<Instant>,
}

impl RenderStats {
    fn record(&mut self, started: Instant, report: &FrameReport) {
        let now = Instant::now();
        self.last_render_milliseconds = (now - started).as_secs_f32() * 1000.0;
        if let Some(previous) = self.last_render_at {
            let seconds = (now - previous).as_secs_f32().max(1e-4);
            let instant_fps = 1.0 / seconds;
            self.frames_per_second = if self.frames_per_second == 0.0 {
                instant_fps
            } else {
                self.frames_per_second * 0.9 + instant_fps * 0.1
            };
        }
        self.last_render_at = Some(now);
        self.draws_submitted = report.draws_submitted;
        self.draws_culled = report.draws_culled;
        self.triangles = report.triangles;
        self.lines = report.line_count;
    }

    /// One statistic by name, as text.
    pub fn get(&self, name: &str) -> Option<String> {
        Some(match name {
            "render_ms" => format!("{:.2}", self.last_render_milliseconds),
            "fps" => format!("{:.0}", self.frames_per_second),
            "draws" => self.draws_submitted.to_string(),
            "culled" => self.draws_culled.to_string(),
            "triangles" => self.triangles.to_string(),
            "lines" => self.lines.to_string(),
            _ => return None,
        })
    }
}

pub struct EditorBackend {
    pub session: EditorSession,
    assets_directory: PathBuf,
    renderer: Option<GpuRenderer>,
    /// Set when opening the GPU failed, so we do not retry every frame.
    renderer_error: Option<String>,
    resources: GpuViewportResources,
    render_world: RenderWorld,
    pixels: Vec<u8>,
    pixel_size: (u32, u32),
    pub stats: RenderStats,
    play: Option<PlaySession>,
    preview: PreviewState,
    pub preferences: EditorPreferences,
    preferences_path: Option<PathBuf>,
}

impl EditorBackend {
    /// Opens `world_path`, or the demo course when `None`, with the palette from
    /// `<assets>/editor/palette.json`. A world that fails to load leaves an empty world and the
    /// reason in the status line, so the editor still opens.
    pub fn new(assets_directory: &Path, world_path: Option<&Path>) -> EditorBackend {
        let palette_path = assets_directory.join("editor/palette.json");
        let palette = Palette::load(&palette_path).unwrap_or_else(|error| {
            log::warn!(target: "editor", "palette not loaded, using the generic one: {error}");
            Palette::fallback()
        });
        let default_world = assets_directory.join("worlds/course.world.json");
        let path = world_path.map(Path::to_path_buf).unwrap_or(default_world);
        let mut session = EditorSession::open(&path, palette.clone()).unwrap_or_else(|error| {
            log::warn!(target: "editor", "{error}");
            let mut session = EditorSession::new(WorldFile::new("untitled"), None, palette);
            session.new_world("untitled");
            session.note(LogLevel::Error, error);
            session
        });
        session.set_palette_path(Some(palette_path));
        let preferences_path = EditorPreferences::default_path();
        let preferences = preferences_path
            .as_deref()
            .filter(|path| path.is_file())
            .map(|path| {
                EditorPreferences::load(path).unwrap_or_else(|error| {
                    log::warn!(target: "editor", "preferences not loaded: {error}");
                    EditorPreferences::default()
                })
            })
            .unwrap_or_default();
        let mut backend = EditorBackend {
            session,
            assets_directory: assets_directory.to_path_buf(),
            renderer: None,
            renderer_error: None,
            resources: GpuViewportResources::default(),
            render_world: RenderWorld::default(),
            pixels: Vec::new(),
            pixel_size: (0, 0),
            stats: RenderStats::default(),
            play: None,
            preview: PreviewState::default(),
            preferences,
            preferences_path,
        };
        backend.apply_preferences();
        backend
    }

    /// Restores what the session keeps from the preferences: recent files, snapping, view
    /// options, the camera projection.
    fn apply_preferences(&mut self) {
        let recent = self.preferences.list("recent_files").into_iter().map(PathBuf::from).collect();
        self.session.set_recent_files(recent);
        if let Some(enabled) = self.preferences.get("snap.enabled") {
            self.session.set_snap_enabled(enabled == "true");
        }
        for step in ["translate", "rotate", "scale"] {
            if let Some(value) = self.preferences.get(&format!("snap.{step}")).and_then(|text| text.parse().ok()) {
                self.session.set_snap_step(step, value);
            }
        }
        for name in lantern_editor_core::ViewOptions::NAMES {
            if let Some(value) = self.preferences.get(&format!("view.{name}")) {
                self.session.set_view_option(name, value);
            }
        }
    }

    /// Writes the preferences, with the session's own state folded in.
    pub fn save_preferences(&mut self) -> bool {
        let recent: Vec<String> = self.session.recent_files().iter().map(|path| path.display().to_string()).collect();
        self.preferences.set_list("recent_files", &recent);
        self.preferences.set("snap.enabled", &self.session.snap.enabled.to_string());
        for step in ["translate", "rotate", "scale"] {
            if let Some(value) = self.session.snap_step(step) {
                self.preferences.set(&format!("snap.{step}"), &value.to_string());
            }
        }
        for name in lantern_editor_core::ViewOptions::NAMES {
            if let Some(value) = self.session.view.get(name) {
                self.preferences.set(&format!("view.{name}"), &value);
            }
        }
        let Some(path) = self.preferences_path.clone() else { return false };
        match self.preferences.save(&path) {
            Ok(()) => true,
            Err(error) => {
                self.session.note(LogLevel::Warning, format!("Preferences not saved: {error}"));
                false
            }
        }
    }

    pub fn assets_directory(&self) -> &Path {
        &self.assets_directory
    }

    fn renderer(&mut self) -> Result<&mut GpuRenderer, String> {
        if self.renderer.is_none() {
            if let Some(error) = &self.renderer_error {
                return Err(error.clone());
            }
            match GpuRenderer::new_offscreen(&GpuOptions::from_environment()) {
                Ok(renderer) => self.renderer = Some(renderer),
                Err(error) => {
                    let message = format!("viewport GPU unavailable: {error}");
                    log::error!(target: "editor", "{message}");
                    self.renderer_error = Some(message.clone());
                    return Err(message);
                }
            }
        }
        Ok(self.renderer.as_mut().expect("created above"))
    }

    /// Draws the viewport at `width` x `height` pixels. The result is in [`Self::pixels`].
    pub fn render(&mut self, width: u32, height: u32) -> Result<(), String> {
        let started = Instant::now();
        let (width, height) = (width.clamp(1, 8192), height.clamp(1, 8192));
        self.session.set_viewport_size(Vec2::new(width as f32, height as f32));
        // Split the borrows: the renderer, the resource cache and the session are separate fields.
        self.renderer()?;
        let renderer = self.renderer.as_mut().expect("opened above");
        self.resources.prepare(renderer, &self.assets_directory);
        build_viewport(
            &mut self.session,
            &mut self.resources.bind(renderer, &self.assets_directory),
            &mut self.render_world,
        );
        let report = renderer
            .render_offscreen(&self.render_world, width, height, &mut self.pixels)
            .map_err(|error| error.to_string())?;
        self.pixel_size = (width, height);
        self.stats.record(started, &report);
        Ok(())
    }

    /// The last rendered viewport: RGBA8, sRGB, top row first, `width * height * 4` bytes.
    pub fn pixels(&self) -> (&[u8], u32, u32) {
        (&self.pixels, self.pixel_size.0, self.pixel_size.1)
    }
}
