//! The platformer executable: the composition root (Umbra `main.cpp` + `SnakeApplication`).
//!
//! Everything long-lived is constructed in `run`, in order, and owned by `PlatformerGame`.
//! Rust drops struct fields in declaration order, so the field order of `PlatformerGame` is the
//! shutdown order: the level scene is unloaded explicitly first, then GPU resources go with the
//! renderer, then the audio device, and SDL itself shuts down when the last guard is dropped.

mod assets;
mod input_config;
mod level;
mod scene;

use std::process::ExitCode;

use glam::{Quat, Vec2, Vec3};
use lantern_core::color::Color;
use lantern_engine::audio::AudioSystem;
use lantern_engine::extract::extract_draws;
use lantern_engine::frame_loop::{FixedStepContext, FrameContext, FrameLoop, Game};
use lantern_engine::input::InputRouter;
use lantern_engine::world::World;
use lantern_platform::{AudioSink, PlatformBackend, WindowConfig};
use lantern_renderer_3d::{Camera, RenderWorld, text};
use lantern_sdl_facade::{SdlAudioOutput, SdlContext, SdlPlatform, SdlWindow};
use lantern_sdl_gpu::{GpuOptions, GpuRenderer};

use crate::assets::GameAssets;
use crate::level::Level;
use crate::scene::LevelScene;

struct PlatformerGame {
    actions: input_config::Actions,
    level: Level,
    world: World,
    /// `None` only between an unload and the following load.
    scene: Option<LevelScene>,
    render_world: RenderWorld,
    assets: GameAssets,
    renderer: GpuRenderer,
    audio: AudioSystem,
    simulation_seconds: f32,
    camera_yaw: f32,
    camera_pitch: f32,
    max_frames: Option<u64>,
    frames: u64,
    quit: bool,
}

impl PlatformerGame {
    fn reload_scene(&mut self) {
        if let Some(scene) = self.scene.take() {
            scene.unload(&mut self.world, &mut self.renderer);
        }
        match LevelScene::load(&self.level, &mut self.world, &mut self.renderer, &self.assets) {
            Ok(scene) => self.scene = Some(scene),
            Err(error) => log::error!(target: "scene", "reloading the level failed: {error}"),
        }
    }
}

impl PlatformerGame {
    /// Headless verification aid: with `LANTERN_CAPTURE_PATH` set, the last frame is also
    /// rendered off-screen and saved as a PNG (used with `LANTERN_MAX_FRAMES` in smoke runs).
    fn save_capture_if_requested(&mut self, frame: &FrameContext) {
        let Some(path) = std::env::var_os("LANTERN_CAPTURE_PATH") else { return };
        let (width, height) = (frame.window.pixel_width, frame.window.pixel_height);
        match self.renderer.capture(&self.render_world, width, height) {
            Ok(image) => match std::fs::write(&path, image.to_png_bytes()) {
                Ok(()) => log::info!(target: "renderer", "saved capture to {}", path.to_string_lossy()),
                Err(error) => log::error!(target: "renderer", "saving capture: {error}"),
            },
            Err(error) => log::error!(target: "renderer", "capture failed: {error}"),
        }
    }
}

impl Game for PlatformerGame {
    fn fixed_update(&mut self, step: &FixedStepContext) {
        self.world.store_previous_transforms();
        self.simulation_seconds += step.step_seconds;
        if step.input.pressed(self.actions.reset) {
            self.reload_scene();
            self.audio.play("reset", 1.0);
        }
        if let Some(scene) = &self.scene {
            scene.animate_gems(&mut self.world, self.simulation_seconds);
            // Placeholder motion until the scripted platform path arrives in M6.
            for platform in &scene.moving_platforms {
                if let Some(entity) = self.world.get_mut(platform.entity) {
                    let offset = (self.simulation_seconds * 0.6).sin() * 3.5 - 3.5;
                    entity.local.translation = platform.start_center + Vec3::Z * offset;
                }
            }
        }
        self.world.update_world_transforms();
    }

    fn frame_update(&mut self, frame: &FrameContext, platform: &mut dyn PlatformBackend) {
        self.frames += 1;
        if frame.input.pressed(self.actions.capture_mouse) {
            platform.set_mouse_captured(true);
        }
        let look = frame.input.vector(self.actions.look_mouse)
            + frame.input.vector(self.actions.look_stick) * frame.frame_seconds * 2.5;
        self.camera_yaw -= look.x;
        self.camera_pitch = (self.camera_pitch - look.y).clamp(-1.2, 0.2);
        if frame.input.pressed(self.actions.pause) || self.max_frames.is_some_and(|max| self.frames >= max) {
            self.quit = true;
        }
        self.audio.update();
    }

    fn render(&mut self, frame: &FrameContext) {
        let render_world = &mut self.render_world;
        render_world.clear();
        let player_position = self
            .scene
            .as_ref()
            .and_then(|scene| self.world.get(scene.player))
            .map_or(Vec3::ZERO, |player| player.world_position());
        let focus = player_position.lerp(Vec3::new(0.0, 1.0, -12.0), 0.7);
        let orbit = Quat::from_rotation_y(self.camera_yaw)
            * Quat::from_rotation_x(self.camera_pitch)
            * Vec3::new(0.0, 0.0, 22.0);
        render_world.camera = Camera::look_at(focus + orbit, focus);
        extract_draws(&self.world, frame.alpha, render_world);
        let scale = text::scale_for_density(2, frame.window.pixel_density);
        render_world.overlay.text(
            &format!(
                "{:.0} fps  {:.2} ms  entities {}  (R reloads the level)",
                frame.stats.frames_per_second(),
                frame.stats.average_frame_milliseconds(),
                self.world.len()
            ),
            Vec2::splat(4.0 * scale),
            scale,
            Color::WHITE,
        );
        if let Err(error) = self.renderer.render(&self.render_world) {
            log::error!(target: "renderer", "{error}");
        }
        if self.quit {
            self.save_capture_if_requested(frame);
        }
    }

    fn wants_to_quit(&self) -> bool {
        self.quit
    }
}

fn open_audio(sdl: &SdlContext) -> Option<Box<dyn AudioSink>> {
    match SdlAudioOutput::open(sdl, 48_000) {
        Ok(output) => Some(Box::new(output)),
        Err(error) => {
            log::warn!(target: "audio", "no audio output ({error}); continuing without sound");
            None
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = SdlContext::init()?;
    let window = SdlWindow::create(&sdl, &WindowConfig { title: "Lantern".to_string(), ..WindowConfig::default() })?;
    let mut renderer = GpuRenderer::new(&window, &GpuOptions::from_environment())?;
    let asset_directory = assets::find_asset_directory();
    let game_assets = GameAssets::load(asset_directory.as_deref(), &mut renderer)?;
    let mut audio = AudioSystem::new(open_audio(&sdl));
    assets::load_sounds(asset_directory.as_deref(), &mut audio);
    audio.play_music("theme");

    let level = level::demo_level();
    let mut world = World::new();
    let scene = LevelScene::load(&level, &mut world, &mut renderer, &game_assets)?;

    let mut platform = SdlPlatform::new(window);
    let (action_set, bindings, actions) = input_config::build();
    let mut frame_loop = FrameLoop::new(InputRouter::new(action_set, bindings));
    let max_frames = std::env::var("LANTERN_MAX_FRAMES").ok().and_then(|text| text.parse().ok());
    let mut game = PlatformerGame {
        actions,
        level,
        world,
        scene: Some(scene),
        render_world: RenderWorld {
            overlay_texture: Some(game_assets.font_atlas),
            clear_color: Color::from_hex(0x87B5E0),
            ..Default::default()
        },
        assets: game_assets,
        renderer,
        audio,
        simulation_seconds: 0.0,
        camera_yaw: 0.5,
        camera_pitch: -0.5,
        max_frames,
        frames: 0,
        quit: false,
    };
    frame_loop.run(&mut platform, &mut game);

    if let Some(scene) = game.scene.take() {
        scene.unload(&mut game.world, &mut game.renderer);
    }
    if !game.world.is_empty() {
        log::warn!(target: "scene", "{} entities outlived the level", game.world.len());
    }
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
