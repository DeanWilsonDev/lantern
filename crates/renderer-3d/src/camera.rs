//! A perspective camera described by position and orientation.

use glam::{Mat4, Vec3};
use lantern_core::math::{WORLD_UP, perspective};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    /// Unit direction the camera looks along.
    pub forward: Vec3,
    pub vertical_fov_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::look_at(Vec3::new(0.0, 2.0, 6.0), Vec3::ZERO)
    }
}

impl Camera {
    pub fn look_at(position: Vec3, target: Vec3) -> Camera {
        let forward = (target - position).try_normalize().unwrap_or(lantern_core::math::WORLD_FORWARD);
        Camera { position, forward, vertical_fov_radians: 60f32.to_radians(), near: 0.1, far: 300.0 }
    }

    pub fn view(&self) -> Mat4 {
        // Avoid a degenerate basis when looking straight up or down.
        let up = if self.forward.cross(WORLD_UP).length_squared() < 1e-6 { Vec3::Z } else { WORLD_UP };
        Mat4::look_to_rh(self.position, self.forward, up)
    }

    pub fn projection(&self, aspect_ratio: f32) -> Mat4 {
        perspective(self.vertical_fov_radians, aspect_ratio.max(1e-3), self.near, self.far)
    }

    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        self.projection(aspect_ratio) * self.view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec4Swizzles;

    #[test]
    fn the_target_lands_in_the_middle_of_the_screen() {
        let camera = Camera::look_at(Vec3::new(3.0, 4.0, 5.0), Vec3::new(-1.0, 0.5, 0.0));
        let clip = camera.view_projection(16.0 / 9.0) * Vec3::new(-1.0, 0.5, 0.0).extend(1.0);
        let ndc = clip.xyz() / clip.w;
        assert!(ndc.x.abs() < 1e-5 && ndc.y.abs() < 1e-5);
        assert!(ndc.z > 0.0 && ndc.z < 1.0);
    }

    #[test]
    fn looking_straight_down_is_not_degenerate() {
        let camera = Camera::look_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
        assert!(camera.view().is_finite());
    }
}
