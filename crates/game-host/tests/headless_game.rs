//! Runs the platformer (games/platformer, all Nyx) through the real frame loop with a headless
//! renderer and a scripted platform: the scripts analyse, start, play under the autopilot and
//! reload without leaking engine resources.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;

use lantern_core::input::{KeyCode, MouseButton};
use lantern_engine::audio::AudioSystem;
use lantern_engine::frame_loop::FrameLoop;
use lantern_engine::input::InputRouter;
use lantern_game_host::{EngineState, GameHost, GameManifest, HeadlessRenderer, HeadlessStats};
use lantern_platform::{PlatformBackend, PlatformEvent, WindowMetrics};

const FRAME_SECONDS: f64 = 1.0 / 60.0;

/// A 60 Hz clock and a queue of events per frame.
#[derive(Default)]
struct ScriptedPlatform {
    frame_index: u64,
    queued_events: VecDeque<Vec<PlatformEvent>>,
    mouse_captured: bool,
}

impl ScriptedPlatform {
    fn press_key_next_frame(&mut self, key: KeyCode) {
        self.queued_events.push_back(vec![PlatformEvent::KeyDown { key, repeat: false }]);
        self.queued_events.push_back(vec![PlatformEvent::KeyUp { key }]);
    }
}

impl ScriptedPlatform {
    fn click_next_frame(&mut self, position: glam::Vec2) {
        let (x, y) = (position.x, position.y);
        self.queued_events.push_back(vec![PlatformEvent::MouseMoved { delta_x: 0.0, delta_y: 0.0, x, y }]);
        self.queued_events.push_back(vec![
            PlatformEvent::MouseButtonDown { button: MouseButton::Left, x, y },
            PlatformEvent::MouseButtonUp { button: MouseButton::Left, x, y },
        ]);
    }

    fn hold_key_for_frames(&mut self, key: KeyCode, frames: usize) {
        self.queued_events.push_back(vec![PlatformEvent::KeyDown { key, repeat: false }]);
        for _ in 1..frames {
            self.queued_events.push_back(Vec::new());
        }
        self.queued_events.push_back(vec![PlatformEvent::KeyUp { key }]);
    }
}

impl PlatformBackend for ScriptedPlatform {
    fn pump_events(&mut self, events: &mut Vec<PlatformEvent>) {
        self.frame_index += 1;
        if let Some(frame_events) = self.queued_events.pop_front() {
            events.extend(frame_events);
        }
    }

    fn seconds_since_start(&self) -> f64 {
        self.frame_index as f64 * FRAME_SECONDS
    }

    fn window_metrics(&self) -> WindowMetrics {
        WindowMetrics {
            logical_width: 1280,
            logical_height: 720,
            pixel_width: 1280,
            pixel_height: 720,
            pixel_density: 1.0,
            minimized: false,
        }
    }

    fn set_mouse_captured(&mut self, captured: bool) {
        self.mouse_captured = captured;
    }
}

fn platformer_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../games/platformer")
}

fn start_platformer(flags: &[(&str, bool)]) -> (GameHost, Rc<RefCell<HeadlessStats>>) {
    let directory = platformer_directory();
    let manifest = GameManifest::load(&directory).expect("game.json");
    let renderer = HeadlessRenderer::default();
    let stats = renderer.stats();
    let mut state = EngineState::new(Box::new(renderer), AudioSystem::new(None), manifest.assets_directory(&directory))
        .expect("engine state");
    for (name, value) in flags {
        state.flags.insert((*name).to_string(), *value);
    }
    let host = GameHost::new(state, manifest.script_source(&directory));
    (host, stats)
}

fn run_frames(frame_loop: &mut FrameLoop, platform: &mut ScriptedPlatform, host: &mut GameHost, count: u32) {
    for _ in 0..count {
        frame_loop.run_frame(platform, host);
    }
}

fn debug_value(host: &GameHost, path: &str) -> String {
    host.state().debug_values.get(path).map(|value| value.to_string()).unwrap_or_default()
}

fn assert_no_script_error(host: &GameHost) {
    assert_eq!(host.script_error(), None, "script error");
    assert_eq!(host.script_error_count(), 0);
}

#[test]
fn the_platformer_loads_and_plays_under_the_autopilot() {
    let (mut host, stats) = start_platformer(&[("LANTERN_AUTOPLAY", true)]);
    assert_no_script_error(&host);
    assert!(host.is_running_script());
    let mut platform = ScriptedPlatform::default();
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));

    let mut farthest_z = f32::MAX;
    let mut was_reset_after_falling = false;
    for _ in 0..60 {
        run_frames(&mut frame_loop, &mut platform, &mut host, 10);
        let position = debug_value(&host, "Player/Position");
        let z: f32 = position.split(", ").nth(2).and_then(|text| text.parse().ok()).expect("Player/Position");
        was_reset_after_falling |= farthest_z < -9.0 && z > 1.0;
        farthest_z = farthest_z.min(z);
    }

    assert_no_script_error(&host);
    assert!(stats.borrow().frames_rendered >= 590);
    assert!(stats.borrow().last_draw_count > 10, "the course and the player are drawn");
    let gems = debug_value(&host, "Game/Gems");
    assert!(gems.ends_with("/ 6"), "six gems in the course: {gems}");
    let time: f32 = debug_value(&host, "Game/Time").parse().expect("Game/Time");
    assert!(time > 9.0, "the simulation ran: {time}");
    assert!(farthest_z < -9.0, "the autopilot ran from the spawn (z = 4) toward the pillars: {farthest_z}");
    assert!(was_reset_after_falling, "falling below the kill plane put the player back at the spawn");
    assert!(!platform.mouse_captured, "the autopilot never captures the mouse");
}

#[test]
fn reloading_the_game_releases_what_it_made() {
    let (mut host, stats) = start_platformer(&[("LANTERN_AUTOPLAY", true)]);
    let mut platform = ScriptedPlatform::default();
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));
    run_frames(&mut frame_loop, &mut platform, &mut host, 30);
    assert_no_script_error(&host);
    let (meshes, textures) = (stats.borrow().live_meshes, stats.borrow().live_textures);
    let entities = host.state().world.len();
    assert!(meshes > 0 && textures > 1 && entities > 10);

    for _ in 0..3 {
        platform.press_key_next_frame(KeyCode::F5);
        run_frames(&mut frame_loop, &mut platform, &mut host, 30);
    }

    assert_no_script_error(&host);
    assert_eq!(stats.borrow().live_meshes, meshes, "meshes after three reloads");
    assert_eq!(stats.borrow().live_textures, textures, "textures after three reloads");
    assert_eq!(host.state().world.len(), entities, "entities after three reloads");

    host.shutdown();
    assert_eq!(stats.borrow().live_meshes, 0);
    assert_eq!(stats.borrow().live_textures, 1, "only the engine's font atlas is left");
    assert_eq!(host.state().world.len(), 0);
}

#[test]
fn the_main_menu_waits_for_play() {
    let (mut host, _stats) = start_platformer(&[]);
    let mut platform = ScriptedPlatform::default();
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));
    run_frames(&mut frame_loop, &mut platform, &mut host, 60);
    assert_no_script_error(&host);
    assert_eq!(debug_value(&host, "Game/Time"), "0.0", "nothing simulates behind the menu");

    platform.press_key_next_frame(KeyCode::Enter);
    run_frames(&mut frame_loop, &mut platform, &mut host, 60);
    assert_no_script_error(&host);
    let time: f32 = debug_value(&host, "Game/Time").parse().expect("Game/Time");
    assert!(time > 0.5, "Play was chosen: {time}");
    assert!(platform.mouse_captured, "playing captures the mouse");
}

#[test]
fn clicking_play_with_the_mouse_starts_the_game() {
    let (mut host, _stats) = start_platformer(&[]);
    let mut platform = ScriptedPlatform::default();
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));
    run_frames(&mut frame_loop, &mut platform, &mut host, 2);
    let buttons = host.script_session().expect("the game loaded").ui_button_rects();
    assert_eq!(buttons.len(), 2, "Play and Quit");
    let play = buttons[0].1;

    platform.click_next_frame((play.min + play.max) * 0.5);
    run_frames(&mut frame_loop, &mut platform, &mut host, 60);
    assert_no_script_error(&host);
    let time: f32 = debug_value(&host, "Game/Time").parse().expect("Game/Time");
    assert!(time > 0.5, "Play was clicked: {time}");
    assert!(platform.mouse_captured, "playing captures the mouse");
}

#[test]
fn walking_into_a_gem_collects_it() {
    let (mut host, _stats) = start_platformer(&[]);
    let mut platform = ScriptedPlatform::default();
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));
    platform.press_key_next_frame(KeyCode::Enter);
    run_frames(&mut frame_loop, &mut platform, &mut host, 2);
    // From the spawn at (-1, 0, 4): left to the stairs at x = -5.5, then up them to gem_0 on the
    // stair top at z = 0.5.
    platform.hold_key_for_frames(KeyCode::A, 50);
    platform.hold_key_for_frames(KeyCode::W, 70);
    run_frames(&mut frame_loop, &mut platform, &mut host, 180);
    assert_no_script_error(&host);
    let position = debug_value(&host, "Player/Position");
    let gems = debug_value(&host, "Game/Gems");
    assert_eq!(gems, "1 / 6", "the player at {position} collected gem_0");
}

#[test]
fn a_hook_error_names_the_nyx_line_and_its_callers() {
    let directory = std::env::temp_dir().join(format!("lantern-hook-error-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("main.nyx"),
        "Broken :: class : Application {\npublic:\n    Run :: () -> void { }\n\
         \x20   OnFixedUpdate~: (dt: f32) -> void {\n        lantern:::world:::SetYaw(Target(), dt);\n    }\n\
         \x20   Target :: () -> i64 {\n        return lantern:::world:::Find(\"missing\");\n    }\n}\n",
    )
    .unwrap();
    let state =
        EngineState::new(Box::new(HeadlessRenderer::default()), AudioSystem::new(None), None).expect("engine state");
    let source = lantern_game_host::ScriptSource {
        scripts_directory: directory.clone(),
        entry_file: "main.nyx".to_string(),
        application_class: "Broken".to_string(),
    };
    let mut host = GameHost::new(state, source);
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));
    let mut platform = ScriptedPlatform::default();
    run_frames(&mut frame_loop, &mut platform, &mut host, 10);
    let error = host.script_error().expect("the hook fails").to_string();
    std::fs::remove_dir_all(&directory).ok();
    assert!(error.starts_with("OnFixedUpdate: "), "{error}");
    assert!(error.contains(" at Broken.Target ("), "{error}");
    assert!(error.contains("main.nyx:8:9)"), "{error}");
    assert!(error.contains("\n    called from Broken.OnFixedUpdate ("), "{error}");
}
