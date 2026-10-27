//! GPU resources for the viewport: meshes per shape, marker meshes and material textures,
//! created on first use and kept while they are used.

use std::collections::HashMap;
use std::path::Path;

use lantern_core::color::Color;
use lantern_editor_core::{MarkerMesh, ViewportResources};
use lantern_renderer_3d::gltf_loader::load_gltf_mesh;
use lantern_renderer_3d::texture::TextureData;
use lantern_renderer_3d::{MeshData, MeshHandle, TextureHandle, primitives};
use lantern_sdl_gpu::GpuRenderer;
use lantern_world_format::Shape;
use lantern_world_format::shape_mesh;

/// A shape mesh unused for this many frames is released.
const UNUSED_FRAMES_BEFORE_RELEASE: u64 = 600;

/// A shape's exact bits, so equal shapes share one mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ShapeKey {
    Box([u32; 3]),
    Ramp([u32; 3]),
}

impl ShapeKey {
    fn of(shape: &Shape) -> ShapeKey {
        match *shape {
            Shape::Box { half_extents } => ShapeKey::Box(half_extents.map(f32::to_bits)),
            Shape::Ramp { width, height, length } => ShapeKey::Ramp([width, height, length].map(f32::to_bits)),
        }
    }
}

#[derive(Default)]
pub struct GpuViewportResources {
    shapes: HashMap<ShapeKey, (MeshHandle, u64)>,
    markers: HashMap<MarkerMesh, MeshHandle>,
    /// `None` remembers a material that failed, so it is not retried every frame.
    textures: HashMap<String, Option<TextureHandle>>,
    frame: u64,
}

impl GpuViewportResources {
    /// Starts a frame: releases meshes nothing has drawn for a while (shapes edited away).
    pub fn prepare(&mut self, renderer: &mut GpuRenderer, _assets: &Path) {
        self.frame += 1;
        let frame = self.frame;
        self.shapes.retain(|_, (mesh, last_used)| {
            let keep = frame - *last_used < UNUSED_FRAMES_BEFORE_RELEASE;
            if !keep {
                renderer.remove_mesh(*mesh);
            }
            keep
        });
    }

    pub fn bind<'frame>(
        &'frame mut self,
        renderer: &'frame mut GpuRenderer,
        assets: &'frame Path,
    ) -> FrameResources<'frame> {
        FrameResources { cache: self, renderer, assets }
    }
}

pub struct FrameResources<'frame> {
    cache: &'frame mut GpuViewportResources,
    renderer: &'frame mut GpuRenderer,
    assets: &'frame Path,
}

fn upload(renderer: &mut GpuRenderer, mesh: &MeshData, what: &str) -> Option<MeshHandle> {
    renderer
        .create_mesh(mesh)
        .map_err(|error| log::error!(target: "editor", "uploading the {what} mesh failed: {error}"))
        .ok()
}

impl ViewportResources for FrameResources<'_> {
    fn shape_mesh(&mut self, shape: &Shape) -> Option<MeshHandle> {
        let key = ShapeKey::of(shape);
        let frame = self.cache.frame;
        if let Some((mesh, last_used)) = self.cache.shapes.get_mut(&key) {
            *last_used = frame;
            return Some(*mesh);
        }
        let mesh = upload(self.renderer, &shape_mesh(shape), "shape")?;
        self.cache.shapes.insert(key, (mesh, frame));
        Some(mesh)
    }

    fn material_texture(&mut self, material: Option<&str>) -> Option<TextureHandle> {
        let material = material?;
        if let Some(texture) = self.cache.textures.get(material) {
            return *texture;
        }
        let path = self.assets.join("textures").join(format!("{material}.png"));
        let data = TextureData::load_png(&path).unwrap_or_else(|error| {
            log::warn!(target: "editor", "material '{material}': {error}; using a checker");
            TextureData::checker(64, 8, Color::rgb(0.8, 0.8, 0.8), Color::rgb(0.55, 0.55, 0.6))
        });
        let texture = self
            .renderer
            .create_texture(&data)
            .map_err(|error| log::error!(target: "editor", "uploading '{material}' failed: {error}"))
            .ok();
        self.cache.textures.insert(material.to_string(), texture);
        texture
    }

    fn marker_mesh(&mut self, kind: MarkerMesh) -> Option<MeshHandle> {
        if let Some(mesh) = self.cache.markers.get(&kind) {
            return Some(*mesh);
        }
        let data = match kind {
            MarkerMesh::Cube => primitives::cube(0.6),
            MarkerMesh::Sphere => primitives::sphere(0.3, 16, 8),
            // The game's own gem model when present, so the editor shows what the game shows.
            MarkerMesh::Gem => load_gltf_mesh(&self.assets.join("models/gem.glb")).unwrap_or_else(|error| {
                log::warn!(target: "editor", "gem model: {error}; using the generated gem");
                primitives::gem(0.3, 0.8)
            }),
            // 0.35 m radius and 1.8 m tall, centred; `marker_local_matrix` stands it up.
            MarkerMesh::Capsule => primitives::capsule(0.35, 0.55, 16, 8),
            MarkerMesh::Plane => return None,
        };
        let mesh = upload(self.renderer, &data, "marker")?;
        self.cache.markers.insert(kind, mesh);
        Some(mesh)
    }
}
