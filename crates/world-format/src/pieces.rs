//! Turning a world file into geometry: one [`WorldPiece`] per entity, with its world matrix, its
//! render mesh and its collision triangles. The game host and the editor both build from this,
//! so what you place in the editor is exactly what the game draws and collides with.

use std::collections::HashMap;

use glam::Mat4;
use lantern_core::color::Color;
use lantern_physics::Triangle;
use lantern_renderer_3d::{MeshData, primitives};

use crate::file::{ColliderKind, EntityKey, Shape, WorldFile};

/// One entity, ready to instantiate. Borrowed fields point into the `WorldFile`.
#[derive(Clone, Debug)]
pub struct WorldPiece<'world> {
    pub key: EntityKey,
    pub name: &'world str,
    pub kind: &'world str,
    pub parent: Option<EntityKey>,
    /// Relative to the parent (or the world).
    pub local_matrix: Mat4,
    /// Parent chain applied: where the piece actually is.
    pub world_matrix: Mat4,
    /// The shape's mesh in the entity's local space. `None` for markers.
    pub mesh: Option<MeshData>,
    pub material: Option<&'world str>,
    pub tint: Color,
    pub collider: Option<ColliderKind>,
}

impl WorldPiece<'_> {
    /// The mesh's triangles in world space, for a `Static` collider.
    pub fn world_triangles(&self) -> Vec<Triangle> {
        self.triangles_with(self.world_matrix)
    }

    /// The mesh's triangles in local space, for a `Kinematic` body whose transform is
    /// `world_matrix`.
    pub fn local_triangles(&self) -> Vec<Triangle> {
        self.triangles_with(Mat4::IDENTITY)
    }

    fn triangles_with(&self, matrix: Mat4) -> Vec<Triangle> {
        let Some(mesh) = &self.mesh else { return Vec::new() };
        mesh.triangles()
            .map(|[a, b, c]| {
                Triangle::new(matrix.transform_point3(a), matrix.transform_point3(b), matrix.transform_point3(c))
            })
            .collect()
    }
}

/// The render mesh for a shape. Boxes and ramps get UVs in metres, so textures tile at the same
/// density on every piece.
pub fn shape_mesh(shape: &Shape) -> MeshData {
    match *shape {
        Shape::Box { half_extents } => primitives::box_mesh(glam::Vec3::from_array(half_extents), true),
        Shape::Ramp { width, height, length } => primitives::ramp(width, height, length),
    }
}

/// Every entity's world matrix (parent chain applied). A broken parent link (which
/// `WorldFile::validate` reports) is treated as no parent.
pub fn world_matrices(world: &WorldFile) -> HashMap<EntityKey, Mat4> {
    let locals: HashMap<EntityKey, (Mat4, Option<EntityKey>)> =
        world.entities.iter().map(|entity| (entity.id, (entity.transform.matrix(), entity.parent))).collect();
    let mut resolved = HashMap::with_capacity(locals.len());
    for entity in &world.entities {
        resolve_world_matrix(entity.id, &locals, &mut resolved, 0);
    }
    resolved
}

fn resolve_world_matrix(
    key: EntityKey,
    locals: &HashMap<EntityKey, (Mat4, Option<EntityKey>)>,
    resolved: &mut HashMap<EntityKey, Mat4>,
    depth: usize,
) -> Mat4 {
    if let Some(matrix) = resolved.get(&key) {
        return *matrix;
    }
    let Some(&(local, parent)) = locals.get(&key) else { return Mat4::IDENTITY };
    let world = match parent {
        // The depth limit stops a cycle (an invalid file) from recursing forever.
        Some(parent) if depth < locals.len() => resolve_world_matrix(parent, locals, resolved, depth + 1) * local,
        _ => local,
    };
    resolved.insert(key, world);
    world
}

/// One piece per entity, in file order.
pub fn build_pieces(world: &WorldFile) -> Vec<WorldPiece<'_>> {
    let matrices = world_matrices(world);
    world
        .entities
        .iter()
        .map(|entity| WorldPiece {
            key: entity.id,
            name: &entity.name,
            kind: &entity.kind,
            parent: entity.parent,
            local_matrix: entity.transform.matrix(),
            world_matrix: matrices[&entity.id],
            mesh: entity.shape.as_ref().map(shape_mesh),
            material: entity.material.as_deref(),
            tint: entity.tint_color(),
            collider: entity.collider,
        })
        .collect()
}
