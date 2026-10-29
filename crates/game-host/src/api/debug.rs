//! `EngineDebug`: the HUD's values and debug drawing. The engine fills the `Frame`, `Renderer`,
//! `Audio`, `World` and `Script` groups itself; the game adds its own (`Player/Velocity`).

use glam::Vec3;
use lantern_core::math::Aabb;
use lantern_debug::debug_draw;
use nyx::Runtime;

use super::{
    SharedState, SharedTypes, bool_argument, color_argument, f32_argument, some_bool, string_argument, vec3_argument,
};

struct DebugApi;

pub fn register(runtime: &Runtime, state: &SharedState, types: &SharedTypes) -> nyx::Result<()> {
    let set_value = state.clone();
    let hud_visible = state.clone();
    let set_hud_visible = state.clone();
    let capsule = (state.clone(), types.clone());
    let sphere = (state.clone(), types.clone());
    let arrow = (state.clone(), types.clone());
    let line = (state.clone(), types.clone());
    let collision = (state.clone(), types.clone());
    runtime
        .register_type::<DebugApi>("EngineDebug")
        // A HUD entry. The first path segment is its group: "Player/Speed".
        .static_method("SetValue", "(path: string, value: string) -> void", move |_context, _, arguments| {
            let path = string_argument(arguments, 0)?;
            let value = string_argument(arguments, 1)?;
            set_value.borrow_mut().debug_values.set(&path, value);
            Ok(None)
        })
        .static_method("IsHudVisible", "() -> bool", move |context, _, _| {
            some_bool(context, hud_visible.borrow().debug_hud.visible)
        })
        .static_method("SetHudVisible", "(visible: bool) -> void", move |_context, _, arguments| {
            set_hud_visible.borrow_mut().debug_hud.visible = bool_argument(arguments, 0)?;
            Ok(None)
        })
        .static_method(
            "Capsule",
            "(center: Vec3, radius: f32, halfSegment: f32, color: string) -> void",
            move |context, _, arguments| {
                let center = vec3_argument(&capsule.1, arguments, 0)?;
                let color = color_argument(context, arguments, 3)?;
                debug_draw::capsule(
                    &mut capsule.0.borrow_mut().render_world,
                    center,
                    f32_argument(arguments, 1)?,
                    f32_argument(arguments, 2)?,
                    color,
                );
                Ok(None)
            },
        )
        .static_method("Sphere", "(center: Vec3, radius: f32, color: string) -> void", move |context, _, arguments| {
            let center = vec3_argument(&sphere.1, arguments, 0)?;
            let color = color_argument(context, arguments, 2)?;
            debug_draw::sphere(&mut sphere.0.borrow_mut().render_world, center, f32_argument(arguments, 1)?, color);
            Ok(None)
        })
        .static_method("Arrow", "(start: Vec3, vector: Vec3, color: string) -> void", move |context, _, arguments| {
            let start = vec3_argument(&arrow.1, arguments, 0)?;
            let vector = vec3_argument(&arrow.1, arguments, 1)?;
            let color = color_argument(context, arguments, 2)?;
            debug_draw::arrow(&mut arrow.0.borrow_mut().render_world, start, vector, color);
            Ok(None)
        })
        .static_method("Line", "(start: Vec3, end: Vec3, color: string) -> void", move |context, _, arguments| {
            let start = vec3_argument(&line.1, arguments, 0)?;
            let end = vec3_argument(&line.1, arguments, 1)?;
            let color = color_argument(context, arguments, 2)?;
            line.0.borrow_mut().render_world.line(start, end, color);
            Ok(None)
        })
        // The edges of every collision triangle within `halfExtent` metres of `center`.
        .static_method(
            "CollisionNear",
            "(center: Vec3, halfExtent: f32, color: string) -> void",
            move |context, _, arguments| {
                let center = vec3_argument(&collision.1, arguments, 0)?;
                let half_extent = f32_argument(arguments, 1)?;
                let color = color_argument(context, arguments, 2)?;
                let state = &mut *collision.0.borrow_mut();
                let bounds = Aabb::from_center_half_extents(center, Vec3::splat(half_extent));
                let triangles = state.collision.triangles_near(&bounds);
                debug_draw::triangle_edges(
                    &mut state.render_world,
                    triangles.iter().map(|triangle| [triangle.a, triangle.b, triangle.c]),
                    color,
                );
                Ok(None)
            },
        )
        .register()?;
    Ok(())
}
