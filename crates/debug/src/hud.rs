use glam::Vec2;
use lantern_core::color::Color;
use lantern_core::debug::DebugValues;
use lantern_renderer_3d::render_world::Overlay;
use lantern_renderer_3d::text::{GLYPH_SIZE, measure_text};

/// The debug panel in the top-left corner. Toggled by the game; drawing it is one call.
#[derive(Debug)]
pub struct DebugHud {
    pub visible: bool,
    pub background: Color,
    pub group_color: Color,
    pub name_color: Color,
    pub value_color: Color,
    pub error_color: Color,
}

impl Default for DebugHud {
    fn default() -> Self {
        DebugHud {
            visible: true,
            background: Color::rgba(0.02, 0.02, 0.05, 0.72),
            group_color: Color::rgb(1.0, 0.82, 0.3),
            name_color: Color::rgb(0.7, 0.72, 0.8),
            value_color: Color::rgb(0.95, 0.95, 0.97),
            error_color: Color::rgb(1.0, 0.4, 0.35),
        }
    }
}

/// One line of the panel before layout.
enum HudLine<'a> {
    Group(&'a str),
    Entry { name: &'a str, value: String, is_error: bool },
}

impl DebugHud {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Draws the panel at `top_left` (pixels) with text `scale`. Entries whose group is
    /// `"Errors"` are drawn in the error colour so script errors stand out.
    pub fn draw(&self, values: &DebugValues, overlay: &mut Overlay, top_left: Vec2, scale: f32) {
        if !self.visible || values.entries().is_empty() {
            return;
        }
        let mut lines = Vec::new();
        let mut current_group = None;
        for entry in values.entries() {
            let group = entry.group();
            if current_group != Some(group) {
                lines.push(HudLine::Group(group));
                current_group = Some(group);
            }
            lines.push(HudLine::Entry {
                name: entry.name(),
                value: entry.value.to_string(),
                is_error: group == "Errors",
            });
        }

        let line_height = GLYPH_SIZE as f32 * scale * 1.25;
        let padding = GLYPH_SIZE as f32 * scale * 0.5;
        let name_column = lines
            .iter()
            .filter_map(|line| match line {
                HudLine::Entry { name, .. } => Some(measure_text(name, scale).x),
                HudLine::Group(_) => None,
            })
            .fold(0.0f32, f32::max)
            + GLYPH_SIZE as f32 * scale * 2.0;
        let width = lines
            .iter()
            .map(|line| match line {
                HudLine::Group(group) => measure_text(group, scale).x,
                HudLine::Entry { value, .. } => name_column + measure_text(value, scale).x,
            })
            .fold(0.0f32, f32::max);
        let height = lines.len() as f32 * line_height;
        overlay.rect(top_left, top_left + Vec2::new(width + padding * 2.0, height + padding * 2.0), self.background);

        let mut cursor = top_left + Vec2::splat(padding);
        for line in &lines {
            match line {
                HudLine::Group(group) => overlay.text(group, cursor, scale, self.group_color),
                HudLine::Entry { name, value, is_error } => {
                    let indent = Vec2::new(GLYPH_SIZE as f32 * scale, 0.0);
                    overlay.text(name, cursor + indent, scale, self.name_color);
                    let color = if *is_error { self.error_color } else { self.value_color };
                    overlay.text(value, cursor + Vec2::new(name_column, 0.0), scale, color);
                }
            }
            cursor.y += line_height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_or_empty_hud_draws_nothing() {
        let mut overlay = Overlay::default();
        let mut values = DebugValues::default();
        DebugHud::default().draw(&values, &mut overlay, Vec2::ZERO, 1.0);
        assert!(overlay.vertices.is_empty());
        values.set("Player/Speed", 3.5f32);
        DebugHud { visible: false, ..Default::default() }.draw(&values, &mut overlay, Vec2::ZERO, 1.0);
        assert!(overlay.vertices.is_empty());
    }

    #[test]
    fn visible_hud_draws_a_panel_and_text() {
        let mut overlay = Overlay::default();
        let mut values = DebugValues::default();
        values.set("Player/Speed", 3.5f32);
        values.set("Frame/FPS", 60u32);
        DebugHud::default().draw(&values, &mut overlay, Vec2::ZERO, 1.0);
        // Background quad plus two group headers, two names and two values.
        assert!(overlay.vertices.len() > 6 * 10);
    }
}
