//! `RenderWorld`: everything the backend needs to draw one frame, and nothing it doesn't
//! (Umbra's render component manager, replaced by an extracted packet).

use glam::{Mat4, Vec2, Vec3};
use lantern_core::color::Color;
use lantern_core::handle::Handle;
use lantern_core::math::Aabb;

use crate::camera::Camera;
use crate::text;

/// Marker types so mesh and texture handles cannot be mixed up.
pub enum MeshTag {}
pub enum TextureTag {}
pub type MeshHandle = Handle<MeshTag>;
pub type TextureHandle = Handle<TextureTag>;

/// One opaque mesh draw.
#[derive(Clone, Copy, Debug)]
pub struct DrawItem {
    pub mesh: MeshHandle,
    /// `None` draws with a plain white texture, so `color` shows as-is.
    pub texture: Option<TextureHandle>,
    pub transform: Mat4,
    /// Multiplies the texture and vertex colour.
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct DirectionalLight {
    /// The direction the light travels (from the sun towards the ground).
    pub direction: Vec3,
    pub color: Color,
    pub ambient: Color,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        DirectionalLight {
            direction: Vec3::new(-0.4, -1.0, -0.3).normalize(),
            color: Color::rgb(1.0, 0.96, 0.88),
            ambient: Color::rgb(0.28, 0.32, 0.4),
        }
    }
}

/// A debug-line vertex. `#[repr(C)]`: uploaded as-is (stride 28).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

/// A 2D overlay vertex in physical pixels, origin top-left, +Y down. `#[repr(C)]` (stride 32).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

/// Triangles drawn after the 3D scene, alpha-blended, sampling the font atlas. Solid
/// rectangles sample the atlas's white cell, so one texture serves text and panels.
#[derive(Clone, Debug, Default)]
pub struct Overlay {
    pub vertices: Vec<OverlayVertex>,
}

impl Overlay {
    pub fn quad(&mut self, min: Vec2, max: Vec2, uv_min: Vec2, uv_max: Vec2, color: Color) {
        let color = color.to_array();
        let corner = |x: f32, y: f32, u: f32, v: f32| OverlayVertex { position: [x, y], uv: [u, v], color };
        let top_left = corner(min.x, min.y, uv_min.x, uv_min.y);
        let top_right = corner(max.x, min.y, uv_max.x, uv_min.y);
        let bottom_right = corner(max.x, max.y, uv_max.x, uv_max.y);
        let bottom_left = corner(min.x, max.y, uv_min.x, uv_max.y);
        self.vertices.extend_from_slice(&[top_left, bottom_left, bottom_right, top_left, bottom_right, top_right]);
    }

    pub fn rect(&mut self, min: Vec2, max: Vec2, color: Color) {
        let white = text::white_texel_uv();
        self.quad(min, max, white, white, color);
    }

    /// Draws `text` with its top-left corner at `top_left`, each glyph `8 * scale` pixels.
    pub fn text(&mut self, text: &str, top_left: Vec2, scale: f32, color: Color) {
        text::push_text(self, text, top_left, scale, color);
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }
}

/// One frame's worth of drawing.
#[derive(Clone, Debug)]
pub struct RenderWorld {
    pub camera: Camera,
    pub light: DirectionalLight,
    pub clear_color: Color,
    /// Linear fog towards `clear_color`, from `fog_start` to `fog_end` metres.
    pub fog_start: f32,
    pub fog_end: f32,
    pub draws: Vec<DrawItem>,
    /// Pairs of vertices, one line each.
    pub lines: Vec<LineVertex>,
    pub overlay: Overlay,
    /// The texture overlay vertices sample (the font atlas).
    pub overlay_texture: Option<TextureHandle>,
}

impl Default for RenderWorld {
    fn default() -> Self {
        RenderWorld {
            camera: Camera::default(),
            light: DirectionalLight::default(),
            clear_color: Color::rgb(0.42, 0.6, 0.85),
            fog_start: 60.0,
            fog_end: 160.0,
            draws: Vec::new(),
            lines: Vec::new(),
            overlay: Overlay::default(),
            overlay_texture: None,
        }
    }
}

impl RenderWorld {
    /// Empties the per-frame lists, keeping their allocations.
    pub fn clear(&mut self) {
        self.draws.clear();
        self.lines.clear();
        self.overlay.clear();
    }

    pub fn draw(&mut self, mesh: MeshHandle, texture: Option<TextureHandle>, transform: Mat4, color: Color) {
        self.draws.push(DrawItem { mesh, texture, transform, color });
    }

    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        let color = color.to_array();
        self.lines.push(LineVertex { position: start.to_array(), color });
        self.lines.push(LineVertex { position: end.to_array(), color });
    }

    pub fn aabb_lines(&mut self, bounds: &Aabb, color: Color) {
        let corner = |index: usize| {
            Vec3::new(
                if index & 1 == 0 { bounds.min.x } else { bounds.max.x },
                if index & 2 == 0 { bounds.min.y } else { bounds.max.y },
                if index & 4 == 0 { bounds.min.z } else { bounds.max.z },
            )
        };
        for (start, end) in
            [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)]
        {
            self.line(corner(start), corner(end), color);
        }
    }

    /// A circle of `segments` lines around `normal`.
    pub fn circle_lines(&mut self, center: Vec3, normal: Vec3, radius: f32, segments: u32, color: Color) {
        let normal = normal.normalize_or(Vec3::Y);
        let tangent = normal.any_orthonormal_vector();
        let bitangent = normal.cross(tangent);
        let point = |index: u32| {
            let angle = std::f32::consts::TAU * index as f32 / segments as f32;
            center + (tangent * angle.cos() + bitangent * angle.sin()) * radius
        };
        for index in 0..segments {
            self.line(point(index), point(index + 1), color);
        }
    }
}
