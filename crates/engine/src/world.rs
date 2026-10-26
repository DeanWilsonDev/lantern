//! The world: entities with names, transforms, an optional parent and an optional mesh
//! renderer (Umbra `EntityManager` + `Entity` + `TransformComponent2D`, without the component
//! pipelines).
//!
//! - Entities are `EntityId`s: generational handles, so a despawned entity's id stops working
//!   instead of silently aliasing a new entity.
//! - Every entity records the scene that spawned it, so leaving a scene despawns exactly what
//!   it added (Umbra `Scene::OnExit`, minus the iterator invalidation).
//! - World transforms are recomputed parents-first, roots in slot order and children in the order
//!   they were attached, so the result never depends on hash-map order.
//! - Each fixed step starts with `store_previous_transforms`; rendering interpolates between
//!   the previous and current world transforms with the loop's `alpha`.
//!
//! Gameplay data that only one game needs (player state, collectibles) lives in the game's own
//! `ComponentStore`s keyed by `EntityId`. There is no ECS: a few typed stores are enough, and a
//! reader can see every store by reading one struct. An ECS would start paying for itself with
//! many entity kinds sharing many optional components, or when systems need parallel queries.

use glam::{Mat4, Quat, Vec3};
use lantern_core::color::Color;
use lantern_core::handle::{Handle, Pool};
use lantern_renderer_3d::{MeshHandle, TextureHandle};

/// An entity's id: a generational handle into the world's entity pool.
pub type EntityId = Handle<EntityData>;

/// Identifies a scene instance; entities spawned while it is active carry it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SceneId(pub u32);

/// The scene id for entities that outlive every scene.
pub const PERSISTENT_SCENE: SceneId = SceneId(0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Transform::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };

    pub fn from_translation(translation: Vec3) -> Transform {
        Transform { translation, ..Transform::IDENTITY }
    }

    pub fn with_rotation(self, rotation: Quat) -> Transform {
        Transform { rotation, ..self }
    }

    pub fn with_scale(self, scale: Vec3) -> Transform {
        Transform { scale, ..self }
    }

    /// `translation * rotation * scale` (see the conventions in `docs/architecture.md`).
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn lerp(&self, other: &Transform, t: f32) -> Transform {
        Transform {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }

    pub fn from_matrix(matrix: &Mat4) -> Transform {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Transform { translation, rotation, scale }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MeshRenderer {
    pub mesh: MeshHandle,
    pub texture: Option<TextureHandle>,
    pub color: Color,
    pub visible: bool,
}

impl MeshRenderer {
    pub fn new(mesh: MeshHandle, texture: Option<TextureHandle>, color: Color) -> MeshRenderer {
        MeshRenderer { mesh, texture, color, visible: true }
    }
}

#[derive(Debug)]
pub struct EntityData {
    pub name: String,
    pub scene: SceneId,
    pub local: Transform,
    pub mesh_renderer: Option<MeshRenderer>,
    parent: Option<EntityId>,
    children: Vec<EntityId>,
    world: Mat4,
    previous_world: Mat4,
}

impl EntityData {
    pub fn parent(&self) -> Option<EntityId> {
        self.parent
    }

    pub fn children(&self) -> &[EntityId] {
        &self.children
    }

    /// The world transform as of the last `update_world_transforms`.
    pub fn world_matrix(&self) -> Mat4 {
        self.world
    }

    pub fn world_position(&self) -> Vec3 {
        self.world.w_axis.truncate()
    }

    /// The world transform blended between the previous and the current fixed step.
    pub fn interpolated_world(&self, alpha: f32) -> Mat4 {
        if alpha >= 1.0 || self.previous_world == self.world {
            return self.world;
        }
        Transform::from_matrix(&self.previous_world).lerp(&Transform::from_matrix(&self.world), alpha).matrix()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchyError {
    UnknownEntity,
    WouldCreateCycle,
}

#[derive(Default)]
pub struct World {
    entities: Pool<EntityData>,
    update_order: Vec<EntityId>,
    hierarchy_dirty: bool,
}

impl World {
    pub fn new() -> World {
        World::default()
    }

    pub fn spawn(&mut self, name: &str, scene: SceneId, local: Transform) -> EntityId {
        let world = local.matrix();
        let id = self.entities.insert(EntityData {
            name: name.to_string(),
            scene,
            local,
            mesh_renderer: None,
            parent: None,
            children: Vec::new(),
            world,
            previous_world: world,
        });
        self.hierarchy_dirty = true;
        id
    }

    pub fn spawn_mesh(&mut self, name: &str, scene: SceneId, local: Transform, renderer: MeshRenderer) -> EntityId {
        let id = self.spawn(name, scene, local);
        self.entities.get_mut(id).expect("just spawned").mesh_renderer = Some(renderer);
        id
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains(id)
    }

    pub fn get(&self, id: EntityId) -> Option<&EntityData> {
        self.entities.get(id)
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut EntityData> {
        self.entities.get_mut(id)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Entities in slot order.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &EntityData)> {
        self.entities.iter()
    }

    pub fn find_by_name(&self, name: &str) -> Option<EntityId> {
        self.entities.iter().find(|(_, entity)| entity.name == name).map(|(id, _)| id)
    }

    /// Moves an entity without interpolating from where it was (respawns, resets).
    pub fn teleport(&mut self, id: EntityId, translation: Vec3) {
        if let Some(entity) = self.entities.get_mut(id) {
            entity.local.translation = translation;
        }
        self.update_world_transforms();
        // The entity and everything attached to it jump without interpolating across the gap.
        let mut pending = vec![id];
        while let Some(current) = pending.pop() {
            if let Some(entity) = self.entities.get_mut(current) {
                entity.previous_world = entity.world;
                pending.extend_from_slice(&entity.children);
            }
        }
    }

    /// Attaches `child` under `parent` (or detaches it with `None`), keeping its local transform.
    pub fn set_parent(&mut self, child: EntityId, parent: Option<EntityId>) -> Result<(), HierarchyError> {
        if !self.entities.contains(child) {
            return Err(HierarchyError::UnknownEntity);
        }
        if let Some(parent) = parent {
            if !self.entities.contains(parent) {
                return Err(HierarchyError::UnknownEntity);
            }
            // Walk up from the new parent; meeting the child means the child is an ancestor.
            let mut ancestor = Some(parent);
            while let Some(current) = ancestor {
                if current == child {
                    return Err(HierarchyError::WouldCreateCycle);
                }
                ancestor = self.entities.get(current).and_then(|entity| entity.parent);
            }
        }
        if let Some(old_parent) = self.entities.get(child).and_then(|entity| entity.parent) {
            if let Some(old) = self.entities.get_mut(old_parent) {
                old.children.retain(|&existing| existing != child);
            }
        }
        if let Some(parent) = parent {
            self.entities.get_mut(parent).expect("checked above").children.push(child);
        }
        self.entities.get_mut(child).expect("checked above").parent = parent;
        self.hierarchy_dirty = true;
        Ok(())
    }

    /// Removes an entity and all of its descendants. Returns how many were removed.
    pub fn despawn(&mut self, id: EntityId) -> usize {
        let Some(entity) = self.entities.get(id) else { return 0 };
        if let Some(parent) = entity.parent {
            if let Some(parent_entity) = self.entities.get_mut(parent) {
                parent_entity.children.retain(|&existing| existing != id);
            }
        }
        let mut removed = 0;
        let mut pending = vec![id];
        while let Some(current) = pending.pop() {
            if let Some(entity) = self.entities.remove(current) {
                pending.extend(entity.children);
                removed += 1;
            }
        }
        self.hierarchy_dirty = true;
        removed
    }

    /// Removes every entity spawned by `scene`. The ids are collected first, then despawned, so
    /// removal never invalidates the walk.
    pub fn despawn_scene(&mut self, scene: SceneId) -> usize {
        let doomed: Vec<EntityId> =
            self.entities.iter().filter(|(_, entity)| entity.scene == scene).map(|(id, _)| id).collect();
        doomed.into_iter().map(|id| self.despawn(id)).sum()
    }

    /// Call at the start of every fixed step.
    pub fn store_previous_transforms(&mut self) {
        for (_, entity) in self.entities.iter_mut() {
            entity.previous_world = entity.world;
        }
    }

    /// Recomputes every world transform, parents before children.
    pub fn update_world_transforms(&mut self) {
        if self.hierarchy_dirty {
            self.rebuild_update_order();
        }
        for index in 0..self.update_order.len() {
            let id = self.update_order[index];
            let parent_world = self
                .entities
                .get(id)
                .and_then(|entity| entity.parent)
                .and_then(|parent| self.entities.get(parent))
                .map_or(Mat4::IDENTITY, |parent| parent.world);
            if let Some(entity) = self.entities.get_mut(id) {
                entity.world = parent_world * entity.local.matrix();
            }
        }
    }

    fn rebuild_update_order(&mut self) {
        self.update_order.clear();
        let roots: Vec<EntityId> =
            self.entities.iter().filter(|(_, entity)| entity.parent.is_none()).map(|(id, _)| id).collect();
        let mut stack: Vec<EntityId> = roots.into_iter().rev().collect();
        while let Some(id) = stack.pop() {
            self.update_order.push(id);
            if let Some(entity) = self.entities.get(id) {
                stack.extend(entity.children.iter().rev());
            }
        }
        self.hierarchy_dirty = false;
    }
}

/// Game-specific data for some entities, stored densely by entity slot and checked by
/// generation, iterated in slot order.
pub struct ComponentStore<T> {
    slots: Vec<Option<(EntityId, T)>>,
}

impl<T> Default for ComponentStore<T> {
    fn default() -> Self {
        ComponentStore { slots: Vec::new() }
    }
}

impl<T> ComponentStore<T> {
    pub fn insert(&mut self, id: EntityId, value: T) {
        let index = id.index() as usize;
        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, || None);
        }
        self.slots[index] = Some((id, value));
    }

    pub fn get(&self, id: EntityId) -> Option<&T> {
        match self.slots.get(id.index() as usize)? {
            Some((stored, value)) if *stored == id => Some(value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut T> {
        match self.slots.get_mut(id.index() as usize)? {
            Some((stored, value)) if *stored == id => Some(value),
            _ => None,
        }
    }

    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        let slot = self.slots.get_mut(id.index() as usize)?;
        if slot.as_ref().is_some_and(|(stored, _)| *stored == id) { slot.take().map(|(_, value)| value) } else { None }
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.slots.iter().filter_map(|slot| slot.as_ref().map(|(id, value)| (*id, value)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.slots.iter_mut().filter_map(|slot| slot.as_mut().map(|(id, value)| (*id, value)))
    }

    /// Drops entries whose entity no longer exists.
    pub fn retain_alive(&mut self, world: &World) {
        for slot in &mut self.slots {
            if slot.as_ref().is_some_and(|(id, _)| !world.contains(*id)) {
                *slot = None;
            }
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL: SceneId = SceneId(1);

    fn near(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn children_follow_their_parents() {
        let mut world = World::new();
        let platform = world.spawn("platform", LEVEL, Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)));
        let crate_on_top = world.spawn("crate", LEVEL, Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)));
        world.set_parent(crate_on_top, Some(platform)).unwrap();
        world.update_world_transforms();
        assert!(near(world.get(crate_on_top).unwrap().world_position(), Vec3::new(5.0, 1.0, 0.0)));
        world.get_mut(platform).unwrap().local.translation.x = 7.0;
        world.update_world_transforms();
        assert!(near(world.get(crate_on_top).unwrap().world_position(), Vec3::new(7.0, 1.0, 0.0)));
    }

    #[test]
    fn a_child_spawned_before_its_parent_still_updates_after_it() {
        let mut world = World::new();
        let child = world.spawn("child", LEVEL, Transform::from_translation(Vec3::X));
        let parent = world.spawn("parent", LEVEL, Transform::from_translation(Vec3::Y));
        world.set_parent(child, Some(parent)).unwrap();
        world.update_world_transforms();
        assert!(near(world.get(child).unwrap().world_position(), Vec3::new(1.0, 1.0, 0.0)));
    }

    #[test]
    fn cycles_are_refused() {
        let mut world = World::new();
        let a = world.spawn("a", LEVEL, Transform::IDENTITY);
        let b = world.spawn("b", LEVEL, Transform::IDENTITY);
        let c = world.spawn("c", LEVEL, Transform::IDENTITY);
        world.set_parent(b, Some(a)).unwrap();
        world.set_parent(c, Some(b)).unwrap();
        assert_eq!(world.set_parent(a, Some(c)), Err(HierarchyError::WouldCreateCycle));
        assert_eq!(world.set_parent(a, Some(a)), Err(HierarchyError::WouldCreateCycle));
    }

    #[test]
    fn despawning_a_parent_removes_its_subtree_and_ids_go_stale() {
        let mut world = World::new();
        let root = world.spawn("root", LEVEL, Transform::IDENTITY);
        let child = world.spawn("child", LEVEL, Transform::IDENTITY);
        world.set_parent(child, Some(root)).unwrap();
        assert_eq!(world.despawn(root), 2);
        assert!(!world.contains(child));
        let reused = world.spawn("new", LEVEL, Transform::IDENTITY);
        assert!(world.get(root).is_none() && world.get(reused).is_some());
    }

    #[test]
    fn leaving_a_scene_removes_only_its_entities() {
        let mut world = World::new();
        let camera = world.spawn("camera", PERSISTENT_SCENE, Transform::IDENTITY);
        for index in 0..5 {
            world.spawn(&format!("block {index}"), LEVEL, Transform::IDENTITY);
        }
        assert_eq!(world.despawn_scene(LEVEL), 5);
        assert_eq!(world.len(), 1);
        assert!(world.contains(camera));
    }

    #[test]
    fn interpolation_blends_previous_and_current_steps() {
        let mut world = World::new();
        let mover = world.spawn("mover", LEVEL, Transform::IDENTITY);
        world.store_previous_transforms();
        world.get_mut(mover).unwrap().local.translation = Vec3::new(2.0, 0.0, 0.0);
        world.update_world_transforms();
        let halfway = world.get(mover).unwrap().interpolated_world(0.5);
        assert!(near(halfway.w_axis.truncate(), Vec3::new(1.0, 0.0, 0.0)));
        world.teleport(mover, Vec3::new(10.0, 0.0, 0.0));
        let snapped = world.get(mover).unwrap().interpolated_world(0.5);
        assert!(near(snapped.w_axis.truncate(), Vec3::new(10.0, 0.0, 0.0)), "teleports don't interpolate");
    }

    #[test]
    fn component_stores_check_generations() {
        let mut world = World::new();
        let mut health = ComponentStore::default();
        let first = world.spawn("first", LEVEL, Transform::IDENTITY);
        health.insert(first, 3);
        world.despawn(first);
        let second = world.spawn("second", LEVEL, Transform::IDENTITY);
        assert_eq!(first.index(), second.index());
        assert_eq!(health.get(second), None, "a new entity in the same slot has no stale component");
        health.retain_alive(&world);
        assert!(health.is_empty());
    }
}
