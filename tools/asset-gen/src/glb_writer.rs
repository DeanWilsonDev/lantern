//! Writes the gem collectible as a binary glTF (.glb), the model format the engine loads.
//!
//! The writer covers exactly what the loader reads: one mesh, one triangle primitive with
//! POSITION, NORMAL, TEXCOORD_0 and COLOR_0, and 32-bit indices.

use glam::{Vec2, Vec3, Vec4};
use lantern_renderer_3d::mesh::{MeshData, Vertex};

const GLB_MAGIC: &[u8; 4] = b"glTF";
const GLB_VERSION: u32 = 2;
const CHUNK_TYPE_JSON: &[u8; 4] = b"JSON";
const CHUNK_TYPE_BIN: &[u8; 4] = b"BIN\0";
const COMPONENT_TYPE_FLOAT: u32 = 5126;
const COMPONENT_TYPE_UNSIGNED_INT: u32 = 5125;
const TARGET_ARRAY_BUFFER: u32 = 34962;
const TARGET_ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// A faceted gem: an eight-sided crown with a flat table and a pointed pavilion, flat-shaded,
/// about 0.5 m wide. Crown facets are bright cyan and the pavilion is deeper blue, so it reads
/// as a collectible from any angle.
pub fn gem_mesh() -> MeshData {
    const SIDES: usize = 8;
    const GIRDLE_RADIUS: f32 = 0.25;
    const TABLE_RADIUS: f32 = 0.15;
    const TABLE_HEIGHT: f32 = 0.14;
    const PAVILION_DEPTH: f32 = 0.32;
    let crown_color = Vec4::new(0.35, 0.95, 1.0, 1.0);
    let pavilion_color = Vec4::new(0.15, 0.55, 0.95, 1.0);

    let ring_point = |radius: f32, height: f32, index: usize| {
        let angle = index as f32 / SIDES as f32 * std::f32::consts::TAU;
        Vec3::new(angle.cos() * radius, height, -angle.sin() * radius)
    };
    let mut mesh = MeshData::default();
    let mut add_facet = |corners: &[Vec3], color: Vec4| {
        let centroid = corners.iter().copied().sum::<Vec3>() / corners.len() as f32;
        let mut normal = (corners[1] - corners[0]).cross(corners[2] - corners[0]).normalize();
        // The gem is convex around the origin, so "outward" is away from the centre. Facets are
        // listed in either winding and fixed here, which keeps the geometry code short.
        let flip_winding = normal.dot(centroid) < 0.0;
        if flip_winding {
            normal = -normal;
        }
        let base = mesh.vertices.len() as u32;
        for corner in corners {
            let uv = Vec2::new(corner.x / GIRDLE_RADIUS * 0.5 + 0.5, corner.z / GIRDLE_RADIUS * 0.5 + 0.5);
            mesh.vertices.push(Vertex::new(*corner, normal, uv, color));
        }
        for fan_index in 1..corners.len() as u32 - 1 {
            if flip_winding {
                mesh.indices.extend_from_slice(&[base, base + fan_index + 1, base + fan_index]);
            } else {
                mesh.indices.extend_from_slice(&[base, base + fan_index, base + fan_index + 1]);
            }
        }
    };

    let table: Vec<Vec3> = (0..SIDES).map(|index| ring_point(TABLE_RADIUS, TABLE_HEIGHT, index)).collect();
    add_facet(&table, crown_color);
    let bottom_point = Vec3::new(0.0, -PAVILION_DEPTH, 0.0);
    for index in 0..SIDES {
        let next = (index + 1) % SIDES;
        let crown_facet =
            [table[index], ring_point(GIRDLE_RADIUS, 0.0, index), ring_point(GIRDLE_RADIUS, 0.0, next), table[next]];
        add_facet(&crown_facet, crown_color);
        add_facet(
            &[ring_point(GIRDLE_RADIUS, 0.0, index), bottom_point, ring_point(GIRDLE_RADIUS, 0.0, next)],
            pavilion_color,
        );
    }
    mesh
}

pub fn gem_glb() -> Vec<u8> {
    write_glb(&gem_mesh(), "gem")
}

fn push_floats(binary: &mut Vec<u8>, values: impl IntoIterator<Item = f32>) {
    for value in values {
        binary.extend_from_slice(&value.to_le_bytes());
    }
}

fn format_float_array(values: &[f32]) -> String {
    let parts: Vec<String> = values.iter().map(|value| format!("{value:?}")).collect();
    format!("[{}]", parts.join(","))
}

pub fn write_glb(mesh: &MeshData, mesh_name: &str) -> Vec<u8> {
    let vertex_count = mesh.vertices.len();
    let mut binary = Vec::new();
    // (byte offset, byte length, target) for each buffer view, in accessor order.
    let mut buffer_views: Vec<(usize, usize, u32)> = Vec::new();
    let mut add_view = |binary: &mut Vec<u8>, write: &dyn Fn(&mut Vec<u8>), target: u32| {
        let offset = binary.len();
        write(binary);
        buffer_views.push((offset, binary.len() - offset, target));
    };
    add_view(&mut binary, &|out| push_floats(out, mesh.vertices.iter().flat_map(|v| v.position)), TARGET_ARRAY_BUFFER);
    add_view(&mut binary, &|out| push_floats(out, mesh.vertices.iter().flat_map(|v| v.normal)), TARGET_ARRAY_BUFFER);
    add_view(&mut binary, &|out| push_floats(out, mesh.vertices.iter().flat_map(|v| v.uv)), TARGET_ARRAY_BUFFER);
    add_view(&mut binary, &|out| push_floats(out, mesh.vertices.iter().flat_map(|v| v.color)), TARGET_ARRAY_BUFFER);
    add_view(
        &mut binary,
        &|out| mesh.indices.iter().for_each(|index| out.extend_from_slice(&index.to_le_bytes())),
        TARGET_ELEMENT_ARRAY_BUFFER,
    );

    let bounds = mesh.bounds();
    let views_json: Vec<String> = buffer_views
        .iter()
        .map(|(offset, length, target)| {
            format!(r#"{{"buffer":0,"byteOffset":{offset},"byteLength":{length},"target":{target}}}"#)
        })
        .collect();
    let accessors_json = [
        format!(
            r#"{{"bufferView":0,"componentType":{COMPONENT_TYPE_FLOAT},"count":{vertex_count},"type":"VEC3","min":{},"max":{}}}"#,
            format_float_array(&bounds.min.to_array()),
            format_float_array(&bounds.max.to_array())
        ),
        format!(r#"{{"bufferView":1,"componentType":{COMPONENT_TYPE_FLOAT},"count":{vertex_count},"type":"VEC3"}}"#),
        format!(r#"{{"bufferView":2,"componentType":{COMPONENT_TYPE_FLOAT},"count":{vertex_count},"type":"VEC2"}}"#),
        format!(r#"{{"bufferView":3,"componentType":{COMPONENT_TYPE_FLOAT},"count":{vertex_count},"type":"VEC4"}}"#),
        format!(
            r#"{{"bufferView":4,"componentType":{COMPONENT_TYPE_UNSIGNED_INT},"count":{},"type":"SCALAR"}}"#,
            mesh.indices.len()
        ),
    ];
    let json = format!(
        concat!(
            r#"{{"asset":{{"version":"2.0","generator":"lantern asset-gen"}},"#,
            r#""scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0,"name":"{name}"}}],"#,
            r#""meshes":[{{"name":"{name}","primitives":[{{"attributes":{{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2,"COLOR_0":3}},"indices":4,"mode":4}}]}}],"#,
            r#""accessors":[{accessors}],"bufferViews":[{views}],"buffers":[{{"byteLength":{binary_length}}}]}}"#
        ),
        name = mesh_name,
        accessors = accessors_json.join(","),
        views = views_json.join(","),
        binary_length = binary.len(),
    );

    // GLB chunks are 4-byte aligned: JSON pads with spaces, binary with zeros.
    let mut json_bytes = json.into_bytes();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    while !binary.len().is_multiple_of(4) {
        binary.push(0);
    }
    let total_length = 12 + 8 + json_bytes.len() + 8 + binary.len();
    let mut glb = Vec::with_capacity(total_length);
    glb.extend_from_slice(GLB_MAGIC);
    glb.extend_from_slice(&GLB_VERSION.to_le_bytes());
    glb.extend_from_slice(&(total_length as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(CHUNK_TYPE_JSON);
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(CHUNK_TYPE_BIN);
    glb.extend_from_slice(&binary);
    glb
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn gem_round_trips_through_the_engine_loader() {
        let original = gem_mesh();
        let loaded = lantern_renderer_3d::gltf_loader::gltf_mesh_from_bytes(&gem_glb(), Path::new(".")).unwrap();
        assert_eq!(loaded.indices, original.indices);
        assert_eq!(loaded.vertices.len(), original.vertices.len());
        // The loader renormalises normals, so compare with a small tolerance.
        for (loaded_vertex, original_vertex) in loaded.vertices.iter().zip(&original.vertices) {
            let loaded_values = [loaded_vertex.position, loaded_vertex.normal].concat();
            let original_values = [original_vertex.position, original_vertex.normal].concat();
            for (loaded_value, original_value) in loaded_values.iter().zip(&original_values) {
                assert!((loaded_value - original_value).abs() < 1e-5);
            }
            assert_eq!(loaded_vertex.uv, original_vertex.uv);
            assert_eq!(loaded_vertex.color, original_vertex.color);
        }
    }

    #[test]
    fn gem_facets_face_outward() {
        let mesh = gem_mesh();
        for triangle in mesh.triangles() {
            let normal = (triangle[1] - triangle[0]).cross(triangle[2] - triangle[0]);
            let centroid = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
            assert!(normal.dot(centroid) > 0.0);
        }
    }
}
