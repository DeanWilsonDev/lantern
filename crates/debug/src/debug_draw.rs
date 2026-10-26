//! Wireframe helpers that append to `RenderWorld::lines`.

use std::f32::consts::{PI, TAU};

use glam::Vec3;
use lantern_core::color::Color;
use lantern_renderer_3d::RenderWorld;

const CIRCLE_SEGMENTS: u32 = 16;

/// An upright capsule: rings at the top and bottom of its segment, four vertical lines, and
/// half-circle arcs over each cap.
pub fn capsule(render_world: &mut RenderWorld, center: Vec3, radius: f32, half_segment: f32, color: Color) {
    let top = center + Vec3::Y * half_segment;
    let bottom = center - Vec3::Y * half_segment;
    render_world.circle_lines(top, Vec3::Y, radius, CIRCLE_SEGMENTS, color);
    render_world.circle_lines(bottom, Vec3::Y, radius, CIRCLE_SEGMENTS, color);
    for side in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
        render_world.line(bottom + side * radius, top + side * radius, color);
    }
    for side in [Vec3::X, Vec3::Z] {
        let arc = |cap: Vec3, up: f32, render_world: &mut RenderWorld| {
            let points: Vec<Vec3> = (0..=CIRCLE_SEGMENTS / 2)
                .map(|index| {
                    let angle = PI * index as f32 / (CIRCLE_SEGMENTS / 2) as f32;
                    cap + side * angle.cos() * radius + Vec3::Y * up * angle.sin() * radius
                })
                .collect();
            for pair in points.windows(2) {
                render_world.line(pair[0], pair[1], color);
            }
        };
        arc(top, 1.0, render_world);
        arc(bottom, -1.0, render_world);
    }
}

/// A sphere as three great circles.
pub fn sphere(render_world: &mut RenderWorld, center: Vec3, radius: f32, color: Color) {
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        render_world.circle_lines(center, axis, radius, CIRCLE_SEGMENTS, color);
    }
}

/// The edges of each triangle, nudged a millimetre along the face normal so they are not
/// hidden by the surface they outline.
pub fn triangle_edges(render_world: &mut RenderWorld, triangles: impl IntoIterator<Item = [Vec3; 3]>, color: Color) {
    for [a, b, c] in triangles {
        let lift = (b - a).cross(c - a).normalize_or_zero() * 0.003;
        render_world.line(a + lift, b + lift, color);
        render_world.line(b + lift, c + lift, color);
        render_world.line(c + lift, a + lift, color);
    }
}

/// An arrow from `start` along `vector`, with a small head.
pub fn arrow(render_world: &mut RenderWorld, start: Vec3, vector: Vec3, color: Color) {
    let end = start + vector;
    render_world.line(start, end, color);
    let length = vector.length();
    if length < 1e-4 {
        return;
    }
    let direction = vector / length;
    let side = direction.any_orthonormal_vector() * length * 0.1;
    for angle in [0.0, TAU / 3.0, 2.0 * TAU / 3.0] {
        let rotated = glam::Quat::from_axis_angle(direction, angle) * side;
        render_world.line(end, end - direction * length * 0.2 + rotated, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_add_lines() {
        let mut render_world = RenderWorld::default();
        capsule(&mut render_world, Vec3::ZERO, 0.5, 0.5, Color::WHITE);
        let after_capsule = render_world.lines.len();
        assert!(after_capsule > 0);
        triangle_edges(&mut render_world, [[Vec3::ZERO, Vec3::X, Vec3::Z]], Color::WHITE);
        assert_eq!(render_world.lines.len(), after_capsule + 6);
    }
}
