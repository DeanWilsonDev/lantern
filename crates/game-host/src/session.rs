//! One loaded game script: a Nyx runtime with the engine API registered, the mounted program,
//! and the hooks its `Application` class defines.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use glam::Vec2;
use lantern_chaos_ui::{ChaosUi, Navigation};
use lantern_engine::audio::AudioSystem;
use lantern_renderer_3d::render_world::Overlay;
use nyx::{Program, Runtime, Value};

use crate::api::game_ui::{GameUiState, NavigationActions, SharedGameUi};
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
    /// The game's Chaos UI (`GameUi`), shared with the host functions.
    game_ui: SharedGameUi,
    defined_hooks: Vec<GameHook>,
    entry_path: PathBuf,
    /// Time spent inside hooks since the last `take_script_time`, for the HUD.
    script_time: Duration,
    /// Time spent inside each hook (in `GameHook::ALL` order) and how often it ran, since loading.
    hook_totals: [(Duration, u64); 5],
}

impl Drop for ScriptSession {
    fn drop(&mut self) {
        // The UI first: its components hold the game's objects. Then the registered types,
        // which hold the runtime; let them go so the runtime can be destroyed.
        let game_ui = std::mem::take(&mut *self.game_ui.borrow_mut());
        drop(game_ui);
        *self.types.borrow_mut() = HostTypes::default();
    }
}

/// Starts Chaos on `runtime` for the game in `scripts_directory`, its stylesheets in `ui/`.
fn start_chaos(runtime: &Runtime, scripts_directory: &Path) -> Result<ChaosUi, String> {
    ChaosUi::new(runtime, scripts_directory, &scripts_directory.join("ui"))
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
        let game_ui = SharedGameUi::default();
        register_engine_api(&runtime, state, &types, &game_ui).map_err(runtime_error)?;
        // On any failure below, empty what the host functions share: the runtime holds those
        // closures and they hold the Chaos UI and the types, so leaving them would keep the Chaos
        // UI (one per process) alive and stop every reload.
        let release_shared = || {
            *game_ui.borrow_mut() = GameUiState::default();
            *types.borrow_mut() = HostTypes::default();
        };
        if let Err(error) = runtime.add_import_search_path(scripts_directory) {
            release_shared();
            return Err(runtime_error(error));
        }
        // Before mounting, so the game's classes can declare `@signal` fields.
        match start_chaos(&runtime, scripts_directory) {
            Ok(chaos) => game_ui.borrow_mut().chaos = Some(Rc::new(chaos)),
            Err(error) => {
                release_shared();
                return Err(error);
            }
        }
        let program = match runtime.mount_file(&entry_path, application_class) {
            Ok(program) => program,
            Err(error) => {
                let diagnostics = runtime.render_diagnostics().unwrap_or_default();
                release_shared();
                return Err(if diagnostics.trim().is_empty() {
                    format!("{}: {error}", entry_path.display())
                } else {
                    diagnostics.trim_end().to_string()
                });
            }
        };
        let find_hooks = || -> nyx::Result<(Value, Vec<GameHook>)> {
            let application = program.application()?;
            let mut defined_hooks = Vec::new();
            for hook in GameHook::ALL {
                if program.defines_method(hook.method_name())? {
                    defined_hooks.push(hook);
                }
            }
            Ok((application, defined_hooks))
        };
        let (application, defined_hooks) = match find_hooks() {
            Ok(found) => found,
            Err(error) => {
                drop(program);
                release_shared();
                return Err(runtime_error(error));
            }
        };
        game_ui.borrow_mut().application = Some(application);
        Ok(ScriptSession {
            program,
            runtime,
            types,
            game_ui,
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

    /// The actions the game named with `GameUi:::SetNavigation`, if it has.
    pub fn ui_navigation_actions(&self) -> Option<NavigationActions> {
        self.game_ui.borrow().navigation.clone()
    }

    /// Moves menu focus and sends `onFocus`/`onRelease` to the game's UI for this frame's
    /// navigation edges. Handlers are game code, so their time counts as script time. Run it
    /// before `OnUpdate`, with no borrow of the engine state held: the handlers call the engine.
    pub fn navigate_ui(&mut self, navigation: Navigation) {
        let chaos = self.game_ui.borrow().chaos.clone();
        let Some(chaos) = chaos else { return };
        let started = Instant::now();
        chaos.navigate(navigation);
        self.script_time += started.elapsed();
    }

    /// Re-renders what the game's signals changed and reads the UI back for drawing. Run it
    /// after `OnRender`, with no borrow of the engine state held.
    pub fn update_ui(&mut self) {
        let chaos = self.game_ui.borrow().chaos.clone();
        if let Some(chaos) = chaos {
            let started = Instant::now();
            chaos.update();
            self.script_time += started.elapsed();
        }
    }

    /// Lays out and draws the UI from the last `update_ui` into `overlay`.
    pub fn draw_ui(&self, overlay: &mut Overlay, screen: Vec2, scale: f32) {
        if let Some(chaos) = self.game_ui.borrow().chaos.clone() {
            chaos.draw(overlay, screen, scale);
        }
    }

    /// The UI's nodes as of the last `update_ui` (tests look at the menus through this).
    pub fn ui_nodes(&self) -> Vec<lantern_chaos_ui::UiNode> {
        self.game_ui.borrow().chaos.as_ref().map(|chaos| chaos.nodes()).unwrap_or_default()
    }

    /// The UI's buttons and their boxes in window pixels, as last drawn (tests click through this).
    pub fn ui_button_rects(&self) -> Vec<(u64, lantern_chaos_ui::Rect)> {
        self.game_ui.borrow().chaos.as_ref().map(|chaos| chaos.button_rects()).unwrap_or_default()
    }

    /// What Chaos and Cosmos reported since the last call: component and stylesheet errors,
    /// failed event handlers.
    pub fn take_ui_messages(&self) -> Vec<String> {
        self.game_ui.borrow().chaos.as_ref().map(|chaos| chaos.take_messages()).unwrap_or_default()
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
    let game_ui = SharedGameUi::default();
    let runtime = Runtime::new().map_err(runtime_error)?;
    register_engine_api(&runtime, &shared_state, &types, &game_ui).map_err(runtime_error)?;
    // Chaos adds `Component`, `Lantern::Component` and the `@signal` decorator.
    let chaos = ChaosUi::new(&runtime, Path::new(""), Path::new(""))?;
    let manifest = runtime.write_host_surface().map_err(runtime_error);
    drop(chaos);
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
