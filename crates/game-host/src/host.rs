//! `GameHost`: the engine side of a Nyx game. It implements the frame loop's `Game` trait, does
//! the engine's work around each script hook, and keeps the game running when a script fails.
//!
//! Per fixed step: remember transforms (interpolation) and body positions (platform riding),
//! call `OnFixedUpdate(dt)`, recompute world transforms.
//! Per frame: engine keys (F5 reload, F1 HUD), `OnUpdate(frameSeconds)`, platform requests,
//! HUD values, audio. Per drawn frame: `OnRender(alpha, frameSeconds)`, then extract the
//! world's meshes, draw script errors and the HUD, and render.

use std::path::PathBuf;

use glam::Vec2;
use lantern_chaos_ui::Navigation;
use lantern_core::color::Color;
use lantern_core::input::{ActionSet, Bindings};
use lantern_engine::extract::extract_draws;
use lantern_engine::frame_loop::{FixedStepContext, FrameContext, Game};
use lantern_platform::PlatformBackend;
use lantern_renderer_3d::text::{self, scale_for_density};
use lantern_sdl_gpu::FrameReport;

use crate::api::SharedState;
use crate::engine_state::{ENGINE_RELOAD_ACTION, ENGINE_TOGGLE_HUD_ACTION, EngineState};
use crate::session::{GameHook, HookArgument, ScriptSession};

/// Where a game's scripts are and which class to run.
#[derive(Clone, Debug)]
pub struct ScriptSource {
    pub scripts_directory: PathBuf,
    pub entry_file: String,
    pub application_class: String,
}

pub struct GameHost {
    // Field order is drop order: the script session (its runtime holds closures that share the
    // engine state) goes before the engine state.
    session: Option<ScriptSession>,
    state: SharedState,
    source: ScriptSource,
    script_error: Option<String>,
    script_error_count: u32,
    last_frame_report: FrameReport,
    frames: u64,
    /// Quit after this many frames (smoke tests).
    pub max_frames: Option<u64>,
    /// Save the last frame here as a PNG when quitting (smoke tests).
    pub capture_path: Option<PathBuf>,
    quit: bool,
}

impl GameHost {
    /// Loads the game and runs its `OnStart`. A script that fails to load is not an error here:
    /// the window opens and shows the problem, and F5 tries again.
    pub fn new(state: EngineState, source: ScriptSource) -> GameHost {
        let mut host = GameHost {
            session: None,
            state: std::rc::Rc::new(std::cell::RefCell::new(state)),
            source,
            script_error: None,
            script_error_count: 0,
            last_frame_report: FrameReport::default(),
            frames: 0,
            max_frames: None,
            capture_path: None,
            quit: false,
        };
        host.start_session();
        host
    }

    pub fn state(&self) -> std::cell::Ref<'_, EngineState> {
        self.state.borrow()
    }

    pub fn state_mut(&self) -> std::cell::RefMut<'_, EngineState> {
        self.state.borrow_mut()
    }

    pub fn script_error(&self) -> Option<&str> {
        self.script_error.as_deref()
    }

    pub fn script_error_count(&self) -> u32 {
        self.script_error_count
    }

    pub fn script_session(&self) -> Option<&ScriptSession> {
        self.session.as_ref()
    }

    pub fn is_running_script(&self) -> bool {
        self.session.is_some()
    }

    fn report_error(&mut self, message: String) {
        self.script_error_count += 1;
        // Log a repeating error once, not sixty times a second.
        if self.script_error.as_deref() != Some(message.as_str()) {
            log::error!(target: "script", "{message}");
            self.script_error = Some(message);
        }
    }

    fn call(&mut self, hook: GameHook, arguments: &[HookArgument]) {
        let Some(session) = self.session.as_mut() else { return };
        if let Err(error) = session.call(hook, arguments) {
            self.report_error(error);
        }
    }

    /// The game's Chaos UI before `OnUpdate`: focus moves and button handlers run. The engine
    /// state isn't borrowed here, since the handlers are game code that calls the engine.
    fn navigate_ui(&mut self) {
        let Some(session) = self.session.as_mut() else { return };
        let navigation = match session.ui_navigation_actions() {
            Some(actions) => {
                let state = self.state.borrow();
                let pressed = |name: &str| state.input.pressed(name).unwrap_or(false);
                Navigation {
                    previous: pressed(&actions.previous),
                    next: pressed(&actions.next),
                    confirm: pressed(&actions.confirm),
                }
            }
            None => Navigation::default(),
        };
        session.navigate_ui(navigation);
        self.report_ui_messages();
    }

    /// Chaos reports a failed component or event handler as a diagnostic, not as a hook error,
    /// so it is shown the same way here. Cosmos's notes about stylesheets go to the log only.
    fn report_ui_messages(&mut self) {
        let Some(session) = self.session.as_ref() else { return };
        for message in session.take_ui_messages() {
            if message.starts_with("cosmos:") {
                log::warn!(target: "ui", "{message}");
            } else {
                self.report_error(format!("UI: {message}"));
            }
        }
    }

    fn start_session(&mut self) {
        self.state.borrow_mut().reset_for_new_session();
        let source = &self.source;
        match ScriptSession::load(&source.scripts_directory, &source.entry_file, &source.application_class, &self.state)
        {
            Ok(session) => {
                log::info!(target: "script", "game: {}", session.entry_path().display());
                self.session = Some(session);
                self.script_error = None;
                self.call(GameHook::OnStart, &[]);
                // Read what OnStart mounted, so the first frame's navigation has buttons to use.
                if let Some(session) = self.session.as_mut() {
                    session.update_ui();
                }
                self.report_ui_messages();
            }
            Err(error) => {
                self.report_error(format!("the game failed to load (F5 retries):\n{error}"));
            }
        }
    }

    /// Unloads the game (its `OnExit` runs first). Called on reload and on shutdown.
    pub fn shutdown(&mut self) {
        self.call(GameHook::OnExit, &[]);
        self.session = None;
        self.state.borrow_mut().reset_for_new_session();
    }

    /// F5: unload everything the game made and load it again from disk.
    pub fn reload(&mut self) {
        log::info!(target: "script", "reloading the game's scripts");
        self.shutdown();
        self.start_session();
    }

    fn update_engine_debug_values(&mut self, frame: &FrameContext) {
        let script_milliseconds =
            self.session.as_mut().map_or(0.0, |session| session.take_script_time().as_secs_f64() * 1000.0);
        let source = self.session.as_ref().map(|session| session.entry_path().display().to_string());
        let state = &mut *self.state.borrow_mut();
        let values = &mut state.debug_values;
        values.set("Frame/FPS", format!("{:.0}", frame.stats.frames_per_second()));
        values.set("Frame/Average ms", frame.stats.average_frame_milliseconds());
        values.set("Frame/Worst ms", frame.stats.worst_frame_milliseconds());
        values.set("Frame/Fixed steps", frame.plan.step_count);
        values.set("Frame/Dropped s", frame.stats.dropped_seconds_total);
        values.set("Script/Source", source.unwrap_or_else(|| "(not loaded)".to_string()));
        values.set("Script/ms this frame", script_milliseconds);
        values.set("Script/Errors", self.script_error_count);
        values.set("Renderer/Driver", state.renderer.driver_name().to_string());
        values.set("Renderer/Draws", self.last_frame_report.draws_submitted);
        values.set("Renderer/Culled", self.last_frame_report.draws_culled);
        values.set("Renderer/Triangles", self.last_frame_report.triangles);
        values.set("World/Entities", state.world.len());
        let audio = state.audio.stats();
        values.set("Audio/Device", audio.has_device);
        values.set("Audio/Voices", audio.active_voices);
        values.set("Audio/Queued ms", audio.queued_milliseconds);
        values.set("Audio/Underruns", audio.underrun_count);
        match &self.script_error {
            Some(error) => values.set("Errors/Script", error.clone()),
            None => values.remove_prefix("Errors/"),
        }
    }

    fn draw_script_error(state: &mut EngineState, error: &str) {
        let (screen, scale) = (state.screen_size, state.ui_scale);
        // The first lines (message and location), clipped to the window; the HUD has the rest.
        let columns = ((screen.x - 16.0 * scale) / (text::GLYPH_SIZE as f32 * scale)).max(8.0) as usize;
        let mut lines: Vec<String> = vec!["script error (F1 for details, F5 to reload):".to_string()];
        lines.extend(error.lines().take(4).map(|line| line.trim_end().chars().take(columns).collect()));
        let line_height = text::GLYPH_SIZE as f32 * scale * 1.3;
        let top = screen.y - line_height * lines.len() as f32 - 6.0 * scale;
        let overlay = &mut state.render_world.overlay;
        overlay.rect(Vec2::new(0.0, top - 4.0 * scale), screen, Color::rgba(0.0, 0.0, 0.0, 0.7));
        for (index, line) in lines.iter().enumerate() {
            let position = Vec2::new(8.0 * scale, top + line_height * index as f32);
            overlay.text(line, position, scale, Color::from_hex(0xFF6650));
        }
    }

    fn save_capture(&mut self, path: &std::path::Path, frame: &FrameContext) {
        let state = &mut *self.state.borrow_mut();
        let EngineState { renderer, render_world, .. } = state;
        match renderer.capture(render_world, frame.window.pixel_width, frame.window.pixel_height) {
            Ok(image) => match std::fs::write(path, image.to_png_bytes()) {
                Ok(()) => log::info!(target: "renderer", "saved capture to {}", path.display()),
                Err(error) => log::error!(target: "renderer", "saving capture: {error}"),
            },
            Err(error) => log::error!(target: "renderer", "capture failed: {error}"),
        }
    }
}

impl Game for GameHost {
    fn fixed_update(&mut self, step: &FixedStepContext) {
        if self.session.is_none() {
            return;
        }
        {
            let mut state = self.state.borrow_mut();
            state.world.store_previous_transforms();
            state.collision.begin_step();
            state.input.snapshot = step.input.clone();
        }
        self.call(GameHook::OnFixedUpdate, &[HookArgument::F32(step.step_seconds)]);
        self.state.borrow_mut().world.update_world_transforms();
    }

    fn frame_update(&mut self, frame: &FrameContext, platform: &mut dyn PlatformBackend) {
        self.frames += 1;
        let (reload, toggle_hud) = {
            let mut state = self.state.borrow_mut();
            state.screen_size = Vec2::new(frame.window.pixel_width as f32, frame.window.pixel_height as f32);
            state.ui_scale = scale_for_density(2, frame.window.pixel_density);
            state.input.snapshot = frame.input.clone();
            (frame.input.pressed(ENGINE_RELOAD_ACTION), frame.input.pressed(ENGINE_TOGGLE_HUD_ACTION))
        };
        if toggle_hud {
            self.state.borrow_mut().debug_hud.toggle();
        }
        if reload {
            self.reload();
        }
        // Before OnUpdate, so the game sees a button chosen this frame in this frame's OnUpdate.
        self.navigate_ui();
        self.call(GameHook::OnUpdate, &[HookArgument::F32(frame.frame_seconds)]);
        {
            let mut state = self.state.borrow_mut();
            if let Some(captured) = state.requests.mouse_captured.take() {
                platform.set_mouse_captured(captured);
            }
            if std::mem::take(&mut state.requests.quit) {
                self.quit = true;
            }
        }
        if self.max_frames.is_some_and(|max| self.frames >= max) {
            self.quit = true;
        }
        self.update_engine_debug_values(frame);
        self.state.borrow_mut().audio.update();
    }

    fn render(&mut self, frame: &FrameContext) {
        {
            let mut state = self.state.borrow_mut();
            state.render_world.clear();
            state.ui_column = None;
        }
        self.call(GameHook::OnRender, &[HookArgument::F32(frame.alpha), HookArgument::F32(frame.frame_seconds)]);
        if let Some(session) = self.session.as_mut() {
            session.update_ui();
        }
        self.report_ui_messages();
        {
            let state = &mut *self.state.borrow_mut();
            if let Some(unfinished) = state.ui_column.take() {
                state.ui.end_column(unfinished);
            }
            if let Some(session) = &self.session {
                // A Cosmos `px` is a window point, as in the editor: `ui_scale` is twice the
                // pixel density (the 8x8 font's normal size), so half of it is one point.
                let (screen, points) = (state.screen_size, state.ui_scale * 0.5);
                session.draw_ui(&mut state.render_world.overlay, screen, points);
            }
            extract_draws(&state.world, frame.alpha, &mut state.render_world);
            if let Some(error) = &self.script_error
                && !state.debug_hud.visible
            {
                GameHost::draw_script_error(state, error);
            }
            let scale = state.ui_scale;
            state.debug_hud.draw(&state.debug_values, &mut state.render_world.overlay, Vec2::splat(8.0 * scale), scale);
            match state.renderer.render(&state.render_world) {
                Ok(report) => self.last_frame_report = report,
                Err(error) => log::error!(target: "renderer", "{error}"),
            }
        }
        if self.quit
            && let Some(path) = self.capture_path.clone()
        {
            self.save_capture(&path, frame);
        }
    }

    fn wants_to_quit(&self) -> bool {
        self.quit
    }

    fn take_input_configuration(&mut self) -> Option<(ActionSet, Bindings)> {
        let mut state = self.state.borrow_mut();
        if !std::mem::take(&mut state.input.configuration_changed) {
            return None;
        }
        Some((state.input.actions.clone(), state.input.bindings.clone()))
    }
}
