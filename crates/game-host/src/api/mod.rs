//! The engine API that game scripts call: the seam between game code (Nyx) and engine code
//! (Rust). Each module registers one group of host types. `docs/game-api.md` lists every call.
//!
//! Rules every host function follows:
//! - it borrows `EngineState` only for the length of the call and never calls into Nyx;
//! - handles cross as `i64` (entity ids, meshes, textures are generational handles packed into
//!   64 bits), so a script can never hold a dangling pointer; a stale handle throws
//!   `Error:::NotFound` in the script;
//! - vectors cross as `Vec3` / `Vec2` host objects, copied in and out.

pub mod assets;
pub mod audio;
pub mod debug;
pub mod engine;
pub mod game_ui;
pub mod input;
pub mod math;
pub mod overlay;
pub mod physics;
pub mod rendering;
pub mod world;

use std::cell::RefCell;
use std::rc::Rc;

use glam::{Vec2, Vec3};
use lantern_core::color::Color;
use lantern_core::handle::Handle;
use lantern_engine::world::EntityId;
use nyx::{CallContext, CallbackError, HostType, Runtime, Value};

use crate::engine_state::{EngineState, parse_color};

pub type SharedState = Rc<RefCell<EngineState>>;

/// The host types whose instances host functions create (a function returning a `Vec3` needs
/// the registered `Vec3` type to wrap it). Held outside the runtime and emptied before the
/// runtime is dropped: a `HostType` keeps its runtime alive, so a closure must not own one.
#[derive(Default)]
pub struct HostTypes {
    pub vec2: Option<HostType<math::ScriptVec2>>,
    pub vec3: Option<HostType<math::ScriptVec3>>,
    pub ray_hit: Option<HostType<physics::ScriptRayHit>>,
    pub controller_step: Option<HostType<physics::ScriptControllerStep>>,
}

pub type SharedTypes = Rc<RefCell<HostTypes>>;

/// Registers every engine type with `runtime`.
pub fn register_engine_api(
    runtime: &Runtime,
    state: &SharedState,
    types: &SharedTypes,
    game_ui: &game_ui::SharedGameUi,
) -> nyx::Result<()> {
    math::register(runtime, types)?;
    engine::register(runtime, state)?;
    input::register(runtime, state, types)?;
    world::register(runtime, state, types)?;
    assets::register(runtime, state)?;
    physics::register(runtime, state, types)?;
    rendering::register(runtime, state, types)?;
    audio::register(runtime, state)?;
    overlay::register(runtime, state)?;
    game_ui::register(runtime, game_ui)?;
    debug::register(runtime, state, types)?;
    Ok(())
}

pub fn argument(arguments: &[Value], index: usize) -> Result<&Value, CallbackError> {
    arguments.get(index).ok_or_else(|| CallbackError::Refusal(format!("missing argument {}", index + 1)))
}

pub fn f32_argument(arguments: &[Value], index: usize) -> Result<f32, CallbackError> {
    Ok(argument(arguments, index)?.as_f32()?)
}

pub fn i32_argument(arguments: &[Value], index: usize) -> Result<i32, CallbackError> {
    Ok(argument(arguments, index)?.as_i32()?)
}

pub fn i64_argument(arguments: &[Value], index: usize) -> Result<i64, CallbackError> {
    Ok(argument(arguments, index)?.as_i64()?)
}

pub fn bool_argument(arguments: &[Value], index: usize) -> Result<bool, CallbackError> {
    Ok(argument(arguments, index)?.as_bool()?)
}

pub fn string_argument(arguments: &[Value], index: usize) -> Result<String, CallbackError> {
    Ok(argument(arguments, index)?.as_str()?.to_string())
}

pub fn color_argument(context: &CallContext, arguments: &[Value], index: usize) -> Result<Color, CallbackError> {
    let text = string_argument(arguments, index)?;
    parse_color(&text)
        .ok_or_else(|| invalid(context, format!("'{text}' is not a colour; write \"RRGGBB\" or \"RRGGBBAA\"")))
}

pub fn vec3_argument(types: &SharedTypes, arguments: &[Value], index: usize) -> Result<Vec3, CallbackError> {
    let types = types.borrow();
    let vec3 = types.vec3.as_ref().ok_or_else(|| CallbackError::Refusal("Vec3 is not registered".to_string()))?;
    Ok(vec3.get(argument(arguments, index)?)?.0)
}

pub fn wrap_vec3(types: &SharedTypes, value: Vec3) -> Result<Option<Value>, CallbackError> {
    let types = types.borrow();
    let vec3 = types.vec3.as_ref().ok_or_else(|| CallbackError::Refusal("Vec3 is not registered".to_string()))?;
    Ok(Some(vec3.wrap(math::ScriptVec3(value))?))
}

pub fn wrap_vec2(types: &SharedTypes, value: Vec2) -> Result<Option<Value>, CallbackError> {
    let types = types.borrow();
    let vec2 = types.vec2.as_ref().ok_or_else(|| CallbackError::Refusal("Vec2 is not registered".to_string()))?;
    Ok(Some(vec2.wrap(math::ScriptVec2(value))?))
}

/// `Error:::NotFound` in the script: a name, id or file that does not exist.
pub fn not_found(context: &CallContext, message: impl AsRef<str>) -> CallbackError {
    context.failure("Error::NotFound", message.as_ref())
}

/// `Error:::Unknown` in the script: an argument the engine cannot use.
pub fn invalid(context: &CallContext, message: impl AsRef<str>) -> CallbackError {
    context.failure("Error::Unknown", message.as_ref())
}

pub fn entity_to_script(entity: EntityId) -> i64 {
    entity.to_bits() as i64
}

pub fn entity_from_script(id: i64) -> EntityId {
    Handle::from_bits(id as u64)
}

pub fn handle_to_script<T>(handle: Handle<T>) -> i64 {
    handle.to_bits() as i64
}

pub fn handle_from_script<T>(id: i64) -> Handle<T> {
    Handle::from_bits(id as u64)
}

pub fn some_f32(context: &CallContext, value: f32) -> Result<Option<Value>, CallbackError> {
    Ok(Some(context.make_f32(value)?))
}

pub fn some_bool(context: &CallContext, value: bool) -> Result<Option<Value>, CallbackError> {
    Ok(Some(context.make_bool(value)?))
}

pub fn some_i64(context: &CallContext, value: i64) -> Result<Option<Value>, CallbackError> {
    Ok(Some(context.make_i64(value)?))
}

pub fn some_string(context: &CallContext, value: &str) -> Result<Option<Value>, CallbackError> {
    Ok(Some(context.make_string(value)?))
}

pub fn null(context: &CallContext) -> Result<Option<Value>, CallbackError> {
    Ok(Some(context.make_null()?))
}
