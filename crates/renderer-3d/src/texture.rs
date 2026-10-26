//! CPU-side textures: RGBA8 pixels, row 0 at the top (see the UV convention).

use std::path::Path;

use lantern_core::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureFilter {
    /// Smooth, with mipmaps: world textures.
    Linear,
    /// Crisp pixels, no mipmaps: the font atlas and pixel art.
    Nearest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureWrap {
    Repeat,
    Clamp,
}

#[derive(Clone, Debug)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, RGBA, top row first. Colour channels are sRGB-encoded, as
    /// image files are; the GPU texture format decodes them to linear when sampled.
    pub rgba8: Vec<u8>,
    pub filter: TextureFilter,
    pub wrap: TextureWrap,
}

#[derive(Debug)]
pub struct TextureLoadError(pub String);

impl std::fmt::Display for TextureLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for TextureLoadError {}

fn to_srgb_byte(linear: f32) -> u8 {
    let linear = linear.clamp(0.0, 1.0);
    let encoded = if linear <= 0.003_130_8 { linear * 12.92 } else { 1.055 * linear.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

impl TextureData {
    pub fn solid(color: Color) -> TextureData {
        TextureData {
            width: 1,
            height: 1,
            rgba8: vec![to_srgb_byte(color.r), to_srgb_byte(color.g), to_srgb_byte(color.b), (color.a * 255.0) as u8],
            filter: TextureFilter::Nearest,
            wrap: TextureWrap::Repeat,
        }
    }

    /// A two-colour checkerboard with `cells` squares per side. On a world-UV mesh with
    /// `cells = 2`, each square is half a metre, which makes speed and distance easy to read.
    pub fn checker(size: u32, cells: u32, first: Color, second: Color) -> TextureData {
        let cell_size = (size / cells).max(1);
        let mut rgba8 = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let color = if ((x / cell_size) + (y / cell_size)).is_multiple_of(2) { first } else { second };
                rgba8.extend_from_slice(&[to_srgb_byte(color.r), to_srgb_byte(color.g), to_srgb_byte(color.b), 255]);
            }
        }
        TextureData { width: size, height: size, rgba8, filter: TextureFilter::Linear, wrap: TextureWrap::Repeat }
    }

    pub fn from_png_bytes(bytes: &[u8]) -> Result<TextureData, TextureLoadError> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        // Expand palettes and low bit depths, strip 16-bit to 8-bit.
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().map_err(|error| TextureLoadError(format!("PNG header: {error}")))?;
        let buffer_size =
            reader.output_buffer_size().ok_or_else(|| TextureLoadError("PNG is too large".to_string()))?;
        let mut buffer = vec![0; buffer_size];
        let info = reader.next_frame(&mut buffer).map_err(|error| TextureLoadError(format!("PNG data: {error}")))?;
        let pixels = &buffer[..info.buffer_size()];
        let rgba8 = match info.color_type {
            png::ColorType::Rgba => pixels.to_vec(),
            png::ColorType::Rgb => pixels.chunks_exact(3).flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255]).collect(),
            png::ColorType::GrayscaleAlpha => {
                pixels.chunks_exact(2).flat_map(|gray| [gray[0], gray[0], gray[0], gray[1]]).collect()
            }
            png::ColorType::Grayscale => pixels.iter().flat_map(|&gray| [gray, gray, gray, 255]).collect(),
            png::ColorType::Indexed => return Err(TextureLoadError("indexed PNG was not expanded".to_string())),
        };
        Ok(TextureData {
            width: info.width,
            height: info.height,
            rgba8,
            filter: TextureFilter::Linear,
            wrap: TextureWrap::Repeat,
        })
    }

    pub fn load_png(path: &Path) -> Result<TextureData, TextureLoadError> {
        let bytes = std::fs::read(path).map_err(|error| TextureLoadError(format!("{}: {error}", path.display())))?;
        TextureData::from_png_bytes(&bytes).map_err(|error| TextureLoadError(format!("{}: {error}", path.display())))
    }

    /// Encodes as PNG (used by the asset generator and for frame captures).
    pub fn to_png_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("writing to a Vec cannot fail");
            writer.write_image_data(&self.rgba8).expect("the buffer size matches the header");
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trips() {
        let texture = TextureData::checker(8, 2, Color::WHITE, Color::BLACK);
        let decoded = TextureData::from_png_bytes(&texture.to_png_bytes()).unwrap();
        assert_eq!((decoded.width, decoded.height), (8, 8));
        assert_eq!(decoded.rgba8, texture.rgba8);
        assert_eq!(&decoded.rgba8[0..4], &[255, 255, 255, 255], "row 0 is the top-left white cell");
    }

    #[test]
    fn srgb_encoding_matches_known_values() {
        assert_eq!(to_srgb_byte(0.0), 0);
        assert_eq!(to_srgb_byte(1.0), 255);
        assert_eq!(to_srgb_byte(0.2159), 128, "sRGB 128 is about 21.6% linear");
    }
}
