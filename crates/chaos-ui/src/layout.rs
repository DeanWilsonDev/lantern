//! Box layout for the bridge's nodes, in physical pixels: a small subset of CSS flexbox, the
//! part Cosmos describes (`display`, `flex-direction`, `gap`, `align-items`, `justify-content`,
//! `flex-grow`, sizes, padding, margin, borders and `position` with offsets).
//!
//! Two passes per node. `measure` asks how big a node wants to be (its content plus padding and
//! border, unless `width`/`height` say otherwise). `arrange` gives a node its rectangle and
//! places its children inside: along the stack's main axis one after another with `gap`
//! between, sharing leftover space by `flex-grow` or distributing it by `justify-content`, and
//! across it by `align-items` (CSS's default, `stretch`, fills the cross axis).

use glam::Vec2;
use lantern_renderer_3d::text::{GLYPH_SIZE, measure_text};

use crate::node::{NodeKind, UiNode};
use crate::style::{Align, Direction, Justify, Length, Position, Style};

/// `font-size` when neither the element nor an ancestor sets one, in logical pixels.
pub const DEFAULT_FONT_SIZE: f32 = 16.0;

/// An axis-aligned rectangle in physical pixels, origin top-left, +Y down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn new(min: Vec2, size: Vec2) -> Rect {
        Rect { min, max: min + size }
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x && point.y >= self.min.y && point.x < self.max.x && point.y < self.max.y
    }

    /// Shrunk by `left`, `top`, `right`, `bottom`, never past empty.
    pub fn inset(&self, left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        let min = self.min + Vec2::new(left, top);
        let max = (self.max - Vec2::new(right, bottom)).max(min);
        Rect { min, max }
    }
}

/// The whole-number scale the 8x8 bitmap font is drawn at for `style`'s `font-size`: Lantern
/// has one font, so a size picks the nearest crisp multiple (decision D-043).
pub fn text_scale(style: &Style, scale: f32) -> f32 {
    let pixels = style.font_size.unwrap_or(DEFAULT_FONT_SIZE) * scale;
    (pixels / GLYPH_SIZE as f32).round().max(1.0)
}

fn main_of(size: Vec2, direction: Direction) -> f32 {
    if direction == Direction::Row { size.x } else { size.y }
}

fn cross_of(size: Vec2, direction: Direction) -> f32 {
    if direction == Direction::Row { size.y } else { size.x }
}

fn from_main_cross(main: f32, cross: f32, direction: Direction) -> Vec2 {
    if direction == Direction::Row { Vec2::new(main, cross) } else { Vec2::new(cross, main) }
}

struct Layout<'a> {
    nodes: &'a [UiNode],
    styles: &'a [&'a Style],
    screen: Vec2,
    scale: f32,
    rects: Vec<Rect>,
}

impl Layout<'_> {
    fn resolve(&self, length: Option<Length>, container: Option<f32>) -> Option<f32> {
        length.and_then(|length| length.resolve(self.scale, container, self.screen))
    }

    /// Border plus padding on each side: left, top, right, bottom.
    fn edges(&self, style: &Style) -> (f32, f32, f32, f32) {
        let padding = style.padding.scaled(self.scale);
        let border = style.border_width * self.scale;
        (padding.left + border, padding.top + border, padding.right + border, padding.bottom + border)
    }

    fn clamp_width(&self, style: &Style, width: f32, container: Vec2) -> f32 {
        let mut width = width;
        if let Some(max) = self.resolve(style.max_width, Some(container.x)) {
            width = width.min(max);
        }
        // `min-width` wins over `max-width`, as in CSS.
        if let Some(min) = self.resolve(style.min_width, Some(container.x)) {
            width = width.max(min);
        }
        width.max(0.0)
    }

    fn flow_children(&self, index: usize) -> impl Iterator<Item = usize> + '_ {
        self.nodes[index].children.iter().copied().filter(|&child| !self.styles[child].is_out_of_flow())
    }

    fn positioned_children(&self, index: usize) -> impl Iterator<Item = usize> + '_ {
        self.nodes[index].children.iter().copied().filter(|&child| self.styles[child].is_out_of_flow())
    }

    /// The border-box size `index` wants inside a container whose content box is `container`.
    fn measure(&self, index: usize, container: Vec2) -> Vec2 {
        let style = self.styles[index];
        let (left, top, right, bottom) = self.edges(style);
        let edge = Vec2::new(left + right, top + bottom);
        let fixed_width = self.resolve(style.width, Some(container.x));
        let fixed_height = self.resolve(style.height, Some(container.y));
        let content = match self.nodes[index].kind {
            NodeKind::Text => measure_text(&self.nodes[index].text, text_scale(style, self.scale)),
            NodeKind::Frame | NodeKind::Unsupported => {
                let inner = Vec2::new(
                    fixed_width.unwrap_or(container.x) - edge.x,
                    fixed_height.unwrap_or(container.y) - edge.y,
                )
                .max(Vec2::ZERO);
                let direction = style.stack_direction();
                let (mut main, mut cross, mut count) = (0.0f32, 0.0f32, 0usize);
                for child in self.flow_children(index) {
                    let margin = self.styles[child].margin.scaled(self.scale);
                    let size = self.measure(child, inner) + Vec2::new(margin.horizontal(), margin.vertical());
                    main += main_of(size, direction);
                    cross = cross.max(cross_of(size, direction));
                    count += 1;
                }
                main += style.gap * self.scale * count.saturating_sub(1) as f32;
                from_main_cross(main, cross, direction)
            }
        };
        let width = self.clamp_width(style, fixed_width.unwrap_or(content.x + edge.x), container);
        let height = fixed_height.unwrap_or(content.y + edge.y).max(0.0);
        Vec2::new(width, height)
    }

    /// Gives `index` its border box and places its children. `containing` is the box an
    /// `absolute` descendant is placed against: the nearest positioned ancestor's padding box,
    /// or the window.
    fn arrange(&mut self, index: usize, rect: Rect, containing: Rect) {
        self.rects[index] = rect;
        let style = self.styles[index];
        let border = style.border_width * self.scale;
        let (left, top, right, bottom) = self.edges(style);
        let content = rect.inset(left, top, right, bottom);
        let child_containing =
            if style.position != Position::Static { rect.inset(border, border, border, border) } else { containing };

        let direction = style.stack_direction();
        let flow: Vec<usize> = self.flow_children(index).collect();
        let content_size = content.size();
        let content_main = main_of(content_size, direction);
        let content_cross = cross_of(content_size, direction);
        let gap = style.gap * self.scale;
        let gaps = gap * flow.len().saturating_sub(1) as f32;

        // Each flow child's border-box size and margins.
        let mut sizes: Vec<Vec2> = flow.iter().map(|&child| self.measure(child, content_size)).collect();
        let margins: Vec<_> = flow.iter().map(|&child| self.styles[child].margin.scaled(self.scale)).collect();
        let outer_main = |size: Vec2, margin: &crate::style::Insets| {
            main_of(size, direction) + if direction == Direction::Row { margin.horizontal() } else { margin.vertical() }
        };

        // flex-grow: growing children share what the others and the gaps leave, in proportion
        // to their values; their own content size isn't added on top (`flex: 1`).
        let total_grow: f32 = flow.iter().map(|&child| self.styles[child].flex_grow).sum();
        let mut free = 0.0;
        if total_grow > 0.0 {
            let fixed: f32 = flow
                .iter()
                .zip(&sizes)
                .zip(&margins)
                .filter(|((child, _), _)| self.styles[**child].flex_grow <= 0.0)
                .map(|((_, size), margin)| outer_main(*size, margin))
                .sum();
            let available = (content_main - fixed - gaps).max(0.0);
            for ((child, size), margin) in flow.iter().zip(sizes.iter_mut()).zip(&margins) {
                let grow = self.styles[*child].flex_grow;
                if grow > 0.0 {
                    let margin_main = if direction == Direction::Row { margin.horizontal() } else { margin.vertical() };
                    let main = (available * grow / total_grow - margin_main).max(0.0);
                    *size = from_main_cross(main, cross_of(*size, direction), direction);
                }
            }
        } else {
            let used: f32 = sizes.iter().zip(&margins).map(|(size, margin)| outer_main(*size, margin)).sum();
            free = (content_main - used - gaps).max(0.0);
        }

        let (mut cursor, between) = match style.justify_content {
            Justify::Start => (0.0, 0.0),
            Justify::Center => (free / 2.0, 0.0),
            Justify::End => (free, 0.0),
            Justify::SpaceBetween if flow.len() > 1 => (0.0, free / (flow.len() - 1) as f32),
            Justify::SpaceBetween => (0.0, 0.0),
        };

        let align = style.align_items.unwrap_or(Align::Stretch);
        for ((child, size), margin) in flow.iter().zip(sizes).zip(margins) {
            let child_style = self.styles[*child];
            let (margin_main_start, margin_main_end, margin_cross_start, margin_cross_end) =
                if direction == Direction::Row {
                    (margin.left, margin.right, margin.top, margin.bottom)
                } else {
                    (margin.top, margin.bottom, margin.left, margin.right)
                };
            let has_cross_size =
                if direction == Direction::Row { child_style.height.is_some() } else { child_style.width.is_some() };
            let room = (content_cross - margin_cross_start - margin_cross_end).max(0.0);
            let mut cross = cross_of(size, direction);
            let cross_offset = match align {
                Align::Stretch if !has_cross_size => {
                    cross = room;
                    0.0
                }
                Align::Start | Align::Stretch => 0.0,
                Align::Center => (room - cross) / 2.0,
                Align::End => room - cross,
            };
            let main = main_of(size, direction);
            let position =
                content.min + from_main_cross(cursor + margin_main_start, cross_offset + margin_cross_start, direction);
            let mut child_rect = Rect::new(position, from_main_cross(main, cross, direction));
            if matches!(child_style.position, Position::Relative | Position::Sticky) {
                child_rect = self.nudged(child_style, child_rect, content_size);
            }
            self.arrange(*child, child_rect, child_containing);
            cursor += margin_main_start + main + margin_main_end + gap + between;
        }

        let positioned: Vec<usize> = self.positioned_children(index).collect();
        for child in positioned {
            let static_position = content.min;
            self.place_positioned(child, child_containing, static_position);
        }
    }

    /// `position: relative`: moved by its offsets, leaving its place in the flow as it was.
    fn nudged(&self, style: &Style, rect: Rect, container: Vec2) -> Rect {
        let dx = self
            .resolve(style.left, Some(container.x))
            .or_else(|| self.resolve(style.right, Some(container.x)).map(|right| -right))
            .unwrap_or(0.0);
        let dy = self
            .resolve(style.top, Some(container.y))
            .or_else(|| self.resolve(style.bottom, Some(container.y)).map(|bottom| -bottom))
            .unwrap_or(0.0);
        Rect { min: rect.min + Vec2::new(dx, dy), max: rect.max + Vec2::new(dx, dy) }
    }

    /// `position: absolute` (against `containing`) or `fixed` (against the window). With
    /// neither offset on an axis it stays where it would have started in the flow.
    fn place_positioned(&mut self, index: usize, containing: Rect, static_position: Vec2) {
        let style = self.styles[index];
        let block = if style.position == Position::Fixed { Rect::new(Vec2::ZERO, self.screen) } else { containing };
        let block_size = block.size();
        let margin = style.margin.scaled(self.scale);
        let mut size = self.measure(index, block_size);
        let left = self.resolve(style.left, Some(block_size.x));
        let right = self.resolve(style.right, Some(block_size.x));
        let top = self.resolve(style.top, Some(block_size.y));
        let bottom = self.resolve(style.bottom, Some(block_size.y));
        if let (Some(left), Some(right), None) = (left, right, style.width) {
            size.x = (block_size.x - left - right - margin.horizontal()).max(0.0);
        }
        if let (Some(top), Some(bottom), None) = (top, bottom, style.height) {
            size.y = (block_size.y - top - bottom - margin.vertical()).max(0.0);
        }
        let x = match (left, right) {
            (Some(left), _) => block.min.x + left + margin.left,
            (None, Some(right)) => block.max.x - right - margin.right - size.x,
            (None, None) => static_position.x + margin.left,
        };
        let y = match (top, bottom) {
            (Some(top), _) => block.min.y + top + margin.top,
            (None, Some(bottom)) => block.max.y - bottom - margin.bottom - size.y,
            (None, None) => static_position.y + margin.top,
        };
        self.arrange(index, Rect::new(Vec2::new(x, y), size), containing);
    }
}

/// Lays out every node: each mount's root against the window. `styles[i]` is the style node
/// `i` is drawn with this frame (its own, or its `:hover` style while focused).
pub fn layout(nodes: &[UiNode], styles: &[&Style], screen: Vec2, scale: f32) -> Vec<Rect> {
    let mut layout = Layout { nodes, styles, screen, scale, rects: vec![Rect::default(); nodes.len()] };
    let window = Rect::new(Vec2::ZERO, screen);
    for index in 0..nodes.len() {
        if nodes[index].parent.is_some() {
            continue;
        }
        if styles[index].is_out_of_flow() {
            layout.place_positioned(index, window, Vec2::ZERO);
        } else {
            let margin = styles[index].margin.scaled(scale);
            let size = layout.measure(index, screen);
            layout.arrange(index, Rect::new(Vec2::new(margin.left, margin.top), size), window);
        }
    }
    layout.rects
}
