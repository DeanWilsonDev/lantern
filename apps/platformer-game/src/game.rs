//! `PlatformerGame`: the game states, the fixed-step schedule and the frame's presentation
//! (Umbra `SnakeApplication` + `GameplayScene` + its game states).
//!
//! Fixed step (only while playing), as listed in `docs/architecture.md`:
//! 1. remember last step's transforms (interpolation) and body positions (platform riding);
//! 2. rules `on_fixed_update`, then apply their commands (platform paths move bodies);
//! 3. the character controller moves the player through the collision world;
//! 4. triggers and the kill height produce events; rules `on_event`; apply commands;
//! 5. world transforms are recomputed, parents before children.

use std::collections::HashSet;

use glam::{Vec2, Vec3};
use lantern_core::color::Color;
use lantern_core::debug::DebugValues;
use lantern_core::input::{ActionSet, InputSnapshot};
use lantern_debug::{DebugHud, debug_draw};
use lantern_engine::audio::AudioSystem;
use lantern_engine::extract::extract_draws;
use lantern_engine::frame_loop::{FixedStepContext, FrameContext, Game};
use lantern_engine::world::{EntityId, World};
use lantern_platform::PlatformBackend;
use lantern_renderer_3d::RenderWorld;
use lantern_renderer_3d::text::{self, measure_text};
use lantern_renderer_3d::ui::{Ui, UiInput};
use lantern_sdl_gpu::{FrameReport, GpuRenderer};

use crate::assets::{GameAssets, PLAYER_RADIUS};
use crate::camera::ThirdPersonCamera;
use crate::input_config::Actions;
use crate::level::Level;
use crate::menu::{self, MainMenuChoice, PauseMenuChoice};
use crate::player::Player;
use crate::rules::{GameRules, RuleCommand, RuleEvent, RulesView};
use crate::scene::LevelScene;

/// Gems are collected when the player's capsule comes this close to their centre.
const GEM_TRIGGER_RADIUS: f32 = 0.45;
/// How fast the camera circles the level behind the main menu (radians per second).
const MAIN_MENU_ORBIT_SPEED: f32 = 0.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    MainMenu,
    Playing,
    Paused,
}

/// Field order is drop order: rules (which may hold a script runtime) go first, the renderer
/// and audio device last.
pub struct PlatformerGame {
    rules: Box<dyn GameRules>,
    rule_commands: Vec<RuleCommand>,
    rule_events: Vec<RuleEvent>,
    rules_error: Option<String>,
    rules_error_count: u32,

    state: GameState,
    /// A state chosen in a menu during `render`, applied at the next `frame_update` (which has
    /// the platform, for mouse capture).
    pending_state: Option<GameState>,
    menu_ui: Ui,
    debug_hud: DebugHud,
    debug_values: DebugValues,
    show_collision_view: bool,
    status_text: String,
    message: Option<(String, f32)>,

    actions: Actions,
    action_set: ActionSet,
    level: Level,
    world: World,
    scene: Option<LevelScene>,
    player: Player,
    triggers_inside: HashSet<EntityId>,
    camera: ThirdPersonCamera,
    simulation_seconds: f32,

    render_world: RenderWorld,
    last_frame_report: FrameReport,
    assets: GameAssets,
    renderer: GpuRenderer,
    audio: AudioSystem,

    pub max_frames: Option<u64>,
    pub autoplay: bool,
    frames: u64,
    quit: bool,
}

pub struct GameParts {
    pub rules: Box<dyn GameRules>,
    pub actions: Actions,
    pub action_set: ActionSet,
    pub level: Level,
    pub assets: GameAssets,
    pub renderer: GpuRenderer,
    pub audio: AudioSystem,
}

impl PlatformerGame {
    pub fn new(parts: GameParts) -> Result<PlatformerGame, Box<dyn std::error::Error>> {
        let GameParts { rules, actions, action_set, level, assets, mut renderer, audio } = parts;
        let mut world = World::new();
        let scene = LevelScene::load(&level, &mut world, &mut renderer, &assets)?;
        let player = Player::new(scene.player, level.spawn_position);
        let render_world = RenderWorld {
            overlay_texture: Some(assets.font_atlas),
            clear_color: Color::from_hex(0x87B5E0),
            fog_start: 30.0,
            fog_end: 90.0,
            ..Default::default()
        };
        let mut game = PlatformerGame {
            rules,
            rule_commands: Vec::new(),
            rule_events: Vec::new(),
            rules_error: None,
            rules_error_count: 0,
            state: GameState::MainMenu,
            pending_state: None,
            menu_ui: Ui::default(),
            debug_hud: DebugHud { visible: false, ..Default::default() },
            debug_values: DebugValues::default(),
            show_collision_view: false,
            status_text: String::new(),
            message: None,
            actions,
            action_set,
            level,
            world,
            scene: Some(scene),
            player,
            triggers_inside: HashSet::new(),
            camera: ThirdPersonCamera::default(),
            simulation_seconds: 0.0,
            render_world,
            last_frame_report: FrameReport::default(),
            assets,
            renderer,
            audio,
            max_frames: None,
            autoplay: false,
            frames: 0,
            quit: false,
        };
        game.start_rules();
        game.audio.play_music("theme");
        Ok(game)
    }

    /// Unloads the level (calling the rules' exit hook first). Called on quit and restart.
    pub fn shutdown_level(&mut self) {
        let empty_input = InputSnapshot::default();
        self.run_rules_hook("on_exit", &empty_input, |rules, view, commands| rules.on_exit(view, commands));
        if let Some(scene) = self.scene.take() {
            scene.unload(&mut self.world, &mut self.renderer);
        }
        self.triggers_inside.clear();
    }

    fn restart_level(&mut self) {
        self.shutdown_level();
        match LevelScene::load(&self.level, &mut self.world, &mut self.renderer, &self.assets) {
            Ok(scene) => {
                self.player = Player::new(scene.player, self.level.spawn_position);
                self.scene = Some(scene);
                self.simulation_seconds = 0.0;
                self.camera.reset_behind(0.0);
                self.start_rules();
            }
            Err(error) => log::error!(target: "scene", "reloading the level failed: {error}"),
        }
    }

    fn start_rules(&mut self) {
        let empty_input = InputSnapshot::default();
        self.run_rules_hook("on_start", &empty_input, |rules, view, commands| rules.on_start(view, commands));
    }

    /// Runs one rules hook with a fresh view, records any error, then applies the commands.
    fn run_rules_hook(
        &mut self,
        hook_name: &'static str,
        input: &InputSnapshot,
        hook: impl FnOnce(&mut dyn GameRules, &RulesView, &mut Vec<RuleCommand>) -> Result<(), crate::rules::RulesError>,
    ) {
        let mut commands = std::mem::take(&mut self.rule_commands);
        commands.clear();
        let view = RulesView {
            world: &self.world,
            input,
            action_set: &self.action_set,
            time_seconds: self.simulation_seconds,
            player: self.player.entity,
        };
        if let Err(error) = hook(self.rules.as_mut(), &view, &mut commands) {
            self.rules_error_count += 1;
            let message = format!("{hook_name}: {error}");
            // Log a repeating error once, not sixty times a second.
            if self.rules_error.as_deref() != Some(message.as_str()) {
                log::error!(target: "rules", "{message}");
                self.rules_error = Some(message);
            }
        }
        for command in commands.drain(..) {
            self.apply_rule_command(command);
        }
        self.rule_commands = commands;
    }

    fn apply_rule_command(&mut self, command: RuleCommand) {
        match command {
            RuleCommand::SetEntityPosition { entity, position } => {
                if let Some(data) = self.world.get_mut(entity) {
                    data.local.translation = position;
                }
                if let Some(scene) = self.scene.as_mut()
                    && let Some(body) = scene.moving_platform_body(entity)
                {
                    scene.collision.set_body_transform(body, glam::Mat4::from_translation(position));
                }
            }
            RuleCommand::SetEntityVisible { entity, visible } => {
                if let Some(renderer) = self.world.get_mut(entity).and_then(|data| data.mesh_renderer.as_mut()) {
                    renderer.visible = visible;
                }
            }
            RuleCommand::PlaySound { name, volume } => self.audio.play(&name, volume),
            RuleCommand::ResetPlayer => {
                self.player.reset();
                self.player.teleport_entity(&mut self.world);
                self.camera.reset_behind(0.0);
            }
            RuleCommand::SetStatus { text } => self.status_text = text,
            RuleCommand::ShowMessage { text, seconds } => self.message = Some((text, seconds)),
            RuleCommand::Log { text } => log::info!(target: "rules", "{text}"),
        }
    }

    fn set_state(&mut self, state: GameState, platform: &mut dyn PlatformBackend) {
        self.state = state;
        self.menu_ui.reset_focus();
        platform.set_mouse_captured(state == GameState::Playing && !self.autoplay);
    }

    /// Finds triggers the player has just entered. Gems are the only triggers.
    fn detect_trigger_entries(&mut self) {
        let Some(scene) = &self.scene else { return };
        let capsule = self.player.controller.settings.capsule();
        let (segment_bottom, segment_top) = capsule.segment(self.player.controller.capsule_center());
        for gem in &scene.gems {
            let Some(data) = self.world.get(gem.entity) else { continue };
            let center = data.local.translation;
            let on_axis = center.clamp(segment_bottom.min(segment_top), segment_bottom.max(segment_top));
            let closest_on_axis = Vec3::new(segment_bottom.x, on_axis.y, segment_bottom.z);
            let inside = closest_on_axis.distance(center) < GEM_TRIGGER_RADIUS + PLAYER_RADIUS;
            if inside && self.triggers_inside.insert(gem.entity) {
                self.rule_events.push(RuleEvent::TriggerEntered { trigger: gem.entity });
            } else if !inside {
                self.triggers_inside.remove(&gem.entity);
            }
        }
    }

    fn update_debug_values(&mut self, frame: &FrameContext) {
        let values = &mut self.debug_values;
        let controller = &self.player.controller;
        values.set("Frame/FPS", format!("{:.0}", frame.stats.frames_per_second()));
        values.set("Frame/Average ms", frame.stats.average_frame_milliseconds());
        values.set("Frame/Worst ms", frame.stats.worst_frame_milliseconds());
        values.set("Frame/Fixed steps", frame.plan.step_count);
        values.set("Frame/Dropped s", frame.stats.dropped_seconds_total);
        values.set("Player/Position", controller.feet_position);
        values.set("Player/Velocity", controller.velocity);
        values.set("Player/Speed", controller.velocity.truncate().length());
        values.set("Player/Grounded", controller.grounded);
        values.set("Player/Ground normal", controller.ground_normal);
        values.set("Camera/Yaw", self.camera.yaw.to_degrees());
        values.set("Camera/Pitch", self.camera.pitch.to_degrees());
        values.set("Renderer/Driver", self.renderer.driver_name().to_string());
        values.set("Renderer/Draws", self.last_frame_report.draws_submitted);
        values.set("Renderer/Culled", self.last_frame_report.draws_culled);
        values.set("Renderer/Triangles", self.last_frame_report.triangles);
        values.set("World/Entities", self.world.len());
        let audio = self.audio.stats();
        values.set("Audio/Device", audio.has_device);
        values.set("Audio/Voices", audio.active_voices);
        values.set("Audio/Queued ms", audio.queued_milliseconds);
        values.set("Audio/Underruns", audio.underrun_count);
        values.set("Rules/Source", self.rules.description());
        values.set("Rules/Errors", self.rules_error_count);
        if let Some(error) = &self.rules_error {
            values.set("Errors/Rules", error.clone());
        }
    }

    fn draw_collision_view(&mut self) {
        let Some(scene) = &self.scene else { return };
        let controller = &self.player.controller;
        let center = controller.capsule_center();
        let capsule = controller.settings.capsule();
        let color = if controller.grounded { Color::GREEN } else { Color::from_hex(0xFF8030) };
        debug_draw::capsule(&mut self.render_world, center, capsule.radius, capsule.half_segment, color);
        let nearby = lantern_core::math::Aabb::from_center_half_extents(center, Vec3::splat(4.0));
        let triangles = scene.collision.triangles_near(&nearby);
        debug_draw::triangle_edges(
            &mut self.render_world,
            triangles.iter().map(|triangle| [triangle.a, triangle.b, triangle.c]),
            Color::rgba(0.2, 1.0, 1.0, 1.0),
        );
        if controller.grounded {
            debug_draw::arrow(&mut self.render_world, controller.feet_position, controller.ground_normal, Color::GREEN);
        }
        debug_draw::arrow(&mut self.render_world, center, controller.velocity * 0.25, Color::from_hex(0xFFE040));
        for gem in &scene.gems {
            if let Some(data) = self.world.get(gem.entity) {
                debug_draw::sphere(
                    &mut self.render_world,
                    data.local.translation,
                    GEM_TRIGGER_RADIUS,
                    Color::from_hex(0xFF60FF),
                );
            }
        }
    }

    fn draw_overlay_text(&mut self, screen: Vec2, scale: f32) {
        let overlay = &mut self.render_world.overlay;
        if self.state != GameState::MainMenu && !self.status_text.is_empty() {
            let size = measure_text(&self.status_text, scale * 1.5);
            let top_left = Vec2::new(screen.x * 0.5 - size.x * 0.5, 12.0 * scale);
            overlay.text(
                &self.status_text,
                top_left + Vec2::splat(scale),
                scale * 1.5,
                Color::rgba(0.0, 0.0, 0.0, 0.6),
            );
            overlay.text(&self.status_text, top_left, scale * 1.5, Color::WHITE);
        }
        if self.state == GameState::Playing
            && let Some((message, _)) = &self.message
        {
            let size = measure_text(message, scale * 2.0);
            let top_left = Vec2::new(screen.x * 0.5 - size.x * 0.5, screen.y * 0.3);
            overlay.text(message, top_left + Vec2::splat(scale * 1.5), scale * 2.0, Color::rgba(0.0, 0.0, 0.0, 0.6));
            overlay.text(message, top_left, scale * 2.0, Color::from_hex(0xFFD24D));
        }
        if let Some(error) = &self.rules_error
            && !self.debug_hud.visible
        {
            let line = format!("rules error (F1 for details): {error}");
            overlay.text(&line, Vec2::new(8.0 * scale, screen.y - 14.0 * scale), scale, Color::from_hex(0xFF6650));
        }
        self.debug_hud.draw(&self.debug_values, overlay, Vec2::splat(8.0 * scale), scale);
    }

    /// Headless verification aid: with `LANTERN_CAPTURE_PATH` set, the last frame is also
    /// rendered off-screen and saved as a PNG (used with `LANTERN_MAX_FRAMES` in smoke runs).
    fn save_capture_if_requested(&mut self, frame: &FrameContext) {
        let Some(path) = std::env::var_os("LANTERN_CAPTURE_PATH") else { return };
        match self.renderer.capture(&self.render_world, frame.window.pixel_width, frame.window.pixel_height) {
            Ok(image) => match std::fs::write(&path, image.to_png_bytes()) {
                Ok(()) => log::info!(target: "renderer", "saved capture to {}", path.to_string_lossy()),
                Err(error) => log::error!(target: "renderer", "saving capture: {error}"),
            },
            Err(error) => log::error!(target: "renderer", "capture failed: {error}"),
        }
    }

    /// The input the fixed step sees. In autoplay (a headless smoke test) a scripted pilot runs
    /// forward and jumps, so the controller, rules and triggers all get exercised.
    fn pilot_input(&self, input: &InputSnapshot) -> (Vec2, bool, bool, bool) {
        if !self.autoplay {
            return (
                input.vector(self.actions.movement),
                input.pressed(self.actions.jump),
                input.held(self.actions.jump),
                input.pressed(self.actions.reset),
            );
        }
        let time = self.simulation_seconds;
        let jump_pressed = time > 1.0 && (time % 1.5) < (1.0 / 60.0) + 1e-4;
        (Vec2::new(0.15, 1.0), jump_pressed, (time % 1.5) < 0.4, false)
    }
}

impl Game for PlatformerGame {
    fn fixed_update(&mut self, step: &FixedStepContext) {
        if self.state != GameState::Playing {
            return;
        }
        let dt = step.step_seconds;
        self.world.store_previous_transforms();
        if let Some(scene) = self.scene.as_mut() {
            scene.collision.begin_step();
        }
        self.simulation_seconds += dt;

        // Rules before physics: platform paths and anything else that moves the world.
        self.run_rules_hook("on_fixed_update", step.input, |rules, view, commands| {
            rules.on_fixed_update(view, dt, commands)
        });

        // Physics: the player.
        let (stick, jump_pressed, jump_held, reset_pressed) = self.pilot_input(step.input);
        let Some(scene) = self.scene.as_ref() else { return };
        let events = self.player.fixed_update(
            &scene.collision,
            stick,
            self.camera.movement_basis(),
            jump_pressed,
            jump_held,
            dt,
        );
        self.player.sync_entity(&mut self.world);

        // Events after physics, in the order they happened.
        if events.jumped {
            self.rule_events.push(RuleEvent::PlayerJumped);
        }
        if let Some(speed) = events.landed_with_speed {
            self.rule_events.push(RuleEvent::PlayerLanded { speed });
        }
        self.detect_trigger_entries();
        if self.player.controller.feet_position.y < self.level.kill_height {
            self.rule_events.push(RuleEvent::PlayerFell);
        }
        if reset_pressed {
            self.rule_events.push(RuleEvent::ResetPressed);
        }
        let events = std::mem::take(&mut self.rule_events);
        for event in &events {
            self.run_rules_hook("on_event", step.input, |rules, view, commands| rules.on_event(view, *event, commands));
        }
        self.rule_events = events;
        self.rule_events.clear();

        if let Some(scene) = &self.scene {
            scene.animate_gems(&mut self.world, self.simulation_seconds);
        }
        self.world.update_world_transforms();
    }

    fn frame_update(&mut self, frame: &FrameContext, platform: &mut dyn PlatformBackend) {
        self.frames += 1;
        if let Some(state) = self.pending_state.take() {
            self.set_state(state, platform);
        }
        let input = frame.input;
        let actions = self.actions;
        if input.pressed(actions.toggle_debug_hud) {
            self.debug_hud.toggle();
        }
        if input.pressed(actions.toggle_collision_view) {
            self.show_collision_view = !self.show_collision_view;
        }
        match self.state {
            GameState::MainMenu => {
                self.camera.yaw += MAIN_MENU_ORBIT_SPEED * frame.frame_seconds;
                if self.autoplay {
                    self.set_state(GameState::Playing, platform);
                }
            }
            GameState::Playing => {
                if input.pressed(actions.pause) {
                    self.set_state(GameState::Paused, platform);
                } else {
                    if input.pressed(actions.capture_mouse) {
                        platform.set_mouse_captured(true);
                    }
                    let look =
                        input.vector(actions.look_mouse) + input.vector(actions.look_stick) * frame.frame_seconds * 2.5;
                    self.camera.look(look);
                }
                if let Some((_, remaining)) = &mut self.message {
                    *remaining -= frame.frame_seconds;
                    if *remaining <= 0.0 {
                        self.message = None;
                    }
                }
            }
            GameState::Paused => {
                if input.pressed(actions.pause) {
                    self.set_state(GameState::Playing, platform);
                }
            }
        }
        if self.max_frames.is_some_and(|max| self.frames >= max) {
            self.quit = true;
        }
        self.update_debug_values(frame);
        self.audio.update();
    }

    fn render(&mut self, frame: &FrameContext) {
        self.render_world.clear();
        let screen = Vec2::new(frame.window.pixel_width as f32, frame.window.pixel_height as f32);
        let scale = text::scale_for_density(2, frame.window.pixel_density);

        let player_feet = self.world.get(self.player.entity).map_or(self.player.controller.feet_position, |data| {
            data.interpolated_world(frame.alpha).w_axis.truncate()
        });
        if let Some(scene) = &self.scene {
            self.render_world.camera = self.camera.update(player_feet, &scene.collision, frame.frame_seconds);
        }
        extract_draws(&self.world, frame.alpha, &mut self.render_world);
        if self.show_collision_view {
            self.draw_collision_view();
        }

        // Menus draw into the overlay and may change state; their input is this frame's edges.
        let ui_input = UiInput {
            up: frame.input.pressed(self.actions.menu_up),
            down: frame.input.pressed(self.actions.menu_down),
            confirm: frame.input.pressed(self.actions.menu_confirm),
        };
        let mut next_state = None;
        match self.state {
            GameState::MainMenu => {
                let previous_focus = self.menu_ui.focused();
                match menu::main_menu(&mut self.menu_ui, &mut self.render_world.overlay, ui_input, screen, scale) {
                    Some(MainMenuChoice::Play) => {
                        self.audio.play("menu_select", 1.0);
                        next_state = Some(GameState::Playing);
                    }
                    Some(MainMenuChoice::Quit) => self.quit = true,
                    None if self.menu_ui.focused() != previous_focus => self.audio.play("menu_move", 1.0),
                    None => {}
                }
            }
            GameState::Paused => {
                let previous_focus = self.menu_ui.focused();
                let choice = menu::pause_menu(
                    &mut self.menu_ui,
                    &mut self.render_world.overlay,
                    ui_input,
                    screen,
                    scale,
                    self.debug_hud.visible,
                );
                if choice.is_some() {
                    self.audio.play("menu_select", 1.0);
                } else if self.menu_ui.focused() != previous_focus {
                    self.audio.play("menu_move", 1.0);
                }
                match choice {
                    Some(PauseMenuChoice::Resume) => next_state = Some(GameState::Playing),
                    Some(PauseMenuChoice::Restart) => {
                        self.restart_level();
                        next_state = Some(GameState::Playing);
                    }
                    Some(PauseMenuChoice::ToggleDebugHud) => self.debug_hud.toggle(),
                    Some(PauseMenuChoice::MainMenu) => {
                        self.restart_level();
                        next_state = Some(GameState::MainMenu);
                    }
                    Some(PauseMenuChoice::Quit) => self.quit = true,
                    None => {}
                }
            }
            GameState::Playing => {}
        }
        self.draw_overlay_text(screen, scale);

        match self.renderer.render(&self.render_world) {
            Ok(report) => self.last_frame_report = report,
            Err(error) => log::error!(target: "renderer", "{error}"),
        }
        if self.quit {
            self.save_capture_if_requested(frame);
        }
        // The state change waits until the frame is drawn; the next frame_update applies the
        // mouse capture through the platform.
        if let Some(state) = next_state {
            self.pending_state = Some(state);
        }
    }

    fn wants_to_quit(&self) -> bool {
        self.quit
    }
}
