//! `World`: entities, their transforms and meshes, and world files.
//!
//! Entities are `i64` ids. A world file (`lantern:::world:::Load`) spawns one entity per file entity,
//! with its render mesh and collision; game code finds them by `kind` and reads their
//! `properties` (`lantern:::world:::FindByKind("gem")`, `lantern:::world:::NumberProperty(id, "period_seconds")`).
//! Everything spawned here belongs to the game and is removed by `lantern:::world:::Clear` or a reload.

use std::collections::HashMap;

use glam::{Quat, Vec3};
use lantern_engine::world::{EntityId, MeshRenderer, Transform};
use lantern_physics::Triangle;
use lantern_world_format::{ColliderKind, PropertyValue, WorldFile, build_pieces};
use nyx::{CallContext, CallbackError, Runtime, Value};

use super::{
    HostNamespace, SharedState, SharedTypes, color_argument, entity_from_script, entity_to_script, f32_argument,
    handle_from_script, i64_argument, not_found, some_bool, some_f32, some_i64, some_string, string_argument,
    vec3_argument, wrap_vec3,
};
use crate::engine_state::{EngineState, EntityTags, GAME_SCENE};

fn live_entity(
    context: &CallContext,
    state: &EngineState,
    arguments: &[Value],
    index: usize,
) -> Result<EntityId, CallbackError> {
    let id = i64_argument(arguments, index)?;
    let entity = entity_from_script(id);
    if state.world.contains(entity) { Ok(entity) } else { Err(not_found(context, format!("no entity with id {id}"))) }
}

/// Spawns a world file's entities, meshes and collision. Returns how many entities it made.
pub fn load_world_file(state: &mut EngineState, world_file: &WorldFile) -> Result<usize, String> {
    let mut entities_by_key = HashMap::new();
    let pieces = build_pieces(world_file);
    for piece in &pieces {
        let transform = Transform::from_matrix(&piece.local_matrix);
        let entity = match &piece.mesh {
            Some(mesh_data) => {
                let mesh = state.renderer.create_mesh(mesh_data)?;
                state.world_meshes.push(mesh);
                let texture = piece.material.and_then(|material| {
                    let texture = state.materials.get(material).copied();
                    if texture.is_none() {
                        log::warn!(target: "world", "'{}': no texture for material '{material}'", piece.name);
                    }
                    texture
                });
                state.world.spawn_mesh(piece.name, GAME_SCENE, transform, MeshRenderer::new(mesh, texture, piece.tint))
            }
            None => state.world.spawn(piece.name, GAME_SCENE, transform),
        };
        entities_by_key.insert(piece.key, entity);
        match piece.collider {
            Some(ColliderKind::Static) => {
                state.collision.add_static_triangles(piece.world_triangles());
            }
            Some(ColliderKind::Kinematic) => {
                let triangles: Vec<Triangle> = piece.local_triangles();
                let body = state.collision.add_body(triangles, piece.world_matrix);
                state.entity_bodies.insert(entity, body);
            }
            None => {}
        }
        let properties = world_file.entity(piece.key).map(|entity| entity.properties.clone()).unwrap_or_default();
        state.entity_tags.insert(entity, EntityTags { kind: piece.kind.to_string(), properties });
    }
    for piece in &pieces {
        if let (Some(parent), Some(&child)) = (piece.parent, entities_by_key.get(&piece.key))
            && let Some(&parent) = entities_by_key.get(&parent)
        {
            state.world.set_parent(child, Some(parent)).map_err(|error| format!("{}: {error:?}", piece.name))?;
        }
    }
    state.world.update_world_transforms();
    state.world.store_previous_transforms();
    Ok(pieces.len())
}

/// Moves an entity, and its kinematic body if it has one, so riders are carried.
fn set_position(state: &mut EngineState, entity: EntityId, position: Vec3) {
    let Some(data) = state.world.get_mut(entity) else { return };
    data.local.translation = position;
    let matrix = data.local.matrix();
    if let Some(&body) = state.entity_bodies.get(&entity) {
        state.collision.set_body_transform(body, matrix);
    }
}

fn property<'a>(
    context: &CallContext,
    state: &'a EngineState,
    entity: EntityId,
    key: &str,
) -> Result<&'a PropertyValue, CallbackError> {
    state
        .entity_tags
        .get(&entity)
        .and_then(|tags| tags.properties.get(key))
        .ok_or_else(|| not_found(context, format!("the entity has no property '{key}'")))
}

pub fn register(runtime: &Runtime, state: &SharedState, types: &SharedTypes) -> nyx::Result<()> {
    let load = state.clone();
    let clear = state.clone();
    let spawn = (state.clone(), types.clone());
    let despawn = state.clone();
    let find = state.clone();
    let find_kind = state.clone();
    let kind = state.clone();
    let has_property = state.clone();
    let number_property = state.clone();
    let text_property = state.clone();
    let vector_property = (state.clone(), types.clone());
    let set_material = state.clone();
    let set_mesh = state.clone();
    let set_textured_mesh = state.clone();
    let set_tint = state.clone();
    let set_parent = state.clone();
    let position = (state.clone(), types.clone());
    let world_position = (state.clone(), types.clone());
    let interpolated = (state.clone(), types.clone());
    let set_position_state = (state.clone(), types.clone());
    let teleport = (state.clone(), types.clone());
    let set_yaw = state.clone();
    let set_scale = (state.clone(), types.clone());
    let visible = state.clone();
    let set_visible = state.clone();
    HostNamespace::new(runtime, "lantern::world")
        .function("Load", "(path: string) -> i32", move |context, arguments| {
            let relative = string_argument(arguments, 0)?;
            let mut state = load.borrow_mut();
            let path = state.asset_path(&relative);
            let world_file = WorldFile::load(&path).map_err(|error| not_found(context, error.to_string()))?;
            for problem in world_file.validate() {
                log::warn!(target: "world", "{}: {problem}", path.display());
            }
            let count = load_world_file(&mut state, &world_file).map_err(|error| super::invalid(context, error))?;
            log::info!(target: "world", "loaded {}: {count} entities", path.display());
            Ok(Some(context.make_i32(count as i32)?))
        })
        .function("Clear", "() -> void", move |_context, _| {
            clear.borrow_mut().clear_world();
            Ok(None)
        })
        .function("Spawn", "(name: string, position: lantern:::math:::Vec3) -> i64", move |context, arguments| {
            let name = string_argument(arguments, 0)?;
            let position = vec3_argument(&spawn.1, arguments, 1)?;
            let entity = spawn.0.borrow_mut().world.spawn(&name, GAME_SCENE, Transform::from_translation(position));
            some_i64(context, entity_to_script(entity))
        })
        .function("Despawn", "(entity: i64) -> void", move |context, arguments| {
            let mut state = despawn.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            state.world.despawn(entity);
            state.entity_tags.remove(&entity);
            Ok(None)
        })
        .function("Find", "(name: string) -> i64", move |context, arguments| {
            let name = string_argument(arguments, 0)?;
            match find.borrow().world.find_by_name(&name) {
                Some(entity) => some_i64(context, entity_to_script(entity)),
                None => Err(not_found(context, format!("no entity named '{name}'"))),
            }
        })
        .function("FindByKind", "(kind: string) -> Array<i64>", move |context, arguments| {
            let wanted = string_argument(arguments, 0)?;
            let state = find_kind.borrow();
            // World order (spawn order), so a file's gem_0 comes before gem_1.
            let ids = state
                .world
                .iter()
                .filter(|(entity, _)| state.entity_tags.get(entity).is_some_and(|tags| tags.kind == wanted))
                .map(|(entity, _)| context.make_i64(entity_to_script(entity)))
                .collect::<nyx::Result<Vec<Value>>>()?;
            Ok(Some(context.make_array(&ids)?))
        })
        .function("Kind", "(entity: i64) -> string", move |context, arguments| {
            let state = kind.borrow();
            let entity = live_entity(context, &state, arguments, 0)?;
            some_string(context, state.entity_tags.get(&entity).map_or("", |tags| tags.kind.as_str()))
        })
        .function("HasProperty", "(entity: i64, key: string) -> bool", move |context, arguments| {
            let state = has_property.borrow();
            let entity = live_entity(context, &state, arguments, 0)?;
            let key = string_argument(arguments, 1)?;
            some_bool(context, state.entity_tags.get(&entity).is_some_and(|tags| tags.properties.contains_key(&key)))
        })
        .function("NumberProperty", "(entity: i64, key: string) -> f32", move |context, arguments| {
            let state = number_property.borrow();
            let entity = live_entity(context, &state, arguments, 0)?;
            let key = string_argument(arguments, 1)?;
            match property(context, &state, entity, &key)? {
                PropertyValue::Number(number) => some_f32(context, *number),
                _ => Err(not_found(context, format!("the property '{key}' is not a number"))),
            }
        })
        .function("TextProperty", "(entity: i64, key: string) -> string", move |context, arguments| {
            let state = text_property.borrow();
            let entity = live_entity(context, &state, arguments, 0)?;
            let key = string_argument(arguments, 1)?;
            match property(context, &state, entity, &key)? {
                PropertyValue::Text(text) => some_string(context, text),
                _ => Err(not_found(context, format!("the property '{key}' is not text"))),
            }
        })
        .function("VectorProperty", "(entity: i64, key: string) -> lantern:::math:::Vec3", move |context, arguments| {
            let value = {
                let state = vector_property.0.borrow();
                let entity = live_entity(context, &state, arguments, 0)?;
                let key = string_argument(arguments, 1)?;
                match property(context, &state, entity, &key)? {
                    PropertyValue::Vector(vector) => Vec3::from_array(*vector),
                    _ => return Err(not_found(context, format!("the property '{key}' is not a vector"))),
                }
            };
            wrap_vec3(&vector_property.1, value)
        })
        .function("SetMaterial", "(material: string, texture: i64) -> void", move |_context, arguments| {
            let material = string_argument(arguments, 0)?;
            let texture = handle_from_script(i64_argument(arguments, 1)?);
            set_material.borrow_mut().materials.insert(material, texture);
            Ok(None)
        })
        .function("SetMesh", "(entity: i64, mesh: i64, tint: string) -> void", move |context, arguments| {
            let mut state = set_mesh.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            let mesh = handle_from_script(i64_argument(arguments, 1)?);
            let tint = super::color_argument(context, arguments, 2)?;
            if let Some(data) = state.world.get_mut(entity) {
                data.mesh_renderer = Some(MeshRenderer::new(mesh, None, tint));
            }
            Ok(None)
        })
        .function(
            "SetTexturedMesh",
            "(entity: i64, mesh: i64, texture: i64, tint: string) -> void",
            move |context, arguments| {
                let mut state = set_textured_mesh.borrow_mut();
                let entity = live_entity(context, &state, arguments, 0)?;
                let mesh = handle_from_script(i64_argument(arguments, 1)?);
                let texture = handle_from_script(i64_argument(arguments, 2)?);
                let tint = color_argument(context, arguments, 3)?;
                if let Some(data) = state.world.get_mut(entity) {
                    data.mesh_renderer = Some(MeshRenderer::new(mesh, Some(texture), tint));
                }
                Ok(None)
            },
        )
        .function("SetTint", "(entity: i64, tint: string) -> void", move |context, arguments| {
            let mut state = set_tint.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            let tint = color_argument(context, arguments, 1)?;
            if let Some(renderer) = state.world.get_mut(entity).and_then(|data| data.mesh_renderer.as_mut()) {
                renderer.color = tint;
            }
            Ok(None)
        })
        .function("SetParent", "(child: i64, parent: i64) -> void", move |context, arguments| {
            let mut state = set_parent.borrow_mut();
            let child = live_entity(context, &state, arguments, 0)?;
            let parent = live_entity(context, &state, arguments, 1)?;
            state
                .world
                .set_parent(child, Some(parent))
                .map_err(|error| super::invalid(context, format!("cannot parent: {error:?}")))?;
            Ok(None)
        })
        .function("Position", "(entity: i64) -> lantern:::math:::Vec3", move |context, arguments| {
            let value = {
                let state = position.0.borrow();
                let entity = live_entity(context, &state, arguments, 0)?;
                state.world.get(entity).map_or(Vec3::ZERO, |data| data.local.translation)
            };
            wrap_vec3(&position.1, value)
        })
        .function("WorldPosition", "(entity: i64) -> lantern:::math:::Vec3", move |context, arguments| {
            let value = {
                let state = world_position.0.borrow();
                let entity = live_entity(context, &state, arguments, 0)?;
                state.world.get(entity).map_or(Vec3::ZERO, |data| data.world_position())
            };
            wrap_vec3(&world_position.1, value)
        })
        // Where the entity is drawn this frame: between the last two fixed steps (`alpha` is
        // `OnRender`'s argument).
        .function(
            "InterpolatedPosition",
            "(entity: i64, alpha: f32) -> lantern:::math:::Vec3",
            move |context, arguments| {
                let value = {
                    let state = interpolated.0.borrow();
                    let entity = live_entity(context, &state, arguments, 0)?;
                    let alpha = f32_argument(arguments, 1)?;
                    state.world.get(entity).map_or(Vec3::ZERO, |data| data.interpolated_world(alpha).w_axis.truncate())
                };
                wrap_vec3(&interpolated.1, value)
            },
        )
        .function("SetPosition", "(entity: i64, position: lantern:::math:::Vec3) -> void", move |context, arguments| {
            let position = vec3_argument(&set_position_state.1, arguments, 1)?;
            let mut state = set_position_state.0.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            set_position(&mut state, entity, position);
            Ok(None)
        })
        // Moves without interpolating from the old position (a respawn).
        .function("Teleport", "(entity: i64, position: lantern:::math:::Vec3) -> void", move |context, arguments| {
            let position = vec3_argument(&teleport.1, arguments, 1)?;
            let mut state = teleport.0.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            set_position(&mut state, entity, position);
            state.world.teleport(entity, position);
            Ok(None)
        })
        .function("SetYaw", "(entity: i64, yaw: f32) -> void", move |context, arguments| {
            let mut state = set_yaw.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            let yaw = f32_argument(arguments, 1)?;
            if let Some(data) = state.world.get_mut(entity) {
                data.local.rotation = Quat::from_rotation_y(yaw);
            }
            Ok(None)
        })
        .function("SetScale", "(entity: i64, scale: lantern:::math:::Vec3) -> void", move |context, arguments| {
            let scale = vec3_argument(&set_scale.1, arguments, 1)?;
            let mut state = set_scale.0.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            if let Some(data) = state.world.get_mut(entity) {
                data.local.scale = scale;
            }
            Ok(None)
        })
        .function("IsVisible", "(entity: i64) -> bool", move |context, arguments| {
            let state = visible.borrow();
            let entity = live_entity(context, &state, arguments, 0)?;
            let shown = state.world.get(entity).and_then(|data| data.mesh_renderer).is_none_or(|mesh| mesh.visible);
            some_bool(context, shown)
        })
        .function("SetVisible", "(entity: i64, visible: bool) -> void", move |context, arguments| {
            let mut state = set_visible.borrow_mut();
            let entity = live_entity(context, &state, arguments, 0)?;
            let shown = super::bool_argument(arguments, 1)?;
            if let Some(renderer) = state.world.get_mut(entity).and_then(|data| data.mesh_renderer.as_mut()) {
                renderer.visible = shown;
            }
            Ok(None)
        })
        .register()?;
    Ok(())
}
