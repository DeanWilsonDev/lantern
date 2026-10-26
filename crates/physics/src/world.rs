//! The collision world: static triangles in a uniform grid, plus kinematic bodies (moving
//! platforms) that the game moves every fixed step. Queries are capsule casts, sphere casts,
//! overlap contacts and rays.
//!
//! Casting uses conservative advancement (decision D-007). For one triangle and a capsule
//! moving by `displacement`, the distance between the capsule's segment and the triangle is a
//! convex function of the travel fraction `t` (both shapes are convex and the motion is a
//! translation). Convexity gives two guarantees the loop relies on:
//! - the tangent at `t` never overestimates the distance further on, so stepping to where the
//!   tangent reaches the radius never passes the true contact (no tunnelling, any speed);
//! - if the distance is not decreasing at `t`, it never will, so the cast can stop early.

use std::collections::HashMap;

use glam::{IVec3, Mat4, Vec3};
use lantern_core::handle::{Handle, Pool};
use lantern_core::math::Aabb;

use crate::geometry::{Triangle, closest_points_segment_triangle, segment_triangle_intersection};

/// Static triangles are bucketed into cubes of this size. The demo's triangles are a few metres
/// across and queries are about a capsule wide, so a few metres keeps buckets short.
const GRID_CELL_SIZE: f32 = 4.0;
/// A cast stops when the gap to a surface is below this.
const CAST_TOLERANCE: f32 = 1e-4;
/// Newton steps per triangle. Convergence is usually 2-4 steps; the cap only guards
/// pathological input, and stopping early is still conservative.
const MAX_CAST_ITERATIONS: u32 = 24;

/// A capsule standing upright: a segment along Y of length `2 * half_segment` around its
/// centre, inflated by `radius`. A sphere is a capsule with `half_segment = 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub radius: f32,
    pub half_segment: f32,
}

impl Capsule {
    /// A capsule `height` tall overall (including both caps).
    pub fn upright(radius: f32, height: f32) -> Capsule {
        Capsule { radius, half_segment: (height * 0.5 - radius).max(0.0) }
    }

    pub fn sphere(radius: f32) -> Capsule {
        Capsule { radius, half_segment: 0.0 }
    }

    pub fn segment(&self, center: Vec3) -> (Vec3, Vec3) {
        (center - Vec3::Y * self.half_segment, center + Vec3::Y * self.half_segment)
    }

    pub fn bounds(&self, center: Vec3) -> Aabb {
        let half = Vec3::new(self.radius, self.radius + self.half_segment, self.radius);
        Aabb::new(center - half, center + half)
    }
}

pub struct KinematicBody {
    local_triangles: Vec<Triangle>,
    world_triangles: Vec<Triangle>,
    world_bounds: Aabb,
    transform: Mat4,
    previous_transform: Mat4,
}

impl KinematicBody {
    pub fn transform(&self) -> Mat4 {
        self.transform
    }
}

pub type BodyHandle = Handle<KinematicBody>;

/// What a query touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColliderId {
    Static,
    Body(BodyHandle),
}

/// The first contact of a cast.
#[derive(Clone, Copy, Debug)]
pub struct ShapeHit {
    /// How far along the displacement the contact happens, 0..=1.
    pub fraction: f32,
    /// Surface normal at the contact, pointing toward the moving shape.
    pub normal: Vec3,
    /// The contact point on the surface.
    pub point: Vec3,
    pub collider: ColliderId,
}

/// An overlap between a shape and a surface.
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    /// Direction that pushes the shape out.
    pub normal: Vec3,
    /// How far the shape is inside the surface.
    pub depth: f32,
    pub point: Vec3,
    pub collider: ColliderId,
}

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub collider: ColliderId,
}

#[derive(Default)]
pub struct CollisionWorld {
    static_triangles: Vec<Triangle>,
    grid: HashMap<IVec3, Vec<u32>>,
    bodies: Pool<KinematicBody>,
}

fn grid_cell(point: Vec3) -> IVec3 {
    (point / GRID_CELL_SIZE).floor().as_ivec3()
}

/// The 12 outward-facing triangles of an axis-aligned box.
pub fn box_triangles(center: Vec3, half_extents: Vec3) -> Vec<Triangle> {
    let corner = |x: f32, y: f32, z: f32| center + half_extents * Vec3::new(x, y, z);
    // Each face lists its corners counter-clockwise as seen from outside.
    let faces = [
        [corner(1.0, -1.0, 1.0), corner(1.0, -1.0, -1.0), corner(1.0, 1.0, -1.0), corner(1.0, 1.0, 1.0)],
        [corner(-1.0, -1.0, -1.0), corner(-1.0, -1.0, 1.0), corner(-1.0, 1.0, 1.0), corner(-1.0, 1.0, -1.0)],
        [corner(-1.0, 1.0, 1.0), corner(1.0, 1.0, 1.0), corner(1.0, 1.0, -1.0), corner(-1.0, 1.0, -1.0)],
        [corner(-1.0, -1.0, -1.0), corner(1.0, -1.0, -1.0), corner(1.0, -1.0, 1.0), corner(-1.0, -1.0, 1.0)],
        [corner(-1.0, -1.0, 1.0), corner(1.0, -1.0, 1.0), corner(1.0, 1.0, 1.0), corner(-1.0, 1.0, 1.0)],
        [corner(1.0, -1.0, -1.0), corner(-1.0, -1.0, -1.0), corner(-1.0, 1.0, -1.0), corner(1.0, 1.0, -1.0)],
    ];
    faces.iter().flat_map(|[a, b, c, d]| [Triangle::new(*a, *b, *c), Triangle::new(*a, *c, *d)]).collect()
}

/// Casts a capsule segment against one triangle. Returns `(fraction, normal, point)`.
fn cast_against_triangle(
    segment: (Vec3, Vec3),
    radius: f32,
    displacement: Vec3,
    triangle: &Triangle,
) -> Option<(f32, Vec3, Vec3)> {
    let mut fraction = 0.0f32;
    for _ in 0..MAX_CAST_ITERATIONS {
        let offset = displacement * fraction;
        let (on_segment, on_triangle) =
            closest_points_segment_triangle(segment.0 + offset, segment.1 + offset, triangle);
        let separation = on_segment - on_triangle;
        let distance = separation.length();
        let normal = if distance > 1e-6 {
            separation / distance
        } else {
            // The core segment touches the triangle: deep overlap. Use the face normal, turned
            // to oppose the motion, so the caller at least stops moving deeper.
            let face_normal = triangle.normal();
            if face_normal.dot(displacement) > 0.0 { -face_normal } else { face_normal }
        };
        let gap = distance - radius;
        let closing_speed = -displacement.dot(normal);
        if gap <= CAST_TOLERANCE {
            // Touching (or overlapping) already. Only a hit if we are moving further in;
            // sliding along or moving away is allowed.
            return (closing_speed > 1e-7).then_some((fraction, normal, on_triangle));
        }
        if closing_speed <= 1e-9 {
            return None;
        }
        fraction += gap / closing_speed;
        if fraction > 1.0 {
            return None;
        }
    }
    // Not converged; reporting the last safe fraction is still conservative.
    let offset = displacement * fraction;
    let (on_segment, on_triangle) = closest_points_segment_triangle(segment.0 + offset, segment.1 + offset, triangle);
    Some((fraction, (on_segment - on_triangle).normalize_or(Vec3::Y), on_triangle))
}

impl CollisionWorld {
    pub fn new() -> CollisionWorld {
        CollisionWorld::default()
    }

    pub fn add_static_triangles(&mut self, triangles: impl IntoIterator<Item = Triangle>) {
        for triangle in triangles {
            if triangle.is_degenerate() {
                continue;
            }
            let index = self.static_triangles.len() as u32;
            let bounds = Aabb::from_points([triangle.a, triangle.b, triangle.c]);
            let (min_cell, max_cell) = (grid_cell(bounds.min), grid_cell(bounds.max));
            for x in min_cell.x..=max_cell.x {
                for y in min_cell.y..=max_cell.y {
                    for z in min_cell.z..=max_cell.z {
                        self.grid.entry(IVec3::new(x, y, z)).or_default().push(index);
                    }
                }
            }
            self.static_triangles.push(triangle);
        }
    }

    pub fn add_static_box(&mut self, center: Vec3, half_extents: Vec3) {
        self.add_static_triangles(box_triangles(center, half_extents));
    }

    pub fn static_triangle_count(&self) -> usize {
        self.static_triangles.len()
    }

    /// Adds a body made of `local_triangles`, placed at `transform`.
    pub fn add_body(&mut self, local_triangles: Vec<Triangle>, transform: Mat4) -> BodyHandle {
        let mut body = KinematicBody {
            local_triangles,
            world_triangles: Vec::new(),
            world_bounds: Aabb::EMPTY,
            transform,
            previous_transform: transform,
        };
        Self::refresh_body(&mut body);
        self.bodies.insert(body)
    }

    pub fn remove_body(&mut self, body: BodyHandle) {
        self.bodies.remove(body);
    }

    pub fn body(&self, body: BodyHandle) -> Option<&KinematicBody> {
        self.bodies.get(body)
    }

    fn refresh_body(body: &mut KinematicBody) {
        body.world_triangles =
            body.local_triangles.iter().map(|triangle| triangle.transformed(&body.transform)).collect();
        body.world_bounds =
            Aabb::from_points(body.world_triangles.iter().flat_map(|triangle| [triangle.a, triangle.b, triangle.c]));
    }

    /// Starts a fixed step: every body's motion so far is forgotten, so a body that is not
    /// moved this step reports no motion. Call before moving bodies.
    pub fn begin_step(&mut self) {
        for (_, body) in self.bodies.iter_mut() {
            body.previous_transform = body.transform;
        }
    }

    pub fn set_body_transform(&mut self, body: BodyHandle, transform: Mat4) {
        if let Some(body) = self.bodies.get_mut(body) {
            body.transform = transform;
            Self::refresh_body(body);
        }
    }

    /// The transform that takes a point riding on the body from last step to this step.
    pub fn body_step_motion(&self, body: BodyHandle) -> Mat4 {
        self.bodies.get(body).map_or(Mat4::IDENTITY, |body| body.transform * body.previous_transform.inverse())
    }

    /// Calls `visit` for every triangle whose bounds may overlap `bounds`, with its collider.
    /// A static triangle can be visited once per grid cell it spans; callers take the minimum
    /// or the deepest result, so repeats are harmless and cheaper than de-duplicating.
    fn for_each_triangle_near(
        &self,
        bounds: &Aabb,
        ignore: Option<ColliderId>,
        mut visit: impl FnMut(&Triangle, ColliderId),
    ) {
        if ignore != Some(ColliderId::Static) {
            let (min_cell, max_cell) = (grid_cell(bounds.min), grid_cell(bounds.max));
            for x in min_cell.x..=max_cell.x {
                for y in min_cell.y..=max_cell.y {
                    for z in min_cell.z..=max_cell.z {
                        let Some(indices) = self.grid.get(&IVec3::new(x, y, z)) else { continue };
                        for &index in indices {
                            visit(&self.static_triangles[index as usize], ColliderId::Static);
                        }
                    }
                }
            }
        }
        for (handle, body) in self.bodies.iter() {
            let collider = ColliderId::Body(handle);
            if ignore == Some(collider) || !body.world_bounds.overlaps(bounds) {
                continue;
            }
            for triangle in &body.world_triangles {
                visit(triangle, collider);
            }
        }
    }

    /// Sweeps `capsule` from `center` by `displacement` and returns the first contact.
    /// Surfaces the capsule already touches only stop it if it moves further into them.
    pub fn capsule_cast(
        &self,
        capsule: Capsule,
        center: Vec3,
        displacement: Vec3,
        ignore: Option<ColliderId>,
    ) -> Option<ShapeHit> {
        let start_bounds = capsule.bounds(center);
        let swept_bounds = start_bounds.union(capsule.bounds(center + displacement)).expanded(CAST_TOLERANCE * 2.0);
        let segment = capsule.segment(center);
        let mut best: Option<ShapeHit> = None;
        self.for_each_triangle_near(&swept_bounds, ignore, |triangle, collider| {
            let Some((fraction, normal, point)) =
                cast_against_triangle(segment, capsule.radius, displacement, triangle)
            else {
                return;
            };
            // Ties are common (a capsule resting where floor meets wall touches both at 0).
            // Prefer the surface that opposes the motion most: for a downward probe, the floor.
            let better = match best {
                None => true,
                Some(best) if (fraction - best.fraction).abs() <= 1e-6 => {
                    normal.dot(displacement) < best.normal.dot(displacement)
                }
                Some(best) => fraction < best.fraction,
            };
            if better {
                best = Some(ShapeHit { fraction, normal, point, collider });
            }
        });
        best
    }

    /// Every surface the capsule overlaps at `center`, deepest first.
    pub fn capsule_contacts(&self, capsule: Capsule, center: Vec3, ignore: Option<ColliderId>) -> Vec<Contact> {
        let segment = capsule.segment(center);
        let mut contacts = Vec::new();
        self.for_each_triangle_near(&capsule.bounds(center), ignore, |triangle, collider| {
            let (on_segment, on_triangle) = closest_points_segment_triangle(segment.0, segment.1, triangle);
            let separation = on_segment - on_triangle;
            let distance = separation.length();
            if distance >= capsule.radius {
                return;
            }
            let normal = if distance > 1e-6 { separation / distance } else { triangle.normal() };
            contacts.push(Contact { normal, depth: capsule.radius - distance, point: on_triangle, collider });
        });
        contacts.sort_by(|first, second| second.depth.total_cmp(&first.depth));
        contacts
    }

    /// A ray from `origin` along unit `direction`, up to `max_distance`. Back faces are hit too.
    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit> {
        let end = origin + direction * max_distance;
        let bounds = Aabb::from_points([origin, end]);
        let mut best: Option<RayHit> = None;
        self.for_each_triangle_near(&bounds, None, |triangle, collider| {
            let Some(point) = segment_triangle_intersection(origin, end, triangle) else { return };
            let distance = point.distance(origin);
            if best.is_none_or(|best| distance < best.distance) {
                let face_normal = triangle.normal();
                let normal = if face_normal.dot(direction) > 0.0 { -face_normal } else { face_normal };
                best = Some(RayHit { distance, point, normal, collider });
            }
        });
        best
    }

    /// Triangles near `bounds`, for debug drawing. Static triangles may repeat.
    pub fn triangles_near(&self, bounds: &Aabb) -> Vec<Triangle> {
        let mut triangles = Vec::new();
        self.for_each_triangle_near(bounds, None, |triangle, _| triangles.push(*triangle));
        triangles.sort_by(|first, second| {
            first.a.to_array().partial_cmp(&second.a.to_array()).unwrap_or(std::cmp::Ordering::Equal)
        });
        triangles.dedup();
        triangles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_world() -> CollisionWorld {
        let mut world = CollisionWorld::new();
        world.add_static_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(10.0, 0.5, 10.0));
        world
    }

    #[test]
    fn box_triangles_face_outward() {
        let center = Vec3::new(1.0, 2.0, 3.0);
        for triangle in box_triangles(center, Vec3::new(1.0, 2.0, 3.0)) {
            let centroid = (triangle.a + triangle.b + triangle.c) / 3.0;
            assert!(triangle.normal().dot(centroid - center) > 0.0, "{triangle:?}");
        }
    }

    #[test]
    fn falling_capsule_stops_one_radius_above_the_floor() {
        let world = floor_world();
        let capsule = Capsule::upright(0.5, 2.0);
        let hit = world.capsule_cast(capsule, Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -10.0, 0.0), None).unwrap();
        // Centre stops at y = 1 (half the height), from y = 5: 4 of the 10 metres.
        assert!((hit.fraction - 0.4).abs() < 1e-3, "{}", hit.fraction);
        assert!(hit.normal.abs_diff_eq(Vec3::Y, 1e-3));
        assert_eq!(hit.collider, ColliderId::Static);
    }

    #[test]
    fn fast_casts_do_not_tunnel_through_thin_walls() {
        let mut world = CollisionWorld::new();
        world.add_static_box(Vec3::new(0.0, 1.0, -5.0), Vec3::new(5.0, 1.0, 0.01));
        let hit =
            world.capsule_cast(Capsule::sphere(0.25), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, -1000.0), None);
        let hit = hit.expect("the wall is in the way");
        let stop = 1000.0 * hit.fraction;
        assert!((stop - (5.0 - 0.01 - 0.25)).abs() < 1e-2, "{stop}");
    }

    #[test]
    fn touching_surfaces_allow_sliding_and_leaving() {
        let world = floor_world();
        let capsule = Capsule::upright(0.5, 2.0);
        let resting = Vec3::new(0.0, 1.0, 0.0);
        assert!(world.capsule_cast(capsule, resting, Vec3::new(3.0, 0.0, 0.0), None).is_none());
        assert!(world.capsule_cast(capsule, resting, Vec3::new(0.0, 1.0, 0.0), None).is_none());
        let into_floor = world.capsule_cast(capsule, resting, Vec3::new(0.0, -1.0, 0.0), None).unwrap();
        assert_eq!(into_floor.fraction, 0.0);
    }

    #[test]
    fn edge_hits_report_a_tilted_normal() {
        // A capsule falling past the edge of a box hits the edge, not the face.
        let world = floor_world();
        let hit = world
            .capsule_cast(Capsule::sphere(0.5), Vec3::new(10.3, 2.0, 0.0), Vec3::new(0.0, -5.0, 0.0), None)
            .unwrap();
        assert!(hit.normal.x > 0.3 && hit.normal.y > 0.3, "{:?}", hit.normal);
    }

    #[test]
    fn contacts_report_depth_and_push_direction() {
        let world = floor_world();
        let contacts = world.capsule_contacts(Capsule::sphere(0.5), Vec3::new(0.0, 0.3, 0.0), None);
        assert!(!contacts.is_empty());
        assert!((contacts[0].depth - 0.2).abs() < 1e-4);
        assert!(contacts[0].normal.abs_diff_eq(Vec3::Y, 1e-4));
    }

    #[test]
    fn bodies_move_and_report_their_step_motion() {
        let mut world = CollisionWorld::new();
        let body = world.add_body(box_triangles(Vec3::ZERO, Vec3::splat(1.0)), Mat4::IDENTITY);
        world.begin_step();
        world.set_body_transform(body, Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0)));
        let motion = world.body_step_motion(body);
        assert!(motion.transform_point3(Vec3::ZERO).abs_diff_eq(Vec3::new(2.0, 0.0, 0.0), 1e-6));
        // The moved body is where casts find it.
        let hit = world
            .capsule_cast(Capsule::sphere(0.1), Vec3::new(2.0, 5.0, 0.0), Vec3::new(0.0, -10.0, 0.0), None)
            .unwrap();
        assert_eq!(hit.collider, ColliderId::Body(body));
        // Ignoring it lets the cast through.
        assert!(
            world
                .capsule_cast(
                    Capsule::sphere(0.1),
                    Vec3::new(2.0, 5.0, 0.0),
                    Vec3::new(0.0, -10.0, 0.0),
                    Some(ColliderId::Body(body))
                )
                .is_none()
        );
        world.begin_step();
        assert!(world.body_step_motion(body).abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }

    #[test]
    fn raycast_finds_the_nearest_surface() {
        let world = floor_world();
        let hit = world.raycast(Vec3::new(1.0, 3.0, 1.0), Vec3::NEG_Y, 10.0).unwrap();
        assert!((hit.distance - 3.0).abs() < 1e-5);
        assert!(hit.normal.abs_diff_eq(Vec3::Y, 1e-5));
        assert!(world.raycast(Vec3::new(1.0, 3.0, 1.0), Vec3::Y, 10.0).is_none());
    }
}
