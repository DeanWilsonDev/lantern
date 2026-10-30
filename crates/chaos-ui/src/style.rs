//! A node's resolved Cosmos style as Rust values: the bridge's flat C struct, with each field
//! that the stylesheets didn't set turned into `None` (or the CSS default).

use lantern_core::color::Color;

use crate::ffi;

/// A length as Cosmos writes it. `Px` is logical pixels (multiplied by the UI scale when laid
/// out); the others are fractions of something only known at layout time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Px(f32),
    /// Percent of the containing box's matching side.
    Percent(f32),
    /// Percent of the window's width.
    Vw(f32),
    /// Percent of the window's height.
    Vh(f32),
}

impl Length {
    /// Physical pixels, given the UI scale, the containing box's matching side (None when it
    /// isn't known yet, which makes a percentage resolve to None) and the window size.
    pub fn resolve(self, scale: f32, container: Option<f32>, screen: glam::Vec2) -> Option<f32> {
        match self {
            Length::Px(value) => Some(value * scale),
            Length::Percent(value) => container.map(|side| side * value / 100.0),
            Length::Vw(value) => Some(screen.x * value / 100.0),
            Length::Vh(value) => Some(screen.y * value / 100.0),
        }
    }
}

/// Left, top, right, bottom, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Insets {
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    pub fn scaled(&self, scale: f32) -> Insets {
        Insets {
            left: self.left * scale,
            top: self.top * scale,
            right: self.right * scale,
            bottom: self.bottom * scale,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Stack,
    Inline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    /// Laid out like `Relative`: the in-game UI has no scroll containers to stick in.
    Sticky,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub background: Option<Color>,
    /// Top and bottom colours of a vertical gradient; drawn instead of `background`.
    pub gradient: Option<(Color, Color)>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub border_radius: f32,
    pub padding: Insets,
    pub margin: Insets,
    /// Inherited: the resolver already copies an ancestor's value down.
    pub text_color: Option<Color>,
    /// Logical pixels. Inherited, like `text_color`.
    pub font_size: Option<f32>,
    pub display: Option<Display>,
    pub direction: Option<Direction>,
    pub gap: f32,
    pub align_items: Option<Align>,
    pub justify_content: Justify,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub min_width: Option<Length>,
    pub max_width: Option<Length>,
    pub flex_grow: f32,
    pub opacity: f32,
    pub position: Position,
    pub top: Option<Length>,
    pub right: Option<Length>,
    pub bottom: Option<Length>,
    pub left: Option<Length>,
    pub z_index: i32,
    /// `box-shadow`: colour and blur radius in logical pixels.
    pub shadow: Option<(Color, f32)>,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            background: None,
            gradient: None,
            border_color: None,
            border_width: 0.0,
            border_radius: 0.0,
            padding: Insets::default(),
            margin: Insets::default(),
            text_color: None,
            font_size: None,
            display: None,
            direction: None,
            gap: 0.0,
            align_items: None,
            justify_content: Justify::Start,
            width: None,
            height: None,
            min_width: None,
            max_width: None,
            flex_grow: 0.0,
            opacity: 1.0,
            position: Position::Static,
            top: None,
            right: None,
            bottom: None,
            left: None,
            z_index: 0,
            shadow: None,
        }
    }
}

fn color(raw: ffi::lantern_chaos_ui_color) -> Color {
    Color::rgba(raw.r as f32 / 255.0, raw.g as f32 / 255.0, raw.b as f32 / 255.0, raw.a as f32 / 255.0)
}

fn length(raw: ffi::lantern_chaos_ui_length) -> Option<Length> {
    match raw.unit {
        ffi::UNIT_PX => Some(Length::Px(raw.value)),
        ffi::UNIT_PERCENT => Some(Length::Percent(raw.value)),
        ffi::UNIT_VW => Some(Length::Vw(raw.value)),
        ffi::UNIT_VH => Some(Length::Vh(raw.value)),
        // `auto` (an offset) and anything unknown leave the side unset.
        _ => None,
    }
}

fn insets(raw: [f32; 4]) -> Insets {
    Insets { left: raw[0], top: raw[1], right: raw[2], bottom: raw[3] }
}

impl Style {
    pub fn from_raw(raw: &ffi::lantern_chaos_ui_style) -> Style {
        let has = |bit: u64| raw.set & bit != 0;
        let when = |bit: u64, value: Option<Length>| if has(bit) { value } else { None };
        Style {
            background: has(ffi::SET_BACKGROUND).then(|| color(raw.background)),
            gradient: has(ffi::SET_GRADIENT).then(|| (color(raw.gradient_top), color(raw.gradient_bottom))),
            border_color: has(ffi::SET_BORDER_COLOR).then(|| color(raw.border_color)),
            border_width: if has(ffi::SET_BORDER_WIDTH) { raw.border_width.max(0.0) } else { 0.0 },
            border_radius: if has(ffi::SET_BORDER_RADIUS) { raw.border_radius.max(0.0) } else { 0.0 },
            padding: if has(ffi::SET_PADDING) { insets(raw.padding) } else { Insets::default() },
            margin: if has(ffi::SET_MARGIN) { insets(raw.margin) } else { Insets::default() },
            text_color: has(ffi::SET_TEXT_COLOR).then(|| color(raw.text_color)),
            font_size: has(ffi::SET_FONT_SIZE).then_some(raw.font_size),
            display: has(ffi::SET_DISPLAY).then_some(if raw.display == 1 { Display::Inline } else { Display::Stack }),
            direction: has(ffi::SET_DIRECTION).then_some(if raw.direction == 1 {
                Direction::Column
            } else {
                Direction::Row
            }),
            gap: if has(ffi::SET_GAP) { raw.gap } else { 0.0 },
            align_items: has(ffi::SET_ALIGN_ITEMS).then_some(match raw.align_items {
                1 => Align::Center,
                2 => Align::End,
                3 => Align::Stretch,
                _ => Align::Start,
            }),
            justify_content: if has(ffi::SET_JUSTIFY_CONTENT) {
                match raw.justify_content {
                    1 => Justify::Center,
                    2 => Justify::End,
                    3 => Justify::SpaceBetween,
                    _ => Justify::Start,
                }
            } else {
                Justify::Start
            },
            width: when(ffi::SET_WIDTH, length(raw.width)),
            height: when(ffi::SET_HEIGHT, length(raw.height)),
            min_width: when(ffi::SET_MIN_WIDTH, length(raw.min_width)),
            max_width: when(ffi::SET_MAX_WIDTH, length(raw.max_width)),
            flex_grow: if has(ffi::SET_FLEX_GROW) { raw.flex_grow.max(0.0) } else { 0.0 },
            opacity: if has(ffi::SET_OPACITY) { raw.opacity.clamp(0.0, 1.0) } else { 1.0 },
            position: if has(ffi::SET_POSITION) {
                match raw.position {
                    1 => Position::Relative,
                    2 => Position::Absolute,
                    3 => Position::Fixed,
                    4 => Position::Sticky,
                    _ => Position::Static,
                }
            } else {
                Position::Static
            },
            top: when(ffi::SET_TOP, length(raw.top)),
            right: when(ffi::SET_RIGHT, length(raw.right)),
            bottom: when(ffi::SET_BOTTOM, length(raw.bottom)),
            left: when(ffi::SET_LEFT, length(raw.left)),
            z_index: if has(ffi::SET_Z_INDEX) { raw.z_index } else { 0 },
            shadow: has(ffi::SET_SHADOW).then(|| (color(raw.shadow_color), raw.shadow_blur.max(0.0))),
        }
    }

    /// The direction a frame stacks its children in. Cosmos's `display: stack` stacks in a row
    /// unless `flex-direction: column` says otherwise, as Penumbra does. A frame with no
    /// `display` at all stacks in a column here (Penumbra leaves its children unarranged
    /// instead), so plain frames read top to bottom like blocks (decision D-043).
    pub fn stack_direction(&self) -> Direction {
        match (self.display, self.direction) {
            (_, Some(direction)) => direction,
            (Some(Display::Stack), None) => Direction::Row,
            _ => Direction::Column,
        }
    }

    pub fn is_out_of_flow(&self) -> bool {
        matches!(self.position, Position::Absolute | Position::Fixed)
    }
}
