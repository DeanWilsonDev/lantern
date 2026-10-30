//! `Camera`, `Renderer` and `Screen`: what the frame looks like and how big the window is.

use lantern_renderer_3d::Camera;
use nyx::Runtime;

use super::{
    HostNamespace, SharedState, SharedTypes, bool_argument, color_argument, f32_argument, some_f32, vec3_argument,
};

pub fn register(runtime: &Runtime, state: &SharedState, types: &SharedTypes) -> nyx::Result<()> {
    let look_at = (state.clone(), types.clone());
    HostNamespace::new(runtime, "lantern::camera")
        // Sets this frame's camera (call it from `OnRender`).
        .function(
            "LookAt",
            "(eye: lantern:::math:::Vec3, target: lantern:::math:::Vec3) -> void",
            move |_context, arguments| {
                let eye = vec3_argument(&look_at.1, arguments, 0)?;
                let target = vec3_argument(&look_at.1, arguments, 1)?;
                look_at.0.borrow_mut().render_world.camera = Camera::look_at(eye, target);
                Ok(None)
            },
        )
        .register()?;

    let clear_color = state.clone();
    let fog = state.clone();
    HostNamespace::new(runtime, "lantern::renderer")
        .function("SetClearColor", "(color: string) -> void", move |context, arguments| {
            let color = color_argument(context, arguments, 0)?;
            clear_color.borrow_mut().render_world.clear_color = color;
            Ok(None)
        })
        // Linear fog toward the clear colour between `start` and `end` metres.
        .function("SetFog", "(start: f32, end: f32) -> void", move |_context, arguments| {
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
    HostNamespace::new(runtime, "lantern::screen")
        .function("Width", "() -> f32", move |context, _| some_f32(context, width.borrow().screen_size.x))
        .function("Height", "() -> f32", move |context, _| some_f32(context, height.borrow().screen_size.y))
        // Whole-pixel text scale for the display's density: the size of one font pixel.
        .function("UiScale", "() -> f32", move |context, _| some_f32(context, scale.borrow().ui_scale))
        // Hides the cursor and turns mouse motion into look input, or gives the cursor back.
        .function("SetMouseCaptured", "(captured: bool) -> void", move |_context, arguments| {
            capture.borrow_mut().requests.mouse_captured = Some(bool_argument(arguments, 0)?);
            Ok(None)
        })
        .register()?;
    Ok(())
}
