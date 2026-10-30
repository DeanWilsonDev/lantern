//! `Overlay`: text and rectangles in screen pixels, drawn on top of the 3D scene. Call it from
//! `OnRender`; the overlay is cleared every frame. Menus and the HUD are Chaos components
//! (`GameUi`).

use glam::Vec2;
use lantern_renderer_3d::text::{GLYPH_SIZE, measure_text};
use nyx::Runtime;

use super::{SharedState, color_argument, f32_argument, some_f32, string_argument};

struct OverlayApi;

pub fn register(runtime: &Runtime, state: &SharedState) -> nyx::Result<()> {
    let rect = state.clone();
    let text = state.clone();
    runtime
        .register_type::<OverlayApi>("Overlay")
        .static_method(
            "Rect",
            "(x: f32, y: f32, width: f32, height: f32, color: string) -> void",
            move |context, _, arguments| {
                let min = Vec2::new(f32_argument(arguments, 0)?, f32_argument(arguments, 1)?);
                let size = Vec2::new(f32_argument(arguments, 2)?, f32_argument(arguments, 3)?);
                let color = color_argument(context, arguments, 4)?;
                rect.borrow_mut().render_world.overlay.rect(min, min + size, color);
                Ok(None)
            },
        )
        // Text with its top-left corner at (x, y). `scale` is pixels per font pixel; the font
        // is 8 x 8 (see `GlyphSize`).
        .static_method(
            "Text",
            "(text: string, x: f32, y: f32, scale: f32, color: string) -> void",
            move |context, _, arguments| {
                let content = string_argument(arguments, 0)?;
                let top_left = Vec2::new(f32_argument(arguments, 1)?, f32_argument(arguments, 2)?);
                let scale = f32_argument(arguments, 3)?;
                let color = color_argument(context, arguments, 4)?;
                text.borrow_mut().render_world.overlay.text(&content, top_left, scale, color);
                Ok(None)
            },
        )
        .static_method("TextWidth", "(text: string, scale: f32) -> f32", |context, _, arguments| {
            some_f32(context, measure_text(&string_argument(arguments, 0)?, f32_argument(arguments, 1)?).x)
        })
        .static_method("TextHeight", "(text: string, scale: f32) -> f32", |context, _, arguments| {
            some_f32(context, measure_text(&string_argument(arguments, 0)?, f32_argument(arguments, 1)?).y)
        })
        .static_method("GlyphSize", "() -> f32", |context, _, _| some_f32(context, GLYPH_SIZE as f32))
        .register()?;
    Ok(())
}
