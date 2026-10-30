//! Lantern's in-game UI in Chaos and Cosmos (decision D-043).
//!
//! The game writes its menus and HUD as Chaos components (`.chaos`, Nyx with element trees)
//! styled by Cosmos stylesheets (`.cosmos`). Chaos and Cosmos are C++ libraries, so a small C++
//! bridge (`native/`) runs them: it keeps the mounted components as plain nodes behind
//! `Umbra::IWidget`, the backend-neutral contract Chaos reconciles against, and resolves each
//! node's style. This crate does the engine's half in Rust: it reads the nodes, lays them out
//! (`layout`), draws them into the overlay (`draw`) and moves menu focus, sending `onFocus`
//! and `onRelease` back to the components.
//!
//! The bridge runs on the game's own Nyx runtime, so components and game code share objects:
//! a component reads a game object's `@signal` fields, and re-renders when the game writes them.
//!
//! Frame order, as `lantern-game-host` drives it: `navigate` just before the game's `OnUpdate`
//! (handlers run then, outside any host call), then `update` and `draw` after `OnRender`. The
//! mouse hits the boxes of the last `draw`.

mod draw;
mod ffi;
pub mod layout;
pub mod node;
pub mod style;

use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use glam::Vec2;
use lantern_renderer_3d::render_world::Overlay;

pub use layout::Rect;
pub use node::{NodeKind, UiNode};
pub use style::Style;

/// Menu navigation for one frame: edges from the game's own actions, and the mouse.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Navigation {
    pub previous: bool,
    pub next: bool,
    pub confirm: bool,
    /// The mouse, when it is free (not captured by the game).
    pub pointer: Option<Pointer>,
}

/// The mouse for one frame, in the pixels `draw` lays out in.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pointer {
    pub position: Vec2,
    /// The left button went down this frame.
    pub pressed: bool,
    /// The left button went up this frame (it may also have gone down: a quick click).
    pub released: bool,
    /// The left button is down at the end of the frame.
    pub held: bool,
}

#[derive(Default)]
struct FrameState {
    nodes: Vec<UiNode>,
    /// The focused button's id. Kept across frames; when that element goes away (a screen
    /// change), focus falls to the first button without an `onFocus` call, so a menu opening
    /// doesn't sound like a move.
    focused: Option<u64>,
    /// The focused button is held down this frame (confirm, or the mouse on it): drawn with
    /// `:active`.
    pressed: bool,
    reported_messages: usize,
    /// Each node's box from the last `draw`, for the mouse to hit.
    rects: Vec<Rect>,
    /// Where the pointer was last frame: only a moving pointer takes focus from the keyboard.
    last_pointer: Option<Vec2>,
    /// The button the left mouse button went down on, until it comes up.
    pointer_target: Option<u64>,
}

/// Chaos's reactive runtime is one per process (chaos board, `lantern-in-game-ui-gaps`), so only
/// one `ChaosUi` may exist at a time. Each holds this lock for its lifetime; another thread's
/// `ChaosUi::new` waits for it.
static ONE_CHAOS_UI: Mutex<()> = Mutex::new(());

thread_local! {
    /// Whether this thread holds `ONE_CHAOS_UI`, so a second `new` on it fails instead of
    /// waiting for itself.
    static HOLDS_CHAOS_UI: Cell<bool> = const { Cell::new(false) };
}

/// One game's Chaos runtime and everything it has mounted. Every method takes `&self`, because
/// a mount, a tick or an event runs Nyx, and Nyx may call back into the engine (and from there
/// into this object) before it returns.
pub struct ChaosUi {
    raw: *mut ffi::lantern_chaos_ui,
    frame: RefCell<FrameState>,
    // Released after the bridge is destroyed (Drop runs before fields drop).
    _only_one: MutexGuard<'static, ()>,
}

impl Drop for ChaosUi {
    fn drop(&mut self) {
        unsafe { ffi::lantern_chaos_ui_destroy(self.raw) };
        HOLDS_CHAOS_UI.with(|holds| holds.set(false));
    }
}

fn c_string(text: &str, what: &str) -> Result<CString, String> {
    CString::new(text).map_err(|_| format!("{what} '{text}' contains a NUL byte"))
}

fn path_string(path: &Path) -> Result<CString, String> {
    let text = path.to_str().ok_or_else(|| format!("the path {} isn't valid UTF-8", path.display()))?;
    c_string(text, "path")
}

impl FrameState {
    fn focus(&mut self, button: u64, events: &mut Vec<(u64, i32)>) {
        if self.focused != Some(button) {
            self.focused = Some(button);
            events.push((button, ffi::EVENT_FOCUS));
        }
    }

    /// The button under `point` in the last drawn frame: the last one in drawing order, so a
    /// button drawn over another wins.
    fn button_at(&self, point: Vec2) -> Option<u64> {
        if self.rects.len() != self.nodes.len() {
            return None;
        }
        self.nodes
            .iter()
            .zip(&self.rects)
            .rev()
            .find(|(node, rect)| node.focusable && rect.contains(point))
            .map(|(node, _)| node.id)
    }

    fn follow_pointer(&mut self, pointer: Pointer, events: &mut Vec<(u64, i32)>) {
        let under_pointer = self.button_at(pointer.position);
        let moved = self.last_pointer != Some(pointer.position);
        self.last_pointer = Some(pointer.position);
        if let Some(button) = under_pointer
            && (moved || pointer.pressed)
        {
            self.focus(button, events);
        }
        if pointer.pressed {
            self.pointer_target = under_pointer;
            if let Some(button) = under_pointer {
                events.push((button, ffi::EVENT_PRESS));
            }
        }
        let Some(target) = self.pointer_target else { return };
        if pointer.released {
            self.pointer_target = None;
            if under_pointer == Some(target) {
                events.push((target, ffi::EVENT_RELEASE));
            }
        } else if under_pointer == Some(target) {
            self.pressed = true;
        }
    }
}

impl ChaosUi {
    /// Starts Chaos on `runtime` and loads the `.cosmos` files in `ui_directory`. `.chaos`
    /// imports resolve from `project_root`. Call it before mounting the game, so the game's own
    /// classes can declare `@signal` fields, and drop it before the runtime.
    ///
    /// Only one exists per process at a time: on another thread this waits until the current
    /// one is dropped, and on the same thread it fails.
    pub fn new(runtime: &nyx::Runtime, project_root: &Path, ui_directory: &Path) -> Result<ChaosUi, String> {
        let root = path_string(project_root)?;
        let ui = path_string(ui_directory)?;
        if HOLDS_CHAOS_UI.with(Cell::get) {
            return Err("this thread already has a Chaos UI; Chaos allows one per process at a time".to_string());
        }
        // A panic while another ChaosUi was alive poisons the lock; the runtime it guards was
        // torn down by that ChaosUi's Drop all the same.
        let only_one = ONE_CHAOS_UI.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut raw = std::ptr::null_mut();
        let status = unsafe { ffi::lantern_chaos_ui_create(runtime.as_raw(), root.as_ptr(), ui.as_ptr(), &mut raw) };
        if status != ffi::LANTERN_CHAOS_UI_OK || raw.is_null() {
            return Err("Chaos couldn't start on the game's Nyx runtime".to_string());
        }
        HOLDS_CHAOS_UI.with(|holds| holds.set(true));
        Ok(ChaosUi { raw, frame: RefCell::new(FrameState::default()), _only_one: only_one })
    }

    /// Mounts `component` from `file` (relative to the project root) with `args` as its
    /// arguments. Returns a number for `unmount`.
    pub fn mount(&self, file: &str, component: &str, args: &[nyx::Value]) -> Result<u32, String> {
        let file_text = c_string(file, "file")?;
        let component_text = c_string(component, "component")?;
        let raw_args: Vec<*mut nyx_sys::nyx_value> = args.iter().map(|value| value.as_raw()).collect();
        let mut mount = 0;
        let status = unsafe {
            ffi::lantern_chaos_ui_mount(
                self.raw,
                file_text.as_ptr(),
                component_text.as_ptr(),
                raw_args.as_ptr(),
                raw_args.len(),
                &mut mount,
            )
        };
        if status != ffi::LANTERN_CHAOS_UI_OK {
            let first = self.frame.borrow().reported_messages;
            let messages = self.messages_since(first);
            return Err(format!("mounting {component} from {file} failed\n{}", messages.join("\n"))
                .trim_end()
                .to_string());
        }
        Ok(mount)
    }

    pub fn unmount(&self, mount: u32) -> bool {
        unsafe { ffi::lantern_chaos_ui_unmount(self.raw, mount) == ffi::LANTERN_CHAOS_UI_OK }
    }

    /// Moves focus and sends events for this frame's navigation edges and mouse. Event
    /// handlers (Nyx) run inside this call.
    ///
    /// The mouse: moving over a button focuses it; the left button going down on a button
    /// sends `onPress` and holds it `:active` while the pointer stays on it; coming up on the same
    /// button sends `onRelease`, and anywhere else cancels.
    pub fn navigate(&self, navigation: Navigation) {
        let events = {
            let mut frame = self.frame.borrow_mut();
            frame.pressed = false;
            let buttons: Vec<u64> = frame.nodes.iter().filter(|node| node.focusable).map(|node| node.id).collect();
            if buttons.is_empty() {
                frame.focused = None;
                frame.pointer_target = None;
                frame.last_pointer = navigation.pointer.map(|pointer| pointer.position);
                return;
            }
            let mut events = Vec::new();
            if frame.focused.is_none_or(|id| !buttons.contains(&id)) {
                frame.focused = Some(buttons[0]);
            }
            if navigation.previous != navigation.next {
                let position =
                    frame.focused.and_then(|id| buttons.iter().position(|&button| button == id)).unwrap_or(0);
                let position = if navigation.next {
                    (position + 1) % buttons.len()
                } else {
                    (position + buttons.len() - 1) % buttons.len()
                };
                frame.focus(buttons[position], &mut events);
            }
            match navigation.pointer {
                Some(pointer) => frame.follow_pointer(pointer, &mut events),
                None => {
                    frame.pointer_target = None;
                    frame.last_pointer = None;
                }
            }
            if navigation.confirm
                && let Some(button) = frame.focused
            {
                frame.pressed = true;
                events.push((button, ffi::EVENT_PRESS));
                events.push((button, ffi::EVENT_RELEASE));
            }
            events
        };
        for (node, event) in events {
            unsafe { ffi::lantern_chaos_ui_dispatch(self.raw, node, event) };
        }
    }

    /// Re-renders the components whose signals changed and reads every node back with its
    /// resolved style.
    pub fn update(&self) {
        unsafe { ffi::lantern_chaos_ui_tick(self.raw) };
        let mut raw_nodes = std::ptr::null();
        let mut count = 0;
        let status = unsafe { ffi::lantern_chaos_ui_snapshot(self.raw, &mut raw_nodes, &mut count) };
        let mut nodes = Vec::with_capacity(count);
        if status == ffi::LANTERN_CHAOS_UI_OK && !raw_nodes.is_null() {
            let raw = unsafe { std::slice::from_raw_parts(raw_nodes, count) };
            nodes.extend(raw.iter().map(|node| unsafe { UiNode::from_raw(node) }));
        }
        node::link_children(&mut nodes);
        self.frame.borrow_mut().nodes = nodes;
    }

    /// The nodes as of the last `update`, a parent before its children.
    pub fn nodes(&self) -> Vec<UiNode> {
        self.frame.borrow().nodes.clone()
    }

    /// Each button's id and box as of the last `draw`, in drawing order.
    pub fn button_rects(&self) -> Vec<(u64, Rect)> {
        let frame = self.frame.borrow();
        if frame.rects.len() != frame.nodes.len() {
            return Vec::new();
        }
        frame
            .nodes
            .iter()
            .zip(&frame.rects)
            .filter(|(node, _)| node.focusable)
            .map(|(node, rect)| (node.id, *rect))
            .collect()
    }

    pub fn focused(&self) -> Option<u64> {
        self.frame.borrow().focused
    }

    /// Lays out and draws the nodes from the last `update` into `overlay`. `screen` is the
    /// window in physical pixels and `scale` the UI scale (physical pixels per logical pixel).
    /// The focused button and what is inside it are drawn with their `:hover` styles, or
    /// `:active` while confirmed.
    pub fn draw(&self, overlay: &mut Overlay, screen: Vec2, scale: f32) -> Vec<Rect> {
        let rects = self.draw_frame(overlay, screen, scale);
        self.frame.borrow_mut().rects = rects.clone();
        rects
    }

    fn draw_frame(&self, overlay: &mut Overlay, screen: Vec2, scale: f32) -> Vec<Rect> {
        let frame = self.frame.borrow();
        // The focused button and everything inside it count as hovered (and pressed, while
        // confirm is down), so a label can change with its button: `.label:hover { }`. Parents
        // come before children, so one pass carries it down.
        let mut in_focus = vec![false; frame.nodes.len()];
        for (index, node) in frame.nodes.iter().enumerate() {
            in_focus[index] = frame.focused == Some(node.id) || node.parent.is_some_and(|parent| in_focus[parent]);
        }
        let styles: Vec<&Style> = frame
            .nodes
            .iter()
            .zip(&in_focus)
            .map(|(node, &focused)| match (focused, frame.pressed) {
                (true, true) => node.active.as_ref().or(node.hover.as_ref()).unwrap_or(&node.style),
                (true, false) => node.hover.as_ref().unwrap_or(&node.style),
                _ => &node.style,
            })
            .collect();
        let rects = layout::layout(&frame.nodes, &styles, screen, scale);
        draw::draw(&frame.nodes, &styles, &rects, overlay, scale);
        rects
    }

    fn messages_since(&self, first: usize) -> Vec<String> {
        let count = unsafe { ffi::lantern_chaos_ui_diagnostic_count(self.raw) };
        (first..count)
            .filter_map(|index| {
                let mut text = std::ptr::null();
                let mut length = 0;
                let status = unsafe { ffi::lantern_chaos_ui_diagnostic(self.raw, index, &mut text, &mut length) };
                if status != ffi::LANTERN_CHAOS_UI_OK || text.is_null() {
                    return None;
                }
                let bytes = unsafe { std::slice::from_raw_parts(text.cast::<u8>(), length) };
                Some(String::from_utf8_lossy(bytes).into_owned())
            })
            .collect()
    }

    /// What Chaos and Cosmos reported since the last call (compile errors, failed handlers,
    /// stylesheet problems), each ready to log.
    pub fn take_messages(&self) -> Vec<String> {
        let first = self.frame.borrow().reported_messages;
        let messages = self.messages_since(first);
        self.frame.borrow_mut().reported_messages = first + messages.len();
        messages
    }
}
