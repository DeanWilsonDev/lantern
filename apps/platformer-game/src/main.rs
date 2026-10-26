//! The platformer executable: the composition root (Umbra `main.cpp` + `SnakeApplication`).
//!
//! Everything long-lived is constructed here, in order, and dropped in reverse.

mod input_config;

use std::process::ExitCode;

use lantern_engine::frame_loop::{FixedStepContext, FrameContext, FrameLoop, Game};
use lantern_engine::input::InputRouter;
use lantern_platform::{PlatformBackend, PlatformEvent, WindowConfig};
use lantern_sdl_facade::{SdlContext, SdlPlatform, SdlWindow};

struct PlatformerGame {
    actions: input_config::Actions,
    max_frames: Option<u64>,
    frames: u64,
    quit: bool,
}

impl Game for PlatformerGame {
    fn handle_event(&mut self, event: &PlatformEvent, _platform: &mut dyn PlatformBackend) {
        if let PlatformEvent::WindowResized = event {
            log::debug!(target: "game", "window resized");
        }
    }

    fn fixed_update(&mut self, step: &FixedStepContext) {
        if step.input.pressed(self.actions.jump) {
            log::info!(target: "game", "jump pressed on step {}", step.step_index);
        }
    }

    fn frame_update(&mut self, frame: &FrameContext, _platform: &mut dyn PlatformBackend) {
        self.frames += 1;
        if frame.input.pressed(self.actions.pause) {
            self.quit = true;
        }
        if self.max_frames.is_some_and(|max| self.frames >= max) {
            self.quit = true;
        }
    }

    fn render(&mut self, _frame: &FrameContext) {}

    fn wants_to_quit(&self) -> bool {
        self.quit
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sdl = SdlContext::init()?;
    let window = SdlWindow::create(&sdl, &WindowConfig { title: "Lantern".to_string(), ..WindowConfig::default() })?;
    let mut platform = SdlPlatform::new(window);
    let (action_set, bindings, actions) = input_config::build();
    let mut frame_loop = FrameLoop::new(InputRouter::new(action_set, bindings));
    let max_frames = std::env::var("LANTERN_MAX_FRAMES").ok().and_then(|text| text.parse().ok());
    let mut game = PlatformerGame { actions, max_frames, frames: 0, quit: false };
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
