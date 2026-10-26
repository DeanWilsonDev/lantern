//! The platformer executable: the composition root (Umbra `main.cpp` + `SnakeApplication`).
//!
//! Everything long-lived is constructed here, in order, and dropped in reverse.

mod input_config;

use std::process::ExitCode;

use glam::{Mat4, Quat, Vec3};
use lantern_core::color::Color;
use lantern_engine::frame_loop::{FixedStepContext, FrameContext, FrameLoop, Game};
use lantern_engine::input::InputRouter;
use lantern_platform::{PlatformBackend, WindowConfig};
use lantern_renderer_3d::{Camera, MeshHandle, RenderWorld, primitives, text};
use lantern_sdl_facade::{SdlContext, SdlPlatform, SdlWindow};
use lantern_sdl_gpu::{GpuOptions, GpuRenderer};

/// M2 milestone scene: an indexed cube spinning in fixed steps, an orbit camera.
struct PlatformerGame {
    actions: input_config::Actions,
    renderer: GpuRenderer,
    cube: MeshHandle,
    render_world: RenderWorld,
    cube_angle: f32,
    previous_cube_angle: f32,
    camera_yaw: f32,
    camera_pitch: f32,
    max_frames: Option<u64>,
    frames: u64,
    quit: bool,
}

impl Game for PlatformerGame {
    fn fixed_update(&mut self, step: &FixedStepContext) {
        self.previous_cube_angle = self.cube_angle;
        self.cube_angle += step.step_seconds * 0.8;
    }

    fn frame_update(&mut self, frame: &FrameContext, platform: &mut dyn PlatformBackend) {
        self.frames += 1;
        if frame.input.pressed(self.actions.capture_mouse) {
            platform.set_mouse_captured(true);
        }
        let look = frame.input.vector(self.actions.look_mouse)
            + frame.input.vector(self.actions.look_stick) * frame.frame_seconds * 2.5;
        self.camera_yaw -= look.x;
        self.camera_pitch = (self.camera_pitch - look.y).clamp(-1.2, 1.2);
        if frame.input.pressed(self.actions.pause) || self.max_frames.is_some_and(|max| self.frames >= max) {
            self.quit = true;
        }
    }

    fn render(&mut self, frame: &FrameContext) {
        let angle = self.previous_cube_angle + (self.cube_angle - self.previous_cube_angle) * frame.alpha;
        let world = &mut self.render_world;
        world.clear();
        let orbit = Quat::from_rotation_y(self.camera_yaw)
            * Quat::from_rotation_x(self.camera_pitch)
            * Vec3::new(0.0, 0.0, 4.0);
        world.camera = Camera::look_at(orbit, Vec3::ZERO);
        world.draw(
            self.cube,
            None,
            Mat4::from_rotation_y(angle) * Mat4::from_rotation_x(angle * 0.5),
            Color::from_hex(0xE8A33D),
        );
        world.line(Vec3::ZERO, Vec3::X * 1.5, Color::RED);
        world.line(Vec3::ZERO, Vec3::Y * 1.5, Color::GREEN);
        world.line(Vec3::ZERO, Vec3::Z * 1.5, Color::BLUE);
        let scale = text::scale_for_density(2, frame.window.pixel_density);
        world.overlay.text(
            &format!("{:.0} fps  {:.2} ms", frame.stats.frames_per_second(), frame.stats.average_frame_milliseconds()),
            glam::Vec2::splat(8.0 * scale / 2.0),
            scale,
            Color::WHITE,
        );
        if let Err(error) = self.renderer.render(&self.render_world) {
            log::error!(target: "renderer", "{error}");
        }
    }

    fn wants_to_quit(&self) -> bool {
        self.quit
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = SdlContext::init()?;
    let window = SdlWindow::create(&sdl, &WindowConfig { title: "Lantern".to_string(), ..WindowConfig::default() })?;
    let mut renderer = GpuRenderer::new(&window, &GpuOptions::from_environment())?;
    let cube = renderer.create_mesh(&primitives::cube(1.0))?;
    let font_atlas = renderer.create_texture(&text::font_atlas())?;
    let mut platform = SdlPlatform::new(window);
    let (action_set, bindings, actions) = input_config::build();
    let mut frame_loop = FrameLoop::new(InputRouter::new(action_set, bindings));
    let max_frames = std::env::var("LANTERN_MAX_FRAMES").ok().and_then(|text| text.parse().ok());
    let mut game = PlatformerGame {
        actions,
        renderer,
        cube,
        render_world: RenderWorld { overlay_texture: Some(font_atlas), ..Default::default() },
        cube_angle: 0.0,
        previous_cube_angle: 0.0,
        camera_yaw: 0.6,
        camera_pitch: -0.4,
        max_frames,
        frames: 0,
        quit: false,
    };
    frame_loop.run(&mut platform, &mut game);
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
