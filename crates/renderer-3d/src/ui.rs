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

/// One frame of UI: a centred column that grows downward from `cursor_y`.
pub struct UiFrame<'a> {
    ui: &'a mut Ui,
    overlay: &'a mut Overlay,
    input: UiInput,
    center_x: f32,
    cursor_y: f32,
    scale: f32,
    button_index: usize,
    theme: UiTheme,
}

impl Ui {
    pub fn focused(&self) -> usize {
        self.focused
    }

    pub fn reset_focus(&mut self) {
        self.focused = 0;
    }

    /// Starts a column centred horizontally, beginning `top` pixels down.
    pub fn begin<'a>(
        &'a mut self,
        overlay: &'a mut Overlay,
        input: UiInput,
        screen_size: Vec2,
        top: f32,
        scale: f32,
    ) -> UiFrame<'a> {
        if self.buttons_last_frame > 0 {
            if input.up {
                self.focused = (self.focused + self.buttons_last_frame - 1) % self.buttons_last_frame;
            }
            if input.down {
                self.focused = (self.focused + 1) % self.buttons_last_frame;
            }
        }
        UiFrame {
            ui: self,
            overlay,
            input,
            center_x: screen_size.x * 0.5,
            cursor_y: top,
            scale,
            button_index: 0,
            theme: UiTheme::default(),
        }
    }
}

impl UiFrame<'_> {
    fn centred_text(&mut self, text: &str, scale: f32, color: Color) -> Vec2 {
        let size = measure_text(text, scale);
        let top_left = Vec2::new(self.center_x - size.x * 0.5, self.cursor_y);
        self.overlay.text(text, top_left, scale, color);
        size
    }

    pub fn title(&mut self, text: &str) {
        let scale = self.scale * 2.0;
        // A drop shadow keeps the title readable over the 3D scene.
        let size = measure_text(text, scale);
        let shadow = Vec2::new(self.center_x - size.x * 0.5 + scale, self.cursor_y + scale);
        self.overlay.text(text, shadow, scale, Color::rgba(0.0, 0.0, 0.0, 0.6));
        self.centred_text(text, scale, self.theme.text);
        self.cursor_y += size.y + GLYPH_SIZE as f32 * self.scale * 2.0;
    }

    pub fn label(&mut self, text: &str) {
        let color = self.theme.muted;
        let size = self.centred_text(text, self.scale, color);
        self.cursor_y += size.y + GLYPH_SIZE as f32 * self.scale;
    }

    pub fn spacing(&mut self, glyph_rows: f32) {
        self.cursor_y += GLYPH_SIZE as f32 * self.scale * glyph_rows;
    }

    /// A focusable button. Returns `true` on the frame it is activated.
    pub fn button(&mut self, text: &str) -> bool {
        let index = self.button_index;
        self.button_index += 1;
        let focused = index == self.ui.focused;
        let padding = GLYPH_SIZE as f32 * self.scale * 0.5;
        let size = measure_text(text, self.scale);
        let width = size.x.max(GLYPH_SIZE as f32 * self.scale * 12.0) + padding * 4.0;
        let min = Vec2::new(self.center_x - width * 0.5, self.cursor_y - padding);
        let max = Vec2::new(self.center_x + width * 0.5, self.cursor_y + size.y + padding);
        if focused {
            self.overlay.rect(min, max, self.theme.focused_fill);
        } else {
            self.overlay.rect(min, max, Color::rgba(0.0, 0.0, 0.0, 0.45));
        }
        let color = if focused { self.theme.focused_text } else { self.theme.text };
        self.centred_text(text, self.scale, color);
        self.cursor_y += size.y + padding * 3.0;
        focused && self.input.confirm
    }
}

impl Drop for UiFrame<'_> {
    fn drop(&mut self) {
        self.ui.buttons_last_frame = self.button_index;
        if self.button_index > 0 && self.ui.focused >= self.button_index {
            self.ui.focused = self.button_index - 1;
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
