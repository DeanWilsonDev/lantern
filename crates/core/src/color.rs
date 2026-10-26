//! Linear RGBA colours (Umbra `core/color`).

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const RED: Color = Color::rgb(1.0, 0.2, 0.2);
    pub const GREEN: Color = Color::rgb(0.2, 1.0, 0.3);
    pub const BLUE: Color = Color::rgb(0.25, 0.45, 1.0);
    pub const YELLOW: Color = Color::rgb(1.0, 0.9, 0.2);
    pub const CYAN: Color = Color::rgb(0.2, 0.9, 1.0);
    pub const MAGENTA: Color = Color::rgb(1.0, 0.3, 0.9);
    pub const GREY: Color = Color::rgb(0.5, 0.5, 0.5);
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    /// `0xRRGGBB`, fully opaque.
    pub fn from_hex(hex: u32) -> Color {
        let channel = |shift: u32| ((hex >> shift) & 0xFF) as f32 / 255.0;
        Color::rgb(channel(16), channel(8), channel(0))
    }

    pub fn with_alpha(self, alpha: f32) -> Color {
        Color { a: alpha, ..self }
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn lerp(self, other: Color, t: f32) -> Color {
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_splits_into_channels() {
        let color = Color::from_hex(0xFF8000);
        assert_eq!(color.to_array(), [1.0, 128.0 / 255.0, 0.0, 1.0]);
    }
}
