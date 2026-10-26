//! Third-person orbit camera with collision (presentation only: it runs per display frame and
//! never affects the simulation, except that movement input is read relative to its yaw).

use glam::{Quat, Vec2, Vec3};
use lantern_physics::{Capsule, CollisionWorld};
use lantern_renderer_3d::Camera;

/// The point the camera orbits, above the player's feet (about head height).
const PIVOT_HEIGHT: f32 = 1.4;
const DEFAULT_DISTANCE: f32 = 6.0;
const MIN_DISTANCE: f32 = 0.6;
/// Radius of the sphere swept from the pivot to the camera. Larger than the near plane's
/// half-diagonal, so the near plane never cuts into a wall.
const COLLISION_RADIUS: f32 = 0.25;
/// After an obstruction clears, the camera eases back out at this speed (m/s). Moving in is
/// instant, so the view is never blocked.
const RETURN_SPEED: f32 = 5.0;
const MIN_PITCH: f32 = -1.35;
const MAX_PITCH: f32 = 0.45;

pub struct ThirdPersonCamera {
    /// Radians about +Y. 0 looks toward -Z (the level's forward).
    pub yaw: f32,
    /// Radians. Negative puts the camera above the pivot, looking down.
    pub pitch: f32,
    pub distance: f32,
    current_distance: f32,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        ThirdPersonCamera { yaw: 0.0, pitch: -0.35, distance: DEFAULT_DISTANCE, current_distance: DEFAULT_DISTANCE }
    }
}

impl ThirdPersonCamera {
    /// Applies a look delta in radians (+x turns right, +y looks up).
    pub fn look(&mut self, delta: Vec2) {
        self.yaw = lantern_core::math::wrap_angle(self.yaw - delta.x);
        self.pitch = (self.pitch + delta.y).clamp(MIN_PITCH, MAX_PITCH);
    }

    pub fn reset_behind(&mut self, yaw: f32) {
        self.yaw = yaw;
        self.pitch = ThirdPersonCamera::default().pitch;
        self.current_distance = self.distance;
    }

    fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch)
    }

    /// The horizontal forward and right directions movement input is mapped onto.
    pub fn movement_basis(&self) -> (Vec3, Vec3) {
        let rotation = Quat::from_rotation_y(self.yaw);
        (rotation * Vec3::NEG_Z, rotation * Vec3::X)
    }

    /// Positions the camera behind `player_feet`, pulled in so nothing sits between the pivot
    /// and the lens.
    pub fn update(&mut self, player_feet: Vec3, collision: &CollisionWorld, frame_seconds: f32) -> Camera {
        let pivot = player_feet + Vec3::Y * PIVOT_HEIGHT;
        let backward = self.rotation() * Vec3::Z;
        let reach = backward * self.distance;
        let allowed_distance = match collision.capsule_cast(Capsule::sphere(COLLISION_RADIUS), pivot, reach, None) {
            Some(hit) => (self.distance * hit.fraction).max(MIN_DISTANCE),
            None => self.distance,
        };
        self.current_distance = if allowed_distance < self.current_distance {
            allowed_distance
        } else {
            (self.current_distance + RETURN_SPEED * frame_seconds).min(allowed_distance)
        };
        Camera::look_at(pivot + backward * self.current_distance, pivot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walls_pull_the_camera_in_and_it_eases_back_out() {
        let mut camera = ThirdPersonCamera { pitch: 0.0, ..Default::default() };
        let mut collision = CollisionWorld::new();
        // A wall 3 m behind the player.
        collision.add_static_box(Vec3::new(0.0, 1.0, 3.5), Vec3::new(5.0, 3.0, 0.5));
        let view = camera.update(Vec3::ZERO, &collision, 1.0 / 60.0);
        assert!(view.position.z < 3.0 - COLLISION_RADIUS + 0.01, "{:?}", view.position);
        let open = CollisionWorld::new();
        let eased = camera.update(Vec3::ZERO, &open, 1.0 / 60.0);
        assert!(eased.position.z < DEFAULT_DISTANCE - 0.5, "eases out rather than jumping");
        for _ in 0..120 {
            camera.update(Vec3::ZERO, &open, 1.0 / 60.0);
        }
        assert!((camera.update(Vec3::ZERO, &open, 1.0 / 60.0).position.z - DEFAULT_DISTANCE).abs() < 1e-3);
    }

    #[test]
    fn movement_follows_the_camera_yaw() {
        let camera = ThirdPersonCamera { yaw: std::f32::consts::FRAC_PI_2, ..Default::default() };
        let (forward, right) = camera.movement_basis();
        assert!(forward.abs_diff_eq(Vec3::NEG_X, 1e-6));
        assert!(right.abs_diff_eq(Vec3::NEG_Z, 1e-6));
    }
}
