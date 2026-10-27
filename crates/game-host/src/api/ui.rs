//! `Ui` (menus) and `Overlay` (text and rectangles in screen pixels), drawn on top of the 3D
//! scene. Call them from `OnRender`; the overlay is cleared every frame.
//!
//! `Ui` is immediate mode: `Ui:::Begin`, then `Title`, `Label`, `Spacing` and `Button` in
//! order, then `Ui:::End`. Buttons are numbered in call order; `Button` returns true on the
//! frame the focused one is confirmed. The game passes its own navigation edges to `Begin`
//! (for example `Input:::IsPressed("menu_up")`), so the engine has no menu actions of its own.

use glam::Vec2;
use lantern_renderer_3d::text::{GLYPH_SIZE, measure_text};
use lantern_renderer_3d::ui::UiInput;
use nyx::{CallContext, CallbackError, Runtime};

use super::{
    SharedState, SharedTypes, bool_argument, color_argument, f32_argument, invalid, some_bool, some_f32,
    string_argument,
};

struct UiApi;
struct OverlayApi;

fn no_column(context: &CallContext, call: &str) -> CallbackError {
    invalid(context, format!("Ui:::{call} needs Ui:::Begin first"))
}

pub fn register(runtime: &Runtime, state: &SharedState, _types: &SharedTypes) -> nyx::Result<()> {
    let begin = state.clone();
    let end = state.clone();
    let title = state.clone();
    let label = state.clone();
    let spacing = state.clone();
    let button = state.clone();
    let focused = state.clone();
    let reset = state.clone();
    runtime
        .register_type::<UiApi>("Ui")
        // Starts a centred column `top` pixels down. `up`, `down` and `confirm` are this
        // frame's navigation edges.
        .static_method(
            "Begin",
            "(top: f32, up: bool, down: bool, confirm: bool) -> void",
            move |_context, _, arguments| {
                let input = UiInput {
                    up: bool_argument(arguments, 1)?,
                    down: bool_argument(arguments, 2)?,
                    confirm: bool_argument(arguments, 3)?,
                };
                let top = f32_argument(arguments, 0)?;
                let mut state = begin.borrow_mut();
                if let Some(unfinished) = state.ui_column.take() {
                    state.ui.end_column(unfinished);
                }
                let (screen, scale) = (state.screen_size, state.ui_scale);
                let column = state.ui.begin_column(input, screen, top, scale);
                state.ui_column = Some(column);
                Ok(None)
            },
        )
        .static_method("End", "() -> void", move |context, _, _| {
            let mut state = end.borrow_mut();
            let column = state.ui_column.take().ok_or_else(|| no_column(context, "End"))?;
            state.ui.end_column(column);
            Ok(None)
        })
        .static_method("Title", "(text: string) -> void", move |context, _, arguments| {
            let text = string_argument(arguments, 0)?;
            let state = &mut *title.borrow_mut();
            let column = state.ui_column.as_mut().ok_or_else(|| no_column(context, "Title"))?;
            column.title(&mut state.render_world.overlay, &text);
            Ok(None)
        })
        .static_method("Label", "(text: string) -> void", move |context, _, arguments| {
            let text = string_argument(arguments, 0)?;
            let state = &mut *label.borrow_mut();
            let column = state.ui_column.as_mut().ok_or_else(|| no_column(context, "Label"))?;
            column.label(&mut state.render_world.overlay, &text);
            Ok(None)
        })
        // Vertical space, in rows of text.
        .static_method("Spacing", "(rows: f32) -> void", move |context, _, arguments| {
            let rows = f32_argument(arguments, 0)?;
            let mut state = spacing.borrow_mut();
            let column = state.ui_column.as_mut().ok_or_else(|| no_column(context, "Spacing"))?;
            column.spacing(rows);
            Ok(None)
        })
        .static_method("Button", "(text: string) -> bool", move |context, _, arguments| {
            let text = string_argument(arguments, 0)?;
            let state = &mut *button.borrow_mut();
            let column = state.ui_column.as_mut().ok_or_else(|| no_column(context, "Button"))?;
            let activated = column.button(&state.ui, &mut state.render_world.overlay, &text);
            some_bool(context, activated)
        })
        // Which button has focus, counting from 0 (compare frames to play a "move" sound).
        .static_method("Focused", "() -> i32", move |context, _, _| {
            Ok(Some(context.make_i32(focused.borrow().ui.focused() as i32)?))
        })
        .static_method("ResetFocus", "() -> void", move |_context, _, _| {
            reset.borrow_mut().ui.reset_focus();
            Ok(None)
        })
        .register()?;

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
