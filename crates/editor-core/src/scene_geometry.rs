//! World-space geometry for every entity, rebuilt when the document changes. Picking, the
//! selection outline, framing and placement all read it.

use std::collections::HashSet;

use glam::{Mat4, Vec3};
use lantern_core::math::Aabb;
use lantern_physics::Triangle;
use lantern_physics::geometry::segment_triangle_intersection;
use lantern_world_format::{EntityKey, Shape, WorldFile, build_pieces};

use crate::camera::Ray;

/// Entities without a shape are picked through a sphere of this radius around their origin.
pub const MARKER_PICK_RADIUS: f32 = 0.45;
const MAX_PICK_DISTANCE: f32 = 2000.0;

#[derive(Clone, Debug)]
pub struct EntityGeometry {
    pub key: EntityKey,
    pub world_matrix: Mat4,
    /// World-space triangles of the shape; empty for markers.
    pub triangles: Vec<Triangle>,
    /// World-space bounds: the shape's, or a small box around a marker.
    pub bounds: Aabb,
    pub shape: Option<Shape>,
}

impl EntityGeometry {
    pub fn origin(&self) -> Vec3 {
        self.world_matrix.transform_point3(Vec3::ZERO)
    }

    pub fn is_marker(&self) -> bool {
        self.shape.is_none()
    }
}

#[derive(Clone, Debug, Default)]
pub struct SceneGeometry {
    pub entities: Vec<EntityGeometry>,
    /// The document revision this was built from.
    pub revision: u64,
}

/// The nearest thing a ray hits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickHit {
    pub key: EntityKey,
    pub distance: f32,
    pub point: Vec3,
}

impl SceneGeometry {
    pub fn build(world: &WorldFile, revision: u64) -> SceneGeometry {
        let entities = build_pieces(world)
            .into_iter()
            .map(|piece| {
                let triangles = piece.world_triangles();
                let origin = piece.world_matrix.transform_point3(Vec3::ZERO);
                let bounds = if triangles.is_empty() {
                    Aabb::from_center_half_extents(origin, Vec3::splat(MARKER_PICK_RADIUS))
                } else {
                    Aabb::from_points(triangles.iter().flat_map(|triangle| [triangle.a, triangle.b, triangle.c]))
                };
                let shape = world.entity(piece.key).and_then(|entity| entity.shape);
                EntityGeometry { key: piece.key, world_matrix: piece.world_matrix, triangles, bounds, shape }
            })
            .collect();
        SceneGeometry { entities, revision }
    }

    pub fn get(&self, key: EntityKey) -> Option<&EntityGeometry> {
        self.entities.iter().find(|entity| entity.key == key)
    }

    /// Bounds of several entities together.
    pub fn bounds_of(&self, keys: &[EntityKey]) -> Aabb {
        keys.iter()
            .filter_map(|key| self.get(*key))
            .fold(Aabb::from_points(std::iter::empty()), |bounds, entity| bounds.union(entity.bounds))
    }

    /// Bounds of every entity with a shape (the "level"), for framing the whole world.
    pub fn level_bounds(&self) -> Aabb {
        self.entities.iter().fold(Aabb::from_points(std::iter::empty()), |bounds, entity| bounds.union(entity.bounds))
    }

    /// The closest entity along `ray`. Markers win ties against shapes they sit inside, since
    /// they are small and would otherwise be unclickable.
    pub fn pick(&self, ray: Ray) -> Option<PickHit> {
        self.pick_excluding(ray, &HashSet::new())
    }

    /// [`Self::pick`] ignoring the entities in `excluded` (hidden or locked ones, or the
    /// selection itself when dropping it onto whatever is below).
    pub fn pick_excluding(&self, ray: Ray, excluded: &HashSet<EntityKey>) -> Option<PickHit> {
        let end = ray.at(MAX_PICK_DISTANCE);
        let mut best: Option<PickHit> = None;
        let mut consider = |key: EntityKey, point: Vec3| {
            let distance = (point - ray.origin).dot(ray.direction);
            if distance >= 0.0 && best.is_none_or(|hit| distance < hit.distance) {
                best = Some(PickHit { key, distance, point });
            }
        };
        for entity in &self.entities {
            if excluded.contains(&entity.key) {
                continue;
            }
            if entity.is_marker() {
                if let Some(distance) = ray_sphere(ray, entity.origin(), MARKER_PICK_RADIUS) {
                    // A small bias so a marker resting on a surface is picked over the surface.
                    consider(entity.key, ray.at((distance - 0.25).max(0.0)));
                }
                continue;
            }
            if ray_aabb(ray, &entity.bounds).is_none() {
                continue;
            }
            for triangle in &entity.triangles {
                if let Some(point) = segment_triangle_intersection(ray.origin, end, triangle) {
                    consider(entity.key, point);
                }
            }
        }
        best
    }

    /// Where `ray` first meets any shape, or the ground plane (y = 0), for placing new things.
    pub fn placement_point(&self, ray: Ray) -> Option<Vec3> {
        let end = ray.at(MAX_PICK_DISTANCE);
        let surface = self
            .entities
            .iter()
            .flat_map(|entity| entity.triangles.iter())
            .filter_map(|triangle| segment_triangle_intersection(ray.origin, end, triangle))
            .min_by(|a, b| (*a - ray.origin).length_squared().total_cmp(&(*b - ray.origin).length_squared()));
        surface.or_else(|| ray.intersect_plane(Vec3::ZERO, Vec3::Y))
    }
}

/// Distance along the ray to the first hit with a sphere.
pub fn ray_sphere(ray: Ray, center: Vec3, radius: f32) -> Option<f32> {
    let to_center = center - ray.origin;
    let along = to_center.dot(ray.direction);
    let squared_miss = to_center.length_squared() - along * along;
    let squared_radius = radius * radius;
    if squared_miss > squared_radius {
        return None;
    }
    let half_chord = (squared_radius - squared_miss).sqrt();
    let near = along - half_chord;
    let far = along + half_chord;
    if far < 0.0 { None } else { Some(near.max(0.0)) }
}

/// Slab test: distance along the ray to the box, if it is hit.
pub fn ray_aabb(ray: Ray, bounds: &Aabb) -> Option<f32> {
    let inverse = Vec3::ONE / ray.direction;
    let first = (bounds.min - ray.origin) * inverse;
    let second = (bounds.max - ray.origin) * inverse;
    let near = first.min(second).max_element();
    let far = first.max(second).min_element();
    (far >= near.max(0.0)).then_some(near.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_world_format::{TransformData, WorldEntity};

    fn world_with_box_and_marker() -> WorldFile {
        let mut world = WorldFile::new("test");
        world.entities.push(WorldEntity {
            id: EntityKey(1),
            name: "floor".into(),
            parent: None,
            kind: "block".into(),
            transform: TransformData::from_position(Vec3::new(0.0, -0.5, 0.0)),
            shape: Some(Shape::Box { half_extents: [5.0, 0.5, 5.0] }),
            material: None,
            tint: None,
            collider: None,
            properties: Default::default(),
        });
        world.entities.push(WorldEntity {
            id: EntityKey(2),
            name: "gem".into(),
            parent: None,
            kind: "gem".into(),
            transform: TransformData::from_position(Vec3::new(2.0, 0.3, 0.0)),
            shape: None,
            material: None,
            tint: None,
            collider: None,
            properties: Default::default(),
        });
        world
    }

    #[test]
    fn picks_the_nearest_surface_and_markers() {
        let scene = SceneGeometry::build(&world_with_box_and_marker(), 1);
        let down = |x: f32| Ray { origin: Vec3::new(x, 10.0, 0.0), direction: Vec3::NEG_Y };
        let hit = scene.pick(down(-1.0)).unwrap();
        assert_eq!(hit.key, EntityKey(1));
        assert!((hit.point.y - 0.0).abs() < 1e-4);
        assert_eq!(scene.pick(down(2.0)).unwrap().key, EntityKey(2));
        assert!(scene.pick(down(20.0)).is_none());
    }

    #[test]
    fn placement_falls_back_to_the_ground_plane() {
        let scene = SceneGeometry::build(&world_with_box_and_marker(), 1);
        let ray = Ray { origin: Vec3::new(30.0, 10.0, 0.0), direction: Vec3::new(0.0, -1.0, 0.0) };
        assert_eq!(scene.placement_point(ray), Some(Vec3::new(30.0, 0.0, 0.0)));
    }

    #[test]
    fn slab_and_sphere_tests() {
        let ray = Ray { origin: Vec3::new(0.0, 0.0, 5.0), direction: Vec3::NEG_Z };
        assert_eq!(ray_aabb(ray, &Aabb::from_center_half_extents(Vec3::ZERO, Vec3::ONE)), Some(4.0));
        assert_eq!(ray_sphere(ray, Vec3::ZERO, 1.0), Some(4.0));
        assert!(ray_sphere(ray, Vec3::new(3.0, 0.0, 0.0), 1.0).is_none());
    }
}
