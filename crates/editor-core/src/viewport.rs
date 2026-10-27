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
const GRID_MINOR_COLOR: Color = Color::rgb(0.25, 0.28, 0.34);
const GRID_MAJOR_COLOR: Color = Color::rgb(0.38, 0.42, 0.5);
const VECTOR_PROPERTY_COLOR: Color = Color::rgb(0.3, 0.9, 1.0);
const GRID_HALF_EXTENT: i32 = 30;
const GRID_MAJOR_EVERY: i32 = 5;
const PLANE_MARKER_HALF_EXTENT: f32 = 40.0;
const EDITOR_SKY: Color = Color::rgb(0.2, 0.23, 0.29);
const HIDDEN_OUTLINE_COLOR: Color = Color::rgb(0.45, 0.45, 0.5);
const WIREFRAME_COLOR: Color = Color::rgb(0.7, 0.75, 0.85);
const STATIC_COLLIDER_COLOR: Color = Color::rgb(0.3, 0.9, 0.4);
const KINEMATIC_COLLIDER_COLOR: Color = Color::rgb(0.95, 0.6, 0.2);
const LEVEL_BOUNDS_COLOR: Color = Color::rgb(0.9, 0.5, 0.9);
const RUBBER_BAND_COLOR: Color = Color::rgb(0.55, 0.75, 1.0);

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

/// A ground grid on y = 0 centred under the camera's target, so it follows you around a big
/// world without drawing thousands of lines. Every fifth line is brighter, and the world axes
/// are drawn through the origin.
fn draw_grid(render_world: &mut RenderWorld, center: Vec3, spacing: f32) {
    let spacing = spacing.max(0.01);
    let major_every = GRID_MAJOR_EVERY as f32 * spacing;
    let extent = GRID_HALF_EXTENT as f32 * spacing;
    let center_x = (center.x / major_every).round() * major_every;
    let center_z = (center.z / major_every).round() * major_every;
    let is_major = |line: i32| line % GRID_MAJOR_EVERY == 0;
    for offset in -GRID_HALF_EXTENT..=GRID_HALF_EXTENT {
        let color = if is_major(offset) { GRID_MAJOR_COLOR } else { GRID_MINOR_COLOR };
        let x = center_x + offset as f32 * spacing;
        let z = center_z + offset as f32 * spacing;
        if x.abs() > 1e-4 {
            render_world.line(Vec3::new(x, 0.0, center_z - extent), Vec3::new(x, 0.0, center_z + extent), color);
        }
        if z.abs() > 1e-4 {
            render_world.line(Vec3::new(center_x - extent, 0.0, z), Vec3::new(center_x + extent, 0.0, z), color);
        }
    }
    let axis_extent = 1000.0;
    render_world.line(
        Vec3::new(-axis_extent, 0.0, 0.0),
        Vec3::new(axis_extent, 0.0, 0.0),
        crate::gizmo::Axis::X.color(),
    );
    render_world.line(
        Vec3::new(0.0, 0.0, -axis_extent),
        Vec3::new(0.0, 0.0, axis_extent),
        crate::gizmo::Axis::Z.color(),
    );
}

fn draw_plane_marker(render_world: &mut RenderWorld, height: f32, color: Color) {
    // An outlined square with its diagonals: visible from any angle without hiding the level.
    let extent = PLANE_MARKER_HALF_EXTENT;
    let corner = |x: f32, z: f32| Vec3::new(x * extent, height, z * extent);
    let corners = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
    for index in 0..4 {
        render_world.line(corners[index], corners[(index + 1) % 4], color);
    }
    render_world.line(corners[0], corners[2], color);
    render_world.line(corners[1], corners[3], color);
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

/// The selection rectangle, drawn as lines just in front of the camera so it needs no overlay
/// texture.
fn draw_rubber_band(render_world: &mut RenderWorld, session: &EditorSession, corners: (glam::Vec2, glam::Vec2)) {
    let projection = session.projection();
    let depth = projection.camera.near * 4.0;
    let (a, b) = corners;
    let pixels = [a, glam::Vec2::new(b.x, a.y), b, glam::Vec2::new(a.x, b.y)];
    let points = pixels.map(|pixel| projection.ray(pixel).at(depth));
    for index in 0..4 {
        render_world.line(points[index], points[(index + 1) % 4], RUBBER_BAND_COLOR);
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
    let view = session.view.clone();
    if view.show_grid {
        draw_grid(render_world, session.camera.target, view.grid_spacing);
    }

    let selection = session.selection().to_vec();
    let hovered = session.hovered_entity();
    let geometry = session.geometry().clone();
    let world = session.world().clone();
    let palette = session.palette().clone();

    for entity in &world.entities {
        let Some(placed) = geometry.get(entity.id) else { continue };
        let hidden = session.is_hidden(entity.id);
        if hidden && !view.show_hidden {
            continue;
        }
        if entity.shape.is_none() && !view.show_markers {
            continue;
        }
        let selected = selection.contains(&entity.id);
        let outline_color = if selected {
            Some(SELECTION_COLOR)
        } else if hovered == Some(entity.id) {
            Some(HOVER_OUTLINE_COLOR)
        } else if hidden {
            Some(HIDDEN_OUTLINE_COLOR)
        } else if view.wireframe {
            Some(WIREFRAME_COLOR)
        } else {
            None
        };
        let draw_solid = !hidden && !view.wireframe;
        match &entity.shape {
            Some(shape) => {
                if draw_solid {
                    if let Some(mesh) = resources.shape_mesh(shape) {
                        let texture = resources.material_texture(entity.material.as_deref());
                        render_world.draw(mesh, texture, placed.world_matrix, entity.tint_color());
                    }
                }
                if let Some(color) = outline_color {
                    oriented_box_lines(render_world, &shape_local_bounds(shape), &placed.world_matrix, color);
                }
                if view.show_colliders && entity.collider.is_some() && outline_color.is_none() {
                    let color = match entity.collider {
                        Some(lantern_world_format::ColliderKind::Kinematic) => KINEMATIC_COLLIDER_COLOR,
                        _ => STATIC_COLLIDER_COLOR,
                    };
                    oriented_box_lines(render_world, &shape_local_bounds(shape), &placed.world_matrix, color);
                }
            }
            None => {
                let style = palette.marker_style(&entity.kind);
                let color = parse_hex_color(&style.color).unwrap_or(Color::WHITE);
                if style.mesh == MarkerMesh::Plane {
                    draw_plane_marker(render_world, placed.origin().y, color);
                } else if draw_solid {
                    if let Some(mesh) = resources.marker_mesh(style.mesh) {
                        render_world.draw(mesh, None, placed.world_matrix * marker_local_matrix(style.mesh), color);
                    }
                }
                if let Some(outline) = outline_color {
                    oriented_box_lines(render_world, &marker_local_bounds(style.mesh), &placed.world_matrix, outline);
                }
                // A spawn point faces somewhere: show which way, so a level starts facing the course.
                if entity.kind == "spawn" {
                    let origin = placed.origin();
                    let facing = placed.world_matrix.transform_vector3(Vec3::NEG_Z).normalize_or(Vec3::NEG_Z);
                    arrow(render_world, origin + Vec3::Y * 0.2, origin + Vec3::Y * 0.2 + facing * 1.5, color);
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

    if view.show_level_bounds {
        let bounds = geometry.level_bounds();
        if !bounds.is_empty() {
            render_world.aabb_lines(&bounds.expanded(0.05), LEVEL_BOUNDS_COLOR);
        }
    }

    if let Some(corners) = session.rubber_band() {
        draw_rubber_band(render_world, session, corners);
    }

    let tool = session.tool();
    let highlighted = session.hovered_handle();
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

    #[test]
    fn view_options_change_what_is_drawn() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/worlds/course.world.json");
        let world = WorldFile::load(&path).unwrap();
        let first = world.entities[0].id;
        let mut session = EditorSession::new(world, None, Palette::fallback());
        let mut resources = CountingResources::default();
        let mut render_world = RenderWorld::default();
        let all = session.world().entities.len();

        session.set_hidden(first, true);
        build_viewport(&mut session, &mut resources, &mut render_world);
        assert_eq!(render_world.draws.len(), all - 1, "a hidden entity isn't drawn");

        session.set_hidden(first, false);
        session.set_view_option("wireframe", "on");
        session.set_view_option("grid", "off");
        build_viewport(&mut session, &mut resources, &mut render_world);
        assert!(render_world.draws.is_empty(), "wireframe draws outlines only");
        assert!(!render_world.lines.is_empty());
    }
}
