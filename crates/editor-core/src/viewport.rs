//! What the viewport shows, as a `RenderWorld`: every entity's mesh, marker meshes for
//! shapeless entities, the ground grid, selection and hover outlines, vector properties drawn
//! as arrows, and the gizmo.
//!
//! GPU resources are behind [`ViewportResources`], so this module (and its tests) never
//! touch a GPU; the backend implements the trait with real uploads.

use glam::{Mat4, Vec3};
use lantern_core::color::Color;
use lantern_core::math::Aabb;
use lantern_renderer_3d::{MeshHandle, RenderWorld, TextureHandle};
use lantern_world_format::{PropertyValue, Shape, parse_hex_color};

use crate::palette::MarkerMesh;
use crate::session::EditorSession;

pub trait ViewportResources {
    /// The mesh for a shape (cached by the implementation; called every frame).
    fn shape_mesh(&mut self, shape: &Shape) -> Option<MeshHandle>;
    /// The surface texture for a material; `None` draws untextured.
    fn material_texture(&mut self, material: Option<&str>) -> Option<TextureHandle>;
    /// The mesh for a marker, built as [`marker_local_matrix`] expects.
    fn marker_mesh(&mut self, mesh: MarkerMesh) -> Option<MeshHandle>;
}

pub const SELECTION_COLOR: Color = Color::rgb(1.0, 0.78, 0.2);
pub const HOVER_OUTLINE_COLOR: Color = Color::rgb(0.85, 0.9, 1.0);
const GRID_MINOR_COLOR: Color = Color::rgb(0.33, 0.36, 0.42);
const GRID_MAJOR_COLOR: Color = Color::rgb(0.5, 0.54, 0.62);
const VECTOR_PROPERTY_COLOR: Color = Color::rgb(0.3, 0.9, 1.0);
const GRID_HALF_EXTENT: i32 = 60;
const GRID_MAJOR_EVERY: i32 = 5;
const PLANE_MARKER_HALF_EXTENT: f32 = 40.0;
const EDITOR_SKY: Color = Color::rgb(0.2, 0.23, 0.29);

/// Marker meshes are built centred on the origin; this places them on the entity.
pub fn marker_local_matrix(mesh: MarkerMesh) -> Mat4 {
    match mesh {
        // The capsule is 1.8 m tall and centred, so lift it to stand on the origin.
        MarkerMesh::Capsule => Mat4::from_translation(Vec3::Y * 0.9),
        _ => Mat4::IDENTITY,
    }
}

/// The local bounds of a marker, for outlines.
pub fn marker_local_bounds(mesh: MarkerMesh) -> Aabb {
    match mesh {
        MarkerMesh::Capsule => Aabb::new(Vec3::new(-0.35, 0.0, -0.35), Vec3::new(0.35, 1.8, 0.35)),
        MarkerMesh::Gem => Aabb::from_center_half_extents(Vec3::ZERO, Vec3::new(0.3, 0.4, 0.3)),
        MarkerMesh::Sphere | MarkerMesh::Cube => Aabb::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.3)),
        MarkerMesh::Plane => Aabb::new(Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 0.0, 1.0)),
    }
}

fn shape_local_bounds(shape: &Shape) -> Aabb {
    match *shape {
        Shape::Box { half_extents } => Aabb::from_center_half_extents(Vec3::ZERO, Vec3::from_array(half_extents)),
        Shape::Ramp { width, height, length } => {
            Aabb::new(Vec3::new(-width * 0.5, 0.0, -length * 0.5), Vec3::new(width * 0.5, height, length * 0.5))
        }
    }
}

/// The twelve edges of `bounds` after `matrix`: an oriented outline.
pub fn oriented_box_lines(render_world: &mut RenderWorld, bounds: &Aabb, matrix: &Mat4, color: Color) {
    let corner = |index: usize| {
        matrix.transform_point3(Vec3::new(
            if index & 1 == 0 { bounds.min.x } else { bounds.max.x },
            if index & 2 == 0 { bounds.min.y } else { bounds.max.y },
            if index & 4 == 0 { bounds.min.z } else { bounds.max.z },
        ))
    };
    for (start, end) in [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)]
    {
        render_world.line(corner(start), corner(end), color);
    }
}

fn draw_grid(render_world: &mut RenderWorld) {
    let extent = GRID_HALF_EXTENT as f32;
    for line in -GRID_HALF_EXTENT..=GRID_HALF_EXTENT {
        let offset = line as f32;
        let (x_color, z_color) = if line == 0 {
            (crate::gizmo::Axis::Z.color(), crate::gizmo::Axis::X.color())
        } else if line % GRID_MAJOR_EVERY == 0 {
            (GRID_MAJOR_COLOR, GRID_MAJOR_COLOR)
        } else {
            (GRID_MINOR_COLOR, GRID_MINOR_COLOR)
        };
        // Lines along Z at x = offset (the x = 0 one is the Z axis), and along X at z = offset.
        render_world.line(Vec3::new(offset, 0.0, -extent), Vec3::new(offset, 0.0, extent), x_color);
        render_world.line(Vec3::new(-extent, 0.0, offset), Vec3::new(extent, 0.0, offset), z_color);
    }
}

fn draw_plane_marker(render_world: &mut RenderWorld, height: f32, color: Color) {
    let extent = PLANE_MARKER_HALF_EXTENT;
    let mut offset = -extent;
    while offset <= extent + 0.01 {
        render_world.line(Vec3::new(offset, height, -extent), Vec3::new(offset, height, extent), color);
        render_world.line(Vec3::new(-extent, height, offset), Vec3::new(extent, height, offset), color);
        offset += 10.0;
    }
}

fn arrow(render_world: &mut RenderWorld, start: Vec3, end: Vec3, color: Color) {
    render_world.line(start, end, color);
    let direction = (end - start).normalize_or_zero();
    if direction == Vec3::ZERO {
        return;
    }
    let side = direction.any_orthonormal_vector();
    let head = ((end - start).length() * 0.15).min(0.4);
    for offset in [side, -side, direction.cross(side), -direction.cross(side)] {
        render_world.line(end, end - direction * head + offset * head * 0.4, color);
    }
}

/// Fills `render_world` with the editor's view of the session's world.
pub fn build_viewport(
    session: &mut EditorSession,
    resources: &mut dyn ViewportResources,
    render_world: &mut RenderWorld,
) {
    render_world.clear();
    render_world.camera = session.camera.render_camera();
    render_world.clear_color = EDITOR_SKY;
    render_world.fog_start = 150.0;
    render_world.fog_end = 500.0;
    draw_grid(render_world);

    let selection = session.selection().to_vec();
    let hovered = session.hovered_entity();
    let geometry = session.geometry().clone();
    let world = session.world().clone();
    let palette = session.palette().clone();

    for entity in &world.entities {
        let Some(placed) = geometry.get(entity.id) else { continue };
        let selected = selection.contains(&entity.id);
        let outline_color = if selected {
            Some(SELECTION_COLOR)
        } else if hovered == Some(entity.id) {
            Some(HOVER_OUTLINE_COLOR)
        } else {
            None
        };
        match &entity.shape {
            Some(shape) => {
                if let Some(mesh) = resources.shape_mesh(shape) {
                    let texture = resources.material_texture(entity.material.as_deref());
                    render_world.draw(mesh, texture, placed.world_matrix, entity.tint_color());
                }
                if let Some(color) = outline_color {
                    oriented_box_lines(render_world, &shape_local_bounds(shape), &placed.world_matrix, color);
                }
            }
            None => {
                let style = palette.marker_style(&entity.kind);
                let color = parse_hex_color(&style.color).unwrap_or(Color::WHITE);
                if style.mesh == MarkerMesh::Plane {
                    draw_plane_marker(render_world, placed.origin().y, color);
                } else if let Some(mesh) = resources.marker_mesh(style.mesh) {
                    render_world.draw(mesh, None, placed.world_matrix * marker_local_matrix(style.mesh), color);
                }
                if let Some(outline) = outline_color {
                    oriented_box_lines(render_world, &marker_local_bounds(style.mesh), &placed.world_matrix, outline);
                }
            }
        }
        if selected {
            // Vector properties (a platform's travel, say) are shown as offsets from the entity.
            for value in entity.properties.values() {
                if let PropertyValue::Vector(offset) = value {
                    let start = placed.origin();
                    let end = start + Vec3::from_array(*offset);
                    arrow(render_world, start, end, VECTOR_PROPERTY_COLOR);
                    if let Some(shape) = &entity.shape {
                        let moved = Mat4::from_translation(Vec3::from_array(*offset)) * placed.world_matrix;
                        oriented_box_lines(render_world, &shape_local_bounds(shape), &moved, VECTOR_PROPERTY_COLOR);
                    }
                }
            }
        }
    }

    let tool = session.tool();
    let highlighted = session.hovered_axis();
    if let Some(frame) = session.visible_gizmo() {
        frame.draw(tool, highlighted, render_world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Palette;
    use lantern_core::handle::Handle;
    use lantern_world_format::WorldFile;

    #[derive(Default)]
    struct CountingResources {
        shape_requests: usize,
        marker_requests: usize,
    }

    impl ViewportResources for CountingResources {
        fn shape_mesh(&mut self, _shape: &Shape) -> Option<MeshHandle> {
            self.shape_requests += 1;
            Some(Handle::from_bits(1))
        }
        fn material_texture(&mut self, _material: Option<&str>) -> Option<TextureHandle> {
            None
        }
        fn marker_mesh(&mut self, _mesh: MarkerMesh) -> Option<MeshHandle> {
            self.marker_requests += 1;
            Some(Handle::from_bits(2))
        }
    }

    #[test]
    fn the_course_draws_every_shape_and_marker() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/worlds/course.world.json");
        let world = WorldFile::load(&path).unwrap();
        let shapes = world.entities.iter().filter(|entity| entity.shape.is_some()).count();
        let mut session = EditorSession::new(world, None, Palette::fallback());
        let mut resources = CountingResources::default();
        let mut render_world = RenderWorld::default();
        build_viewport(&mut session, &mut resources, &mut render_world);
        assert_eq!(resources.shape_requests, shapes);
        // Fallback palette: every shapeless entity is a cube marker.
        assert_eq!(render_world.draws.len(), session.world().entities.len());
        assert!(!render_world.lines.is_empty(), "the grid is drawn");
    }
}
