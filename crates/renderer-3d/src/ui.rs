//! A very small immediate-mode UI for menus (Umbra `core/user-interface` + `MainMenuUI`).
//!
//! Each frame the game calls `Ui::begin`, then `title`, `label` and `button` in order. Buttons
//! are numbered in call order; the focused one is drawn highlighted and returns `true` when the
//! confirm action is pressed. Up/down move focus with wrap-around. There is no retained widget
//! tree: the code that draws the menu *is* the menu.

use glam::Vec2;
use lantern_core::color::Color;

use crate::render_world::Overlay;
use crate::text::{GLYPH_SIZE, measure_text};

/// Navigation edges for this frame (from actions such as `menu_up`).
#[derive(Clone, Copy, Debug, Default)]
pub struct UiInput {
    pub up: bool,
    pub down: bool,
    pub confirm: bool,
}

#[derive(Debug, Default)]
pub struct Ui {
    focused: usize,
    buttons_last_frame: usize,
}

pub struct UiTheme {
    pub text: Color,
    pub muted: Color,
    pub focused_text: Color,
    pub focused_fill: Color,
}

impl Default for UiTheme {
    fn default() -> Self {
        UiTheme {
            text: Color::rgb(0.92, 0.92, 0.95),
            muted: Color::rgb(0.6, 0.62, 0.7),
            focused_text: Color::rgb(0.08, 0.08, 0.1),
            focused_fill: Color::rgb(1.0, 0.82, 0.3),
        }
    }
}

/// One frame's column of widgets, without borrows: a host that draws a menu through separate
/// calls (a script) keeps this between them and passes the `Ui` and `Overlay` to each call.
/// `Ui::begin_column` starts one and `Ui::end_column` finishes it.
pub struct UiColumn {
    input: UiInput,
    center_x: f32,
    cursor_y: f32,
    scale: f32,
    button_index: usize,
    theme: UiTheme,
}

/// One frame of UI: a centred column that grows downward from `top`. Finishes the column when
/// dropped.
pub struct UiFrame<'a> {
    ui: &'a mut Ui,
    overlay: &'a mut Overlay,
    column: Option<UiColumn>,
}

impl Ui {
    pub fn focused(&self) -> usize {
        self.focused
    }

    pub fn reset_focus(&mut self) {
        self.focused = 0;
    }

    /// Starts a column centred horizontally, beginning `top` pixels down, and moves the focus
    /// with this frame's up/down edges.
    pub fn begin_column(&mut self, input: UiInput, screen_size: Vec2, top: f32, scale: f32) -> UiColumn {
        if self.buttons_last_frame > 0 {
            if input.up {
                self.focused = (self.focused + self.buttons_last_frame - 1) % self.buttons_last_frame;
            }
            if input.down {
                self.focused = (self.focused + 1) % self.buttons_last_frame;
            }
        }
        UiColumn {
            input,
            center_x: screen_size.x * 0.5,
            cursor_y: top,
            scale,
            button_index: 0,
            theme: UiTheme::default(),
        }
    }

    /// Records how many buttons the column had, so next frame's navigation wraps correctly.
    pub fn end_column(&mut self, column: UiColumn) {
        self.buttons_last_frame = column.button_index;
        if column.button_index > 0 && self.focused >= column.button_index {
            self.focused = column.button_index - 1;
        }
    }

    /// `begin_column` with the `Ui` and `Overlay` borrowed for the frame.
    pub fn begin<'a>(
        &'a mut self,
        overlay: &'a mut Overlay,
        input: UiInput,
        screen_size: Vec2,
        top: f32,
        scale: f32,
    ) -> UiFrame<'a> {
        let column = self.begin_column(input, screen_size, top, scale);
        UiFrame { ui: self, overlay, column: Some(column) }
    }
}

impl UiColumn {
    fn centred_text(&mut self, overlay: &mut Overlay, text: &str, scale: f32, color: Color) -> Vec2 {
        let size = measure_text(text, scale);
        let top_left = Vec2::new(self.center_x - size.x * 0.5, self.cursor_y);
        overlay.text(text, top_left, scale, color);
        size
    }

    pub fn title(&mut self, overlay: &mut Overlay, text: &str) {
        let scale = self.scale * 2.0;
        // A drop shadow keeps the title readable over the 3D scene.
        let size = measure_text(text, scale);
        let shadow = Vec2::new(self.center_x - size.x * 0.5 + scale, self.cursor_y + scale);
        overlay.text(text, shadow, scale, Color::rgba(0.0, 0.0, 0.0, 0.6));
        let color = self.theme.text;
        self.centred_text(overlay, text, scale, color);
        self.cursor_y += size.y + GLYPH_SIZE as f32 * self.scale * 2.0;
    }

    pub fn label(&mut self, overlay: &mut Overlay, text: &str) {
        let color = self.theme.muted;
        let scale = self.scale;
        let size = self.centred_text(overlay, text, scale, color);
        self.cursor_y += size.y + GLYPH_SIZE as f32 * self.scale;
    }

    pub fn spacing(&mut self, glyph_rows: f32) {
        self.cursor_y += GLYPH_SIZE as f32 * self.scale * glyph_rows;
    }

    /// A focusable button. Returns `true` on the frame it is activated.
    pub fn button(&mut self, ui: &Ui, overlay: &mut Overlay, text: &str) -> bool {
        let index = self.button_index;
        self.button_index += 1;
        let focused = index == ui.focused;
        let padding = GLYPH_SIZE as f32 * self.scale * 0.5;
        let size = measure_text(text, self.scale);
        let width = size.x.max(GLYPH_SIZE as f32 * self.scale * 12.0) + padding * 4.0;
        let min = Vec2::new(self.center_x - width * 0.5, self.cursor_y - padding);
        let max = Vec2::new(self.center_x + width * 0.5, self.cursor_y + size.y + padding);
        if focused {
            overlay.rect(min, max, self.theme.focused_fill);
        } else {
            overlay.rect(min, max, Color::rgba(0.0, 0.0, 0.0, 0.45));
        }
        let color = if focused { self.theme.focused_text } else { self.theme.text };
        let scale = self.scale;
        self.centred_text(overlay, text, scale, color);
        self.cursor_y += size.y + padding * 3.0;
        focused && self.input.confirm
    }
}

impl UiFrame<'_> {
    fn column(&mut self) -> &mut UiColumn {
        self.column.as_mut().expect("the column lives until the frame is dropped")
    }

    pub fn title(&mut self, text: &str) {
        let overlay = &mut *self.overlay;
        self.column.as_mut().expect("live column").title(overlay, text);
    }

    pub fn label(&mut self, text: &str) {
        let overlay = &mut *self.overlay;
        self.column.as_mut().expect("live column").label(overlay, text);
    }

    pub fn spacing(&mut self, glyph_rows: f32) {
        self.column().spacing(glyph_rows);
    }

    /// A focusable button. Returns `true` on the frame it is activated.
    pub fn button(&mut self, text: &str) -> bool {
        let (ui, overlay) = (&*self.ui, &mut *self.overlay);
        self.column.as_mut().expect("live column").button(ui, overlay, text)
    }
}

impl Drop for UiFrame<'_> {
    fn drop(&mut self) {
        if let Some(column) = self.column.take() {
            self.ui.end_column(column);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(ui: &mut Ui, input: UiInput) -> Option<usize> {
        let mut overlay = Overlay::default();
        let mut frame = ui.begin(&mut overlay, input, Vec2::new(800.0, 600.0), 100.0, 2.0);
        frame.title("Lantern");
        let pressed = [frame.button("Start"), frame.button("Options"), frame.button("Quit")];
        pressed.iter().position(|&pressed| pressed)
    }

    #[test]
    fn focus_moves_wraps_and_confirms() {
        let mut ui = Ui::default();
        assert_eq!(menu(&mut ui, UiInput::default()), None);
        assert_eq!(menu(&mut ui, UiInput { confirm: true, ..Default::default() }), Some(0));
        menu(&mut ui, UiInput { up: true, ..Default::default() });
        assert_eq!(ui.focused(), 2, "up from the first item wraps to the last");
        menu(&mut ui, UiInput { down: true, ..Default::default() });
        assert_eq!(ui.focused(), 0);
        menu(&mut ui, UiInput { down: true, ..Default::default() });
        assert_eq!(menu(&mut ui, UiInput { confirm: true, ..Default::default() }), Some(1));
    }
}
