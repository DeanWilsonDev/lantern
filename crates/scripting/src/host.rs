use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use glam::Vec3;
use nyx::{CallContext, CallbackError, Program, Receiver, Runtime, Value};

use crate::api::{CallState, ScriptCommand, ScriptSnapshot};

/// The hooks a game script may define. Missing hooks are skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptHook {
    OnStart,
    OnFixedUpdate,
    OnTriggerEnter,
    OnPlayerFell,
    OnJump,
    OnLand,
    OnResetPressed,
    OnExit,
}

impl ScriptHook {
    pub fn method_name(self) -> &'static str {
        match self {
            ScriptHook::OnStart => "OnStart",
            ScriptHook::OnFixedUpdate => "OnFixedUpdate",
            ScriptHook::OnTriggerEnter => "OnTriggerEnter",
            ScriptHook::OnPlayerFell => "OnPlayerFell",
            ScriptHook::OnJump => "OnJump",
            ScriptHook::OnLand => "OnLand",
            ScriptHook::OnResetPressed => "OnResetPressed",
            ScriptHook::OnExit => "OnExit",
        }
    }
}

/// A script problem, ready to show: loading (with rendered diagnostics) or a failed hook.
#[derive(Clone, Debug)]
pub struct ScriptError {
    pub message: String,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScriptError {}

/// A 3D vector as a Nyx value (`Vector:::New(x, y, z)`).
struct ScriptVector(Vec3);

/// Marker types: `Game`, `Input`, `Entity` and `Sound` only carry static methods.
struct GameApi;
struct InputApi;
struct EntityApi;
struct SoundApi;

fn argument(arguments: &[Value], index: usize) -> Result<&Value, CallbackError> {
    arguments.get(index).ok_or_else(|| CallbackError::Refusal(format!("missing argument {}", index + 1)))
}

fn string_argument(arguments: &[Value], index: usize) -> Result<String, CallbackError> {
    Ok(argument(arguments, index)?.as_str()?.to_string())
}

fn not_found(context: &CallContext, what: String) -> CallbackError {
    context.failure("Error::NotFound", &what)
}

/// Mounts a game script and calls its hooks.
pub struct ScriptHost {
    // Drop order: the program before the runtime that owns its classes.
    program: Program,
    runtime: Runtime,
    state: Rc<RefCell<CallState>>,
    source_path: PathBuf,
    application_class: String,
    defined_hooks: Vec<ScriptHook>,
}

impl ScriptHost {
    /// Loads `path` and instantiates `application_class`. Analysis errors come back rendered
    /// with file, line and column, and nothing runs.
    pub fn load(path: &Path, application_class: &str) -> Result<ScriptHost, ScriptError> {
        let source =
            std::fs::read_to_string(path).map_err(|io| ScriptError { message: format!("{}: {io}", path.display()) })?;
        let name =
            path.file_name().map_or_else(|| "script.nyx".to_string(), |name| name.to_string_lossy().into_owned());
        let mut host = ScriptHost::load_source(&source, &name, application_class)?;
        host.source_path = path.to_path_buf();
        Ok(host)
    }

    /// Like `load`, from a string (tests, embedded scripts).
    pub fn load_source(source: &str, name: &str, application_class: &str) -> Result<ScriptHost, ScriptError> {
        let runtime_error = |error: nyx::Error| ScriptError { message: format!("nyx: {error}") };
        let runtime = Runtime::new().map_err(runtime_error)?;
        let state = Rc::new(RefCell::new(CallState::default()));
        register_engine_api(&runtime, &state).map_err(runtime_error)?;
        let program = match runtime.mount_source(source, name, application_class) {
            Ok(program) => program,
            Err(error) => {
                let diagnostics = runtime.render_diagnostics().unwrap_or_default();
                let message = if diagnostics.trim().is_empty() {
                    format!("{name}: {error}")
                } else {
                    diagnostics.trim_end().to_string()
                };
                return Err(ScriptError { message });
            }
        };
        let defined_hooks = probe_defined_hooks(source);
        Ok(ScriptHost {
            program,
            runtime,
            state,
            source_path: PathBuf::from(name),
            application_class: application_class.to_string(),
            defined_hooks,
        })
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn application_class(&self) -> &str {
        &self.application_class
    }

    pub fn defines(&self, hook: ScriptHook) -> bool {
        self.defined_hooks.contains(&hook)
    }

    /// Calls `hook` with `snapshot` as the readable world. Returns the commands the script
    /// wrote, also when the hook fails part-way (commands written before the failure stand,
    /// as statements already executed would in any language).
    pub fn call(
        &mut self,
        hook: ScriptHook,
        snapshot: ScriptSnapshot,
        arguments: &[HookArgument],
    ) -> (Vec<ScriptCommand>, Result<(), ScriptError>) {
        if !self.defines(hook) {
            return (Vec::new(), Ok(()));
        }
        {
            let mut state = self.state.borrow_mut();
            state.snapshot = snapshot;
            state.commands.clear();
        }
        let result = self.call_hook(hook, arguments);
        let commands = std::mem::take(&mut self.state.borrow_mut().commands);
        (commands, result)
    }

    fn call_hook(&self, hook: ScriptHook, arguments: &[HookArgument]) -> Result<(), ScriptError> {
        let failed = |error: nyx::Error| ScriptError { message: format!("{}: {error}", hook.method_name()) };
        let values = arguments
            .iter()
            .map(|argument| match argument {
                HookArgument::F32(value) => self.runtime.make_f32(*value),
                HookArgument::I64(value) => self.runtime.make_i64(*value),
            })
            .collect::<Result<Vec<Value>, nyx::Error>>()
            .map_err(failed)?;
        self.program.call_application_method(hook.method_name(), &values).map_err(failed)?;
        Ok(())
    }
}

/// A hook argument (only the kinds the hooks use).
#[derive(Clone, Copy, Debug)]
pub enum HookArgument {
    F32(f32),
    I64(i64),
}

/// Which hooks the script defines. The C API reports a missing method only when it is called,
/// so the host looks for `Name~:` / `Name::` definitions once at load instead, and never calls
/// a hook that is not there.
fn probe_defined_hooks(source: &str) -> Vec<ScriptHook> {
    [
        ScriptHook::OnStart,
        ScriptHook::OnFixedUpdate,
        ScriptHook::OnTriggerEnter,
        ScriptHook::OnPlayerFell,
        ScriptHook::OnJump,
        ScriptHook::OnLand,
        ScriptHook::OnResetPressed,
        ScriptHook::OnExit,
    ]
    .into_iter()
    .filter(|hook| {
        let name = hook.method_name();
        source.contains(&format!("{name}~:")) || source.contains(&format!("{name}::"))
    })
    .collect()
}

fn push_command(state: &Rc<RefCell<CallState>>, command: ScriptCommand) {
    state.borrow_mut().commands.push(command);
}

fn register_engine_api(runtime: &Runtime, state: &Rc<RefCell<CallState>>) -> nyx::Result<()> {
    let vector_type = runtime
        .register_type::<ScriptVector>("Vector")
        .static_method("New", "(x: f32, y: f32, z: f32) -> Vector", |_context, vector_type, arguments| {
            let component = |index| -> Result<f32, CallbackError> { Ok(argument(arguments, index)?.as_f32()?) };
            let vector = Vec3::new(component(0)?, component(1)?, component(2)?);
            Ok(Some(vector_type.wrap(ScriptVector(vector))?))
        })
        .method("X", "() -> f32", Receiver::Immutable, |context, vector, _| Ok(Some(context.make_f32(vector.0.x)?)))
        .method("Y", "() -> f32", Receiver::Immutable, |context, vector, _| Ok(Some(context.make_f32(vector.0.y)?)))
        .method("Z", "() -> f32", Receiver::Immutable, |context, vector, _| Ok(Some(context.make_f32(vector.0.z)?)))
        .register()?;

    let log_state = state.clone();
    let reset_state = state.clone();
    let status_state = state.clone();
    let message_state = state.clone();
    let time_state = state.clone();
    runtime
        .register_type::<GameApi>("Game")
        .static_method("Log", "(text: string) -> void", move |_context, _, arguments| {
            push_command(&log_state, ScriptCommand::Log { text: string_argument(arguments, 0)? });
            Ok(None)
        })
        .static_method("Time", "() -> f32", move |context, _, _| {
            Ok(Some(context.make_f32(time_state.borrow().snapshot.time_seconds)?))
        })
        .static_method("ResetPlayer", "() -> void", move |_context, _, _| {
            push_command(&reset_state, ScriptCommand::ResetPlayer);
            Ok(None)
        })
        .static_method("SetStatus", "(text: string) -> void", move |_context, _, arguments| {
            push_command(&status_state, ScriptCommand::SetStatus { text: string_argument(arguments, 0)? });
            Ok(None)
        })
        .static_method("ShowMessage", "(text: string, seconds: f32) -> void", move |_context, _, arguments| {
            let text = string_argument(arguments, 0)?;
            let seconds = argument(arguments, 1)?.as_f32()?;
            push_command(&message_state, ScriptCommand::ShowMessage { text, seconds });
            Ok(None)
        })
        .register()?;

    let pressed_state = state.clone();
    let held_state = state.clone();
    runtime
        .register_type::<InputApi>("Input")
        .static_method("IsPressed", "(action: string) -> bool", move |context, _, arguments| {
            let action = string_argument(arguments, 0)?;
            Ok(Some(context.make_bool(pressed_state.borrow().snapshot.pressed_actions.contains(&action))?))
        })
        .static_method("IsHeld", "(action: string) -> bool", move |context, _, arguments| {
            let action = string_argument(arguments, 0)?;
            Ok(Some(context.make_bool(held_state.borrow().snapshot.held_actions.contains(&action))?))
        })
        .register()?;

    let find_state = state.clone();
    let position_state = state.clone();
    let set_position_state = state.clone();
    let visible_state = state.clone();
    let set_visible_state = state.clone();
    let position_vector_type = vector_type.clone();
    let set_position_vector_type = vector_type;
    runtime
        .register_type::<EntityApi>("Entity")
        .static_method("Find", "(name: string) -> i64", move |context, _, arguments| {
            let name = string_argument(arguments, 0)?;
            match find_state.borrow().snapshot.entity_ids_by_name.get(&name) {
                Some(&id) => Ok(Some(context.make_i64(id)?)),
                None => Err(not_found(context, format!("no entity named '{name}'"))),
            }
        })
        .static_method("Position", "(entity: i64) -> Vector", move |context, _, arguments| {
            let id = argument(arguments, 0)?.as_i64()?;
            let position = position_state.borrow().snapshot.entities.get(&id).map(|entity| entity.position);
            match position {
                Some(position) => Ok(Some(position_vector_type.wrap(ScriptVector(position))?)),
                None => Err(not_found(context, format!("no entity with id {id}"))),
            }
        })
        .static_method("SetPosition", "(entity: i64, position: Vector) -> void", move |context, _, arguments| {
            let id = argument(arguments, 0)?.as_i64()?;
            let position = set_position_vector_type.get(argument(arguments, 1)?)?.0;
            let mut state = set_position_state.borrow_mut();
            let Some(entity) = state.snapshot.entities.get_mut(&id) else {
                return Err(not_found(context, format!("no entity with id {id}")));
            };
            entity.position = position;
            state.commands.push(ScriptCommand::SetEntityPosition { entity: id, position });
            Ok(None)
        })
        .static_method("IsVisible", "(entity: i64) -> bool", move |context, _, arguments| {
            let id = argument(arguments, 0)?.as_i64()?;
            let visible = visible_state.borrow().snapshot.entities.get(&id).map(|entity| entity.visible);
            match visible {
                Some(visible) => Ok(Some(context.make_bool(visible)?)),
                None => Err(not_found(context, format!("no entity with id {id}"))),
            }
        })
        .static_method("SetVisible", "(entity: i64, visible: bool) -> void", move |context, _, arguments| {
            let id = argument(arguments, 0)?.as_i64()?;
            let visible = argument(arguments, 1)?.as_bool()?;
            let mut state = set_visible_state.borrow_mut();
            let Some(entity) = state.snapshot.entities.get_mut(&id) else {
                return Err(not_found(context, format!("no entity with id {id}")));
            };
            entity.visible = visible;
            state.commands.push(ScriptCommand::SetEntityVisible { entity: id, visible });
            Ok(None)
        })
        .register()?;

    let sound_state = state.clone();
    runtime
        .register_type::<SoundApi>("Sound")
        .static_method("Play", "(name: string, volume: f32) -> void", move |_context, _, arguments| {
            let name = string_argument(arguments, 0)?;
            let volume = argument(arguments, 1)?.as_f32()?;
            push_command(&sound_state, ScriptCommand::PlaySound { name, volume });
            Ok(None)
        })
        .register()?;
    Ok(())
}
