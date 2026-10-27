//! `Camera`, `Renderer` and `Screen`: what the frame looks like and how big the window is.

use lantern_renderer_3d::Camera;
use nyx::Runtime;

use super::{SharedState, SharedTypes, bool_argument, color_argument, f32_argument, some_f32, vec3_argument};

struct CameraApi;
struct RendererApi;
struct ScreenApi;

pub fn register(runtime: &Runtime, state: &SharedState, types: &SharedTypes) -> nyx::Result<()> {
    let look_at = (state.clone(), types.clone());
    runtime
        .register_type::<CameraApi>("Camera")
        // Sets this frame's camera (call it from `OnRender`).
        .static_method("LookAt", "(eye: Vec3, target: Vec3) -> void", move |_context, _, arguments| {
            let eye = vec3_argument(&look_at.1, arguments, 0)?;
            let target = vec3_argument(&look_at.1, arguments, 1)?;
            look_at.0.borrow_mut().render_world.camera = Camera::look_at(eye, target);
            Ok(None)
        })
        .register()?;

    let clear_color = state.clone();
    let fog = state.clone();
    runtime
        .register_type::<RendererApi>("Renderer")
        .static_method("SetClearColor", "(color: string) -> void", move |context, _, arguments| {
            let color = color_argument(context, arguments, 0)?;
            clear_color.borrow_mut().render_world.clear_color = color;
            Ok(None)
        })
        // Linear fog toward the clear colour between `start` and `end` metres.
        .static_method("SetFog", "(start: f32, end: f32) -> void", move |_context, _, arguments| {
            let mut state = fog.borrow_mut();
            state.render_world.fog_start = f32_argument(arguments, 0)?;
            state.render_world.fog_end = f32_argument(arguments, 1)?;
            Ok(None)
        })
        .register()?;

    let width = state.clone();
    let height = state.clone();
    let scale = state.clone();
    let capture = state.clone();
    runtime
        .register_type::<ScreenApi>("Screen")
        .static_method("Width", "() -> f32", move |context, _, _| some_f32(context, width.borrow().screen_size.x))
        .static_method("Height", "() -> f32", move |context, _, _| some_f32(context, height.borrow().screen_size.y))
        // Whole-pixel text scale for the display's density: the size of one font pixel.
        .static_method("UiScale", "() -> f32", move |context, _, _| some_f32(context, scale.borrow().ui_scale))
        // Hides the cursor and turns mouse motion into look input, or gives the cursor back.
        .static_method("SetMouseCaptured", "(captured: bool) -> void", move |_context, _, arguments| {
            capture.borrow_mut().requests.mouse_captured = Some(bool_argument(arguments, 0)?);
            Ok(None)
        })
        .register()?;
    Ok(())
}
