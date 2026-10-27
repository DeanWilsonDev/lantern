//! The Lantern world editor's logic, with no window, GPU or UI toolkit.
//!
//! The editor is split like the engine (see `docs/editor.md`):
//!
//! - **this crate** holds every decision the editor makes: the open document and its undo
//!   history, selection, picking, gizmo maths, the viewport camera, the inspector's fields and
//!   the viewport's contents as a `RenderWorld`. It is plain Rust and fully unit-tested.
//! - **`lantern-editor-backend`** renders that `RenderWorld` off-screen with the engine's GPU
//!   renderer and exposes everything through a C API.
//! - **`apps/world-editor`** is the front end: panels written in Nyx, Chaos and Cosmos on
//!   Penumbra, plus a thin C++ shell that links the backend.
//!
//! Nothing here knows about the game. Game-specific content (what a gem looks like, which
//! templates the Add menu offers) comes from the palette file (`palette.rs`).

pub mod camera;
pub mod document;
pub mod gizmo;
pub mod inspector;
pub mod palette;
pub mod scene_geometry;
pub mod session;
pub mod viewport;

pub use camera::{OrbitCamera, Ray, ViewportProjection};
pub use document::{Edit, EditorDocument};
pub use gizmo::{Axis, GizmoTool, SnapSettings};
pub use inspector::{FieldKind, InspectorField};
pub use palette::{EntityTemplate, MarkerMesh, MarkerStyle, Palette};
pub use session::{EditorKey, EditorSession, Modifiers, OutlineRow, PointerButton};
pub use viewport::{ViewportResources, build_viewport};
