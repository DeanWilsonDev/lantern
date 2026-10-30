//! Lantern's 3D collision and character movement.
//!
//! - `geometry`: closest-point queries between points, segments and triangles.
//! - `world`: static triangles in a uniform grid, kinematic bodies (moving platforms), and the
//!   queries: capsule cast, sphere cast (a capsule with no segment), overlaps and rays.
//! - `controller`: the kinematic character controller used by the player.
//!
//! There are no rigid bodies, joints or dynamic simulation: the brief puts a general physics
//! engine out of scope, and a platformer only needs a character that moves through static and
//! moving geometry.

#![forbid(unsafe_code)]

pub mod controller;
pub mod geometry;
pub mod world;

pub use controller::{CharacterController, ControllerEvents, ControllerInput, ControllerSettings};
pub use geometry::Triangle;
pub use world::{BodyHandle, Capsule, ColliderId, CollisionWorld, Contact, RayHit, ShapeHit, box_triangles};
