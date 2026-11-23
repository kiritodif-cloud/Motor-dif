use glam::Vec3;
use serde::{Deserialize, Serialize};
use crate::modeler::ForgeMesh;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SdfPrimitive {
    Sphere { radius: f32 },
    Box { half_extents: [f32; 3] },
    Capsule { a: [f32; 3], b: [f32; 3], radius: f32 },
    Cylinder { radius: f32, half_height: f32 },
    Torus { major_radius: f32, minor_radius: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SdfOp {
    Union,
    Subtract,
    Intersect,
    SmoothUnion { k: f32 },
    SmoothSubtract { k: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdfNodeDef {
    pub name: String,
    pub primitive: SdfPrimitive,
    pub op: SdfOp,
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation_deg: [f32; 3],
    #[serde(default = "default_color")]
    pub color: [f32; 4],
}

fn default_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

impl SdfNodeDef {
    pub fn sphere(name: &str, radius: f32, color: [f32; 4]) -> Self {
        Self {
            name: name.to_string(),
            primitive: SdfPrimitive::Sphere { radius },
            op: SdfOp::Union,
            position: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            color,
        }
    }

    pub fn box_shape(name: &str, half_extents: [f32; 3], color: [f32; 4]) -> Self {
        Self {
            name: name.to_string(),
            primitive: SdfPrimitive::Box { half_extents },
            op: SdfOp::Union,
            position: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            color,
        }
    }

    pub fn eval_primitive(prim: &SdfPrimitive, p: Vec3) -> f32 {
        match prim {
            SdfPrimitive::Sphere { radius } => p.length() - radius,
            SdfPrimitive::Box { half_extents } => {
                let h = Vec3::from(*half_extents);
                let q = p.abs() - h;
                let outside = q.max(Vec3::ZERO).length();
                let inside = q.x.max(q.y.max(q.z)).min(0.0);
                outside + inside
            }
            SdfPrimitive::Capsule { a, b, radius } => {
                let pa = p - Vec3::from(*a);
                let ba = Vec3::from(*b) - Vec3::from(*a);
                let ba_len_sq = ba.length_squared();
                let h = if ba_len_sq > 1e-6 {
                    (pa.dot(ba) / ba_len_sq).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (pa - ba * h).length() - radius
            }
            SdfPrimitive::Cylinder { radius, half_height } => {
                let d_x = (p.x * p.x + p.z * p.z).sqrt() - radius;
                let d_y = p.y.abs() - half_height;
                let outside = Vec3::new(d_x.max(0.0), d_y.max(0.0), 0.0).length();
                let inside = d_x.max(d_y).min(0.0);
                outside + inside
            }
            SdfPrimitive::Torus { major_radius, minor_radius } => {
                let q_x = (p.x * p.x + p.z * p.z).sqrt() - major_radius;
                let q_y = p.y;
                (q_x * q_x + q_y * q_y).sqrt() - minor_radius
            }
        }
    }

    pub fn eval_local(&self, world_p: Vec3) -> f32 {
        let p_rel = world_p - Vec3::from(self.position);
        // Inverse Euler rotation
        let rx = -self.rotation_deg[0].to_radians();
        let ry = -self.rotation_deg[1].to_radians();
        let rz = -self.rotation_deg[2].to_radians();
        let rot = glam::Mat3::from_rotation_z(rz)
            * glam::Mat3::from_rotation_y(ry)
            * glam::Mat3::from_rotation_x(rx);
        let p_local = rot * p_rel;
        Self::eval_primitive(&self.primitive, p_local)
    }
}

pub fn eval_sdf_tree(nodes: &[SdfNodeDef], p: Vec3) -> (f32, [f32; 4]) {
    if nodes.is_empty() {
        return (1000.0, [1.0, 1.0, 1.0, 1.0]);
    }

    let mut current_d = nodes[0].eval_local(p);
    let mut current_col = nodes[0].color;

    for node in &nodes[1..] {
        let d = node.eval_local(p);
        let col = node.color;

        match &node.op {
            SdfOp::Union => {
                if d < current_d {
                    current_d = d;
                    current_col = col;
                }
            }
            SdfOp::Subtract => {
                let neg_d = -d;
                if neg_d > current_d {
                    current_d = neg_d;
                    current_col = col;
                }
            }
            SdfOp::Intersect => {
                if d > current_d {
                    current_d = d;
                    current_col = col;
                }
            }
            SdfOp::SmoothUnion { k } => {
                let factor = *k;
                let h = (0.5 + 0.5 * (current_d - d) / factor).clamp(0.0, 1.0);
                current_d = (1.0 - h) * current_d + h * d - factor * h * (1.0 - h);
                current_col = [
                    (1.0 - h) * current_col[0] + h * col[0],
                    (1.0 - h) * current_col[1] + h * col[1],
                    (1.0 - h) * current_col[2] + h * col[2],
                    (1.0 - h) * current_col[3] + h * col[3],
                ];
            }
            SdfOp::SmoothSubtract { k } => {
                let factor = *k;
                let h = (0.5 - 0.5 * (current_d + d) / factor).clamp(0.0, 1.0);
                current_d = (1.0 - h) * current_d + h * (-d) + factor * h * (1.0 - h);
                current_col = [
                    (1.0 - h) * current_col[0] + h * col[0],
                    (1.0 - h) * current_col[1] + h * col[1],
                    (1.0 - h) * current_col[2] + h * col[2],
                    (1.0 - h) * current_col[3] + h * col[3],
                ];
            }
        }
    }

    (current_d, current_col)
}

pub fn eval_sdf_normal(nodes: &[SdfNodeDef], p: Vec3) -> [f32; 3] {
    let eps = 0.005;
    let dx = eval_sdf_tree(nodes, p + Vec3::new(eps, 0.0, 0.0)).0 - eval_sdf_tree(nodes, p - Vec3::new(eps, 0.0, 0.0)).0;
    let dy = eval_sdf_tree(nodes, p + Vec3::new(0.0, eps, 0.0)).0 - eval_sdf_tree(nodes, p - Vec3::new(0.0, eps, 0.0)).0;
    let dz = eval_sdf_tree(nodes, p + Vec3::new(0.0, 0.0, eps)).0 - eval_sdf_tree(nodes, p - Vec3::new(0.0, 0.0, eps)).0;
    let n = Vec3::new(dx, dy, dz).normalize_or_zero();
    [n.x, n.y, n.z]
}

/// Marching Tetrahedra / Surface Nets polygonizer over regular 3D grid
pub fn sdf_to_mesh(nodes: &[SdfNodeDef], resolution: usize, bounds: f32) -> ForgeMesh {
    if nodes.is_empty() {
        return ForgeMesh::default();
    }

    let res = resolution.clamp(16, 64);
    let cell_size = (bounds * 2.0) / (res as f32);
    let min_b = -bounds;

    // 1. Evaluate distance and color at all grid vertices
    let num_verts = (res + 1) * (res + 1) * (res + 1);
    let mut field_d = Vec::with_capacity(num_verts);
    let mut field_col = Vec::with_capacity(num_verts);

    for gz in 0..=res {
        let z = min_b + gz as f32 * cell_size;
        for gy in 0..=res {
            let y = min_b + gy as f32 * cell_size;
            for gx in 0..=res {
                let x = min_b + gx as f32 * cell_size;
                let (d, col) = eval_sdf_tree(nodes, Vec3::new(x, y, z));
                field_d.push(d);
                field_col.push(col);
            }
        }
    }

    let idx = |x: usize, y: usize, z: usize| -> usize {
        z * (res + 1) * (res + 1) + y * (res + 1) + x
    };

    let mut mesh = ForgeMesh::default();
    let mut cell_vertex_map = vec![u32::MAX; res * res * res];

    let cell_idx = |x: usize, y: usize, z: usize| -> usize {
        z * res * res + y * res + x
    };

    // 2. Identify active cells and compute one vertex per active cell (Surface Nets)
    for cz in 0..res {
        for cy in 0..res {
            for cx in 0..res {

                // 8 corners of the cell
                let c = [
                    (cx, cy, cz),
                    (cx + 1, cy, cz),
                    (cx, cy + 1, cz),
                    (cx + 1, cy + 1, cz),
                    (cx, cy, cz + 1),
                    (cx + 1, cy, cz + 1),
                    (cx, cy + 1, cz + 1),
                    (cx + 1, cy + 1, cz + 1),
                ];

                let mut inside_count = 0;
                for &(gx, gy, gz) in &c {
                    if field_d[idx(gx, gy, gz)] < 0.0 {
                        inside_count += 1;
                    }
                }

                if inside_count > 0 && inside_count < 8 {
                    // Cell intersects surface: calculate average intersection point
                    let mut avg_p = Vec3::ZERO;
                    let mut avg_col = [0.0; 4];
                    let mut edge_count = 0.0;

                    // 12 edges of cell
                    let edges = [
                        (0, 1, Vec3::X),
                        (2, 3, Vec3::X),
                        (4, 5, Vec3::X),
                        (6, 7, Vec3::X),
                        (0, 2, Vec3::Y),
                        (1, 3, Vec3::Y),
                        (4, 6, Vec3::Y),
                        (5, 7, Vec3::Y),
                        (0, 4, Vec3::Z),
                        (1, 5, Vec3::Z),
                        (2, 6, Vec3::Z),
                        (3, 7, Vec3::Z),
                    ];

                    for &(e0, e1, _dir) in &edges {
                        let i0 = idx(c[e0].0, c[e0].1, c[e0].2);
                        let i1 = idx(c[e1].0, c[e1].1, c[e1].2);
                        let d0 = field_d[i0];
                        let d1 = field_d[i1];

                        if (d0 < 0.0) != (d1 < 0.0) {
                            let t = (0.0 - d0) / (d1 - d0);
                            let p0 = Vec3::new(
                                min_b + c[e0].0 as f32 * cell_size,
                                min_b + c[e0].1 as f32 * cell_size,
                                min_b + c[e0].2 as f32 * cell_size,
                            );
                            let p1 = Vec3::new(
                                min_b + c[e1].0 as f32 * cell_size,
                                min_b + c[e1].1 as f32 * cell_size,
                                min_b + c[e1].2 as f32 * cell_size,
                            );
                            let pt = p0 + (p1 - p0) * t;
                            avg_p += pt;

                            let col0 = field_col[i0];
                            let col1 = field_col[i1];
                            avg_col[0] += (1.0 - t) * col0[0] + t * col1[0];
                            avg_col[1] += (1.0 - t) * col0[1] + t * col1[1];
                            avg_col[2] += (1.0 - t) * col0[2] + t * col1[2];
                            avg_col[3] += (1.0 - t) * col0[3] + t * col1[3];
                            edge_count += 1.0;
                        }
                    }

                    if edge_count > 0.0 {
                        let final_p = avg_p / edge_count;
                        let final_col = [
                            avg_col[0] / edge_count,
                            avg_col[1] / edge_count,
                            avg_col[2] / edge_count,
                            avg_col[3] / edge_count,
                        ];

                        let norm = eval_sdf_normal(nodes, final_p);
                        let u = ((final_p.x / bounds) * 0.5 + 0.5).clamp(0.0, 1.0);
                        let v = ((final_p.z / bounds) * 0.5 + 0.5).clamp(0.0, 1.0);

                        let v_id = mesh.positions.len() as u32;
                        mesh.positions.push([final_p.x, final_p.y, final_p.z]);
                        mesh.normals.push(norm);
                        mesh.uvs.push([u, v]);
                        mesh.colors.push(final_col);
                        cell_vertex_map[cell_idx(cx, cy, cz)] = v_id;
                    }
                }
            }
        }
    }

    // 3. Connect adjacent cell vertices across sign-changing interior edges
    for cz in 0..res {
        for cy in 0..res {
            for cx in 0..res {
                // X-edge crossing
                if cx + 1 < res && cy > 0 && cz > 0 {
                    let d0 = field_d[idx(cx + 1, cy, cz)];
                    let d1 = field_d[idx(cx + 1, cy, cz + 1)];
                    if (d0 < 0.0) != (d1 < 0.0) {
                        let v0 = cell_vertex_map[cell_idx(cx, cy - 1, cz - 1)];
                        let v1 = cell_vertex_map[cell_idx(cx + 1, cy - 1, cz - 1)];
                        let v2 = cell_vertex_map[cell_idx(cx + 1, cy, cz - 1)];
                        let v3 = cell_vertex_map[cell_idx(cx, cy, cz - 1)];

                        if v0 != u32::MAX && v1 != u32::MAX && v2 != u32::MAX && v3 != u32::MAX {
                            if d0 < 0.0 {
                                mesh.indices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
                            } else {
                                mesh.indices.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
                            }
                        }
                    }
                }

                // Y-edge crossing
                if cy + 1 < res && cx > 0 && cz > 0 {
                    let d0 = field_d[idx(cx, cy + 1, cz)];
                    let d1 = field_d[idx(cx, cy + 1, cz + 1)];
                    if (d0 < 0.0) != (d1 < 0.0) {
                        let v0 = cell_vertex_map[cell_idx(cx - 1, cy, cz - 1)];
                        let v1 = cell_vertex_map[cell_idx(cx, cy, cz - 1)];
                        let v2 = cell_vertex_map[cell_idx(cx, cy + 1, cz - 1)];
                        let v3 = cell_vertex_map[cell_idx(cx - 1, cy + 1, cz - 1)];

                        if v0 != u32::MAX && v1 != u32::MAX && v2 != u32::MAX && v3 != u32::MAX {
                            if d0 < 0.0 {
                                mesh.indices.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
                            } else {
                                mesh.indices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
                            }
                        }
                    }
                }

                // Z-edge crossing
                if cz + 1 < res && cx > 0 && cy > 0 {
                    let d0 = field_d[idx(cx, cy, cz + 1)];
                    let d1 = field_d[idx(cx + 1, cy, cz + 1)];
                    if (d0 < 0.0) != (d1 < 0.0) {
                        let v0 = cell_vertex_map[cell_idx(cx - 1, cy - 1, cz)];
                        let v1 = cell_vertex_map[cell_idx(cx, cy - 1, cz)];
                        let v2 = cell_vertex_map[cell_idx(cx, cy, cz)];
                        let v3 = cell_vertex_map[cell_idx(cx - 1, cy, cz)];

                        if v0 != u32::MAX && v1 != u32::MAX && v2 != u32::MAX && v3 != u32::MAX {
                            if d0 < 0.0 {
                                mesh.indices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
                            } else {
                                mesh.indices.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
                            }
                        }
                    }
                }
            }
        }
    }

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdf_primitives_eval() {
        let sphere = SdfNodeDef::sphere("s1", 1.0, [1.0, 0.0, 0.0, 1.0]);
        assert!((sphere.eval_local(Vec3::new(0.0, 0.0, 0.0)) - (-1.0)).abs() < 1e-4);
        assert!((sphere.eval_local(Vec3::new(1.0, 0.0, 0.0)) - 0.0).abs() < 1e-4);
        assert!((sphere.eval_local(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-4);

        let box_node = SdfNodeDef::box_shape("b1", [1.0, 1.0, 1.0], [0.0, 1.0, 0.0, 1.0]);
        assert!(box_node.eval_local(Vec3::new(0.0, 0.0, 0.0)) < 0.0);
        assert!((box_node.eval_local(Vec3::new(1.0, 0.0, 0.0)) - 0.0).abs() < 1e-4);
        assert!((box_node.eval_local(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_sdf_boolean_operations() {
        let s1 = SdfNodeDef::sphere("s1", 1.0, [1.0, 0.0, 0.0, 1.0]);
        let mut s2 = SdfNodeDef::sphere("s2", 1.0, [0.0, 1.0, 0.0, 1.0]);
        s2.position = [0.8, 0.0, 0.0];
        s2.op = SdfOp::SmoothUnion { k: 0.2 };

        let nodes = vec![s1, s2];
        let (d, col) = eval_sdf_tree(&nodes, Vec3::new(0.4, 0.0, 0.0));
        assert!(d < 0.0, "Midpoint between smooth-unioned spheres should be inside");
        assert!(col[0] > 0.1 && col[1] > 0.1, "Color should be smoothly blended");
    }

    #[test]
    fn test_sdf_to_mesh_generation() {
        let s = SdfNodeDef::sphere("s1", 1.0, [0.2, 0.8, 1.0, 1.0]);
        let mesh = sdf_to_mesh(&[s], 24, 1.5);
        assert!(!mesh.positions.is_empty(), "SDF mesh should have vertices");
        assert!(!mesh.indices.is_empty(), "SDF mesh should have indices");
        assert_eq!(mesh.positions.len(), mesh.normals.len());
        assert_eq!(mesh.positions.len(), mesh.colors.len());
    }
}
