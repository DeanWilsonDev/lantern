//! One loaded game script: a Nyx runtime with the engine API registered, the mounted program,
//! and the hooks its `Application` class defines.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use lantern_engine::audio::AudioSystem;
use nyx::{Program, Runtime, Value};

use crate::api::{HostTypes, SharedState, SharedTypes, register_engine_api};
use crate::engine_state::EngineState;
use crate::render_backend::HeadlessRenderer;

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
    /// Time spent inside each hook (in `GameHook::ALL` order) and how often it ran, since loading.
    hook_totals: [(Duration, u64); 5],
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
        let mut defined_hooks = Vec::new();
        for hook in GameHook::ALL {
            if program.defines_method(hook.method_name()).map_err(runtime_error)? {
                defined_hooks.push(hook);
            }
        }
        Ok(ScriptSession {
            program,
            runtime,
            types,
            defined_hooks,
            entry_path,
            script_time: Duration::ZERO,
            hook_totals: [(Duration::ZERO, 0); 5],
        })
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
        let failed = |error: nyx::Error| describe_hook_error(hook, &error);
        let values = arguments
            .iter()
            .map(|argument| match argument {
                HookArgument::F32(value) => self.runtime.make_f32(*value),
            })
            .collect::<nyx::Result<Vec<Value>>>()
            .map_err(failed)?;
        let started = Instant::now();
        let result = self.program.call_application_method(hook.method_name(), &values);
        let elapsed = started.elapsed();
        self.script_time += elapsed;
        let total = &mut self.hook_totals[hook as usize];
        total.0 += elapsed;
        total.1 += 1;
        result.map(|_| ()).map_err(failed)
    }

    /// Total time inside `hook` since loading, and how many calls that was.
    pub fn hook_total(&self, hook: GameHook) -> (Duration, u64) {
        self.hook_totals[hook as usize]
    }

    pub fn take_script_time(&mut self) -> Duration {
        std::mem::take(&mut self.script_time)
    }
}

/// `OnFixedUpdate: NYX_ERROR_FAILURE: Error::NotFound: ... at Player.Step (player.nyx:48:9)`, then
/// one `called from` line per Nyx caller.
/// The engine API as a Nyx host surface manifest: every type and function a game script can
/// name, for `nyx check` and nyx-lsp to read through a `.nyx.json`'s `hostSurfaces`.
pub fn engine_host_surface() -> Result<String, String> {
    let runtime_error = |error: nyx::Error| format!("nyx: {error}");
    let state = EngineState::new(Box::new(HeadlessRenderer::default()), AudioSystem::new(None), None)?;
    let shared_state: SharedState = Rc::new(RefCell::new(state));
    let types = SharedTypes::default();
    let runtime = Runtime::new().map_err(runtime_error)?;
    register_engine_api(&runtime, &shared_state, &types).map_err(runtime_error)?;
    let manifest = runtime.write_host_surface().map_err(runtime_error);
    *types.borrow_mut() = HostTypes::default();
    manifest
}

fn describe_hook_error(hook: GameHook, error: &nyx::Error) -> String {
    let mut text = format!("{}: {error}", hook.method_name());
    for frame in error.stack.iter().skip(1) {
        text.push_str(&format!("\n    called from {frame}"));
    }
    text
}
