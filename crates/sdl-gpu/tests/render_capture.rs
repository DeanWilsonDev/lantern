//! Renders known scenes off-screen and checks pixels. Needs a display (X11 or Wayland) and a GPU
//! or a software Vulkan driver such as lavapipe; without one the test prints why and passes, so
//! `cargo test` stays green on headless CI. Run with `LANTERN_REQUIRE_GPU=1` to make that a
//! failure instead.

use glam::{Mat4, Vec2, Vec3};
use lantern_core::color::Color;
use lantern_platform::WindowConfig;
use lantern_renderer_3d::{Camera, RenderWorld, primitives, text};
use lantern_sdl_facade::{SdlContext, SdlWindow};
use lantern_sdl_gpu::{GpuOptions, GpuRenderer};

fn pixel(image: &lantern_renderer_3d::TextureData, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * image.width + x) * 4) as usize;
    image.rgba8[offset..offset + 4].try_into().unwrap()
}

/// SDL video is not thread-safe and the test harness runs tests on several threads, so the
/// tests take turns.
static SDL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn setup() -> Option<(SdlContext, SdlWindow, GpuRenderer)> {
    let require = std::env::var("LANTERN_REQUIRE_GPU").is_ok_and(|value| value == "1");
    let skip = |reason: String| {
        assert!(!require, "GPU required but unavailable: {reason}");
        eprintln!("skipping GPU test: {reason}");
        None
    };
    let sdl = match SdlContext::init() {
        Ok(sdl) => sdl,
        Err(error) => return skip(error.to_string()),
    };
    let window = match SdlWindow::create(
        &sdl,
        &WindowConfig { title: "capture test".into(), width: 320, height: 240, ..Default::default() },
    ) {
        Ok(window) => window,
        Err(error) => return skip(error.to_string()),
    };
    let options = GpuOptions { debug: true, driver: None, vsync: true };
    match GpuRenderer::new(&window, &options) {
        Ok(renderer) => Some((sdl, window, renderer)),
        Err(error) => skip(error.to_string()),
    }
}

#[test]
fn a_lit_cube_lines_and_text_appear_where_expected() {
    let _turn = SDL_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some((_sdl, _window, mut renderer)) = setup() else { return };
    let cube = renderer.create_mesh(&primitives::cube(1.0)).unwrap();
    let atlas = renderer.create_texture(&text::font_atlas()).unwrap();

    let mut world = RenderWorld {
        camera: Camera::look_at(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO),
        clear_color: Color::rgb(0.0, 0.0, 1.0),
        overlay_texture: Some(atlas),
        ..Default::default()
    };
    world.draw(cube, None, Mat4::IDENTITY, Color::rgb(1.0, 0.0, 0.0));
    // Behind the camera: must be culled, not drawn.
    world.draw(cube, None, Mat4::from_translation(Vec3::new(0.0, 0.0, 10.0)), Color::WHITE);
    world.line(Vec3::new(-5.0, -0.9, 0.0), Vec3::new(5.0, -0.9, 0.0), Color::GREEN);
    world.overlay.rect(Vec2::new(0.0, 0.0), Vec2::new(16.0, 16.0), Color::WHITE);
    world.overlay.text("Hi", Vec2::new(20.0, 0.0), 2.0, Color::YELLOW);

    let image = renderer.capture(&world, 128, 128).unwrap();
    if let Ok(path) = std::env::var("LANTERN_CAPTURE_PATH") {
        std::fs::write(path, image.to_png_bytes()).unwrap();
    }
    let center = pixel(&image, 64, 64);
    assert!(center[0] > 60 && center[1] < 40 && center[2] < 40, "the cube's lit front face is red: {center:?}");
    let corner = pixel(&image, 127, 127);
    assert!(corner[2] > 200 && corner[0] < 30, "the clear colour fills the corner: {corner:?}");
    assert_eq!(pixel(&image, 4, 4)[..3], [255, 255, 255], "the overlay rectangle is white");
    let green_row = (80..128).any(|y| {
        let p = pixel(&image, 10, y);
        p[1] > 200 && p[2] < 200
    });
    assert!(green_row, "the debug line crosses the bottom of the view");

    let frame = renderer.render(&world).unwrap();
    assert_eq!(frame.draws_culled, 1, "the cube behind the camera is culled");
    assert_eq!(frame.draws_submitted, 1);
}

#[test]
fn resources_can_be_released_and_handles_go_stale() {
    let _turn = SDL_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some((_sdl, _window, mut renderer)) = setup() else { return };
    let first = renderer.create_mesh(&primitives::cube(1.0)).unwrap();
    renderer.remove_mesh(first);
    let second = renderer.create_mesh(&primitives::sphere(0.5, 8, 4)).unwrap();
    assert_ne!(first, second);
    assert_eq!(renderer.mesh_count(), 1);
    let mut world = RenderWorld::default();
    world.draw(first, None, Mat4::IDENTITY, Color::WHITE); // stale: skipped with a warning
    world.draw(second, None, Mat4::IDENTITY, Color::WHITE);
    let report = renderer.render(&world).unwrap();
    assert_eq!(report.draws_submitted + report.draws_culled, 1);
}
