//! What the game host needs from a renderer: create and release GPU resources, draw a
//! `RenderWorld`. `GpuRenderer` (SDL GPU) is the real one; `HeadlessRenderer` hands out handles
//! and draws nothing, so a whole Nyx game can run in a test without a window or a GPU.

use std::cell::RefCell;
use std::rc::Rc;

use lantern_core::handle::Pool;
use lantern_renderer_3d::{MeshData, MeshHandle, RenderWorld, TextureData, TextureHandle};
use lantern_sdl_gpu::{FrameReport, GpuRenderer};

pub trait RenderBackend {
    fn driver_name(&self) -> &str;
    fn create_mesh(&mut self, mesh: &MeshData) -> Result<MeshHandle, String>;
    fn remove_mesh(&mut self, mesh: MeshHandle);
    fn create_texture(&mut self, texture: &TextureData) -> Result<TextureHandle, String>;
    fn remove_texture(&mut self, texture: TextureHandle);
    fn render(&mut self, world: &RenderWorld) -> Result<FrameReport, String>;
    /// Renders `world` off-screen and reads it back (smoke-test captures).
    fn capture(&mut self, world: &RenderWorld, width: u32, height: u32) -> Result<TextureData, String>;
}

impl RenderBackend for GpuRenderer {
    fn driver_name(&self) -> &str {
        GpuRenderer::driver_name(self)
    }

    fn create_mesh(&mut self, mesh: &MeshData) -> Result<MeshHandle, String> {
        GpuRenderer::create_mesh(self, mesh).map_err(|error| error.to_string())
    }

    fn remove_mesh(&mut self, mesh: MeshHandle) {
        GpuRenderer::remove_mesh(self, mesh);
    }

    fn create_texture(&mut self, texture: &TextureData) -> Result<TextureHandle, String> {
        GpuRenderer::create_texture(self, texture).map_err(|error| error.to_string())
    }

    fn remove_texture(&mut self, texture: TextureHandle) {
        GpuRenderer::remove_texture(self, texture);
    }

    fn render(&mut self, world: &RenderWorld) -> Result<FrameReport, String> {
        GpuRenderer::render(self, world).map_err(|error| error.to_string())
    }

    fn capture(&mut self, world: &RenderWorld, width: u32, height: u32) -> Result<TextureData, String> {
        GpuRenderer::capture(self, world, width, height).map_err(|error| error.to_string())
    }
}

/// What a [`HeadlessRenderer`] has seen, shared with the test that made it (the renderer itself
/// is owned by the engine state).
#[derive(Debug, Default)]
pub struct HeadlessStats {
    pub live_meshes: usize,
    pub live_textures: usize,
    pub frames_rendered: u64,
    pub last_draw_count: usize,
}

/// Keeps track of live meshes and textures, so tests can check that a reload or a restart
/// releases everything it created.
#[derive(Default)]
pub struct HeadlessRenderer {
    meshes: Pool<()>,
    textures: Pool<()>,
    stats: Rc<RefCell<HeadlessStats>>,
}

impl HeadlessRenderer {
    pub fn stats(&self) -> Rc<RefCell<HeadlessStats>> {
        self.stats.clone()
    }

    fn update_live_counts(&self) {
        let mut stats = self.stats.borrow_mut();
        stats.live_meshes = self.meshes.len();
        stats.live_textures = self.textures.len();
    }
}

impl RenderBackend for HeadlessRenderer {
    fn driver_name(&self) -> &str {
        "headless"
    }

    fn create_mesh(&mut self, _mesh: &MeshData) -> Result<MeshHandle, String> {
        let handle = self.meshes.insert(()).cast();
        self.update_live_counts();
        Ok(handle)
    }

    fn remove_mesh(&mut self, mesh: MeshHandle) {
        self.meshes.remove(mesh.cast());
        self.update_live_counts();
    }

    fn create_texture(&mut self, _texture: &TextureData) -> Result<TextureHandle, String> {
        let handle = self.textures.insert(()).cast();
        self.update_live_counts();
        Ok(handle)
    }

    fn remove_texture(&mut self, texture: TextureHandle) {
        self.textures.remove(texture.cast());
        self.update_live_counts();
    }

    fn render(&mut self, world: &RenderWorld) -> Result<FrameReport, String> {
        let mut stats = self.stats.borrow_mut();
        stats.frames_rendered += 1;
        stats.last_draw_count = world.draws.len();
        Ok(FrameReport { presented: true, draws_submitted: world.draws.len() as u32, ..FrameReport::default() })
    }

    fn capture(&mut self, _world: &RenderWorld, width: u32, height: u32) -> Result<TextureData, String> {
        let _ = (width, height);
        Err("the headless renderer cannot capture".to_string())
    }
}
