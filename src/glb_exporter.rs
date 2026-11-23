use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use serde_json::json;
use crate::modeler::ForgeMesh;

pub fn mesh_to_glb_bytes(mesh: &ForgeMesh, model_name: &str) -> Vec<u8> {
    let num_verts = mesh.positions.len();
    let num_indices = mesh.indices.len();

    if num_verts == 0 || num_indices == 0 {
        return Vec::new();
    }

    // 1. Calculate bounding box for POSITION accessor
    let mut min_pos = [f32::INFINITY; 3];
    let mut max_pos = [f32::NEG_INFINITY; 3];
    for p in &mesh.positions {
        min_pos[0] = min_pos[0].min(p[0]);
        min_pos[1] = min_pos[1].min(p[1]);
        min_pos[2] = min_pos[2].min(p[2]);

        max_pos[0] = max_pos[0].max(p[0]);
        max_pos[1] = max_pos[1].max(p[1]);
        max_pos[2] = max_pos[2].max(p[2]);
    }

    // 2. Build Binary Chunk
    let pos_len = num_verts * 3 * 4;
    let norm_len = num_verts * 3 * 4;
    let col_len = num_verts * 4 * 4;
    let uv_len = num_verts * 2 * 4;
    let idx_len = num_indices * 4;

    let offset_pos = 0;
    let offset_norm = offset_pos + pos_len;
    let offset_col = offset_norm + norm_len;
    let offset_uv = offset_col + col_len;
    let offset_idx = offset_uv + uv_len;
    let total_bin_len = offset_idx + idx_len;

    let mut bin_data = Vec::with_capacity(total_bin_len);

    // Write positions
    for p in &mesh.positions {
        bin_data.extend_from_slice(&p[0].to_le_bytes());
        bin_data.extend_from_slice(&p[1].to_le_bytes());
        bin_data.extend_from_slice(&p[2].to_le_bytes());
    }

    // Write normals
    for n in &mesh.normals {
        bin_data.extend_from_slice(&n[0].to_le_bytes());
        bin_data.extend_from_slice(&n[1].to_le_bytes());
        bin_data.extend_from_slice(&n[2].to_le_bytes());
    }

    // Write colors (COLOR_0)
    for c in &mesh.colors {
        bin_data.extend_from_slice(&c[0].to_le_bytes());
        bin_data.extend_from_slice(&c[1].to_le_bytes());
        bin_data.extend_from_slice(&c[2].to_le_bytes());
        bin_data.extend_from_slice(&c[3].to_le_bytes());
    }

    // Write UVs
    for uv in &mesh.uvs {
        bin_data.extend_from_slice(&uv[0].to_le_bytes());
        bin_data.extend_from_slice(&uv[1].to_le_bytes());
    }

    // Write Indices
    for idx in &mesh.indices {
        bin_data.extend_from_slice(&idx.to_le_bytes());
    }

    // Pad bin data to 4-byte boundary if needed
    while bin_data.len() % 4 != 0 {
        bin_data.push(0x00);
    }
    let padded_bin_len = bin_data.len();

    // 3. Build glTF JSON metadata
    let gltf_json = json!({
        "asset": {
            "version": "2.0",
            "generator": "Motor-dif Asset Forge glTF 2.0 Exporter"
        },
        "scene": 0,
        "scenes": [
            {
                "nodes": [0]
            }
        ],
        "nodes": [
            {
                "name": model_name,
                "mesh": 0
            }
        ],
        "meshes": [
            {
                "name": model_name,
                "primitives": [
                    {
                        "attributes": {
                            "POSITION": 0,
                            "NORMAL": 1,
                            "COLOR_0": 2,
                            "TEXCOORD_0": 3
                        },
                        "indices": 4,
                        "mode": 4
                    }
                ]
            }
        ],
        "accessors": [
            {
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,
                "count": num_verts,
                "type": "VEC3",
                "min": min_pos,
                "max": max_pos
            },
            {
                "bufferView": 1,
                "byteOffset": 0,
                "componentType": 5126,
                "count": num_verts,
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "byteOffset": 0,
                "componentType": 5126,
                "count": num_verts,
                "type": "VEC4"
            },
            {
                "bufferView": 3,
                "byteOffset": 0,
                "componentType": 5126,
                "count": num_verts,
                "type": "VEC2"
            },
            {
                "bufferView": 4,
                "byteOffset": 0,
                "componentType": 5125,
                "count": num_indices,
                "type": "SCALAR"
            }
        ],
        "bufferViews": [
            {
                "buffer": 0,
                "byteOffset": offset_pos,
                "byteLength": pos_len,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": offset_norm,
                "byteLength": norm_len,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": offset_col,
                "byteLength": col_len,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": offset_uv,
                "byteLength": uv_len,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": offset_idx,
                "byteLength": idx_len,
                "target": 34963
            }
        ],
        "buffers": [
            {
                "byteLength": padded_bin_len
            }
        ]
    });

    let mut json_str = serde_json::to_string(&gltf_json).unwrap_or_default();
    // Pad JSON text with spaces (0x20) to 4-byte boundary
    while json_str.len() % 4 != 0 {
        json_str.push(' ');
    }
    let json_bytes = json_str.as_bytes();
    let json_len = json_bytes.len() as u32;

    // 4. Build Full .GLB file
    // Header (12 bytes) + Chunk 0 (8 + json_len) + Chunk 1 (8 + padded_bin_len)
    let total_file_size = 12 + 8 + json_len + 8 + padded_bin_len as u32;
    let mut glb = Vec::with_capacity(total_file_size as usize);

    // Header
    glb.extend_from_slice(&0x46546C67u32.to_le_bytes()); // magic: "glTF"
    glb.extend_from_slice(&2u32.to_le_bytes());          // version: 2
    glb.extend_from_slice(&total_file_size.to_le_bytes());

    // Chunk 0: JSON
    glb.extend_from_slice(&json_len.to_le_bytes());
    glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // chunkType: "JSON"
    glb.extend_from_slice(json_bytes);

    // Chunk 1: BIN
    glb.extend_from_slice(&(padded_bin_len as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // chunkType: "BIN\0"
    glb.extend_from_slice(&bin_data);

    glb
}

pub fn export_glb_to_file(mesh: &ForgeMesh, model_name: &str, output_path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let bytes = mesh_to_glb_bytes(mesh, model_name);
    let p = output_path.as_ref();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(p)?;
    file.write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_glb_valid_format() {
        let box_mesh = crate::modeler::create_box(1.0, 1.0, 1.0, [0.8, 0.2, 0.3, 1.0]);
        let glb_bytes = mesh_to_glb_bytes(&box_mesh, "test_box");

        assert!(glb_bytes.len() > 20);
        // Verify magic number "glTF"
        assert_eq!(&glb_bytes[0..4], b"glTF");
        // Verify version 2
        let version = u32::from_le_bytes([glb_bytes[4], glb_bytes[5], glb_bytes[6], glb_bytes[7]]);
        assert_eq!(version, 2);

        // Save to temporary file and verify with gltf crate importer
        let temp_path = Path::new("target/test_glb_export.glb");
        export_glb_to_file(&box_mesh, "test_box", temp_path).expect("Failed to export GLB");
        assert!(temp_path.exists());

        let (doc, buffers, _) = gltf::import(temp_path).expect("gltf::import failed on exported GLB!");
        assert_eq!(doc.meshes().count(), 1);

        let mesh = doc.meshes().next().unwrap();
        let prim = mesh.primitives().next().unwrap();
        let reader = prim.reader(|b| Some(&buffers[b.index()]));

        let pos: Vec<_> = reader.read_positions().unwrap().collect();
        assert_eq!(pos.len(), 24);

        let colors: Vec<[f32; 4]> = reader.read_colors(0).unwrap().into_rgba_f32().collect();
        assert_eq!(colors.len(), 24);
        assert!((colors[0][0] - 0.8).abs() < 1e-4);

        let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();
        assert_eq!(indices.len(), 36);

        let _ = fs::remove_file(temp_path);
    }
}
