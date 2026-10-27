//! The viewport camera: orbits a target point, pans, dollies and flies.
//!
//! Controls (applied by `EditorSession`): right drag or Alt + left drag orbits, middle drag or
//! Shift + right drag pans, the wheel dollies towards the target, and W/A/S/D/Q/E fly while the
//! right button is held. `F` frames the selection.

use glam::{Vec2, Vec3};
use lantern_core::math::Aabb;
use lantern_renderer_3d::Camera;

/// Radians of orbit per pixel of mouse movement.
const ORBIT_RADIANS_PER_PIXEL: f32 = 0.006;
/// Each wheel notch moves this fraction of the distance to the target.
const DOLLY_FRACTION_PER_NOTCH: f32 = 0.12;
const MIN_DISTANCE: f32 = 0.5;
const MAX_DISTANCE: f32 = 250.0;
/// Keeps the camera off the poles, where yaw stops meaning anything.
const MAX_PITCH: f32 = 1.54;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    pub target: Vec3,
    /// Heading about +Y. 0 looks along -Z (the engine's forward).
    pub yaw: f32,
    /// Positive looks down at the target.
    pub pitch: f32,
    pub distance: f32,
    pub vertical_fov_radians: f32,
    /// Metres per second when flying.
    pub fly_speed: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        OrbitCamera {
            target: Vec3::new(0.0, 0.0, -8.0),
            yaw: 0.35,
            pitch: 0.55,
            distance: 30.0,
            vertical_fov_radians: 60f32.to_radians(),
            fly_speed: 10.0,
        }
    }
}

impl OrbitCamera {
    /// Unit vector from the camera to the target.
    pub fn forward(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        Vec3::new(-sin_yaw * cos_pitch, -sin_pitch, -cos_yaw * cos_pitch)
    }

    pub fn position(&self) -> Vec3 {
        self.target - self.forward() * self.distance
    }

    /// Screen-right on the ground plane.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or(Vec3::X)
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    pub fn render_camera(&self) -> Camera {
        let mut camera = Camera::look_at(self.position(), self.target);
        camera.vertical_fov_radians = self.vertical_fov_radians;
        camera.near = (self.distance * 0.002).clamp(0.02, 0.2);
        camera.far = 600.0;
        camera
    }

    pub fn orbit(&mut self, mouse_delta: Vec2) {
        self.yaw -= mouse_delta.x * ORBIT_RADIANS_PER_PIXEL;
        self.pitch = (self.pitch + mouse_delta.y * ORBIT_RADIANS_PER_PIXEL).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Moves the target so the point under the cursor follows it. `viewport_height` is in the
    /// same pixels as `mouse_delta`.
    pub fn pan(&mut self, mouse_delta: Vec2, viewport_height: f32) {
        let metres_per_pixel = 2.0 * self.distance * (self.vertical_fov_radians * 0.5).tan() / viewport_height.max(1.0);
        self.target += (-self.right() * mouse_delta.x + self.up() * mouse_delta.y) * metres_per_pixel;
    }

    /// Positive notches move closer.
    pub fn dolly(&mut self, notches: f32) {
        let factor = (1.0 - DOLLY_FRACTION_PER_NOTCH).powf(notches);
        self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    /// Moves camera and target together. `local_direction` is (right, up, forward).
    pub fn fly(&mut self, local_direction: Vec3, seconds: f32) {
        let direction =
            self.right() * local_direction.x + Vec3::Y * local_direction.y + self.forward() * local_direction.z;
        self.target += direction.normalize_or_zero() * self.fly_speed * seconds;
    }

    /// Looks at `bounds` from the current direction, close enough that it fills most of the view.
    pub fn frame(&mut self, bounds: &Aabb) {
        if bounds.is_empty() {
            return;
        }
        let radius = bounds.half_extents().length().max(0.5);
        self.target = bounds.center();
        self.distance = (radius / (self.vertical_fov_radians * 0.5).sin() * 1.1).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

/// A ray in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    /// Unit length.
    pub direction: Vec3,
}

impl Ray {
    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// Where the ray crosses the plane through `point` with `normal`, if ahead of the origin.
    pub fn intersect_plane(&self, point: Vec3, normal: Vec3) -> Option<Vec3> {
        let denominator = self.direction.dot(normal);
        if denominator.abs() < 1e-6 {
            return None;
        }
        let distance = (point - self.origin).dot(normal) / denominator;
        (distance >= 0.0).then(|| self.at(distance))
    }
}

/// Converts between viewport pixels (origin top-left, +Y down) and the world.
#[derive(Clone, Copy, Debug)]
pub struct ViewportProjection {
    pub camera: Camera,
    pub size: Vec2,
}

impl ViewportProjection {
    pub fn new(camera: Camera, size: Vec2) -> ViewportProjection {
        ViewportProjection { camera, size: size.max(Vec2::ONE) }
    }

    fn view_projection(&self) -> glam::Mat4 {
        self.camera.view_projection(self.size.x / self.size.y)
    }

    /// The ray from the camera through a viewport pixel.
    pub fn ray(&self, pixel: Vec2) -> Ray {
        let ndc = Vec2::new(pixel.x / self.size.x * 2.0 - 1.0, 1.0 - pixel.y / self.size.y * 2.0);
        let inverse = self.view_projection().inverse();
        let near = inverse.project_point3(ndc.extend(0.0));
        let far = inverse.project_point3(ndc.extend(1.0));
        Ray { origin: near, direction: (far - near).normalize_or(self.camera.forward) }
    }

    /// The pixel a world point lands on, or `None` when it is behind the camera.
    pub fn project(&self, point: Vec3) -> Option<Vec2> {
        let clip = self.view_projection() * point.extend(1.0);
        if clip.w <= 1e-5 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new((ndc.x + 1.0) * 0.5 * self.size.x, (1.0 - ndc.y) * 0.5 * self.size.y))
    }

    /// World units per pixel at `point`'s distance, for screen-constant gizmo sizes.
    pub fn world_per_pixel(&self, point: Vec3) -> f32 {
        let depth = (point - self.camera.position).dot(self.camera.forward).max(self.camera.near);
        2.0 * depth * (self.camera.vertical_fov_radians * 0.5).tan() / self.size.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_in_the_middle_of_the_view() {
        let camera = OrbitCamera::default();
        let projection = ViewportProjection::new(camera.render_camera(), Vec2::new(800.0, 600.0));
        let pixel = projection.project(camera.target).unwrap();
        assert!((pixel - Vec2::new(400.0, 300.0)).length() < 0.01, "{pixel}");
        let ray = projection.ray(Vec2::new(400.0, 300.0));
        assert!(ray.direction.dot(camera.forward()) > 0.9999);
    }

    #[test]
    fn rays_and_projection_agree() {
        let camera = OrbitCamera::default();
        let projection = ViewportProjection::new(camera.render_camera(), Vec2::new(1280.0, 720.0));
        let pixel = Vec2::new(200.0, 500.0);
        let point = projection.ray(pixel).at(20.0);
        assert!((projection.project(point).unwrap() - pixel).length() < 0.05);
    }

    #[test]
    fn panning_keeps_distance_and_dolly_is_clamped() {
        let mut camera = OrbitCamera::default();
        let before = camera.position() - camera.target;
        camera.pan(Vec2::new(10.0, -5.0), 600.0);
        assert!(((camera.position() - camera.target) - before).length() < 1e-4);
        camera.dolly(1000.0);
        assert_eq!(camera.distance, MIN_DISTANCE);
    }

    #[test]
    fn framing_centres_the_bounds() {
        let mut camera = OrbitCamera::default();
        camera.frame(&Aabb::from_center_half_extents(Vec3::new(3.0, 1.0, -2.0), Vec3::ONE));
        assert_eq!(camera.target, Vec3::new(3.0, 1.0, -2.0));
        assert!(camera.distance > 1.7 && camera.distance < 10.0);
    }
}
