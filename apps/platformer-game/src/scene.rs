//! The level scene: turns `Level` data into entities and GPU meshes, and removes exactly those
//! again on unload (Umbra `Scene::OnEnter` / `OnExit`).
//!
//! Ownership: the scene owns the meshes it created (one per block, because world-space UVs
//! depend on each block's size) and remembers its `SceneId`. `unload` takes `self` by value, so
//! a scene cannot be used after it has released its resources.

use glam::{Quat, Vec3};
use lantern_core::color::Color;
use lantern_engine::world::{EntityId, MeshRenderer, SceneId, Transform, World};
use lantern_renderer_3d::{MeshHandle, TextureHandle, primitives};
use lantern_sdl_gpu::{GpuError, GpuRenderer};

use crate::assets::GameAssets;
use crate::level::{Level, SurfaceMaterial};

/// The scene id used for the level. There is one level, so a constant is enough.
pub const LEVEL_SCENE: SceneId = SceneId(1);

pub struct Gem {
    pub entity: EntityId,
    pub home_position: Vec3,
    pub collected: bool,
}

pub struct MovingPlatform {
    pub entity: EntityId,
    #[expect(dead_code, reason = "the M4 controller builds the platform's collision box from it")]
    pub half_extents: Vec3,
    pub start_center: Vec3,
}

pub struct LevelScene {
    level_meshes: Vec<MeshHandle>,
    pub player: EntityId,
    pub gems: Vec<Gem>,
    pub moving_platforms: Vec<MovingPlatform>,
}

fn texture_for(material: SurfaceMaterial, assets: &GameAssets) -> TextureHandle {
    match material {
        SurfaceMaterial::Ground => assets.ground_texture,
        SurfaceMaterial::Wall => assets.wall_texture,
        SurfaceMaterial::Platform => assets.platform_texture,
    }
}

impl LevelScene {
    pub fn load(
        level: &Level,
        world: &mut World,
        renderer: &mut GpuRenderer,
        assets: &GameAssets,
    ) -> Result<LevelScene, GpuError> {
        let mut level_meshes = Vec::new();
        let mut create_mesh = |renderer: &mut GpuRenderer, mesh| -> Result<MeshHandle, GpuError> {
            let handle = renderer.create_mesh(&mesh)?;
            level_meshes.push(handle);
            Ok(handle)
        };
        for block in &level.blocks {
            let mesh = create_mesh(renderer, primitives::box_mesh(block.half_extents, true))?;
            let renderer_component = MeshRenderer::new(mesh, Some(texture_for(block.material, assets)), Color::WHITE);
            world.spawn_mesh(block.name, LEVEL_SCENE, Transform::from_translation(block.center), renderer_component);
        }
        for ramp in &level.ramps {
            let mesh = create_mesh(renderer, primitives::ramp(ramp.width, ramp.height, ramp.length))?;
            let renderer_component = MeshRenderer::new(mesh, Some(assets.ground_texture), Color::WHITE);
            world.spawn_mesh(ramp.name, LEVEL_SCENE, Transform::from_translation(ramp.base_center), renderer_component);
        }
        let mut moving_platforms = Vec::new();
        for platform in &level.moving_platforms {
            let mesh = create_mesh(renderer, primitives::box_mesh(platform.half_extents, true))?;
            let renderer_component = MeshRenderer::new(mesh, Some(assets.platform_texture), Color::from_hex(0xFFE0A0));
            let entity = world.spawn_mesh(
                platform.name,
                LEVEL_SCENE,
                Transform::from_translation(platform.start_center),
                renderer_component,
            );
            moving_platforms.push(MovingPlatform {
                entity,
                half_extents: platform.half_extents,
                start_center: platform.start_center,
            });
        }
        let gems = level
            .gem_positions
            .iter()
            .enumerate()
            .map(|(index, &position)| {
                let renderer_component = MeshRenderer::new(assets.gem_mesh, None, Color::WHITE);
                let entity = world.spawn_mesh(
                    &format!("gem_{index}"),
                    LEVEL_SCENE,
                    Transform::from_translation(position),
                    renderer_component,
                );
                Gem { entity, home_position: position, collected: false }
            })
            .collect();
        // The player's transform is at its feet; the capsule mesh is centred, so it is a child
        // lifted by half the height. This also exercises the hierarchy every frame.
        let player = world.spawn("player", LEVEL_SCENE, Transform::from_translation(level.spawn_position));
        let player_body = world.spawn_mesh(
            "player_body",
            LEVEL_SCENE,
            Transform::from_translation(Vec3::Y * crate::assets::PLAYER_HEIGHT * 0.5),
            MeshRenderer::new(assets.player_mesh, Some(assets.player_texture), Color::from_hex(0xE8A33D)),
        );
        world.set_parent(player_body, Some(player)).expect("a fresh entity cannot form a cycle");
        world.update_world_transforms();
        world.store_previous_transforms();
        log::info!(
            target: "scene",
            "level loaded: {} entities, {} meshes",
            world.iter().filter(|(_, entity)| entity.scene == LEVEL_SCENE).count(),
            level_meshes.len()
        );
        Ok(LevelScene { level_meshes, player, gems, moving_platforms })
    }

    /// Despawns every entity the scene spawned and releases its meshes.
    pub fn unload(self, world: &mut World, renderer: &mut GpuRenderer) {
        let despawned = world.despawn_scene(LEVEL_SCENE);
        for mesh in &self.level_meshes {
            renderer.remove_mesh(*mesh);
        }
        log::info!(target: "scene", "level unloaded: {despawned} entities, {} meshes", self.level_meshes.len());
    }

    /// Visual-only gem animation: spin and bob around the home position.
    pub fn animate_gems(&self, world: &mut World, elapsed_seconds: f32) {
        for (index, gem) in self.gems.iter().enumerate() {
            let Some(entity) = world.get_mut(gem.entity) else { continue };
            let phase = elapsed_seconds * 2.0 + index as f32;
            entity.local.translation = gem.home_position + Vec3::Y * phase.sin() * 0.1;
            entity.local.rotation = Quat::from_rotation_y(elapsed_seconds * 1.5);
            if let Some(renderer) = entity.mesh_renderer.as_mut() {
                renderer.visible = !gem.collected;
            }
        }
    }
}
