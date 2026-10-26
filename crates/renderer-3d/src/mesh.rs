//! The single mesh vertex format and CPU-side mesh data.

use glam::{Vec2, Vec3, Vec4};
use lantern_core::math::Aabb;

/// One vertex of a lit mesh. `#[repr(C)]` because its bytes are uploaded to the GPU as-is; the
/// layout (offsets 0, 12, 24, 32, stride 48) is mirrored by the vertex input description in
/// `lantern-sdl-gpu` and the `VertexInput` struct in `shaders/lit.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub const STRIDE: u32 = std::mem::size_of::<Vertex>() as u32;

    pub fn new(position: Vec3, normal: Vec3, uv: Vec2, color: Vec4) -> Vertex {
        Vertex { position: position.to_array(), normal: normal.to_array(), uv: uv.to_array(), color: color.to_array() }
    }

    pub fn position(&self) -> Vec3 {
        Vec3::from_array(self.position)
    }
}

/// Indexed triangles (counter-clockwise front faces) plus their local-space bounds.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn bounds(&self) -> Aabb {
        Aabb::from_points(self.vertices.iter().map(Vertex::position))
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Checks the invariants a GPU upload relies on.
    pub fn validate(&self) -> Result<(), String> {
        if !self.indices.len().is_multiple_of(3) {
            return Err(format!("{} indices is not a whole number of triangles", self.indices.len()));
        }
        if let Some(bad) = self.indices.iter().find(|&&index| index as usize >= self.vertices.len()) {
            return Err(format!("index {bad} is past the {} vertices", self.vertices.len()));
        }
        if self.vertices.is_empty() {
            return Err("a mesh needs at least one vertex".to_string());
        }
        Ok(())
    }

    /// Positions of each triangle, for building collision geometry from a render mesh.
    pub fn triangles(&self) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        self.indices.chunks_exact(3).map(|triangle| {
            [
                self.vertices[triangle[0] as usize].position(),
                self.vertices[triangle[1] as usize].position(),
                self.vertices[triangle[2] as usize].position(),
            ]
        })
    }

    /// Appends another mesh, offsetting its indices.
    pub fn append(&mut self, other: &MeshData) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        self.indices.extend(other.indices.iter().map(|index| index + offset));
    }

    /// Byte view of the vertices for upload.
    pub fn vertex_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.vertices.len() * Vertex::STRIDE as usize);
        for vertex in &self.vertices {
            for value in vertex.position.iter().chain(&vertex.normal).chain(&vertex.uv).chain(&vertex.color) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    pub fn index_bytes(&self) -> Vec<u8> {
        self.indices.iter().flat_map(|index| index.to_le_bytes()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_layout_matches_the_shader() {
        assert_eq!(Vertex::STRIDE, 48);
        assert_eq!(std::mem::offset_of!(Vertex, normal), 12);
        assert_eq!(std::mem::offset_of!(Vertex, uv), 24);
        assert_eq!(std::mem::offset_of!(Vertex, color), 32);
    }

    #[test]
    fn byte_view_matches_the_vertex_size() {
        let mesh = crate::primitives::cube(1.0);
        assert_eq!(mesh.vertex_bytes().len(), mesh.vertices.len() * 48);
        assert_eq!(mesh.index_bytes().len(), mesh.indices.len() * 4);
    }

    #[test]
    fn validation_catches_bad_indices() {
        let mut mesh = crate::primitives::cube(1.0);
        assert!(mesh.validate().is_ok());
        mesh.indices.push(9999);
        assert!(mesh.validate().is_err());
    }
}
