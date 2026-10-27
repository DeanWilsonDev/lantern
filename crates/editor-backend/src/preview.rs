//! Template previews: a palette template drawn on its own, as a thumbnail for the asset
//! browser or as a turntable in the preview viewport.
//!
//! Thumbnails are rendered once per template and palette revision and kept; the preview is
//! redrawn every frame it is shown, turning slowly.

use std::collections::HashMap;

use glam::Vec3;
use lantern_core::color::Color;
use lantern_core::math::Aabb;
use lantern_editor_core::palette::{EntityTemplate, MarkerMesh};
use lantern_editor_core::viewport::{marker_local_bounds, marker_local_matrix, oriented_box_lines};
use lantern_editor_core::{Palette, ViewportResources};
use lantern_renderer_3d::{Camera, RenderWorld};
use lantern_world_format::{Shape, parse_hex_color};

use crate::EditorBackend;

const PREVIEW_SKY: Color = Color::rgb(0.13, 0.15, 0.2);
const PREVIEW_GRID_COLOR: Color = Color::rgb(0.28, 0.31, 0.38);
const PREVIEW_OUTLINE_COLOR: Color = Color::rgb(0.6, 0.65, 0.75);

#[derive(Default)]
pub struct PreviewState {
    pub render_world: RenderWorld,
    pub pixels: Vec<u8>,
    pub size: (u32, u32),
    /// Thumbnails by template id, with the palette revision they were drawn for.
    pub thumbnails: HashMap<String, Thumbnail>,
}

pub struct Thumbnail {
    pub pixels: Vec<u8>,
    pub size: u32,
    pub palette_revision: u64,
}

fn shape_local_bounds(shape: &Shape) -> Aabb {
    match *shape {
        Shape::Box { half_extents } => Aabb::from_center_half_extents(Vec3::ZERO, Vec3::from_array(half_extents)),
        Shape::Ramp { width, height, length } => {
            Aabb::new(Vec3::new(-width * 0.5, 0.0, -length * 0.5), Vec3::new(width * 0.5, height, length * 0.5))
        }
    }
}

/// Fills `render_world` with one template, seen from a three-quarter angle turned by `spin`
/// radians.
pub fn build_template_preview(
    template: &EntityTemplate,
    palette: &Palette,
    resources: &mut dyn ViewportResources,
    render_world: &mut RenderWorld,
    spin: f32,
) {
    render_world.clear();
    render_world.clear_color = PREVIEW_SKY;
    render_world.fog_start = 1000.0;
    render_world.fog_end = 2000.0;

    let bounds = match &template.shape {
        Some(shape) => shape_local_bounds(shape),
        None => marker_local_bounds(palette.marker_style(&template.kind).mesh),
    };
    let bounds = if bounds.is_empty() { Aabb::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.5)) } else { bounds };
    let radius = bounds.half_extents().length().max(0.3);
    let center = bounds.center();
    let orbit = Vec3::new((spin + 0.7).sin(), 0.65, (spin + 0.7).cos()).normalize();
    let mut camera = Camera::look_at(center + orbit * radius * 2.6, center);
    camera.vertical_fov_radians = 45f32.to_radians();
    camera.near = 0.05;
    camera.far = 200.0;
    render_world.camera = camera;

    let floor_y = bounds.min.y - 0.01;
    let step = (radius * 0.5).max(0.25);
    for index in -4..=4 {
        let offset = index as f32 * step;
        render_world.line(
            Vec3::new(center.x + offset, floor_y, center.z - 4.0 * step),
            Vec3::new(center.x + offset, floor_y, center.z + 4.0 * step),
            PREVIEW_GRID_COLOR,
        );
        render_world.line(
            Vec3::new(center.x - 4.0 * step, floor_y, center.z + offset),
            Vec3::new(center.x + 4.0 * step, floor_y, center.z + offset),
            PREVIEW_GRID_COLOR,
        );
    }

    match &template.shape {
        Some(shape) => {
            if let Some(mesh) = resources.shape_mesh(shape) {
                let texture = resources.material_texture(template.material.as_deref());
                let tint = template.tint.as_deref().and_then(parse_hex_color).unwrap_or(Color::WHITE);
                render_world.draw(mesh, texture, glam::Mat4::IDENTITY, tint);
            }
            oriented_box_lines(render_world, &bounds, &glam::Mat4::IDENTITY, PREVIEW_OUTLINE_COLOR);
        }
        None => {
            let style = palette.marker_style(&template.kind);
            let color = parse_hex_color(&style.color).unwrap_or(Color::WHITE);
            if style.mesh == MarkerMesh::Plane {
                let corners = [
                    Vec3::new(-1.0, 0.0, -1.0),
                    Vec3::new(1.0, 0.0, -1.0),
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(-1.0, 0.0, 1.0),
                ];
                for index in 0..4 {
                    render_world.line(corners[index], corners[(index + 1) % 4], color);
                }
            } else if let Some(mesh) = resources.marker_mesh(style.mesh) {
                render_world.draw(mesh, None, marker_local_matrix(style.mesh), color);
            }
        }
    }
}

impl EditorBackend {
    /// Draws `template_id` at `width` x `height`, turned by `spin` radians, into the preview
    /// pixels.
    pub fn render_template_preview(
        &mut self,
        template_id: &str,
        width: u32,
        height: u32,
        spin: f32,
    ) -> Result<(), String> {
        let (width, height) = (width.clamp(1, 4096), height.clamp(1, 4096));
        let Some(template) = self.session.palette().template(template_id).cloned() else {
            return Err(format!("no template '{template_id}'"));
        };
        let palette = self.session.palette().clone();
        self.renderer()?;
        let renderer = self.renderer.as_mut().expect("opened above");
        let mut render_world = std::mem::take(&mut self.preview.render_world);
        build_template_preview(
            &template,
            &palette,
            &mut self.resources.bind(renderer, &self.assets_directory),
            &mut render_world,
            spin,
        );
        let result = renderer
            .render_offscreen(&render_world, width, height, &mut self.preview.pixels)
            .map(|_| ())
            .map_err(|error| error.to_string());
        self.preview.render_world = render_world;
        self.preview.size = (width, height);
        result
    }

    /// The last preview: RGBA8, top row first.
    pub fn preview_pixels(&self) -> (&[u8], u32, u32) {
        (&self.preview.pixels, self.preview.size.0, self.preview.size.1)
    }

    /// A square thumbnail of `template_id`, drawn once per palette revision and kept.
    pub fn thumbnail(&mut self, template_id: &str, size: u32) -> Result<(&[u8], u32), String> {
        let size = size.clamp(8, 512);
        let revision = self.session.palette_revision();
        let fresh = self
            .preview
            .thumbnails
            .get(template_id)
            .is_some_and(|thumbnail| thumbnail.palette_revision == revision && thumbnail.size == size);
        if !fresh {
            self.render_template_preview(template_id, size, size, 0.0)?;
            let pixels = std::mem::take(&mut self.preview.pixels);
            self.preview
                .thumbnails
                .insert(template_id.to_string(), Thumbnail { pixels, size, palette_revision: revision });
        }
        let thumbnail = self.preview.thumbnails.get(template_id).expect("inserted above");
        Ok((&thumbnail.pixels, thumbnail.size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_core::handle::Handle;
    use lantern_renderer_3d::{MeshHandle, TextureHandle};

    struct StubResources;

    impl ViewportResources for StubResources {
        fn shape_mesh(&mut self, _shape: &Shape) -> Option<MeshHandle> {
            Some(Handle::from_bits(1))
        }
        fn material_texture(&mut self, _material: Option<&str>) -> Option<TextureHandle> {
            None
        }
        fn marker_mesh(&mut self, _mesh: MarkerMesh) -> Option<MeshHandle> {
            Some(Handle::from_bits(2))
        }
    }

    #[test]
    fn a_template_preview_draws_the_template_and_looks_at_it() {
        let palette = Palette::fallback();
        let mut render_world = RenderWorld::default();
        for template in &palette.templates {
            build_template_preview(template, &palette, &mut StubResources, &mut render_world, 0.3);
            assert_eq!(render_world.draws.len(), 1, "{}", template.id);
            assert!(!render_world.lines.is_empty());
            let towards = (Vec3::ZERO - render_world.camera.position).normalize();
            assert!(towards.dot(render_world.camera.forward) > 0.9, "{}", template.id);
        }
    }
}
