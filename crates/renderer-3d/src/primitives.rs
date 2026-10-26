//! Generated meshes: the "start with generated primitives" step, and the fallback when an asset
//! file is missing. All have outward normals and counter-clockwise front faces (tested below).

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3, Vec4};

use crate::mesh::{MeshData, Vertex};

const WHITE: Vec4 = Vec4::ONE;

/// Adds a quad given its centre, outward normal, "right" and "up" axes as seen from outside
/// (`right × up` must equal `normal`), half sizes and UV scale.
fn push_quad(mesh: &mut MeshData, center: Vec3, normal: Vec3, right: Vec3, up: Vec3, half_size: Vec2, uv_size: Vec2) {
    let base = mesh.vertices.len() as u32;
    let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    for (sign_right, sign_up) in corners {
        let position = center + right * half_size.x * sign_right + up * half_size.y * sign_up;
        // Texture v grows downward, so the top edge of the face gets v = 0.
        let uv = Vec2::new((sign_right + 1.0) * 0.5 * uv_size.x, (1.0 - sign_up) * 0.5 * uv_size.y);
        mesh.vertices.push(Vertex::new(position, normal, uv, WHITE));
    }
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// `(normal, right, up)` for the six faces of a box, each with `right × up = normal`.
const BOX_FACES: [(Vec3, Vec3, Vec3); 6] = [
    (Vec3::X, Vec3::NEG_Z, Vec3::Y),
    (Vec3::NEG_X, Vec3::Z, Vec3::Y),
    (Vec3::Y, Vec3::X, Vec3::NEG_Z),
    (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    (Vec3::Z, Vec3::X, Vec3::Y),
    (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
];

fn component(vector: Vec3, axis: Vec3) -> f32 {
    vector.dot(axis.abs())
}

/// A box centred on the origin. With `world_uvs`, UVs are in metres so a texture tiles once per
/// metre whatever the box size (used for level geometry); otherwise each face spans 0..1.
pub fn box_mesh(half_extents: Vec3, world_uvs: bool) -> MeshData {
    let mut mesh = MeshData::default();
    for (normal, right, up) in BOX_FACES {
        let half_size = Vec2::new(component(half_extents, right), component(half_extents, up));
        let uv_size = if world_uvs { half_size * 2.0 } else { Vec2::ONE };
        push_quad(&mut mesh, normal * component(half_extents, normal), normal, right, up, half_size, uv_size);
    }
    mesh
}

/// A cube with edge length `size`.
pub fn cube(size: f32) -> MeshData {
    box_mesh(Vec3::splat(size * 0.5), false)
}

/// A flat plane on y = 0 facing +Y, UVs in metres.
pub fn plane(width: f32, depth: f32) -> MeshData {
    let mut mesh = MeshData::default();
    let half_size = Vec2::new(width * 0.5, depth * 0.5);
    push_quad(&mut mesh, Vec3::ZERO, Vec3::Y, Vec3::X, Vec3::NEG_Z, half_size, half_size * 2.0);
    mesh
}

/// A wedge: `width` along X, `length` along Z, rising from height 0 at +Z to `height` at -Z.
/// Its base is on y = 0. UVs are in metres.
pub fn ramp(width: f32, height: f32, length: f32) -> MeshData {
    let (w, l) = (width * 0.5, length * 0.5);
    let mut mesh = MeshData::default();
    let mut push_polygon = |corners: &[Vec3], uvs: &[Vec2]| {
        let normal = (corners[1] - corners[0]).cross(corners[2] - corners[0]).normalize();
        let base = mesh.vertices.len() as u32;
        for (corner, uv) in corners.iter().zip(uvs) {
            mesh.vertices.push(Vertex::new(*corner, normal, *uv, WHITE));
        }
        for triangle in 1..corners.len() as u32 - 1 {
            mesh.indices.extend_from_slice(&[base, base + triangle, base + triangle + 1]);
        }
    };
    let slope_length = height.hypot(length);
    // Slope.
    push_polygon(
        &[Vec3::new(-w, 0.0, l), Vec3::new(w, 0.0, l), Vec3::new(w, height, -l), Vec3::new(-w, height, -l)],
        &[Vec2::new(0.0, slope_length), Vec2::new(width, slope_length), Vec2::new(width, 0.0), Vec2::ZERO],
    );
    // Back wall (-Z).
    push_polygon(
        &[Vec3::new(w, 0.0, -l), Vec3::new(-w, 0.0, -l), Vec3::new(-w, height, -l), Vec3::new(w, height, -l)],
        &[Vec2::new(0.0, height), Vec2::new(width, height), Vec2::new(width, 0.0), Vec2::ZERO],
    );
    // Bottom (-Y).
    push_polygon(
        &[Vec3::new(-w, 0.0, -l), Vec3::new(w, 0.0, -l), Vec3::new(w, 0.0, l), Vec3::new(-w, 0.0, l)],
        &[Vec2::ZERO, Vec2::new(width, 0.0), Vec2::new(width, length), Vec2::new(0.0, length)],
    );
    // Sides.
    push_polygon(
        &[Vec3::new(w, 0.0, l), Vec3::new(w, 0.0, -l), Vec3::new(w, height, -l)],
        &[Vec2::new(0.0, height), Vec2::new(length, height), Vec2::new(length, 0.0)],
    );
    push_polygon(
        &[Vec3::new(-w, 0.0, -l), Vec3::new(-w, 0.0, l), Vec3::new(-w, height, -l)],
        &[Vec2::new(0.0, height), Vec2::new(length, height), Vec2::new(0.0, 0.0)],
    );
    mesh
}

/// A capsule along +Y: two hemispheres of `radius` joined by a cylinder of length
/// `2 * half_height`. With `half_height = 0` it is a sphere.
pub fn capsule(radius: f32, half_height: f32, segments: u32, rings: u32) -> MeshData {
    assert!(segments >= 3 && rings >= 2 && rings.is_multiple_of(2), "need at least 3 segments and an even ring count");
    // Rows from the top pole to the bottom pole; the equator appears twice (top and bottom of
    // the cylinder) so the cylinder gets its own band of quads.
    let mut rows: Vec<(f32, f32)> = Vec::new(); // (polar angle, y offset)
    for ring in 0..=rings / 2 {
        rows.push((PI * ring as f32 / rings as f32, half_height));
    }
    for ring in rings / 2..=rings {
        rows.push((PI * ring as f32 / rings as f32, -half_height));
    }
    let total_height = 2.0 * (radius + half_height);
    let mut mesh = MeshData::default();
    for (polar, y_offset) in &rows {
        for segment in 0..=segments {
            let azimuth = TAU * segment as f32 / segments as f32;
            let normal = Vec3::new(polar.sin() * azimuth.cos(), polar.cos(), -polar.sin() * azimuth.sin());
            let position = normal * radius + Vec3::new(0.0, *y_offset, 0.0);
            let uv = Vec2::new(segment as f32 / segments as f32, (radius + half_height - position.y) / total_height);
            mesh.vertices.push(Vertex::new(position, normal, uv, WHITE));
        }
    }
    let columns = segments + 1;
    for row in 0..rows.len() as u32 - 1 {
        for segment in 0..segments {
            let top_left = row * columns + segment;
            let top_right = top_left + 1;
            let bottom_left = top_left + columns;
            let bottom_right = bottom_left + 1;
            mesh.indices.extend_from_slice(&[top_left, bottom_left, bottom_right, top_left, bottom_right, top_right]);
        }
    }
    // Remove degenerate triangles at the poles (two vertices in the same place).
    let vertices = &mesh.vertices;
    let indices: Vec<u32> = mesh
        .indices
        .chunks_exact(3)
        .filter(|triangle| {
            let [a, b, c] = [triangle[0], triangle[1], triangle[2]].map(|index| vertices[index as usize].position());
            (b - a).cross(c - a).length_squared() > 1e-12
        })
        .flatten()
        .copied()
        .collect();
    mesh.indices = indices;
    mesh
}

pub fn sphere(radius: f32, segments: u32, rings: u32) -> MeshData {
    capsule(radius, 0.0, segments, rings)
}

/// An octahedron "gem", the fallback collectible mesh. `height` tip to tip.
pub fn gem(radius: f32, height: f32) -> MeshData {
    let top = Vec3::new(0.0, height * 0.5, 0.0);
    let bottom = -top;
    let ring = [Vec3::X, Vec3::NEG_Z, Vec3::NEG_X, Vec3::Z].map(|direction| direction * radius);
    let mut mesh = MeshData::default();
    for index in 0..4 {
        let (a, b) = (ring[index], ring[(index + 1) % 4]);
        for triangle in [[a, b, top], [b, a, bottom]] {
            let normal = (triangle[1] - triangle[0]).cross(triangle[2] - triangle[0]).normalize();
            let base = mesh.vertices.len() as u32;
            for (corner, uv) in triangle.iter().zip([Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(0.5, 0.0)]) {
                mesh.vertices.push(Vertex::new(*corner, normal, uv, WHITE));
            }
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every triangle's winding (counter-clockwise from outside) must agree with its normals.
    fn assert_outward_and_ccw(mesh: &MeshData, name: &str) {
        mesh.validate().unwrap();
        for triangle in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [triangle[0], triangle[1], triangle[2]].map(|index| mesh.vertices[index as usize]);
            let face_normal = (b.position() - a.position()).cross(c.position() - a.position());
            let vertex_normal = Vec3::from_array(a.normal) + Vec3::from_array(b.normal) + Vec3::from_array(c.normal);
            assert!(face_normal.dot(vertex_normal) > 0.0, "{name}: triangle {triangle:?} winds the wrong way");
        }
    }

    #[test]
    fn primitives_wind_counter_clockwise_with_outward_normals() {
        assert_outward_and_ccw(&cube(1.0), "cube");
        assert_outward_and_ccw(&box_mesh(Vec3::new(2.0, 0.5, 3.0), true), "box");
        assert_outward_and_ccw(&plane(4.0, 4.0), "plane");
        assert_outward_and_ccw(&ramp(2.0, 1.0, 4.0), "ramp");
        assert_outward_and_ccw(&capsule(0.4, 0.5, 16, 8), "capsule");
        assert_outward_and_ccw(&sphere(1.0, 12, 6), "sphere");
        assert_outward_and_ccw(&gem(0.3, 0.8), "gem");
    }

    #[test]
    fn bounds_are_as_built() {
        let bounds = box_mesh(Vec3::new(2.0, 0.5, 3.0), true).bounds();
        assert_eq!(bounds.min, Vec3::new(-2.0, -0.5, -3.0));
        let capsule_bounds = capsule(0.4, 0.5, 16, 8).bounds();
        assert!((capsule_bounds.max.y - 0.9).abs() < 1e-5 && (capsule_bounds.min.y + 0.9).abs() < 1e-5);
        let ramp_bounds = ramp(2.0, 1.0, 4.0).bounds();
        assert_eq!(ramp_bounds.min, Vec3::new(-1.0, 0.0, -2.0));
        assert_eq!(ramp_bounds.max, Vec3::new(1.0, 1.0, 2.0));
    }

    #[test]
    fn world_uvs_tile_per_metre() {
        let mesh = box_mesh(Vec3::new(2.0, 0.5, 3.0), true);
        let max_u = mesh.vertices.iter().map(|vertex| vertex.uv[0]).fold(0.0, f32::max);
        assert_eq!(max_u, 6.0, "the 6 m side spans six texture repeats");
    }
}
