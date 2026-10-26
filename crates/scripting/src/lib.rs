//! Lantern's Nyx scripting host (decision D-002: Nyx through its C ABI, via the `nyx` crate).
//!
//! A game script is a Nyx class deriving `Application`. The host calls its hooks, and the
//! script reaches the engine through four host types with static methods:
//!
//! | Nyx | What it does |
//! |---|---|
//! | `Game:::Log(text)` | log a line (target `script`) |
//! | `Game:::Time() -> f32` | simulation seconds since the level started |
//! | `Game:::ResetPlayer()` | put the player back at the spawn point |
//! | `Game:::SetStatus(text)` | the status line at the top of the screen |
//! | `Game:::ShowMessage(text, seconds)` | a large message in the middle of the screen |
//! | `Input:::IsPressed(action) -> bool`, `Input:::IsHeld(action) -> bool` | named actions |
//! | `Entity:::Find(name) -> i64` | an entity id; throws `Error:::NotFound` if there is none |
//! | `Entity:::Position(id) -> Vector`, `Entity:::SetPosition(id, position)` | local position |
//! | `Entity:::IsVisible(id) -> bool`, `Entity:::SetVisible(id, visible)` | mesh visibility |
//! | `Sound:::Play(name, volume)` | a one-shot sound effect |
//! | `Vector:::New(x, y, z)`, `v.X()`, `v.Y()`, `v.Z()` | a 3D vector value |
//!
//! Scripts never hold pointers. Entities are `i64` ids (generational handles packed into 64
//! bits), checked on every call; a stale or unknown id throws `Error:::NotFound` in the script.
//!
//! Each hook call works on a `ScriptSnapshot`: a per-step copy of what scripts may read,
//! filled in by the game before the call. Writes become `ScriptCommand`s, which the game applies
//! after the call, in order. Writes also update the snapshot, so a script that sets a position
//! and reads it back in the same hook sees its own write.

#![forbid(unsafe_code)]

mod api;
mod host;

pub use api::{ScriptCommand, ScriptEntity, ScriptSnapshot};
pub use host::{HookArgument, ScriptError, ScriptHook, ScriptHost};
