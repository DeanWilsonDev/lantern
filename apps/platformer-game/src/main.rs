//! The platformer executable: the composition root (Umbra `main.cpp` + `SnakeApplication`).
//!
//! `run` builds everything in dependency order: SDL, the window, the GPU renderer, assets,
//! audio, rules, then the game. Shutdown is the reverse: the level is unloaded explicitly, then
//! `PlatformerGame`'s fields drop in declaration order (rules, world, renderer, audio), and SDL
//! shuts down when the last object holding its guard is gone.
//!
//! Environment variables (all optional):
//! - `LANTERN_LOG=debug` more logging; `LANTERN_GPU_DEBUG=1` GPU validation;
//!   `LANTERN_GPU_DRIVER=vulkan|metal|direct3d12`; `LANTERN_VSYNC=0`;
//! - `LANTERN_ASSETS=path` asset directory;
//! - `LANTERN_RULES=native` use the Rust rules instead of the Nyx script;
//!   `LANTERN_SCRIPTS=dir` where `platformer.nyx` is;
//! - `LANTERN_MAX_FRAMES=n` quit after n frames; `LANTERN_CAPTURE_PATH=x.png` save the last
//!   frame; `LANTERN_AUTOPLAY=1` skip the menu and let a scripted pilot play (smoke tests);
//! - `LANTERN_DEBUG_HUD=1`, `LANTERN_COLLISION_VIEW=1` start with those views on.

mod assets;
mod camera;
#[cfg(test)]
mod course_test;
mod game;
mod input_config;
mod level;
mod menu;
mod player;
mod rules;
mod scene;
#[cfg(feature = "nyx-scripting")]
mod scripted_rules;

use std::process::ExitCode;

use lantern_engine::audio::AudioSystem;
use lantern_engine::frame_loop::FrameLoop;
use lantern_engine::input::InputRouter;
use lantern_platform::{AudioSink, WindowConfig};
use lantern_sdl_facade::{SdlAudioOutput, SdlContext, SdlPlatform, SdlWindow};
use lantern_sdl_gpu::{GpuOptions, GpuRenderer};

use crate::assets::GameAssets;
use crate::game::{GameParts, PlatformerGame};
use crate::rules::{GameRules, NativeRules};

fn open_audio(sdl: &SdlContext) -> Option<Box<dyn AudioSink>> {
    match SdlAudioOutput::open(sdl, 48_000) {
        Ok(output) => Some(Box::new(output)),
        Err(error) => {
            log::warn!(target: "audio", "no audio output ({error}); continuing without sound");
            None
        }
    }
}

/// The Nyx rules by default; the Rust rules with `LANTERN_RULES=native`, without the
/// `nyx-scripting` feature, or when the script fails to load (the error is returned for the
/// HUD so it is seen, and the game stays playable).
fn choose_rules() -> (Box<dyn GameRules>, Option<String>) {
    if std::env::var("LANTERN_RULES").is_ok_and(|value| value == "native") {
        return (Box::new(NativeRules::default()), None);
    }
    #[cfg(feature = "nyx-scripting")]
    {
        let path = scripted_rules::find_script();
        match scripted_rules::ScriptedRules::load(&path) {
            Ok(rules) => {
                log::info!(target: "rules", "game rules: {}", path.display());
                return (Box::new(rules), None);
            }
            Err(error) => {
                log::error!(target: "rules", "could not load {}; using the Rust rules\n{error}", path.display());
                return (Box::new(NativeRules::default()), Some(format!("script load failed: {error}")));
            }
        }
    }
    #[allow(unreachable_code)]
    (Box::new(NativeRules::default()), None)
}

fn environment_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| !value.is_empty() && value != "0")
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = SdlContext::init()?;
    let window = SdlWindow::create(&sdl, &WindowConfig { title: "Lantern".to_string(), ..WindowConfig::default() })?;
    let mut renderer = GpuRenderer::new(&window, &GpuOptions::from_environment())?;
    let asset_directory = assets::find_asset_directory();
    let game_assets = GameAssets::load(asset_directory.as_deref(), &mut renderer)?;
    let mut audio = AudioSystem::new(open_audio(&sdl));
    assets::load_sounds(asset_directory.as_deref(), &mut audio);

    let (action_set, bindings, actions) = input_config::build();
    let (rules, rules_load_error) = choose_rules();
    let mut game = PlatformerGame::new(GameParts {
        rules,
        actions,
        action_set: action_set.clone(),
        level: level::demo_level(),
        assets: game_assets,
        renderer,
        audio,
    })?;
    game.max_frames = std::env::var("LANTERN_MAX_FRAMES").ok().and_then(|text| text.parse().ok());
    game.autoplay = environment_flag("LANTERN_AUTOPLAY");
    if let Some(error) = rules_load_error {
        game.report_rules_error(error);
    }
    game.set_debug_views(environment_flag("LANTERN_DEBUG_HUD"), environment_flag("LANTERN_COLLISION_VIEW"));

    let mut platform = SdlPlatform::new(window);
    let mut frame_loop = FrameLoop::new(InputRouter::new(action_set, bindings));
    frame_loop.run(&mut platform, &mut game);
    game.shutdown_level();
    Ok(())
}

fn main() -> ExitCode {
    lantern_core::logging::init(log::LevelFilter::Info);
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!(target: "game", "{error}");
            ExitCode::FAILURE
        }
    }
}
