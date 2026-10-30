//! One element of a mounted component, copied out of the bridge's snapshot.

use crate::ffi;
use crate::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// `<frame>`: a box that stacks its children.
    Frame,
    /// `<text>`: one run of text.
    Text,
    /// Another Core primitive (`<image>`, `<input>`, ...). Lantern doesn't draw these yet; they
    /// lay out as empty frames.
    Unsupported,
}

#[derive(Clone, Debug)]
pub struct UiNode {
    /// Stable while the element lives: Chaos updates elements in place.
    pub id: u64,
    /// The parent's index, `None` for a mount's root. A parent always comes before its children.
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub kind: NodeKind,
    pub text: String,
    /// Has `onPress` or `onRelease`: a button, which menu navigation can focus.
    pub focusable: bool,
    pub style: Style,
    /// The style while hovered or focused, when the stylesheets have a `:hover` block for it.
    pub hover: Option<Style>,
    /// The style while pressed, when the stylesheets have an `:active` block for it.
    pub active: Option<Style>,
}

impl UiNode {
    /// # Safety
    /// `raw.text` must point to `raw.text_length` readable bytes (the bridge's snapshot, until
    /// its next call).
    pub(crate) unsafe fn from_raw(raw: &ffi::lantern_chaos_ui_node) -> UiNode {
        let text = if raw.text.is_null() || raw.text_length == 0 {
            String::new()
        } else {
            let bytes = unsafe { std::slice::from_raw_parts(raw.text.cast::<u8>(), raw.text_length) };
            String::from_utf8_lossy(bytes).into_owned()
        };
        UiNode {
            id: raw.id,
            parent: usize::try_from(raw.parent).ok(),
            children: Vec::new(),
            kind: match raw.kind {
                ffi::KIND_FRAME => NodeKind::Frame,
                ffi::KIND_TEXT => NodeKind::Text,
                _ => NodeKind::Unsupported,
            },
            text,
            focusable: raw.events & (ffi::HAS_PRESS | ffi::HAS_RELEASE) != 0,
            style: Style::from_raw(&raw.style),
            hover: (raw.has_hover != 0).then(|| Style::from_raw(&raw.hover)),
            active: (raw.has_active != 0).then(|| Style::from_raw(&raw.active)),
        }
    }
}

/// Fills in each node's `children` from the parents.
pub(crate) fn link_children(nodes: &mut [UiNode]) {
    for index in 0..nodes.len() {
        if let Some(parent) = nodes[index].parent {
            nodes[parent].children.push(index);
        }
    }
}
