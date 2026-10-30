//! The frame loop, with a fixed simulation step.
//!
//! ```text
//! loop {
//!     pump platform events → input router, game.handle_event
//!     input_router.end_frame_events()
//!     plan = clock.advance(frame time)
//!     plan.step_count × game.fixed_update(fixed-step snapshot)
//!     game.frame_update(frame snapshot, alpha)
//!     game.render(alpha)            (skipped while minimised)
//! }
//! ```
//!
//! The loop is a struct with a `run_frame` method rather than one big function so that tests
//! can drive it a frame at a time with a fake platform.

use lantern_core::input::{ActionSet, Bindings, InputSnapshot};
use lantern_core::time::{FixedStepClock, FixedStepPlan, FrameStats};
use lantern_platform::{PlatformBackend, PlatformEvent, WindowMetrics};

use crate::input::InputRouter;

/// What one fixed step sees.
pub struct FixedStepContext<'a> {
    pub input: &'a InputSnapshot,
    pub step_seconds: f32,
    /// Counts every fixed step since the loop started.
    pub step_index: u64,
}

/// What the per-frame callbacks see.
pub struct FrameContext<'a> {
    pub input: &'a InputSnapshot,
    /// Measured wall time of the last frame (for presentation only, never for simulation).
    pub frame_seconds: f32,
    /// Interpolation factor between the previous and current fixed-step states.
    pub alpha: f32,
    pub plan: FixedStepPlan,
    pub stats: &'a FrameStats,
    pub window: WindowMetrics,
}

/// The game's side of the loop: its update and render hooks.
pub trait Game {
    /// Called for every platform event, after the input router has seen it.
    fn handle_event(&mut self, _event: &PlatformEvent, _platform: &mut dyn PlatformBackend) {}
    /// Advance the simulation by exactly `step.step_seconds`.
    fn fixed_update(&mut self, step: &FixedStepContext);
    /// Presentation work that runs once per display frame: camera, UI, HUD.
    fn frame_update(&mut self, frame: &FrameContext, platform: &mut dyn PlatformBackend);
    /// Draw the frame. Not called while the window is minimised.
    fn render(&mut self, frame: &FrameContext);
    /// The game can end the loop (for example "Quit" on the main menu).
    fn wants_to_quit(&self) -> bool;
    /// New actions and bindings for the input router, if the game has defined some since the
    /// last frame. Checked at the start of every frame, before events are read.
    fn take_input_configuration(&mut self) -> Option<(ActionSet, Bindings)> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopStatus {
    Running,
    Finished,
}

pub struct FrameLoop {
    pub input: InputRouter,
    clock: FixedStepClock,
    stats: FrameStats,
    events: Vec<PlatformEvent>,
    previous_time: Option<f64>,
    was_minimized: bool,
    quit_requested: bool,
}

impl FrameLoop {
    pub fn new(input: InputRouter) -> FrameLoop {
        FrameLoop {
            input,
            clock: FixedStepClock::default(),
            stats: FrameStats::default(),
            events: Vec::new(),
            previous_time: None,
            was_minimized: false,
            quit_requested: false,
        }
    }

    pub fn stats(&self) -> &FrameStats {
        &self.stats
    }

    /// Runs until the platform asks to quit or the game wants to.
    pub fn run(&mut self, platform: &mut dyn PlatformBackend, game: &mut dyn Game) {
        log::info!(target: "engine", "frame loop started");
        while self.run_frame(platform, game) == LoopStatus::Running {}
        log::info!(target: "engine", "frame loop finished after {} frames, {} fixed steps", self.stats.frames, self.clock.total_steps());
    }

    pub fn run_frame(&mut self, platform: &mut dyn PlatformBackend, game: &mut dyn Game) -> LoopStatus {
        if let Some((actions, bindings)) = game.take_input_configuration() {
            self.input.reconfigure(actions, bindings);
        }

        // 1. Events.
        self.events.clear();
        platform.pump_events(&mut self.events);
        for event in &self.events {
            if *event == PlatformEvent::QuitRequested {
                self.quit_requested = true;
            }
            self.input.handle_event(event);
            game.handle_event(event, platform);
        }
        self.input.end_frame_events();

        // 2. Time.
        let now = platform.seconds_since_start();
        let window = platform.window_metrics();
        // The first frame, and the first frame after a restore, measure no time: otherwise the
        // simulation would try to catch up on however long the window was hidden.
        let frame_seconds = match self.previous_time {
            Some(previous) if !self.was_minimized => now - previous,
            _ => 0.0,
        };
        self.previous_time = Some(now);
        self.was_minimized = window.minimized;
        let plan = if window.minimized {
            // While minimised the simulation pauses rather than racing to catch up on restore.
            FixedStepPlan { step_count: 0, alpha: 0.0, dropped_seconds: 0.0 }
        } else {
            self.clock.advance(frame_seconds)
        };
        self.stats.record(frame_seconds, &plan);

        // 3. Fixed steps.
        let step_seconds = self.clock.step_seconds();
        let first_step_index = self.clock.total_steps() - u64::from(plan.step_count);
        for step in 0..plan.step_count {
            let snapshot = self.input.take_fixed_step_snapshot();
            game.fixed_update(&FixedStepContext {
                input: &snapshot,
                step_seconds,
                step_index: first_step_index + u64::from(step),
            });
        }

        // 4. Frame update and render.
        let frame = FrameContext {
            input: self.input.frame_snapshot(),
            frame_seconds: frame_seconds as f32,
            alpha: plan.alpha,
            plan,
            stats: &self.stats,
            window,
        };
        game.frame_update(&frame, platform);
        if window.is_drawable_empty() {
            // Nothing to draw into; don't spin the CPU either.
            std::thread::sleep(std::time::Duration::from_millis(16));
        } else {
            game.render(&frame);
        }

        if self.quit_requested || game.wants_to_quit() { LoopStatus::Finished } else { LoopStatus::Running }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_core::input::{ActionId, ActionKind, ActionSet, Binding, Bindings, KeyCode};
    use std::collections::VecDeque;

    /// A platform whose clock and events are scripted per frame.
    struct ScriptedPlatform {
        frames: VecDeque<(f64, Vec<PlatformEvent>)>,
        now: f64,
        minimized: bool,
    }

    impl PlatformBackend for ScriptedPlatform {
        fn pump_events(&mut self, events: &mut Vec<PlatformEvent>) {
            if let Some((time, frame_events)) = self.frames.pop_front() {
                self.now = time;
                for event in &frame_events {
                    match event {
                        PlatformEvent::WindowMinimized => self.minimized = true,
                        PlatformEvent::WindowRestored => self.minimized = false,
                        _ => {}
                    }
                }
                events.extend(frame_events);
            }
        }
        fn seconds_since_start(&self) -> f64 {
            self.now
        }
        fn window_metrics(&self) -> WindowMetrics {
            WindowMetrics {
                logical_width: 640,
                logical_height: 360,
                pixel_width: if self.minimized { 0 } else { 640 },
                pixel_height: if self.minimized { 0 } else { 360 },
                pixel_density: 1.0,
                minimized: self.minimized,
            }
        }
        fn set_mouse_captured(&mut self, _captured: bool) {}
    }

    #[derive(Default)]
    struct RecordingGame {
        jump: Option<ActionId>,
        steps: u32,
        jumps_seen: u32,
        renders: u32,
        position: f32,
        velocity_per_second: f32,
    }

    impl Game for RecordingGame {
        fn fixed_update(&mut self, step: &FixedStepContext) {
            self.steps += 1;
            if step.input.pressed(self.jump.expect("set")) {
                self.jumps_seen += 1;
            }
            self.position += self.velocity_per_second * step.step_seconds;
        }
        fn frame_update(&mut self, _frame: &FrameContext, _platform: &mut dyn PlatformBackend) {}
        fn render(&mut self, _frame: &FrameContext) {
            self.renders += 1;
        }
        fn wants_to_quit(&self) -> bool {
            false
        }
    }

    fn setup(frames: Vec<(f64, Vec<PlatformEvent>)>) -> (FrameLoop, ScriptedPlatform, RecordingGame) {
        let mut actions = ActionSet::default();
        let jump = actions.register("jump", ActionKind::Button);
        let mut bindings = Bindings::default();
        bindings.bind(jump, Binding::Key(KeyCode::Space));
        let frame_loop = FrameLoop::new(InputRouter::new(actions, bindings));
        let platform = ScriptedPlatform { frames: frames.into(), now: 0.0, minimized: false };
        let game = RecordingGame { jump: Some(jump), velocity_per_second: 3.0, ..Default::default() };
        (frame_loop, platform, game)
    }

    fn run(frame_loop: &mut FrameLoop, platform: &mut ScriptedPlatform, game: &mut RecordingGame) {
        while !platform.frames.is_empty() {
            if frame_loop.run_frame(platform, game) == LoopStatus::Finished {
                break;
            }
        }
    }

    #[test]
    fn quit_event_ends_the_loop() {
        let (mut frame_loop, mut platform, mut game) =
            setup(vec![(0.0, vec![]), (0.016, vec![PlatformEvent::QuitRequested]), (0.032, vec![])]);
        assert_eq!(frame_loop.run_frame(&mut platform, &mut game), LoopStatus::Running);
        assert_eq!(frame_loop.run_frame(&mut platform, &mut game), LoopStatus::Finished);
    }

    #[test]
    fn simulation_distance_does_not_depend_on_frame_rate() {
        for hz in [30.0, 60.0, 144.0, 250.0] {
            let frames = (0..=(hz as usize * 2)).map(|frame| (frame as f64 / hz, vec![])).collect();
            let (mut frame_loop, mut platform, mut game) = setup(frames);
            run(&mut frame_loop, &mut platform, &mut game);
            assert!((game.position - 6.0).abs() < 0.06, "{hz} Hz moved {}", game.position);
        }
    }

    #[test]
    fn a_jump_pressed_on_a_short_frame_reaches_the_simulation_once() {
        let space_down = PlatformEvent::KeyDown { key: KeyCode::Space, repeat: false };
        // 240 Hz display: most frames run no fixed step.
        let frames =
            (0..40).map(|frame| (frame as f64 / 240.0, if frame == 5 { vec![space_down] } else { vec![] })).collect();
        let (mut frame_loop, mut platform, mut game) = setup(frames);
        run(&mut frame_loop, &mut platform, &mut game);
        assert_eq!(game.jumps_seen, 1);
    }

    #[test]
    fn minimised_windows_neither_render_nor_simulate() {
        let mut frames = vec![(0.0, vec![]), (0.016, vec![PlatformEvent::WindowMinimized])];
        frames.extend((2..10).map(|frame| (frame as f64 * 0.016, vec![])));
        frames.push((0.5, vec![PlatformEvent::WindowRestored]));
        let (mut frame_loop, mut platform, mut game) = setup(frames);
        run(&mut frame_loop, &mut platform, &mut game);
        assert_eq!(game.renders, 2, "only the first frame and the restored frame render");
        assert!(game.steps <= 2, "simulation paused while minimised, ran {}", game.steps);
    }
}
