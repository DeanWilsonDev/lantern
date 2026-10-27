//! Measures what the platformer's Nyx code costs per frame: runs games/platformer headless under
//! the autopilot at 60 Hz through the real frame loop and the C ABI, then prints the time spent in
//! each hook. `cargo run --release -p lantern-game-host --example script_cost [frames]`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use lantern_engine::audio::AudioSystem;
use lantern_engine::frame_loop::FrameLoop;
use lantern_engine::input::InputRouter;
use lantern_game_host::{EngineState, GameHook, GameHost, GameManifest, HeadlessRenderer};
use lantern_platform::{PlatformBackend, PlatformEvent, WindowMetrics};

const FRAME_SECONDS: f64 = 1.0 / 60.0;
const WARM_UP_FRAMES: u64 = 120;
const DEFAULT_MEASURED_FRAMES: u64 = 1800;

#[derive(Default)]
struct SixtyHertzPlatform {
    frame_index: u64,
}

impl PlatformBackend for SixtyHertzPlatform {
    fn pump_events(&mut self, _events: &mut Vec<PlatformEvent>) {
        self.frame_index += 1;
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

    fn set_mouse_captured(&mut self, _captured: bool) {}
}

fn hook_totals(host: &GameHost) -> Vec<(GameHook, Duration, u64)> {
    let session = host.script_session().expect("the platformer loaded");
    GameHook::ALL
        .into_iter()
        .map(|hook| {
            let (time, calls) = session.hook_total(hook);
            (hook, time, calls)
        })
        .collect()
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * fraction).round() as usize]
}

fn main() {
    let measured_frames: u64 =
        std::env::args().nth(1).and_then(|text| text.parse().ok()).unwrap_or(DEFAULT_MEASURED_FRAMES);
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../games/platformer");
    let manifest = GameManifest::load(&directory).expect("game.json");
    let mut state = EngineState::new(
        Box::new(HeadlessRenderer::default()),
        AudioSystem::new(None),
        manifest.assets_directory(&directory),
    )
    .expect("engine state");
    state.flags.insert("LANTERN_AUTOPLAY".to_string(), true);
    let mut host = GameHost::new(state, manifest.script_source(&directory));
    let mut platform = SixtyHertzPlatform::default();
    let mut frame_loop = FrameLoop::new(InputRouter::new(Default::default(), Default::default()));

    for _ in 0..WARM_UP_FRAMES {
        frame_loop.run_frame(&mut platform, &mut host);
    }
    let before = hook_totals(&host);
    let mut frame_script_milliseconds = Vec::with_capacity(measured_frames as usize);
    let started = Instant::now();
    for _ in 0..measured_frames {
        let script_before: Duration = hook_totals(&host).iter().map(|(_, time, _)| *time).sum();
        frame_loop.run_frame(&mut platform, &mut host);
        let script_after: Duration = hook_totals(&host).iter().map(|(_, time, _)| *time).sum();
        frame_script_milliseconds.push((script_after - script_before).as_secs_f64() * 1000.0);
    }
    let wall = started.elapsed();
    if let Some(error) = host.script_error() {
        panic!("the game failed while measuring: {error}");
    }
    let after = hook_totals(&host);

    println!("{measured_frames} frames at 60 Hz after {WARM_UP_FRAMES} warm-up frames");
    println!("{:<14} {:>8} {:>12} {:>12}", "hook", "calls", "total ms", "µs per call");
    for ((hook, time_before, calls_before), (_, time_after, calls_after)) in before.iter().zip(&after) {
        let calls = calls_after - calls_before;
        if calls == 0 {
            continue;
        }
        let time = *time_after - *time_before;
        println!(
            "{:<14} {:>8} {:>12.2} {:>12.1}",
            hook.method_name(),
            calls,
            time.as_secs_f64() * 1000.0,
            time.as_secs_f64() * 1e6 / calls as f64
        );
    }
    let mut sorted = frame_script_milliseconds.clone();
    sorted.sort_by(f64::total_cmp);
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
    println!(
        "script ms per frame: mean {mean:.3}, p50 {:.3}, p95 {:.3}, max {:.3}",
        percentile(&sorted, 0.5),
        percentile(&sorted, 0.95),
        sorted[sorted.len() - 1]
    );
    println!("whole frame (headless) ms: {:.3}", wall.as_secs_f64() * 1000.0 / measured_frames as f64);
}
