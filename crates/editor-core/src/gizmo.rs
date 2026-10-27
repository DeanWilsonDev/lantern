//! Gizmo maths: the move, rotate and scale handles drawn at the selection's pivot, hit-testing
//! them, and turning a pointer ray into an axis parameter, a plane point or a ring angle.
//!
//! A gizmo has three axis handles, three plane handles (a square between each pair of axes,
//! moving or scaling in that plane) and one uniform handle at the pivot (moving in the view
//! plane, or scaling every axis together). The frame is world-aligned or local to the
//! primary entity ([`GizmoSpace`]); scaling is always local, since a world-aligned scale of a
//! rotated box would shear it.

use glam::{EulerRot, Quat, Vec2, Vec3};
use lantern_core::color::Color;
use lantern_renderer_3d::RenderWorld;

use crate::camera::{Ray, ViewportProjection};

pub const GIZMO_SIZE_PIXELS: f32 = 90.0;
pub const GRAB_DISTANCE_PIXELS: f32 = 9.0;
const RING_SEGMENTS: u32 = 64;
/// Where a plane handle's square starts and ends along each of its two axes, as fractions of
/// the gizmo size.
const PLANE_HANDLE_FROM: f32 = 0.35;
const PLANE_HANDLE_TO: f32 = 0.6;
const UNIFORM_HANDLE_RADIUS: f32 = 0.11;

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

/// Which axes the move and rotate handles follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GizmoSpace {
    #[default]
    World,
    /// The primary entity's own axes.
    Local,
}

impl GizmoSpace {
    pub fn name(self) -> &'static str {
        match self {
            GizmoSpace::World => "world",
            GizmoSpace::Local => "local",
        }
    }

    pub fn from_name(name: &str) -> Option<GizmoSpace> {
        match name {
            "world" => Some(GizmoSpace::World),
            "local" => Some(GizmoSpace::Local),
            _ => None,
        }
    }

    pub fn toggled(self) -> GizmoSpace {
        match self {
            GizmoSpace::World => GizmoSpace::Local,
            GizmoSpace::Local => GizmoSpace::World,
        }
    }
}

/// Where the gizmo sits when several entities are selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PivotMode {
    /// The primary (most recently selected) entity's origin.
    #[default]
    Primary,
    /// The centre of the selection's bounds.
    Center,
}

impl PivotMode {
    pub fn name(self) -> &'static str {
        match self {
            PivotMode::Primary => "primary",
            PivotMode::Center => "center",
        }
    }

    pub fn from_name(name: &str) -> Option<PivotMode> {
        match name {
            "primary" | "pivot" => Some(PivotMode::Primary),
            "center" | "centre" | "bounds" => Some(PivotMode::Center),
            _ => None,
        }
    }

    pub fn toggled(self) -> PivotMode {
        match self {
            PivotMode::Primary => PivotMode::Center,
            PivotMode::Center => PivotMode::Primary,
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

    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    pub fn from_name(name: &str) -> Option<Axis> {
        match name {
            "x" | "X" => Some(Axis::X),
            "y" | "Y" => Some(Axis::Y),
            "z" | "Z" => Some(Axis::Z),
            _ => None,
        }
    }

    /// The two other axes, in order.
    pub fn others(self) -> [Axis; 2] {
        match self {
            Axis::X => [Axis::Y, Axis::Z],
            Axis::Y => [Axis::Z, Axis::X],
            Axis::Z => [Axis::X, Axis::Y],
        }
    }
}

/// One grabbable part of a gizmo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Handle {
    Axis(Axis),
    /// The square between the two axes other than this one: it moves or scales in the plane
    /// whose normal this axis is.
    Plane(Axis),
    /// The pivot itself: a view-plane move, or a uniform scale.
    Uniform,
}

impl Handle {
    pub fn color(self) -> Color {
        match self {
            Handle::Axis(axis) | Handle::Plane(axis) => axis.color(),
            Handle::Uniform => Color::rgb(0.85, 0.85, 0.9),
        }
    }

    pub fn describe(self) -> String {
        match self {
            Handle::Axis(axis) => axis.name().to_uppercase(),
            Handle::Plane(axis) => {
                let [first, second] = axis.others();
                format!("{}{}", first.name().to_uppercase(), second.name().to_uppercase())
            }
            Handle::Uniform => "all".to_string(),
        }
    }
}

pub const HOVER_COLOR: Color = Color::rgb(1.0, 0.86, 0.3);

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

#[derive(Clone, Copy, Debug)]
pub struct GizmoFrame {
    pub pivot: Vec3,
    pub axes: [Vec3; 3],
    pub size: f32,
    /// The camera's view direction when the frame was made: the uniform move handle slides
    /// in the plane perpendicular to it.
    pub view_direction: Vec3,
}

impl GizmoFrame {
    pub fn new(
        tool: GizmoTool,
        space: GizmoSpace,
        pivot: Vec3,
        orientation: Quat,
        projection: &ViewportProjection,
    ) -> GizmoFrame {
        let local = tool == GizmoTool::Scale || space == GizmoSpace::Local;
        let axes = if local { Axis::ALL.map(|axis| orientation * axis.unit()) } else { Axis::ALL.map(Axis::unit) };
        GizmoFrame {
            pivot,
            axes,
            size: projection.world_per_pixel(pivot) * GIZMO_SIZE_PIXELS,
            view_direction: projection.camera.forward,
        }
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

    /// The four corners of a plane handle's square, in order around it.
    fn plane_corners(&self, normal: Axis) -> [Vec3; 4] {
        let [first, second] = normal.others().map(|axis| self.axis(axis));
        let from = self.size * PLANE_HANDLE_FROM;
        let to = self.size * PLANE_HANDLE_TO;
        [
            self.pivot + first * from + second * from,
            self.pivot + first * to + second * from,
            self.pivot + first * to + second * to,
            self.pivot + first * from + second * to,
        ]
    }

    /// The handle under `cursor`, nearest first when several overlap. Plane and uniform
    /// handles win over an axis at the same distance, since they are smaller targets.
    pub fn hit_test(&self, tool: GizmoTool, projection: &ViewportProjection, cursor: Vec2) -> Option<Handle> {
        let mut best: Option<(f32, Handle)> = None;
        let mut consider = |distance: Option<f32>, handle: Handle, bias: f32| {
            if let Some(distance) = distance {
                let scored = distance - bias;
                if distance <= GRAB_DISTANCE_PIXELS && best.is_none_or(|(closest, _)| scored < closest) {
                    best = Some((scored, handle));
                }
            }
        };
        if tool != GizmoTool::Rotate {
            if let Some(center) = projection.project(self.pivot) {
                let radius_pixels = UNIFORM_HANDLE_RADIUS * GIZMO_SIZE_PIXELS;
                let distance = ((cursor - center).length() - radius_pixels).max(0.0);
                consider(Some(distance), Handle::Uniform, 2.0);
            }
            for normal in Axis::ALL {
                let corners = self.plane_corners(normal);
                let projected: Option<Vec<Vec2>> = corners.iter().map(|corner| projection.project(*corner)).collect();
                if let Some(projected) = projected {
                    let inside = point_in_quad(&projected, cursor);
                    let edge_distance = (0..4)
                        .filter_map(|index| {
                            screen_segment_distance(projection, corners[index], corners[(index + 1) % 4], cursor)
                        })
                        .reduce(f32::min);
                    let distance = if inside { Some(0.0) } else { edge_distance };
                    consider(distance, Handle::Plane(normal), 1.0);
                }
            }
        }
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
            consider(distance, Handle::Axis(axis), 0.0);
        }
        best.map(|(_, handle)| handle)
    }

    /// Where along an axis (from the pivot, in world units) the ray passes closest.
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

    /// The normal of the plane a handle drags in: the plane handle's own, or the view plane
    /// for the uniform handle.
    pub fn drag_plane_normal(&self, handle: Handle) -> Vec3 {
        match handle {
            Handle::Plane(axis) => self.axis(axis),
            Handle::Uniform => -self.view_direction,
            Handle::Axis(axis) => self.axis(axis),
        }
    }

    /// Where the ray meets the handle's drag plane through the pivot.
    pub fn plane_point(&self, handle: Handle, ray: Ray) -> Option<Vec3> {
        ray.intersect_plane(self.pivot, self.drag_plane_normal(handle))
    }

    pub fn ring_angle(&self, axis: Axis, ray: Ray) -> Option<f32> {
        let normal = self.axis(axis);
        let point = ray.intersect_plane(self.pivot, normal)?;
        let offset = point - self.pivot;
        let tangent = normal.any_orthonormal_vector();
        let bitangent = normal.cross(tangent);
        Some(offset.dot(bitangent).atan2(offset.dot(tangent)))
    }

    pub fn draw(&self, tool: GizmoTool, highlighted: Option<Handle>, render_world: &mut RenderWorld) {
        let color_for = |handle: Handle| if highlighted == Some(handle) { HOVER_COLOR } else { handle.color() };
        for axis in Axis::ALL {
            let color = color_for(Handle::Axis(axis));
            let direction = self.axis(axis);
            match tool {
                GizmoTool::Translate => {
                    let tip = self.pivot + direction * self.size;
                    render_world.line(self.pivot, tip, color);
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
        if tool == GizmoTool::Rotate {
            return;
        }
        for normal in Axis::ALL {
            let color = color_for(Handle::Plane(normal));
            let corners = self.plane_corners(normal);
            for index in 0..4 {
                render_world.line(corners[index], corners[(index + 1) % 4], color);
            }
            if highlighted == Some(Handle::Plane(normal)) {
                render_world.line(corners[0], corners[2], color);
                render_world.line(corners[1], corners[3], color);
            }
        }
        let uniform_color = color_for(Handle::Uniform);
        render_world.circle_lines(
            self.pivot,
            -self.view_direction,
            self.size * UNIFORM_HANDLE_RADIUS,
            24,
            uniform_color,
        );
    }
}

fn point_in_quad(corners: &[Vec2], point: Vec2) -> bool {
    let mut sign = 0.0f32;
    for index in 0..corners.len() {
        let edge = corners[(index + 1) % corners.len()] - corners[index];
        let to_point = point - corners[index];
        let cross = edge.perp_dot(to_point);
        if cross.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    sign != 0.0
}

fn screen_segment_distance(projection: &ViewportProjection, start: Vec3, end: Vec3, cursor: Vec2) -> Option<f32> {
    let a = projection.project(start)?;
    let b = projection.project(end)?;
    let segment = b - a;
    let length_squared = segment.length_squared();
    let t = if length_squared < 1e-6 { 0.0 } else { ((cursor - a).dot(segment) / length_squared).clamp(0.0, 1.0) };
    Some((a + segment * t - cursor).length())
}

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

    fn frame(tool: GizmoTool) -> GizmoFrame {
        GizmoFrame::new(tool, GizmoSpace::World, Vec3::ZERO, Quat::IDENTITY, &projection())
    }

    #[test]
    fn grabbing_the_x_arrow() {
        let projection = projection();
        let frame = frame(GizmoTool::Translate);
        let on_arrow = projection.project(Vec3::X * frame.size * 0.8).unwrap();
        assert_eq!(frame.hit_test(GizmoTool::Translate, &projection, on_arrow), Some(Handle::Axis(Axis::X)));
        let far_away = Vec2::new(5.0, 5.0);
        assert_eq!(frame.hit_test(GizmoTool::Translate, &projection, far_away), None);
    }

    #[test]
    fn grabbing_the_xz_plane_and_the_pivot() {
        let projection = projection();
        let frame = frame(GizmoTool::Translate);
        let middle = (PLANE_HANDLE_FROM + PLANE_HANDLE_TO) * 0.5 * frame.size;
        let on_plane = projection.project(Vec3::new(middle, 0.0, middle)).unwrap();
        assert_eq!(frame.hit_test(GizmoTool::Translate, &projection, on_plane), Some(Handle::Plane(Axis::Y)));
        let on_pivot = projection.project(Vec3::ZERO).unwrap();
        assert_eq!(frame.hit_test(GizmoTool::Translate, &projection, on_pivot), Some(Handle::Uniform));
        assert_eq!(frame.hit_test(GizmoTool::Rotate, &projection, on_pivot), None);
    }

    #[test]
    fn local_space_follows_the_orientation() {
        let turned = Quat::from_rotation_y(90f32.to_radians());
        let frame = GizmoFrame::new(GizmoTool::Translate, GizmoSpace::Local, Vec3::ZERO, turned, &projection());
        assert!(frame.axis(Axis::X).abs_diff_eq(Vec3::NEG_Z, 1e-5), "{}", frame.axis(Axis::X));
        let world = GizmoFrame::new(GizmoTool::Translate, GizmoSpace::World, Vec3::ZERO, turned, &projection());
        assert_eq!(world.axis(Axis::X), Vec3::X);
        let scale = GizmoFrame::new(GizmoTool::Scale, GizmoSpace::World, Vec3::ZERO, turned, &projection());
        assert!(scale.axis(Axis::X).abs_diff_eq(Vec3::NEG_Z, 1e-5));
    }

    #[test]
    fn axis_parameter_follows_the_cursor() {
        let projection = projection();
        let frame = frame(GizmoTool::Translate);
        let pixel = projection.project(Vec3::new(2.0, 0.0, 0.0)).unwrap();
        let parameter = frame.axis_parameter(Axis::X, projection.ray(pixel)).unwrap();
        assert!((parameter - 2.0).abs() < 1e-3, "{parameter}");
        let point = frame.plane_point(Handle::Plane(Axis::Y), projection.ray(pixel)).unwrap();
        assert!(point.abs_diff_eq(Vec3::new(2.0, 0.0, 0.0), 1e-3), "{point}");
    }

    #[test]
    fn ring_angles_turn_a_quarter() {
        let projection = projection();
        let frame = frame(GizmoTool::Rotate);
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
