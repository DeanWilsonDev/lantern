//! `Overlay`: text and rectangles in screen pixels, drawn on top of the 3D scene. Call it from
//! `OnRender`; the overlay is cleared every frame. Menus and the HUD are Chaos components
//! (`GameUi`).

use glam::Vec2;
use lantern_renderer_3d::text::{GLYPH_SIZE, measure_text};
use nyx::Runtime;

use super::{HostNamespace, SharedState, color_argument, f32_argument, some_f32, string_argument};

pub fn register(runtime: &Runtime, state: &SharedState) -> nyx::Result<()> {
    let rect = state.clone();
    let text = state.clone();
    HostNamespace::new(runtime, "lantern::overlay")
        .function(
            "Rect",
            "(x: f32, y: f32, width: f32, height: f32, color: string) -> void",
            move |context, arguments| {
                let min = Vec2::new(f32_argument(arguments, 0)?, f32_argument(arguments, 1)?);
                let size = Vec2::new(f32_argument(arguments, 2)?, f32_argument(arguments, 3)?);
                let color = color_argument(context, arguments, 4)?;
                rect.borrow_mut().render_world.overlay.rect(min, min + size, color);
                Ok(None)
            },
        )
        // Text with its top-left corner at (x, y). `scale` is pixels per font pixel; the font
        // is 8 x 8 (see `GlyphSize`).
        .function(
            "Text",
            "(text: string, x: f32, y: f32, scale: f32, color: string) -> void",
            move |context, arguments| {
                let content = string_argument(arguments, 0)?;
                let top_left = Vec2::new(f32_argument(arguments, 1)?, f32_argument(arguments, 2)?);
                let scale = f32_argument(arguments, 3)?;
                let color = color_argument(context, arguments, 4)?;
                text.borrow_mut().render_world.overlay.text(&content, top_left, scale, color);
                Ok(None)
            },
        )
        .function("TextWidth", "(text: string, scale: f32) -> f32", |context, arguments| {
            some_f32(context, measure_text(&string_argument(arguments, 0)?, f32_argument(arguments, 1)?).x)
        })
        .function("TextHeight", "(text: string, scale: f32) -> f32", |context, arguments| {
            some_f32(context, measure_text(&string_argument(arguments, 0)?, f32_argument(arguments, 1)?).y)
        })
        .function("GlyphSize", "() -> f32", |context, _| some_f32(context, GLYPH_SIZE as f32))
        .register()?;
    Ok(())
}
