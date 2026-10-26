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
    fn helpers_behave() {
        assert_eq!(move_towards(0.0, 10.0, 3.0), 3.0);
        assert_eq!(move_towards(9.0, 10.0, 3.0), 10.0);
        assert!((wrap_angle(3.0 * std::f32::consts::PI) - std::f32::consts::PI).abs() < 1e-5);
    }
}
