//! Lantern's world file.
//!
//! A world file (`assets/worlds/<name>.world.json`) describes a level as data: a flat list of
//! entities, each with a stable id, a name, an optional parent, a local transform, an optional
//! solid shape, and a `kind` plus free-form `properties` that game code interprets. It is the
//! seam between three users that must not know about each other:
//!
//! - the **world editor** reads and writes it, and never knows what a gem or a spawn point means;
//! - the **game host** loads it and spawns entities, meshes and collision from it;
//! - **game scripts** (Nyx) find entities by `kind` and read their `properties`.
//!
//! This crate is pure data and maths: no SDL, no GPU, no engine `World`. It depends on
//! `lantern-renderer-3d` for `MeshData` and `lantern-physics` for `Triangle` so that
//! [`build_pieces`] can hand both users the exact same render and collision geometry.
//!
//! # Conventions
//!
//! Same as the engine (`docs/architecture.md`): right-handed, +Y up, -Z forward, metres.
//! Rotations are stored as Euler angles in **degrees**, `[x, y, z]`, applied yaw first, then
//! pitch, then roll (`glam::EulerRot::YXZ`), which is what an editor's inspector shows.

mod file;
mod pieces;

pub use file::{
    CURRENT_FORMAT_VERSION, ColliderKind, EntityKey, PropertyValue, Shape, TransformData, WorldEntity, WorldFile,
    WorldFileError,
};
pub use pieces::{WorldPiece, build_pieces, world_matrices};

/// The `kind` strings the demo course uses. Only game code gives them meaning; the editor treats
/// them as labels. They live here so the game host and tools spell them the same way.
pub mod kinds {
    /// A static box. Shape: `Box`.
    pub const BLOCK: &str = "block";
    /// A static wedge. Shape: `Ramp`.
    pub const RAMP: &str = "ramp";
    /// A box moved by game rules. Shape: `Box`, collider `Kinematic`.
    pub const MOVING_PLATFORM: &str = "moving_platform";
    /// A collectible. No shape: the game draws its own mesh at the entity's position.
    pub const GEM: &str = "gem";
    /// Where the player's feet start. No shape.
    pub const SPAWN: &str = "spawn";
    /// Falling below this entity's world-space height resets the player. No shape.
    pub const KILL_PLANE: &str = "kill_plane";
}
