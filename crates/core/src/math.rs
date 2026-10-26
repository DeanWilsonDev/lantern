//! Coordinate conventions, stated once. See `docs/architecture.md` ("Conventions").
//!
//! - right-handed, +Y up, -Z forward, +X right;
//! - 1 unit = 1 metre;
//! - angles in radians; yaw is a rotation about +Y, counter-clockwise seen from above;
//! - column vectors: a point is transformed as `matrix * point`;
//! - `local = translation * rotation * scale`, `world = parent_world * local`;
//! - clip space depth 0..1 (`Mat4::perspective_rh`).

use glam::{Mat4, Quat, Vec3};

pub const WORLD_UP: Vec3 = Vec3::Y;
pub const WORLD_FORWARD: Vec3 = Vec3::NEG_Z;
pub const WORLD_RIGHT: Vec3 = Vec3::X;

/// The rotation for a heading. Yaw 0 faces `WORLD_FORWARD`.
pub fn yaw_rotation(yaw_radians: f32) -> Quat {
    Quat::from_rotation_y(yaw_radians)
}

/// The horizontal unit direction for a heading. Yaw 0 is -Z, yaw +90° is -X (turning left).
pub fn direction_from_yaw(yaw_radians: f32) -> Vec3 {
    yaw_rotation(yaw_radians) * WORLD_FORWARD
}

/// The yaw that faces along `direction` projected on the ground plane.
pub fn yaw_from_direction(direction: Vec3) -> f32 {
    // direction_from_yaw(yaw) = (-sin yaw, 0, -cos yaw)
    (-direction.x).atan2(-direction.z)
}

/// A perspective projection in Lantern's clip space (right-handed, depth 0..1).
pub fn perspective(vertical_fov_radians: f32, aspect_ratio: f32, near: f32, far: f32) -> Mat4 {
    Mat4::perspective_rh(vertical_fov_radians, aspect_ratio, near, far)
}

/// Move `current` towards `target` by at most `max_delta`.
pub fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta { target } else { current + (target - current).signum() * max_delta }
}

/// Wrap an angle into (-π, π].
pub fn wrap_angle(angle_radians: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut wrapped = (angle_radians + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
    if wrapped <= -std::f32::consts::PI {
        wrapped += tau;
    }
    wrapped
}

/// An axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// An "inverted" box that any point will grow.
    pub const EMPTY: Aabb = Aabb { min: Vec3::splat(f32::INFINITY), max: Vec3::splat(f32::NEG_INFINITY) };

    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min: min.min(max), max: min.max(max) }
    }

    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Aabb {
        Aabb { min: center - half_extents.abs(), max: center + half_extents.abs() }
    }

    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Aabb {
        points.into_iter().fold(Aabb::EMPTY, |bounds, point| bounds.grown_to(point))
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    pub fn grown_to(self, point: Vec3) -> Aabb {
        Aabb { min: self.min.min(point), max: self.max.max(point) }
    }

    pub fn union(self, other: Aabb) -> Aabb {
        Aabb { min: self.min.min(other.min), max: self.max.max(other.max) }
    }

    pub fn expanded(self, amount: f32) -> Aabb {
        Aabb { min: self.min - Vec3::splat(amount), max: self.max + Vec3::splat(amount) }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        self.min.cmple(point).all() && self.max.cmpge(point).all()
    }

    /// The box that encloses this box after `transform` (exact for the eight corners).
    pub fn transformed(&self, transform: &Mat4) -> Aabb {
        if self.is_empty() {
            return *self;
        }
        let corners = (0..8).map(|corner| {
            let point = Vec3::new(
                if corner & 1 == 0 { self.min.x } else { self.max.x },
                if corner & 2 == 0 { self.min.y } else { self.max.y },
                if corner & 4 == 0 { self.min.z } else { self.max.z },
            );
            transform.transform_point3(point)
        });
        Aabb::from_points(corners)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Vec4, Vec4Swizzles};
    use std::f32::consts::FRAC_PI_2;

    fn near(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn axes_are_right_handed() {
        assert!(near(WORLD_RIGHT.cross(WORLD_UP), -WORLD_FORWARD));
    }

    #[test]
    fn yaw_zero_faces_forward_and_positive_yaw_turns_left() {
        assert!(near(direction_from_yaw(0.0), WORLD_FORWARD));
        assert!(near(direction_from_yaw(FRAC_PI_2), -WORLD_RIGHT));
    }

    #[test]
    fn yaw_round_trips_through_direction() {
        for yaw in [-3.0f32, -1.0, 0.0, 0.5, 2.5] {
            let back = yaw_from_direction(direction_from_yaw(yaw));
            assert!((wrap_angle(back - yaw)).abs() < 1e-5, "{yaw} -> {back}");
        }
    }

    #[test]
    fn transforms_compose_parent_times_local() {
        let parent = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let local = Mat4::from_rotation_translation(yaw_rotation(FRAC_PI_2), Vec3::new(0.0, 0.0, -1.0));
        let world = parent * local;
        // The child's forward axis is rotated by the child, then moved by the parent.
        let point = world * Vec4::new(0.0, 0.0, -1.0, 1.0);
        assert!(near(point.xyz(), Vec3::new(9.0, 0.0, -1.0)));
    }

    #[test]
    fn projection_maps_near_to_zero_and_far_to_one() {
        let projection = perspective(1.0, 1.0, 0.1, 100.0);
        let near_point = projection * Vec4::new(0.0, 0.0, -0.1, 1.0);
        let far_point = projection * Vec4::new(0.0, 0.0, -100.0, 1.0);
        assert!((near_point.z / near_point.w).abs() < 1e-5);
        assert!((far_point.z / far_point.w - 1.0).abs() < 1e-5);
        let above = projection * Vec4::new(0.0, 1.0, -1.0, 1.0);
        assert!(above.y / above.w > 0.0, "up in view space is up in clip space");
    }

    #[test]
    fn aabb_transforms_and_overlaps() {
        let unit = Aabb::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.5));
        let moved = unit.transformed(&Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0)));
        assert!(near(moved.center(), Vec3::new(2.0, 0.0, 0.0)));
        assert!(!unit.overlaps(&moved));
        assert!(unit.overlaps(&unit.expanded(0.1)));
        let rotated = unit.transformed(&Mat4::from_quat(yaw_rotation(std::f32::consts::FRAC_PI_4)));
        assert!((rotated.half_extents().x - 0.5f32.hypot(0.5)).abs() < 1e-5);
        assert!(Aabb::EMPTY.is_empty());
    }

    #[test]
    fn helpers_behave() {
        assert_eq!(move_towards(0.0, 10.0, 3.0), 3.0);
        assert_eq!(move_towards(9.0, 10.0, 3.0), 10.0);
        assert!((wrap_angle(3.0 * std::f32::consts::PI) - std::f32::consts::PI).abs() < 1e-5);
    }
}
