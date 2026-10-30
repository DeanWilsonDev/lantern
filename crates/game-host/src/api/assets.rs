//! `Textures` and `Meshes`: GPU resources the game creates. Each returns an `i64` handle and
//! is released when the game is reloaded or closed. Paths are relative to the assets
//! directory. A file that cannot be loaded throws `Error:::NotFound`, so the game decides what
//! to use instead (the platformer falls back to generated textures and meshes).

use lantern_renderer_3d::gltf_loader::load_gltf_mesh;
use lantern_renderer_3d::{MeshData, TextureData, primitives};
use nyx::{CallContext, CallbackError, Runtime, Value};

use super::{
    HostNamespace, SharedState, color_argument, f32_argument, handle_to_script, i32_argument, invalid, not_found,
    some_i64, string_argument,
};

fn create_texture(context: &CallContext, state: &SharedState, texture: &TextureData) -> nyx::CallbackResult {
    let mut state = state.borrow_mut();
    let handle = state.renderer.create_texture(texture).map_err(|error| invalid(context, error))?;
    state.script_textures.push(handle);
    some_i64(context, handle_to_script(handle))
}

fn create_mesh(context: &CallContext, state: &SharedState, mesh: &MeshData) -> nyx::CallbackResult {
    let mut state = state.borrow_mut();
    let handle = state.renderer.create_mesh(mesh).map_err(|error| invalid(context, error))?;
    state.script_meshes.push(handle);
    some_i64(context, handle_to_script(handle))
}

fn positive_count(context: &CallContext, arguments: &[Value], index: usize) -> Result<u32, CallbackError> {
    let count = i32_argument(arguments, index)?;
    u32::try_from(count)
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| invalid(context, format!("{count} must be positive")))
}

pub fn register(runtime: &Runtime, state: &SharedState) -> nyx::Result<()> {
    let load_texture = state.clone();
    let checker = state.clone();
    let solid = state.clone();
    HostNamespace::new(runtime, "lantern::textures")
        .function("Load", "(path: string) -> i64", move |context, arguments| {
            let relative = string_argument(arguments, 0)?;
            let path = load_texture.borrow().asset_path(&relative);
            let texture = TextureData::load_png(&path).map_err(|error| not_found(context, error.0))?;
            create_texture(context, &load_texture, &texture)
        })
        .function(
            "Checker",
            "(size: i32, cells: i32, first: string, second: string) -> i64",
            move |context, arguments| {
                let size = positive_count(context, arguments, 0)?;
                let cells = positive_count(context, arguments, 1)?;
                let first = color_argument(context, arguments, 2)?;
                let second = color_argument(context, arguments, 3)?;
                create_texture(context, &checker, &TextureData::checker(size, cells, first, second))
            },
        )
        .function("Solid", "(color: string) -> i64", move |context, arguments| {
            let color = color_argument(context, arguments, 0)?;
            create_texture(context, &solid, &TextureData::solid(color))
        })
        .register()?;

    let load_mesh = state.clone();
    let capsule = state.clone();
    let sphere = state.clone();
    let cube = state.clone();
    let gem = state.clone();
    HostNamespace::new(runtime, "lantern::meshes")
        .function("Load", "(path: string) -> i64", move |context, arguments| {
            let relative = string_argument(arguments, 0)?;
            let path = load_mesh.borrow().asset_path(&relative);
            let mesh = load_gltf_mesh(&path).map_err(|error| not_found(context, error.to_string()))?;
            create_mesh(context, &load_mesh, &mesh)
        })
        .function(
            "Capsule",
            "(radius: f32, halfHeight: f32, segments: i32, rings: i32) -> i64",
            move |context, arguments| {
                let mesh = primitives::capsule(
                    f32_argument(arguments, 0)?,
                    f32_argument(arguments, 1)?,
                    positive_count(context, arguments, 2)?,
                    positive_count(context, arguments, 3)?,
                );
                create_mesh(context, &capsule, &mesh)
            },
        )
        .function("Sphere", "(radius: f32, segments: i32, rings: i32) -> i64", move |context, arguments| {
            let mesh = primitives::sphere(
                f32_argument(arguments, 0)?,
                positive_count(context, arguments, 1)?,
                positive_count(context, arguments, 2)?,
            );
            create_mesh(context, &sphere, &mesh)
        })
        .function("Cube", "(size: f32) -> i64", move |context, arguments| {
            create_mesh(context, &cube, &primitives::cube(f32_argument(arguments, 0)?))
        })
        .function("Gem", "(radius: f32, height: f32) -> i64", move |context, arguments| {
            create_mesh(context, &gem, &primitives::gem(f32_argument(arguments, 0)?, f32_argument(arguments, 1)?))
        })
        .register()?;
    Ok(())
}
