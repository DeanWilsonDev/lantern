//! Backend-neutral rendering data.
//!
//! The game describes a frame as a `RenderWorld`: a camera, a light, a list of draw items
//! (mesh + texture + transform + colour), debug lines and a 2D overlay. A backend
//! (`lantern-sdl-gpu`) turns that into GPU commands. Nothing in this crate knows about a GPU API,
//! so it is all unit-testable.

#![forbid(unsafe_code)]

pub mod camera;
pub mod gltf_loader;
pub mod mesh;
pub mod primitives;
pub mod render_world;
pub mod text;
pub mod texture;

mod font_data;

pub use camera::Camera;
pub use mesh::{MeshData, Vertex};
pub use render_world::{DirectionalLight, DrawItem, LineVertex, MeshHandle, OverlayVertex, RenderWorld, TextureHandle};
pub use texture::TextureData;
