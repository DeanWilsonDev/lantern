//! A camera described by position and orientation: perspective by default, orthographic when
//! `orthographic_half_height` is set (an editor's top or side view).

use glam::{Mat4, Vec3};
use lantern_core::math::{WORLD_FORWARD, WORLD_UP, perspective};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    /// Unit direction the camera looks along.
    pub forward: Vec3,
    pub vertical_fov_radians: f32,
    pub near: f32,
    pub far: f32,
    /// Half the world height the view shows when orthographic; `None` for perspective.
    pub orthographic_half_height: Option<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::look_at(Vec3::new(0.0, 2.0, 6.0), Vec3::ZERO)
    }
}

impl Camera {
    pub fn look_at(position: Vec3, target: Vec3) -> Camera {
        let forward = (target - position).try_normalize().unwrap_or(lantern_core::math::WORLD_FORWARD);
        Camera {
            position,
            forward,
            vertical_fov_radians: 60f32.to_radians(),
            near: 0.1,
            far: 300.0,
            orthographic_half_height: None,
        }
    }

    pub fn view(&self) -> Mat4 {
        // Avoid a degenerate basis when looking straight up or down: screen-up is then the
        // world's forward, so a top view keeps +X on the right.
        let up = if self.forward.cross(WORLD_UP).length_squared() < 1e-6 { WORLD_FORWARD } else { WORLD_UP };
        Mat4::look_to_rh(self.position, self.forward, up)
    }

    pub fn projection(&self, aspect_ratio: f32) -> Mat4 {
        match self.orthographic_half_height {
            Some(half_height) => {
                let half_width = half_height * aspect_ratio.max(1e-3);
                // The view may sit on either side of the target, so the depth range is symmetric.
                Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, -self.far, self.far)
            }
            None => perspective(self.vertical_fov_radians, aspect_ratio.max(1e-3), self.near, self.far),
        }
    }

    pub fn is_orthographic(&self) -> bool {
        self.orthographic_half_height.is_some()
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
    fn an_orthographic_camera_keeps_parallel_lines_parallel() {
        let mut camera = Camera::look_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
        camera.orthographic_half_height = Some(5.0);
        let project = |point: Vec3| {
            let clip = camera.view_projection(1.0) * point.extend(1.0);
            clip.xyz() / clip.w
        };
        let near = project(Vec3::new(2.5, 0.0, 0.0));
        let far = project(Vec3::new(2.5, -50.0, 0.0));
        assert!((near.x - far.x).abs() < 1e-5, "{near} {far}");
        assert!((near.x - 0.5).abs() < 1e-5, "{near} {far}");
    }

    #[test]
    fn looking_straight_down_is_not_degenerate() {
        let camera = Camera::look_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
        assert!(camera.view().is_finite());
    }
}
