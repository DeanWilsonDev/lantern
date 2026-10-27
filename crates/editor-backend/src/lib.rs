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
mod resources;

use std::path::{Path, PathBuf};

use glam::Vec2;
use lantern_editor_core::{EditorSession, Palette, build_viewport};
use lantern_renderer_3d::RenderWorld;
use lantern_sdl_gpu::{GpuOptions, GpuRenderer};
use lantern_world_format::WorldFile;

pub use ffi::*;
use resources::GpuViewportResources;

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
        let session = EditorSession::open(&path, palette.clone()).unwrap_or_else(|error| {
            log::warn!(target: "editor", "{error}");
            let mut session = EditorSession::new(WorldFile::new("untitled"), None, palette);
            session.new_world("untitled");
            session
        });
        EditorBackend {
            session,
            assets_directory: assets_directory.to_path_buf(),
            renderer: None,
            renderer_error: None,
            resources: GpuViewportResources::default(),
            render_world: RenderWorld::default(),
            pixels: Vec::new(),
            pixel_size: (0, 0),
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
        renderer
            .render_offscreen(&self.render_world, width, height, &mut self.pixels)
            .map_err(|error| error.to_string())?;
        self.pixel_size = (width, height);
        Ok(())
    }

    /// The last rendered viewport: RGBA8, sRGB, top row first, `width * height * 4` bytes.
    pub fn pixels(&self) -> (&[u8], u32, u32) {
        (&self.pixels, self.pixel_size.0, self.pixel_size.1)
    }
}
