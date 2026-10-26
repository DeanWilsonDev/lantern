//! The level scene: turns `Level` data into entities and GPU meshes, and removes exactly those
//! again on unload (Umbra `Scene::OnEnter` / `OnExit`).
//!
//! Ownership: the scene owns the meshes it created (one per block, because world-space UVs
//! depend on each block's size), the collision world built from the same meshes, and remembers
//! its `SceneId`. `unload` takes `self` by value, so a scene cannot be used after it has
//! released its resources.

use glam::{Mat4, Quat, Vec3};
use lantern_core::color::Color;
use lantern_engine::world::{EntityId, MeshRenderer, SceneId, Transform, World};
use lantern_physics::{BodyHandle, CollisionWorld, Triangle};
use lantern_renderer_3d::{MeshData, MeshHandle, TextureHandle, primitives};
use lantern_sdl_gpu::{GpuError, GpuRenderer};

use crate::assets::GameAssets;
use crate::level::{Level, SurfaceMaterial};

/// The scene id used for the level. There is one level, so a constant is enough.
pub const LEVEL_SCENE: SceneId = SceneId(1);

pub struct Gem {
    pub entity: EntityId,
    pub home_position: Vec3,
}

pub struct MovingPlatform {
    pub entity: EntityId,
    pub body: BodyHandle,
}

pub struct LevelScene {
    level_meshes: Vec<MeshHandle>,
    pub collision: CollisionWorld,
    pub player: EntityId,
    /// A dark disc under the player, so you can judge where you will land.
    pub player_shadow: EntityId,
    pub gems: Vec<Gem>,
    pub moving_platforms: Vec<MovingPlatform>,
}

/// The mesh's triangles, moved by `offset`: the collision copy of exactly what is drawn.
fn collision_triangles(mesh: &MeshData, offset: Vec3) -> impl Iterator<Item = Triangle> + '_ {
    mesh.triangles().map(move |[a, b, c]| Triangle::new(a + offset, b + offset, c + offset))
}

/// One solid piece of the level, before it becomes an entity: a mesh and where it goes.
pub struct LevelPiece {
    pub name: &'static str,
    pub mesh: MeshData,
    pub position: Vec3,
    pub material: SurfaceMaterial,
    pub moving: bool,
}

/// Every solid piece of `level`. Rendering and collision are both built from this list, so
/// what you see is exactly what you collide with.
pub fn level_pieces(level: &Level) -> Vec<LevelPiece> {
    let mut pieces = Vec::new();
    for block in &level.blocks {
        pieces.push(LevelPiece {
            name: block.name,
            mesh: primitives::box_mesh(block.half_extents, true),
            position: block.center,
            material: block.material,
            moving: false,
        });
    }
    for ramp in &level.ramps {
        pieces.push(LevelPiece {
            name: ramp.name,
            mesh: primitives::ramp(ramp.width, ramp.height, ramp.length),
            position: ramp.base_center,
            material: SurfaceMaterial::Ground,
            moving: false,
        });
    }
    for platform in &level.moving_platforms {
        pieces.push(LevelPiece {
            name: platform.name,
            mesh: primitives::box_mesh(platform.half_extents, true),
            position: platform.start_center,
            material: SurfaceMaterial::Platform,
            moving: true,
        });
    }
    pieces
}

/// Adds a piece to the collision world: static triangles, or a kinematic body for a mover.
pub fn add_piece_collision(collision: &mut CollisionWorld, piece: &LevelPiece) -> Option<BodyHandle> {
    if piece.moving {
        let local_triangles = collision_triangles(&piece.mesh, Vec3::ZERO).collect();
        Some(collision.add_body(local_triangles, Mat4::from_translation(piece.position)))
    } else {
        collision.add_static_triangles(collision_triangles(&piece.mesh, piece.position));
        None
    }
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
        let mut collision = CollisionWorld::new();
        let mut moving_platforms = Vec::new();
        for piece in level_pieces(level) {
            let mesh = renderer.create_mesh(&piece.mesh)?;
            level_meshes.push(mesh);
            let tint = if piece.moving { Color::from_hex(0xFFE0A0) } else { Color::WHITE };
            let renderer_component = MeshRenderer::new(mesh, Some(texture_for(piece.material, assets)), tint);
            let entity = world.spawn_mesh(
                piece.name,
                LEVEL_SCENE,
                Transform::from_translation(piece.position),
                renderer_component,
            );
            if let Some(body) = add_piece_collision(&mut collision, &piece) {
                moving_platforms.push(MovingPlatform { entity, body });
            }
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
                Gem { entity, home_position: position }
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
        // A visor on the body's front (-Z), so you can see which way the player faces.
        let visor = world.spawn_mesh(
            "player_visor",
            LEVEL_SCENE,
            Transform::from_translation(Vec3::new(0.0, 0.42, -0.3)).with_scale(Vec3::new(0.4, 0.14, 0.16)),
            MeshRenderer::new(assets.unit_cube_mesh, None, Color::from_hex(0x203040)),
        );
        world.set_parent(visor, Some(player_body)).expect("a fresh entity cannot form a cycle");
        let player_shadow = world.spawn_mesh(
            "player_shadow",
            LEVEL_SCENE,
            Transform::from_translation(level.spawn_position),
            MeshRenderer::new(assets.unit_sphere_mesh, None, Color::rgb(0.05, 0.07, 0.05)),
        );
        world.update_world_transforms();
        world.store_previous_transforms();
        log::info!(
            target: "scene",
            "level loaded: {} entities, {} meshes, {} collision triangles",
            world.iter().filter(|(_, entity)| entity.scene == LEVEL_SCENE).count(),
            level_meshes.len(),
            collision.static_triangle_count()
        );
        Ok(LevelScene { level_meshes, collision, player, player_shadow, gems, moving_platforms })
    }

    /// Despawns every entity the scene spawned and releases its meshes.
    pub fn unload(self, world: &mut World, renderer: &mut GpuRenderer) {
        let despawned = world.despawn_scene(LEVEL_SCENE);
        for mesh in &self.level_meshes {
            renderer.remove_mesh(*mesh);
        }
        log::info!(target: "scene", "level unloaded: {despawned} entities, {} meshes", self.level_meshes.len());
    }

    /// Visual-only gem animation: spin and bob around the home position. (Whether a gem is
    /// shown is a rule, decided by the rules through `SetEntityVisible`.)
    pub fn animate_gems(&self, world: &mut World, elapsed_seconds: f32) {
        for (index, gem) in self.gems.iter().enumerate() {
            let Some(entity) = world.get_mut(gem.entity) else { continue };
            let phase = elapsed_seconds * 2.0 + index as f32;
            entity.local.translation = gem.home_position + Vec3::Y * phase.sin() * 0.1;
            entity.local.rotation = Quat::from_rotation_y(elapsed_seconds * 1.5);
        }
    }

    pub fn moving_platform_body(&self, entity: EntityId) -> Option<lantern_physics::BodyHandle> {
        self.moving_platforms.iter().find(|platform| platform.entity == entity).map(|platform| platform.body)
    }
}
