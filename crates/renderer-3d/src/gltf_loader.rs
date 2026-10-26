//! Static glTF 2.0 mesh loading (`.glb`, or `.gltf` with external `.bin` buffers).
//!
//! Every triangle primitive in the default scene is flattened into one `MeshData` in the
//! scene's space: node transforms are applied, the material's base colour factor is multiplied
//! into the vertex colours, and missing normals are computed per face. Textures, skins,
//! animations and morph targets are ignored (out of scope for the prototype).

use std::path::Path;

use glam::{Mat3, Mat4, Vec2, Vec3, Vec4};

use crate::mesh::{MeshData, Vertex};

#[derive(Debug)]
pub struct GltfLoadError(pub String);

impl std::fmt::Display for GltfLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for GltfLoadError {}

/// Loads a `.glb` or `.gltf` file into one mesh.
pub fn load_gltf_mesh(path: &Path) -> Result<MeshData, GltfLoadError> {
    let error = |message: String| GltfLoadError(format!("{}: {message}", path.display()));
    let bytes = std::fs::read(path).map_err(|io| error(io.to_string()))?;
    let base_dir = path.parent().unwrap_or(Path::new("."));
    gltf_mesh_from_bytes(&bytes, base_dir).map_err(|GltfLoadError(message)| error(message))
}

/// Parses glTF bytes. External buffer URIs are resolved against `base_dir`.
pub fn gltf_mesh_from_bytes(bytes: &[u8], base_dir: &Path) -> Result<MeshData, GltfLoadError> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|parse| GltfLoadError(format!("not valid glTF: {parse}")))?;
    let mut buffers: Vec<Vec<u8>> = Vec::new();
    for buffer in gltf.document.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => {
                gltf.blob.clone().ok_or_else(|| GltfLoadError("the GLB has no binary chunk".to_string()))?
            }
            gltf::buffer::Source::Uri(uri) if uri.starts_with("data:") => {
                return Err(GltfLoadError("data: URIs are not supported; export as .glb".to_string()));
            }
            gltf::buffer::Source::Uri(uri) => {
                std::fs::read(base_dir.join(uri)).map_err(|io| GltfLoadError(format!("buffer {uri}: {io}")))?
            }
        };
        if data.len() < buffer.length() {
            return Err(GltfLoadError(format!("buffer {} is shorter than declared", buffer.index())));
        }
        buffers.push(data);
    }

    let scene = gltf
        .document
        .default_scene()
        .or_else(|| gltf.document.scenes().next())
        .ok_or_else(|| GltfLoadError("the file has no scene".to_string()))?;
    let mut mesh = MeshData::default();
    for node in scene.nodes() {
        append_node(&node, Mat4::IDENTITY, &buffers, &mut mesh)?;
    }
    mesh.validate().map_err(|message| GltfLoadError(format!("no usable triangles: {message}")))?;
    Ok(mesh)
}

fn append_node(node: &gltf::Node, parent: Mat4, buffers: &[Vec<u8>], out: &mut MeshData) -> Result<(), GltfLoadError> {
    let world = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                log::warn!(target: "assets", "skipping a non-triangle primitive in mesh {:?}", mesh.name());
                continue;
            }
            append_primitive(&primitive, world, buffers, out)?;
        }
    }
    for child in node.children() {
        append_node(&child, world, buffers, out)?;
    }
    Ok(())
}

fn append_primitive(
    primitive: &gltf::Primitive,
    world: Mat4,
    buffers: &[Vec<u8>],
    out: &mut MeshData,
) -> Result<(), GltfLoadError> {
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
    let positions: Vec<Vec3> = reader
        .read_positions()
        .ok_or_else(|| GltfLoadError("a primitive has no POSITION".to_string()))?
        .map(Vec3::from_array)
        .collect();
    let normals: Option<Vec<Vec3>> = reader.read_normals().map(|normals| normals.map(Vec3::from_array).collect());
    let uvs: Vec<Vec2> = reader
        .read_tex_coords(0)
        .map(|coords| coords.into_f32().map(Vec2::from_array).collect())
        .unwrap_or_else(|| vec![Vec2::ZERO; positions.len()]);
    let base_color = Vec4::from_array(primitive.material().pbr_metallic_roughness().base_color_factor());
    let colors: Vec<Vec4> = reader
        .read_colors(0)
        .map(|colors| colors.into_rgba_f32().map(|color| Vec4::from_array(color) * base_color).collect())
        .unwrap_or_else(|| vec![base_color; positions.len()]);
    let mut indices: Vec<u32> = match reader.read_indices() {
        Some(indices) => indices.into_u32().collect(),
        None => (0..positions.len() as u32).collect(),
    };
    if indices.iter().any(|&index| index as usize >= positions.len()) {
        return Err(GltfLoadError("an index is out of range".to_string()));
    }
    // A mirroring transform turns counter-clockwise triangles clockwise; swap two corners back.
    if world.determinant() < 0.0 {
        for triangle in indices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }

    let normal_matrix = Mat3::from_mat4(world).inverse().transpose();
    let base = out.vertices.len() as u32;
    match normals {
        Some(normals) => {
            for index in 0..positions.len() {
                let position = world.transform_point3(positions[index]);
                let normal = (normal_matrix * normals[index]).normalize_or(Vec3::Y);
                out.vertices.push(Vertex::new(position, normal, uvs[index], colors[index]));
            }
            out.indices.extend(indices.iter().map(|index| base + index));
        }
        None => {
            // Flat shading: three unique vertices per triangle with the face normal.
            for triangle in indices.chunks_exact(3) {
                let corners = [triangle[0], triangle[1], triangle[2]]
                    .map(|index| world.transform_point3(positions[index as usize]));
                let normal = (corners[1] - corners[0]).cross(corners[2] - corners[0]).normalize_or(Vec3::Y);
                for (corner, &index) in corners.iter().zip(triangle) {
                    out.indices.push(out.vertices.len() as u32);
                    out.vertices.push(Vertex::new(*corner, normal, uvs[index as usize], colors[index as usize]));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal GLB: one triangle, positions only, with a translated node and a red material.
    pub(crate) fn tiny_glb() -> Vec<u8> {
        let mut binary = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            binary.extend_from_slice(&value.to_le_bytes());
        }
        let json = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],
            "nodes":[{"mesh":0,"translation":[0,2,0]}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0}]}],
            "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[1,0,0,1]}}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],
            "bufferViews":[{"buffer":0,"byteLength":36}],
            "buffers":[{"byteLength":36}]}"#;
        let mut json_bytes = json.as_bytes().to_vec();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let total = 12 + 8 + json_bytes.len() + 8 + binary.len();
        let mut glb = Vec::new();
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&binary);
        glb
    }

    #[test]
    fn a_glb_triangle_loads_with_node_transform_material_and_normals() {
        let mesh = gltf_mesh_from_bytes(&tiny_glb(), Path::new(".")).unwrap();
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertices[0].position, [0.0, 2.0, 0.0]);
        assert_eq!(mesh.vertices[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(mesh.vertices[0].normal, [0.0, 0.0, 1.0], "computed face normal faces +Z");
    }

    #[test]
    fn garbage_is_a_clear_error() {
        let error = gltf_mesh_from_bytes(b"not a gltf", Path::new(".")).unwrap_err();
        assert!(error.0.contains("not valid glTF"), "{error}");
    }
}
