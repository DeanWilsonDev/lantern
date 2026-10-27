//! Transform gizmos: translate arrows, rotate rings and scale handles, drawn at the selection's
//! pivot at a constant size on screen. This module does the maths (hit testing, turning mouse
//! movement into a delta, drawing lines); `EditorSession` owns the drag state.

use glam::{EulerRot, Quat, Vec2, Vec3};
use lantern_core::color::Color;
use lantern_renderer_3d::RenderWorld;

use crate::camera::{Ray, ViewportProjection};

/// The gizmo's arm length on screen.
pub const GIZMO_SIZE_PIXELS: f32 = 90.0;
/// How close (in pixels) the cursor must be to a handle to grab it.
pub const GRAB_DISTANCE_PIXELS: f32 = 9.0;
const RING_SEGMENTS: u32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GizmoTool {
    Translate,
    Rotate,
    Scale,
}

impl GizmoTool {
    pub fn name(self) -> &'static str {
        match self {
            GizmoTool::Translate => "move",
            GizmoTool::Rotate => "rotate",
            GizmoTool::Scale => "scale",
        }
    }

    pub fn from_name(name: &str) -> Option<GizmoTool> {
        match name {
            "move" | "translate" => Some(GizmoTool::Translate),
            "rotate" => Some(GizmoTool::Rotate),
            "scale" => Some(GizmoTool::Scale),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn unit(self) -> Vec3 {
        match self {
            Axis::X => Vec3::X,
            Axis::Y => Vec3::Y,
            Axis::Z => Vec3::Z,
        }
    }

    pub fn color(self) -> Color {
        match self {
            Axis::X => Color::from_hex(0xE8524A),
            Axis::Y => Color::from_hex(0x7BD04B),
            Axis::Z => Color::from_hex(0x4A90FF),
        }
    }
}

pub const HOVER_COLOR: Color = Color::rgb(1.0, 0.86, 0.3);

/// Snapping steps. Off means free movement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapSettings {
    pub enabled: bool,
    pub translate_step: f32,
    pub rotate_step_degrees: f32,
    pub scale_step: f32,
}

impl Default for SnapSettings {
    fn default() -> Self {
        SnapSettings { enabled: true, translate_step: 0.25, rotate_step_degrees: 15.0, scale_step: 0.1 }
    }
}

pub fn snap(value: f32, step: f32) -> f32 {
    if step <= 0.0 { value } else { (value / step).round() * step }
}

/// Where the gizmo is and which way its axes point this frame.
#[derive(Clone, Copy, Debug)]
pub struct GizmoFrame {
    pub pivot: Vec3,
    /// World axes for move and rotate; the entity's own axes for scale.
    pub axes: [Vec3; 3],
    /// Arm length in world units.
    pub size: f32,
}

impl GizmoFrame {
    pub fn new(tool: GizmoTool, pivot: Vec3, orientation: Quat, projection: &ViewportProjection) -> GizmoFrame {
        let axes = match tool {
            GizmoTool::Scale => Axis::ALL.map(|axis| orientation * axis.unit()),
            GizmoTool::Translate | GizmoTool::Rotate => Axis::ALL.map(Axis::unit),
        };
        GizmoFrame { pivot, axes, size: projection.world_per_pixel(pivot) * GIZMO_SIZE_PIXELS }
    }

    pub fn axis(&self, axis: Axis) -> Vec3 {
        self.axes[axis.index()]
    }

    fn ring_radius(&self) -> f32 {
        self.size * 0.85
    }

    fn ring_point(&self, axis: Axis, index: u32) -> Vec3 {
        let normal = self.axis(axis);
        let tangent = normal.any_orthonormal_vector();
        let bitangent = normal.cross(tangent);
        let angle = std::f32::consts::TAU * index as f32 / RING_SEGMENTS as f32;
        self.pivot + (tangent * angle.cos() + bitangent * angle.sin()) * self.ring_radius()
    }

    /// The handle under the cursor, if any.
    pub fn hit_test(&self, tool: GizmoTool, projection: &ViewportProjection, cursor: Vec2) -> Option<Axis> {
        let mut best: Option<(f32, Axis)> = None;
        for axis in Axis::ALL {
            let distance = match tool {
                GizmoTool::Translate | GizmoTool::Scale => {
                    let start = self.pivot + self.axis(axis) * self.size * 0.15;
                    let end = self.pivot + self.axis(axis) * self.size;
                    screen_segment_distance(projection, start, end, cursor)
                }
                GizmoTool::Rotate => (0..RING_SEGMENTS)
                    .filter_map(|index| {
                        screen_segment_distance(
                            projection,
                            self.ring_point(axis, index),
                            self.ring_point(axis, index + 1),
                            cursor,
                        )
                    })
                    .reduce(f32::min),
            };
            if let Some(distance) = distance {
                if distance <= GRAB_DISTANCE_PIXELS && best.is_none_or(|(closest, _)| distance < closest) {
                    best = Some((distance, axis));
                }
            }
        }
        best.map(|(_, axis)| axis)
    }

    /// How far along `axis` (from the pivot) the ray passes closest, in world units. `None`
    /// when looking straight down the axis, where movement along it is undefined.
    pub fn axis_parameter(&self, axis: Axis, ray: Ray) -> Option<f32> {
        let direction = self.axis(axis);
        let to_pivot = self.pivot - ray.origin;
        let cosine = direction.dot(ray.direction);
        let denominator = 1.0 - cosine * cosine;
        if denominator < 1e-4 {
            return None;
        }
        let along_ray = ray.direction.dot(to_pivot);
        let along_axis = direction.dot(to_pivot);
        Some((cosine * along_ray - along_axis) / denominator)
    }

    /// The angle (radians, right-handed about `axis`) of the point where the ray meets the
    /// ring's plane, measured from an arbitrary but fixed reference direction.
    pub fn ring_angle(&self, axis: Axis, ray: Ray) -> Option<f32> {
        let normal = self.axis(axis);
        let point = ray.intersect_plane(self.pivot, normal)?;
        let offset = point - self.pivot;
        let tangent = normal.any_orthonormal_vector();
        let bitangent = normal.cross(tangent);
        Some(offset.dot(bitangent).atan2(offset.dot(tangent)))
    }

    pub fn draw(&self, tool: GizmoTool, highlighted: Option<Axis>, render_world: &mut RenderWorld) {
        for axis in Axis::ALL {
            let color = if highlighted == Some(axis) { HOVER_COLOR } else { axis.color() };
            let direction = self.axis(axis);
            match tool {
                GizmoTool::Translate => {
                    let tip = self.pivot + direction * self.size;
                    render_world.line(self.pivot, tip, color);
                    // An arrowhead: four lines back from the tip.
                    let side = direction.any_orthonormal_vector();
                    let other = direction.cross(side);
                    let base = tip - direction * self.size * 0.18;
                    for offset in [side, -side, other, -other] {
                        render_world.line(tip, base + offset * self.size * 0.06, color);
                    }
                }
                GizmoTool::Scale => {
                    let tip = self.pivot + direction * self.size;
                    render_world.line(self.pivot, tip, color);
                    let half = self.size * 0.06;
                    let bounds = lantern_core::math::Aabb::from_center_half_extents(tip, Vec3::splat(half));
                    render_world.aabb_lines(&bounds, color);
                }
                GizmoTool::Rotate => {
                    for index in 0..RING_SEGMENTS {
                        render_world.line(self.ring_point(axis, index), self.ring_point(axis, index + 1), color);
                    }
                }
            }
        }
    }
}

/// Pixel distance from the cursor to a world-space segment drawn on screen.
fn screen_segment_distance(projection: &ViewportProjection, start: Vec3, end: Vec3, cursor: Vec2) -> Option<f32> {
    let a = projection.project(start)?;
    let b = projection.project(end)?;
    let segment = b - a;
    let length_squared = segment.length_squared();
    let t = if length_squared < 1e-6 { 0.0 } else { ((cursor - a).dot(segment) / length_squared).clamp(0.0, 1.0) };
    Some((a + segment * t - cursor).length())
}

/// Euler degrees `[x, y, z]` (yaw Y, then pitch X, then roll Z) for a rotation, rounded so
/// that 90° stays 90 rather than 89.99999.
pub fn euler_degrees(rotation: Quat) -> [f32; 3] {
    let (yaw, pitch, roll) = rotation.normalize().to_euler(EulerRot::YXZ);
    let tidy = |radians: f32| {
        let degrees = (radians.to_degrees() * 1000.0).round() / 1000.0;
        if degrees == -0.0 { 0.0 } else { degrees }
    };
    [tidy(pitch), tidy(yaw), tidy(roll)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::OrbitCamera;

    fn projection() -> ViewportProjection {
        let camera = OrbitCamera { target: Vec3::ZERO, distance: 10.0, ..OrbitCamera::default() };
        ViewportProjection::new(camera.render_camera(), Vec2::new(800.0, 600.0))
    }

    #[test]
    fn grabbing_the_x_arrow() {
        let projection = projection();
        let frame = GizmoFrame::new(GizmoTool::Translate, Vec3::ZERO, Quat::IDENTITY, &projection);
        let on_arrow = projection.project(Vec3::X * frame.size * 0.7).unwrap();
        assert_eq!(frame.hit_test(GizmoTool::Translate, &projection, on_arrow), Some(Axis::X));
        let far_away = Vec2::new(5.0, 5.0);
        assert_eq!(frame.hit_test(GizmoTool::Translate, &projection, far_away), None);
    }

    #[test]
    fn axis_parameter_follows_the_cursor() {
        let projection = projection();
        let frame = GizmoFrame::new(GizmoTool::Translate, Vec3::ZERO, Quat::IDENTITY, &projection);
        let pixel = projection.project(Vec3::new(2.0, 0.0, 0.0)).unwrap();
        let parameter = frame.axis_parameter(Axis::X, projection.ray(pixel)).unwrap();
        assert!((parameter - 2.0).abs() < 1e-3, "{parameter}");
    }

    #[test]
    fn ring_angles_turn_a_quarter() {
        let projection = projection();
        let frame = GizmoFrame::new(GizmoTool::Rotate, Vec3::ZERO, Quat::IDENTITY, &projection);
        let tangent = Vec3::Y.any_orthonormal_vector();
        let bitangent = Vec3::Y.cross(tangent);
        let ray_to = |point: Vec3| projection.ray(projection.project(point).unwrap());
        let start = frame.ring_angle(Axis::Y, ray_to(tangent * 2.0)).unwrap();
        let end = frame.ring_angle(Axis::Y, ray_to(bitangent * 2.0)).unwrap();
        assert!(((end - start) - std::f32::consts::FRAC_PI_2).abs() < 1e-3);
    }

    #[test]
    fn euler_round_trip() {
        let rotation = Quat::from_euler(EulerRot::YXZ, 90f32.to_radians(), 15f32.to_radians(), 0.0);
        assert_eq!(euler_degrees(rotation), [15.0, 90.0, 0.0]);
        assert_eq!(snap(1.13, 0.25), 1.25);
    }
}
