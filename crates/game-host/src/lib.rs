//! Lantern's game host: runs a game written in Nyx.
//!
//! This crate is the seam between engine code and game code. Everything game-specific (rules,
//! menus, the player, the camera, tuning, what a gem is) is Nyx in the game's directory;
//! everything here is engine: it owns the world, renderer, physics, audio, input and UI, and
//! exposes them to the game through the host types in [`api`]. The engine never names a game
//! concept, and the game never touches a pointer, a GPU resource or the frame loop.
//!
//! ```text
//! games/<name>/game.json      title, entry file, application class, assets directory
//! games/<name>/**/*.nyx       the game (imports resolve under games/<name>/)
//! ```
//!
//! [`run`] is the whole program for the `lantern` runner: SDL, window, GPU renderer, audio,
//! the game host and the frame loop, in dependency order. Tests use [`GameHost`] with a
//! [`HeadlessRenderer`] instead and drive the frame loop themselves.

#![forbid(unsafe_code)]

pub mod api;
mod engine_state;
mod host;
mod render_backend;
mod session;

use std::path::{Path, PathBuf};

pub use engine_state::{EngineState, EntityTags, GAME_SCENE, parse_color};
pub use host::{GameHost, ScriptSource};
pub use render_backend::{HeadlessRenderer, HeadlessStats, RenderBackend};
pub use session::{GameHook, ScriptSession, engine_host_surface};

/// `game.json`: what the runner needs to start a game.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct GameManifest {
    /// The window title.
    pub title: String,
    /// The script that holds the `Application` class, relative to the game directory.
    pub entry: String,
    /// The name of the class deriving `Application`.
    pub application_class: String,
    /// The assets directory, relative to the game directory.
    #[serde(default = "default_assets")]
    pub assets: String,
}

fn default_assets() -> String {
    "assets".to_string()
}

impl GameManifest {
    pub fn load(game_directory: &Path) -> Result<GameManifest, String> {
        let path = game_directory.join("game.json");
        let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
    }

    pub fn script_source(&self, game_directory: &Path) -> ScriptSource {
        ScriptSource {
            scripts_directory: game_directory.to_path_buf(),
            entry_file: self.entry.clone(),
            application_class: self.application_class.clone(),
        }
    }

    /// `LANTERN_ASSETS` if set, else the manifest's directory. `None` (with a warning) when
    /// neither exists; games then use their generated fallbacks.
    pub fn assets_directory(&self, game_directory: &Path) -> Option<PathBuf> {
        let candidate = match std::env::var_os("LANTERN_ASSETS") {
            Some(explicit) => PathBuf::from(explicit),
            None => game_directory.join(&self.assets),
        };
        if candidate.is_dir() {
            log::info!(target: "assets", "assets directory: {}", candidate.display());
            Some(candidate)
        } else {
            log::warn!(target: "assets", "no assets directory at {}", candidate.display());
            None
        }
    }
}

fn environment_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| !value.is_empty() && value != "0")
}

/// Opens a window and runs the game in `game_directory` until it quits.
///
/// Environment variables (all optional): `LANTERN_LOG=debug`, `LANTERN_GPU_DEBUG=1`,
/// `LANTERN_GPU_DRIVER=vulkan|metal|direct3d12`, `LANTERN_VSYNC=0`, `LANTERN_ASSETS=dir`,
/// `LANTERN_MAX_FRAMES=n` (quit after n frames), `LANTERN_CAPTURE_PATH=x.png` (save the last
/// frame), `LANTERN_DEBUG_HUD=1` (start with the HUD on). Games read their own flags with
/// `Engine:::Flag` (the platformer: `LANTERN_AUTOPLAY`, `LANTERN_COLLISION_VIEW`).
pub fn run(game_directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use lantern_engine::audio::AudioSystem;
    use lantern_engine::frame_loop::FrameLoop;
    use lantern_engine::input::InputRouter;
    use lantern_platform::{AudioSink, WindowConfig};
    use lantern_sdl_facade::{SdlAudioOutput, SdlContext, SdlPlatform, SdlWindow};
    use lantern_sdl_gpu::{GpuOptions, GpuRenderer};

    let manifest = GameManifest::load(game_directory)?;
    let sdl = SdlContext::init()?;
    let window = SdlWindow::create(&sdl, &WindowConfig { title: manifest.title.clone(), ..WindowConfig::default() })?;
    let renderer = GpuRenderer::new(&window, &GpuOptions::from_environment())?;
    let audio_output: Option<Box<dyn AudioSink>> = match SdlAudioOutput::open(&sdl, 48_000) {
        Ok(output) => Some(Box::new(output)),
        Err(error) => {
            log::warn!(target: "audio", "no audio output ({error}); continuing without sound");
            None
        }
    };
    let state = EngineState::new(
        Box::new(renderer),
        AudioSystem::new(audio_output),
        manifest.assets_directory(game_directory),
    )?;
    let mut host = GameHost::new(state, manifest.script_source(game_directory));
    host.max_frames = std::env::var("LANTERN_MAX_FRAMES").ok().and_then(|text| text.parse().ok());
    host.capture_path = std::env::var_os("LANTERN_CAPTURE_PATH").map(PathBuf::from);
    host.state_mut().debug_hud.visible = environment_flag("LANTERN_DEBUG_HUD");

    let mut platform = SdlPlatform::new(window);
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));
    frame_loop.run(&mut platform, &mut host);
    host.shutdown();
    // Drop order: the host (script runtime, then engine state: GPU resources, audio stream),
    // then the platform (window); SDL quits when the last object holding its guard is gone.
    drop(host);
    Ok(())
}
