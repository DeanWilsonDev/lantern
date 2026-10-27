//! One loaded game script: a Nyx runtime with the engine API registered, the mounted program,
//! and the hooks its `Application` class defines.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nyx::{Program, Runtime, Value};

use crate::api::{HostTypes, SharedState, SharedTypes, register_engine_api};

/// The hooks a game's `Application` class may define. Missing hooks are skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameHook {
    /// Once, after loading: define input, load assets, load the world.
    OnStart,
    /// Every fixed step (`dt` = 1/60 s). The simulation lives here.
    OnFixedUpdate,
    /// Once per display frame, before rendering (`frameSeconds`): menus' state, camera look.
    OnUpdate,
    /// Once per drawn frame (`alpha`, `frameSeconds`): camera, overlay text, menus, debug
    /// drawing. `alpha` blends the last two fixed steps.
    OnRender,
    /// Before the game is unloaded (quit or reload).
    OnExit,
}

impl GameHook {
    pub const ALL: [GameHook; 5] =
        [GameHook::OnStart, GameHook::OnFixedUpdate, GameHook::OnUpdate, GameHook::OnRender, GameHook::OnExit];

    pub fn method_name(self) -> &'static str {
        match self {
            GameHook::OnStart => "OnStart",
            GameHook::OnFixedUpdate => "OnFixedUpdate",
            GameHook::OnUpdate => "OnUpdate",
            GameHook::OnRender => "OnRender",
            GameHook::OnExit => "OnExit",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum HookArgument {
    F32(f32),
}

pub struct ScriptSession {
    // Drop order: the program before the runtime that owns its classes.
    program: Program,
    runtime: Runtime,
    types: SharedTypes,
    defined_hooks: Vec<GameHook>,
    entry_path: PathBuf,
    /// Time spent inside hooks since the last `take_script_time`, for the HUD.
    script_time: Duration,
}

impl Drop for ScriptSession {
    fn drop(&mut self) {
        // The registered types hold the runtime; let it go so the runtime can be destroyed.
        *self.types.borrow_mut() = HostTypes::default();
    }
}

impl ScriptSession {
    /// Mounts `entry_file` from `scripts_directory` (which is also the import search path) and
    /// instantiates `application_class`. Analysis errors come back rendered with file, line
    /// and column, and nothing runs.
    pub fn load(
        scripts_directory: &Path,
        entry_file: &str,
        application_class: &str,
        state: &SharedState,
    ) -> Result<ScriptSession, String> {
        let entry_path = scripts_directory.join(entry_file);
        let source =
            std::fs::read_to_string(&entry_path).map_err(|error| format!("{}: {error}", entry_path.display()))?;
        let runtime_error = |error: nyx::Error| format!("nyx: {error}");
        let runtime = Runtime::new().map_err(runtime_error)?;
        let types = SharedTypes::default();
        register_engine_api(&runtime, state, &types).map_err(runtime_error)?;
        runtime.add_import_search_path(scripts_directory).map_err(runtime_error)?;
        let program = match runtime.mount_file(&entry_path, application_class) {
            Ok(program) => program,
            Err(error) => {
                let diagnostics = runtime.render_diagnostics().unwrap_or_default();
                *types.borrow_mut() = HostTypes::default();
                return Err(if diagnostics.trim().is_empty() {
                    format!("{}: {error}", entry_path.display())
                } else {
                    diagnostics.trim_end().to_string()
                });
            }
        };
        let defined_hooks = GameHook::ALL
            .into_iter()
            .filter(|hook| {
                let name = hook.method_name();
                source.contains(&format!("{name}~:")) || source.contains(&format!("{name}::"))
            })
            .collect();
        Ok(ScriptSession { program, runtime, types, defined_hooks, entry_path, script_time: Duration::ZERO })
    }

    pub fn entry_path(&self) -> &Path {
        &self.entry_path
    }

    pub fn defines(&self, hook: GameHook) -> bool {
        self.defined_hooks.contains(&hook)
    }

    /// Calls `hook` on the application instance. The error, if any, is ready to show.
    pub fn call(&mut self, hook: GameHook, arguments: &[HookArgument]) -> Result<(), String> {
        if !self.defines(hook) {
            return Ok(());
        }
        let failed = |error: nyx::Error| format!("{}: {error}", hook.method_name());
        let values = arguments
            .iter()
            .map(|argument| match argument {
                HookArgument::F32(value) => self.runtime.make_f32(*value),
            })
            .collect::<nyx::Result<Vec<Value>>>()
            .map_err(failed)?;
        let started = Instant::now();
        let result = self.program.call_application_method(hook.method_name(), &values);
        self.script_time += started.elapsed();
        result.map(|_| ()).map_err(failed)
    }

    pub fn take_script_time(&mut self) -> Duration {
        std::mem::take(&mut self.script_time)
    }
}
