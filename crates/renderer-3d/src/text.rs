//! Bitmap text (decision D-005): an 8×8 ASCII font baked into a small atlas at startup.
//!
//! Atlas layout: 16 columns × 6 rows of 8×8 cells, 128×48 pixels. Cells 0..95 are glyphs
//! U+0020..U+007E; the last cell is solid white for untextured overlay rectangles. Glyph pixels
//! are white with alpha = coverage, so the overlay colour tints them.

use glam::Vec2;
use lantern_core::color::Color;

use crate::font_data::{FIRST_GLYPH, GLYPH_COUNT, GLYPHS};
use crate::render_world::Overlay;
use crate::texture::{TextureData, TextureFilter, TextureWrap};

pub const GLYPH_SIZE: u32 = 8;
const ATLAS_COLUMNS: u32 = 16;
const ATLAS_ROWS: u32 = 6;
pub const ATLAS_WIDTH: u32 = ATLAS_COLUMNS * GLYPH_SIZE;
pub const ATLAS_HEIGHT: u32 = ATLAS_ROWS * GLYPH_SIZE;
const WHITE_CELL: u32 = ATLAS_COLUMNS * ATLAS_ROWS - 1;
/// Lines are 10 font pixels apart: 8 for the glyph, 2 of spacing.
pub const LINE_HEIGHT_IN_GLYPH_PIXELS: f32 = 10.0;
/// Shown for characters outside printable ASCII.
const REPLACEMENT_GLYPH: u8 = b'?';

fn cell_origin(cell: u32) -> (u32, u32) {
    ((cell % ATLAS_COLUMNS) * GLYPH_SIZE, (cell / ATLAS_COLUMNS) * GLYPH_SIZE)
}

/// Builds the RGBA atlas texture.
pub fn font_atlas() -> TextureData {
    let mut rgba8 = vec![0u8; (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize];
    let mut set_pixel = |x: u32, y: u32, alpha: u8| {
        let offset = ((y * ATLAS_WIDTH + x) * 4) as usize;
        rgba8[offset..offset + 4].copy_from_slice(&[255, 255, 255, alpha]);
    };
    for (cell, glyph) in GLYPHS.iter().enumerate() {
        let (origin_x, origin_y) = cell_origin(cell as u32);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..8 {
                if bits & (1 << column) != 0 {
                    set_pixel(origin_x + column, origin_y + row as u32, 255);
                }
            }
        }
    }
    let (white_x, white_y) = cell_origin(WHITE_CELL);
    for y in 0..GLYPH_SIZE {
        for x in 0..GLYPH_SIZE {
            set_pixel(white_x + x, white_y + y, 255);
        }
    }
    TextureData {
        width: ATLAS_WIDTH,
        height: ATLAS_HEIGHT,
        rgba8,
        filter: TextureFilter::Nearest,
        wrap: TextureWrap::Clamp,
    }
}

/// The centre of the white cell, for solid fills.
pub fn white_texel_uv() -> Vec2 {
    let (x, y) = cell_origin(WHITE_CELL);
    Vec2::new((x as f32 + 4.0) / ATLAS_WIDTH as f32, (y as f32 + 4.0) / ATLAS_HEIGHT as f32)
}

fn glyph_cell(character: char) -> u32 {
    let byte = if character.is_ascii()
        && (character as u8) >= FIRST_GLYPH
        && ((character as u8 - FIRST_GLYPH) as usize) < GLYPH_COUNT
    {
        character as u8
    } else {
        REPLACEMENT_GLYPH
    };
    u32::from(byte - FIRST_GLYPH)
}

/// Size in pixels of `text` drawn at `scale` (multi-line aware).
pub fn measure_text(text: &str, scale: f32) -> Vec2 {
    let glyph = GLYPH_SIZE as f32 * scale;
    let mut widest = 0usize;
    let mut lines = 0usize;
    for line in text.split('\n') {
        widest = widest.max(line.chars().count());
        lines += 1;
    }
    let height = if lines == 0 { 0.0 } else { glyph + (lines - 1) as f32 * LINE_HEIGHT_IN_GLYPH_PIXELS * scale };
    Vec2::new(widest as f32 * glyph, height)
}

pub(crate) fn push_text(overlay: &mut Overlay, text: &str, top_left: Vec2, scale: f32, color: Color) {
    let glyph = GLYPH_SIZE as f32 * scale;
    let mut pen = top_left;
    for character in text.chars() {
        if character == '\n' {
            pen = Vec2::new(top_left.x, pen.y + LINE_HEIGHT_IN_GLYPH_PIXELS * scale);
            continue;
        }
        if character != ' ' {
            let (x, y) = cell_origin(glyph_cell(character));
            let uv_min = Vec2::new(x as f32 / ATLAS_WIDTH as f32, y as f32 / ATLAS_HEIGHT as f32);
            let uv_max =
                uv_min + Vec2::new(GLYPH_SIZE as f32 / ATLAS_WIDTH as f32, GLYPH_SIZE as f32 / ATLAS_HEIGHT as f32);
            // Snap to whole pixels so nearest sampling never splits a font pixel.
            let min = pen.round();
            overlay.quad(min, min + Vec2::splat(glyph), uv_min, uv_max, color);
        }
        pen.x += glyph;
    }
}

/// A whole-number text scale for a window's pixel density, so 8×8 glyphs stay sharp.
pub fn scale_for_density(base_scale: u32, pixel_density: f32) -> f32 {
    (base_scale as f32 * pixel_density).round().max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_has_the_letter_a_and_a_white_cell() {
        let atlas = font_atlas();
        assert_eq!(atlas.rgba8.len(), (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize);
        let (x, y) = cell_origin(glyph_cell('A'));
        let lit = (0..64).filter(|index| {
            let (column, row) = (x + index % 8, y + index / 8);
            atlas.rgba8[((row * ATLAS_WIDTH + column) * 4 + 3) as usize] == 255
        });
        assert!(lit.count() > 10, "'A' has pixels");
        let white = white_texel_uv();
        let (pixel_x, pixel_y) = ((white.x * ATLAS_WIDTH as f32) as u32, (white.y * ATLAS_HEIGHT as f32) as u32);
        assert_eq!(atlas.rgba8[((pixel_y * ATLAS_WIDTH + pixel_x) * 4 + 3) as usize], 255);
    }

    #[test]
    fn spaces_make_no_quads_and_non_ascii_shows_a_replacement() {
        let mut overlay = Overlay::default();
        push_text(&mut overlay, "a b", Vec2::ZERO, 2.0, Color::WHITE);
        assert_eq!(overlay.vertices.len(), 12);
        assert_eq!(glyph_cell('é'), glyph_cell('?'));
    }

    #[test]
    fn measuring_counts_the_widest_line() {
        assert_eq!(measure_text("abc\nde", 2.0), Vec2::new(48.0, 16.0 + 20.0));
        assert_eq!(scale_for_density(2, 2.0), 4.0);
        assert_eq!(scale_for_density(2, 1.25), 3.0);
    }
}
