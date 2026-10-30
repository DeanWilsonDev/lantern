//! Draws laid-out nodes into the overlay: shadows, backgrounds (solid or a vertical gradient),
//! borders, rounded corners and text. Everything is triangles in the overlay's one batch, so
//! the in-game UI costs no extra draw calls.

use std::f32::consts::FRAC_PI_2;

use glam::Vec2;
use lantern_core::color::Color;
use lantern_renderer_3d::render_world::Overlay;

use crate::layout::{Rect, text_scale};
use crate::node::{NodeKind, UiNode};
use crate::style::Style;

/// Points per rounded corner; enough that a 24 px radius at 2x looks round.
const CORNER_SEGMENTS: usize = 6;
/// Rings a soft shadow is built from, outermost faintest.
const SHADOW_LAYERS: usize = 6;
/// Text with no `color` on it or any ancestor.
const DEFAULT_TEXT_COLOR: Color = Color::WHITE;

/// The outline of `rect` with corners rounded by `radius`, clockwise from the top-left corner's
/// left end. Always `4 * (CORNER_SEGMENTS + 1)` points, so two outlines can be stitched into a
/// ring.
fn rounded_outline(rect: Rect, radius: f32) -> Vec<Vec2> {
    let size = rect.size();
    let radius = radius.min(size.x / 2.0).min(size.y / 2.0).max(0.0);
    // Corner centres with the angle each arc starts at (y grows downward, so a quarter turn
    // from pi goes from the left up to the top).
    let corners = [
        (Vec2::new(rect.min.x + radius, rect.min.y + radius), std::f32::consts::PI),
        (Vec2::new(rect.max.x - radius, rect.min.y + radius), 3.0 * FRAC_PI_2),
        (Vec2::new(rect.max.x - radius, rect.max.y - radius), 0.0),
        (Vec2::new(rect.min.x + radius, rect.max.y - radius), FRAC_PI_2),
    ];
    let mut points = Vec::with_capacity(4 * (CORNER_SEGMENTS + 1));
    for (centre, start) in corners {
        for step in 0..=CORNER_SEGMENTS {
            let angle = start + FRAC_PI_2 * step as f32 / CORNER_SEGMENTS as f32;
            points.push(centre + Vec2::new(angle.cos(), angle.sin()) * radius);
        }
    }
    points
}

/// The colour at height `y` of a fill running from `top` (at `rect.min.y`) to `bottom`.
fn shade(rect: Rect, top: Color, bottom: Color, y: f32) -> Color {
    let height = rect.size().y;
    let t = if height > 0.0 { ((y - rect.min.y) / height).clamp(0.0, 1.0) } else { 0.0 };
    top.lerp(bottom, t)
}

/// A filled rounded rectangle, shaded from `top` to `bottom`: a fan of triangles from the
/// centre to each pair of neighbouring outline points.
fn fill_rounded(overlay: &mut Overlay, rect: Rect, radius: f32, top: Color, bottom: Color) {
    if rect.size().x <= 0.0 || rect.size().y <= 0.0 {
        return;
    }
    if radius <= 0.0 && top == bottom {
        overlay.rect(rect.min, rect.max, top);
        return;
    }
    let outline = rounded_outline(rect, radius);
    let centre = (rect.min + rect.max) / 2.0;
    let centre_color = shade(rect, top, bottom, centre.y);
    for index in 0..outline.len() {
        let (a, b) = (outline[index], outline[(index + 1) % outline.len()]);
        overlay.triangle([centre, a, b], [centre_color, shade(rect, top, bottom, a.y), shade(rect, top, bottom, b.y)]);
    }
}

/// The band between two outlines with the same number of points, `outer_color` on the outer
/// edge fading to `inner_color` on the inner one.
fn fill_ring(overlay: &mut Overlay, outer: &[Vec2], inner: &[Vec2], outer_color: Color, inner_color: Color) {
    for index in 0..outer.len() {
        let next = (index + 1) % outer.len();
        overlay.triangle([outer[index], outer[next], inner[next]], [outer_color, outer_color, inner_color]);
        overlay.triangle([outer[index], inner[next], inner[index]], [outer_color, inner_color, inner_color]);
    }
}

fn faded(color: Color, opacity: f32) -> Color {
    color.with_alpha(color.a * opacity)
}

/// `box-shadow`: rings that fade from the shadow colour at the box's edge to nothing `blur`
/// pixels out, under a solid copy of the box, nudged a little down so it reads as cast by a
/// light above. Cosmos's shadow has a colour and a blur radius, no offset or spread.
fn draw_shadow(overlay: &mut Overlay, rect: Rect, radius: f32, color: Color, blur: f32) {
    let drop = Vec2::new(0.0, blur * 0.25);
    let base = Rect { min: rect.min + drop, max: rect.max + drop };
    let mut inner_outline = rounded_outline(base, radius);
    fill_rounded(overlay, base, radius, color, color);
    for layer in 1..=SHADOW_LAYERS {
        let spread = blur * layer as f32 / SHADOW_LAYERS as f32;
        let grown = Rect { min: base.min - Vec2::splat(spread), max: base.max + Vec2::splat(spread) };
        let outer_outline = rounded_outline(grown, radius + spread);
        // Quadratic falloff looks closer to a gaussian than a straight fade.
        let inner_t = 1.0 - (layer - 1) as f32 / SHADOW_LAYERS as f32;
        let outer_t = 1.0 - layer as f32 / SHADOW_LAYERS as f32;
        fill_ring(
            overlay,
            &outer_outline,
            &inner_outline,
            faded(color, outer_t * outer_t),
            faded(color, inner_t * inner_t),
        );
        inner_outline = outer_outline;
    }
}

fn draw_box(overlay: &mut Overlay, rect: Rect, style: &Style, scale: f32, opacity: f32) {
    let radius = style.border_radius * scale;
    if let Some((color, blur)) = style.shadow
        && color.a > 0.0
    {
        draw_shadow(overlay, rect, radius, faded(color, opacity), blur * scale);
    }
    let border = style.border_width * scale;
    let fill_rect =
        if style.border_color.is_some() && border > 0.0 { rect.inset(border, border, border, border) } else { rect };
    let fill_radius = (radius - border).max(0.0);
    if let Some((top, bottom)) = style.gradient {
        fill_rounded(overlay, fill_rect, fill_radius, faded(top, opacity), faded(bottom, opacity));
    } else if let Some(background) = style.background
        && background.a > 0.0
    {
        let color = faded(background, opacity);
        fill_rounded(overlay, fill_rect, fill_radius, color, color);
    }
    if let Some(border_color) = style.border_color
        && border > 0.0
    {
        let color = faded(border_color, opacity);
        fill_ring(overlay, &rounded_outline(rect, radius), &rounded_outline(fill_rect, fill_radius), color, color);
    }
}

/// Draws every node into `overlay`, in tree order within each `z-index` level (lowest level
/// first). `styles[i]` is the style node `i` is drawn with this frame.
pub fn draw(nodes: &[UiNode], styles: &[&Style], rects: &[Rect], overlay: &mut Overlay, scale: f32) {
    // Opacity multiplies down the tree, and a node with no z-index of its own draws at its
    // parent's level. A parent always comes before its children, so one pass fills both.
    let mut opacity = vec![1.0f32; nodes.len()];
    let mut level = vec![0i32; nodes.len()];
    for index in 0..nodes.len() {
        let (parent_opacity, parent_level) =
            nodes[index].parent.map_or((1.0, 0), |parent| (opacity[parent], level[parent]));
        opacity[index] = parent_opacity * styles[index].opacity;
        level[index] = if styles[index].z_index != 0 { styles[index].z_index } else { parent_level };
    }
    let mut order: Vec<usize> = (0..nodes.len()).collect();
    order.sort_by_key(|&index| level[index]);

    for index in order {
        let (node, style, rect) = (&nodes[index], styles[index], rects[index]);
        if opacity[index] <= 0.0 {
            continue;
        }
        draw_box(overlay, rect, style, scale, opacity[index]);
        if node.kind == NodeKind::Text && !node.text.is_empty() {
            let border = style.border_width * scale;
            let padding = style.padding.scaled(scale);
            let origin = rect.min + Vec2::new(padding.left + border, padding.top + border);
            let color = faded(style.text_color.unwrap_or(DEFAULT_TEXT_COLOR), opacity[index]);
            overlay.text(&node.text, origin, text_scale(style, scale), color);
        }
    }
}
