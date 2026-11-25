use glam::{EulerRot, Mat3, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ForgeMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl ForgeMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(v_cap: usize, i_cap: usize) -> Self {
        Self {
            positions: Vec::with_capacity(v_cap),
            normals: Vec::with_capacity(v_cap),
            uvs: Vec::with_capacity(v_cap),
            colors: Vec::with_capacity(v_cap),
            indices: Vec::with_capacity(i_cap),
        }
    }

    pub fn merge(&mut self, other: &ForgeMesh) {
        let base_index = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);
        self.colors.extend_from_slice(&other.colors);
        for &idx in &other.indices {
            self.indices.push(base_index + idx);
        }
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        for c in &mut self.colors {
            *c = color;
        }
    }

    pub fn transform(&mut self, translation: [f32; 3], rotation_deg: [f32; 3], scale: [f32; 3]) {
        let rx = rotation_deg[0].to_radians();
        let ry = rotation_deg[1].to_radians();
        let rz = rotation_deg[2].to_radians();
        let rot = Mat3::from_euler(EulerRot::YXZ, ry, rx, rz);
        let trans = Vec3::from(translation);
        let sc = Vec3::from(scale);

        for p in &mut self.positions {
            let v = Vec3::from(*p);
            let scaled = v * sc;
            let rotated = rot * scaled;
            let final_p = rotated + trans;
            *p = [final_p.x, final_p.y, final_p.z];
        }

        for n in &mut self.normals {
            let v = Vec3::from(*n);
            let rot_n = (rot * v).normalize_or_zero();
            *n = [rot_n.x, rot_n.y, rot_n.z];
        }
    }

    pub fn recompute_normals(&mut self, flat: bool) {
        if flat {
            let mut new_pos = Vec::with_capacity(self.indices.len());
            let mut new_norm = Vec::with_capacity(self.indices.len());
            let mut new_uv = Vec::with_capacity(self.indices.len());
            let mut new_col = Vec::with_capacity(self.indices.len());
            let mut new_idx = Vec::with_capacity(self.indices.len());

            for chunk in self.indices.chunks_exact(3) {
                let i0 = chunk[0] as usize;
                let i1 = chunk[1] as usize;
                let i2 = chunk[2] as usize;
                if i0 < self.positions.len() && i1 < self.positions.len() && i2 < self.positions.len() {
                    let p0 = Vec3::from(self.positions[i0]);
                    let p1 = Vec3::from(self.positions[i1]);
                    let p2 = Vec3::from(self.positions[i2]);
                    let n = (p1 - p0).cross(p2 - p0).normalize_or_zero();
                    let n_arr = [n.x, n.y, n.z];

                    let base = new_pos.len() as u32;
                    new_pos.push(self.positions[i0]);
                    new_norm.push(n_arr);
                    new_uv.push(if i0 < self.uvs.len() { self.uvs[i0] } else { [0.0, 0.0] });
                    new_col.push(if i0 < self.colors.len() { self.colors[i0] } else { [1.0, 1.0, 1.0, 1.0] });

                    new_pos.push(self.positions[i1]);
                    new_norm.push(n_arr);
                    new_uv.push(if i1 < self.uvs.len() { self.uvs[i1] } else { [0.0, 0.0] });
                    new_col.push(if i1 < self.colors.len() { self.colors[i1] } else { [1.0, 1.0, 1.0, 1.0] });

                    new_pos.push(self.positions[i2]);
                    new_norm.push(n_arr);
                    new_uv.push(if i2 < self.uvs.len() { self.uvs[i2] } else { [0.0, 0.0] });
                    new_col.push(if i2 < self.colors.len() { self.colors[i2] } else { [1.0, 1.0, 1.0, 1.0] });

                    new_idx.push(base);
                    new_idx.push(base + 1);
                    new_idx.push(base + 2);
                }
            }
            self.positions = new_pos;
            self.normals = new_norm;
            self.uvs = new_uv;
            self.colors = new_col;
            self.indices = new_idx;
        } else {
            let mut accum = vec![Vec3::ZERO; self.positions.len()];
            for chunk in self.indices.chunks_exact(3) {
                let i0 = chunk[0] as usize;
                let i1 = chunk[1] as usize;
                let i2 = chunk[2] as usize;
                if i0 < self.positions.len() && i1 < self.positions.len() && i2 < self.positions.len() {
                    let p0 = Vec3::from(self.positions[i0]);
                    let p1 = Vec3::from(self.positions[i1]);
                    let p2 = Vec3::from(self.positions[i2]);
                    let n = (p1 - p0).cross(p2 - p0);
                    accum[i0] += n;
                    accum[i1] += n;
                    accum[i2] += n;
                }
            }
            for i in 0..self.positions.len() {
                let n = accum[i].normalize_or_zero();
                self.normals[i] = [n.x, n.y, n.z];
            }
        }
    }

    pub fn normalize_pivot_bottom(&mut self) {
        if self.positions.is_empty() {
            return;
        }

        let mut min_y = f32::INFINITY;
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for p in &self.positions {
            if p[1] < min_y { min_y = p[1]; }
            if p[0] < min_x { min_x = p[0]; }
            if p[0] > max_x { max_x = p[0]; }
            if p[2] < min_z { min_z = p[2]; }
            if p[2] > max_z { max_z = p[2]; }
        }

        let cx = (min_x + max_x) * 0.5;
        let cz = (min_z + max_z) * 0.5;

        for p in &mut self.positions {
            p[0] -= cx;
            p[1] -= min_y;
            p[2] -= cz;
        }
    }

    pub fn apply_taper(&mut self, factor: f32, axis: &str) {
        if self.positions.is_empty() { return; }
        let ax = axis.to_lowercase();
        let (min_v, max_v) = match ax.as_str() {
            "x" => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[0]), mx.max(p[0]))),
            "z" => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[2]), mx.max(p[2]))),
            _ => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[1]), mx.max(p[1]))),
        };
        let range = (max_v - min_v).max(1e-4);

        for p in &mut self.positions {
            let t = match ax.as_str() {
                "x" => (p[0] - min_v) / range,
                "z" => (p[2] - min_v) / range,
                _ => (p[1] - min_v) / range,
            };
            let s = (1.0 + factor * t).max(0.01);
            match ax.as_str() {
                "x" => { p[1] *= s; p[2] *= s; }
                "z" => { p[0] *= s; p[1] *= s; }
                _ => { p[0] *= s; p[2] *= s; }
            }
        }
    }

    pub fn apply_twist(&mut self, angle_deg: f32, axis: &str) {
        if self.positions.is_empty() { return; }
        let ax = axis.to_lowercase();
        let (min_v, max_v) = match ax.as_str() {
            "x" => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[0]), mx.max(p[0]))),
            "z" => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[2]), mx.max(p[2]))),
            _ => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[1]), mx.max(p[1]))),
        };
        let range = (max_v - min_v).max(1e-4);
        let max_rad = angle_deg.to_radians();

        for p in &mut self.positions {
            let t = match ax.as_str() {
                "x" => (p[0] - min_v) / range,
                "z" => (p[2] - min_v) / range,
                _ => (p[1] - min_v) / range,
            };
            let angle = max_rad * t;
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            match ax.as_str() {
                "x" => {
                    let y = p[1]; let z = p[2];
                    p[1] = y * cos_a - z * sin_a;
                    p[2] = y * sin_a + z * cos_a;
                }
                "z" => {
                    let x = p[0]; let y = p[1];
                    p[0] = x * cos_a - y * sin_a;
                    p[1] = x * sin_a + y * cos_a;
                }
                _ => {
                    let x = p[0]; let z = p[2];
                    p[0] = x * cos_a - z * sin_a;
                    p[2] = x * sin_a + z * cos_a;
                }
            }
        }
    }

    pub fn apply_bend(&mut self, angle_deg: f32, axis: &str) {
        if self.positions.is_empty() || angle_deg.abs() < 1e-4 { return; }
        let ax = axis.to_lowercase();
        let (min_v, max_v) = match ax.as_str() {
            "x" => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[0]), mx.max(p[0]))),
            _ => self.positions.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), p| (mn.min(p[1]), mx.max(p[1]))),
        };
        let range = (max_v - min_v).max(1e-4);
        let total_angle = angle_deg.to_radians();
        let radius = range / total_angle;

        for p in &mut self.positions {
            let t = (p[1] - min_v) / range;
            let theta = t * total_angle;
            let x = p[0];
            p[0] = x + (radius - radius * theta.cos());
            p[1] = min_v + radius * theta.sin();
        }
    }

    pub fn apply_noise_displace(&mut self, freq: f32, amp: f32, seed: u32) {
        let f = freq.max(0.01);
        let s = seed as f32 * 1.337;

        for i in 0..self.positions.len() {
            let p = self.positions[i];
            let n = self.normals[i];

            let noise = ((p[0] * f + s).sin() * 2.37
                + (p[1] * f + s * 1.71).cos() * 3.14
                + (p[2] * f + s * 2.11).sin() * 1.89)
                * 0.333;

            let disp = noise * amp;
            self.positions[i][0] += n[0] * disp;
            self.positions[i][1] += n[1] * disp;
            self.positions[i][2] += n[2] * disp;
        }
    }
}

// ----------------------------------------------------------------------------
// Primitives
// ----------------------------------------------------------------------------

pub fn create_box(half_x: f32, half_y: f32, half_z: f32, color: [f32; 4]) -> ForgeMesh {
    let mut mesh = ForgeMesh::with_capacity(24, 36);

    let faces = [
        // Front (+Z)
        ([0.0, 0.0, 1.0], [
            [-half_x, -half_y,  half_z],
            [ half_x, -half_y,  half_z],
            [ half_x,  half_y,  half_z],
            [-half_x,  half_y,  half_z],
        ]),
        // Back (-Z)
        ([0.0, 0.0, -1.0], [
            [ half_x, -half_y, -half_z],
            [-half_x, -half_y, -half_z],
            [-half_x,  half_y, -half_z],
            [ half_x,  half_y, -half_z],
        ]),
        // Top (+Y)
        ([0.0, 1.0, 0.0], [
            [-half_x,  half_y,  half_z],
            [ half_x,  half_y,  half_z],
            [ half_x,  half_y, -half_z],
            [-half_x,  half_y, -half_z],
        ]),
        // Bottom (-Y)
        ([0.0, -1.0, 0.0], [
            [-half_x, -half_y, -half_z],
            [ half_x, -half_y, -half_z],
            [ half_x, -half_y,  half_z],
            [-half_x, -half_y,  half_z],
        ]),
        // Right (+X)
        ([1.0, 0.0, 0.0], [
            [ half_x, -half_y,  half_z],
            [ half_x, -half_y, -half_z],
            [ half_x,  half_y, -half_z],
            [ half_x,  half_y,  half_z],
        ]),
        // Left (-X)
        ([-1.0, 0.0, 0.0], [
            [-half_x, -half_y, -half_z],
            [-half_x, -half_y,  half_z],
            [-half_x,  half_y,  half_z],
            [-half_x,  half_y, -half_z],
        ]),
    ];

    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    for (normal, quad) in &faces {
        let base = mesh.positions.len() as u32;
        for i in 0..4 {
            mesh.positions.push(quad[i]);
            mesh.normals.push(*normal);
            mesh.uvs.push(uvs[i]);
            mesh.colors.push(color);
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    mesh
}

pub fn create_sphere(radius: f32, segments: usize, rings: usize, color: [f32; 4]) -> ForgeMesh {
    let segs = segments.max(6);
    let r_count = rings.max(4);
    let mut mesh = ForgeMesh::new();

    for i in 0..=r_count {
        let theta = (i as f32 / r_count as f32) * PI;
        let sin_t = theta.sin();
        let cos_t = theta.cos();

        for j in 0..=segs {
            let phi = (j as f32 / segs as f32) * 2.0 * PI;
            let sin_p = phi.sin();
            let cos_p = phi.cos();

            let x = cos_p * sin_t;
            let y = cos_t;
            let z = sin_p * sin_t;

            mesh.positions.push([x * radius, y * radius, z * radius]);
            mesh.normals.push([x, y, z]);
            mesh.uvs.push([j as f32 / segs as f32, i as f32 / r_count as f32]);
            mesh.colors.push(color);
        }
    }

    for i in 0..r_count {
        for j in 0..segs {
            let first = (i * (segs + 1) + j) as u32;
            let second = first + segs as u32 + 1;

            mesh.indices.push(first);
            mesh.indices.push(second);
            mesh.indices.push(first + 1);

            mesh.indices.push(second);
            mesh.indices.push(second + 1);
            mesh.indices.push(first + 1);
        }
    }

    mesh
}

pub fn create_cylinder(radius_top: f32, radius_bottom: f32, height: f32, segments: usize, color: [f32; 4]) -> ForgeMesh {
    let segs = segments.max(5);
    let mut mesh = ForgeMesh::new();
    let half_h = height * 0.5;

    // Side vertices
    for i in 0..=segs {
        let phi = (i as f32 / segs as f32) * 2.0 * PI;
        let cos_p = phi.cos();
        let sin_p = phi.sin();
        let u = i as f32 / segs as f32;

        let n = Vec3::new(cos_p, 0.0, sin_p).normalize_or_zero();

        // Top ring
        mesh.positions.push([cos_p * radius_top, half_h, sin_p * radius_top]);
        mesh.normals.push([n.x, 0.0, n.z]);
        mesh.uvs.push([u, 1.0]);
        mesh.colors.push(color);

        // Bottom ring
        mesh.positions.push([cos_p * radius_bottom, -half_h, sin_p * radius_bottom]);
        mesh.normals.push([n.x, 0.0, n.z]);
        mesh.uvs.push([u, 0.0]);
        mesh.colors.push(color);
    }

    for i in 0..segs as u32 {
        let i0 = i * 2;
        let i1 = i0 + 1;
        let i2 = i0 + 2;
        let i3 = i0 + 3;

        mesh.indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
    }

    // Top cap
    if radius_top > 1e-4 {
        let top_center_idx = mesh.positions.len() as u32;
        mesh.positions.push([0.0, half_h, 0.0]);
        mesh.normals.push([0.0, 1.0, 0.0]);
        mesh.uvs.push([0.5, 0.5]);
        mesh.colors.push(color);

        let cap_start = mesh.positions.len() as u32;
        for i in 0..=segs {
            let phi = (i as f32 / segs as f32) * 2.0 * PI;
            mesh.positions.push([phi.cos() * radius_top, half_h, phi.sin() * radius_top]);
            mesh.normals.push([0.0, 1.0, 0.0]);
            mesh.uvs.push([(phi.cos() + 1.0) * 0.5, (phi.sin() + 1.0) * 0.5]);
            mesh.colors.push(color);
        }
        for i in 0..segs as u32 {
            mesh.indices.extend_from_slice(&[top_center_idx, cap_start + i + 1, cap_start + i]);
        }
    }

    // Bottom cap
    if radius_bottom > 1e-4 {
        let bot_center_idx = mesh.positions.len() as u32;
        mesh.positions.push([0.0, -half_h, 0.0]);
        mesh.normals.push([0.0, -1.0, 0.0]);
        mesh.uvs.push([0.5, 0.5]);
        mesh.colors.push(color);

        let cap_start = mesh.positions.len() as u32;
        for i in 0..=segs {
            let phi = (i as f32 / segs as f32) * 2.0 * PI;
            mesh.positions.push([phi.cos() * radius_bottom, -half_h, phi.sin() * radius_bottom]);
            mesh.normals.push([0.0, -1.0, 0.0]);
            mesh.uvs.push([(phi.cos() + 1.0) * 0.5, (phi.sin() + 1.0) * 0.5]);
            mesh.colors.push(color);
        }
        for i in 0..segs as u32 {
            mesh.indices.extend_from_slice(&[bot_center_idx, cap_start + i, cap_start + i + 1]);
        }
    }

    mesh
}

pub fn create_cone(radius: f32, height: f32, segments: usize, color: [f32; 4]) -> ForgeMesh {
    create_cylinder(0.001, radius, height, segments, color)
}

pub fn create_torus(major_radius: f32, minor_radius: f32, segments_r: usize, segments_t: usize, color: [f32; 4]) -> ForgeMesh {
    let seg_r = segments_r.max(6);
    let seg_t = segments_t.max(4);
    let mut mesh = ForgeMesh::new();

    for i in 0..=seg_r {
        let u = (i as f32 / seg_r as f32) * 2.0 * PI;
        let cos_u = u.cos();
        let sin_u = u.sin();

        for j in 0..=seg_t {
            let v = (j as f32 / seg_t as f32) * 2.0 * PI;
            let cos_v = v.cos();
            let sin_v = v.sin();

            let x = (major_radius + minor_radius * cos_v) * cos_u;
            let y = minor_radius * sin_v;
            let z = (major_radius + minor_radius * cos_v) * sin_u;

            let nx = cos_v * cos_u;
            let ny = sin_v;
            let nz = cos_v * sin_u;

            mesh.positions.push([x, y, z]);
            mesh.normals.push([nx, ny, nz]);
            mesh.uvs.push([i as f32 / seg_r as f32, j as f32 / seg_t as f32]);
            mesh.colors.push(color);
        }
    }

    for i in 0..seg_r {
        for j in 0..seg_t {
            let first = (i * (seg_t + 1) + j) as u32;
            let second = first + seg_t as u32 + 1;

            mesh.indices.extend_from_slice(&[first, second, first + 1, second, second + 1, first + 1]);
        }
    }

    mesh
}

pub fn create_capsule(radius: f32, height: f32, segments: usize, rings: usize, color: [f32; 4]) -> ForgeMesh {
    let mut mesh = create_cylinder(radius, radius, height, segments, color);
    let half_h = height * 0.5;

    let mut top_hemi = create_sphere(radius, segments, rings, color);
    for p in &mut top_hemi.positions {
        if p[1] < 0.0 { p[1] = 0.0; }
        p[1] += half_h;
    }
    mesh.merge(&top_hemi);

    let mut bot_hemi = create_sphere(radius, segments, rings, color);
    for p in &mut bot_hemi.positions {
        if p[1] > 0.0 { p[1] = 0.0; }
        p[1] -= half_h;
    }
    mesh.merge(&bot_hemi);

    mesh
}

pub fn create_prism(sides: usize, radius: f32, height: f32, color: [f32; 4]) -> ForgeMesh {
    create_cylinder(radius, radius, height, sides.max(3), color)
}

pub fn create_lathe(points: &[[f32; 2]], segments: usize, color: [f32; 4]) -> ForgeMesh {
    if points.len() < 2 {
        return ForgeMesh::default();
    }
    let segs = segments.max(6);
    let mut mesh = ForgeMesh::new();

    for j in 0..points.len() {
        let r = points[j][0].max(0.0);
        let y = points[j][1];
        let v = j as f32 / (points.len() - 1) as f32;

        for i in 0..=segs {
            let phi = (i as f32 / segs as f32) * 2.0 * PI;
            let cos_p = phi.cos();
            let sin_p = phi.sin();
            let u = i as f32 / segs as f32;

            let x = cos_p * r;
            let z = sin_p * r;

            mesh.positions.push([x, y, z]);
            mesh.normals.push([cos_p, 0.0, sin_p]);
            mesh.uvs.push([u, v]);
            mesh.colors.push(color);
        }
    }

    let reverse_winding = points[0][1] > points[points.len() - 1][1];
    for j in 0..points.len() - 1 {
        for i in 0..segs {
            let row1 = (j * (segs + 1) + i) as u32;
            let row2 = ((j + 1) * (segs + 1) + i) as u32;

            if reverse_winding {
                mesh.indices.extend_from_slice(&[row1, row1 + 1, row2, row2, row1 + 1, row2 + 1]);
            } else {
                mesh.indices.extend_from_slice(&[row1, row2, row1 + 1, row2, row2 + 1, row1 + 1]);
            }
        }
    }

    mesh.recompute_normals(false);
    mesh
}

// ----------------------------------------------------------------------------
// Model Recipe and Part Definitions
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModifierDef {
    Taper { factor: f32, axis: String },
    Twist { angle_deg: f32, axis: String },
    Bend { angle_deg: f32, axis: String },
    NoiseDisplace { frequency: f32, amplitude: f32, seed: u32 },
    FlatShading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartDef {
    pub name: String,
    pub primitive: String,
    #[serde(default)]
    pub points: Option<Vec<[f32; 2]>>,
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation_deg: [f32; 3],
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
    #[serde(default = "default_part_color")]
    pub color: [f32; 4],
    #[serde(default)]
    pub mirror_x: bool,
    #[serde(default)]
    pub mirror_z: bool,
    #[serde(default)]
    pub modifiers: Vec<ModifierDef>,
}

fn default_scale() -> [f32; 3] { [1.0, 1.0, 1.0] }
fn default_part_color() -> [f32; 4] { [0.8, 0.8, 0.8, 1.0] }

impl PartDef {
    pub fn build_mesh(&self) -> ForgeMesh {
        let prim = self.primitive.to_lowercase();
        let mut mesh = match prim.as_str() {
            "sphere" => create_sphere(0.5, 16, 12, self.color),
            "cylinder" => create_cylinder(0.5, 0.5, 1.0, 16, self.color),
            "cone" => create_cone(0.5, 1.0, 16, self.color),
            "torus" => create_torus(0.5, 0.15, 16, 10, self.color),
            "capsule" => create_capsule(0.3, 0.8, 14, 8, self.color),
            "prism" => create_prism(6, 0.5, 1.0, self.color),
            "lathe" => {
                if let Some(pts) = &self.points {
                    create_lathe(pts, 16, self.color)
                } else {
                    let default_lathe = vec![[0.0, 0.0], [0.3, 0.2], [0.5, 0.6], [0.2, 1.0], [0.0, 1.0]];
                    create_lathe(&default_lathe, 16, self.color)
                }
            }
            _ => create_box(0.5, 0.5, 0.5, self.color),
        };

        for m in &self.modifiers {
            apply_modifier(&mut mesh, m);
        }

        mesh.transform(self.position, self.rotation_deg, self.scale);
        mesh
    }
}

pub fn apply_modifier(mesh: &mut ForgeMesh, modifier: &ModifierDef) {
    match modifier {
        ModifierDef::Taper { factor, axis } => mesh.apply_taper(*factor, axis),
        ModifierDef::Twist { angle_deg, axis } => mesh.apply_twist(*angle_deg, axis),
        ModifierDef::Bend { angle_deg, axis } => mesh.apply_bend(*angle_deg, axis),
        ModifierDef::NoiseDisplace { frequency, amplitude, seed } => mesh.apply_noise_displace(*frequency, *amplitude, *seed),
        ModifierDef::FlatShading => mesh.recompute_normals(true),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecipe {
    pub name: String,
    #[serde(default)]
    pub parts: Vec<PartDef>,
    #[serde(default)]
    pub sdf_nodes: Vec<crate::sdf::SdfNodeDef>,
    #[serde(default)]
    pub global_modifiers: Vec<ModifierDef>,
    #[serde(default)]
    pub normalize_pivot: bool,
}

impl Default for ModelRecipe {
    fn default() -> Self {
        Self {
            name: "CustomModel".to_string(),
            parts: Vec::new(),
            sdf_nodes: Vec::new(),
            global_modifiers: Vec::new(),
            normalize_pivot: true,
        }
    }
}

impl ModelRecipe {
    pub fn build_mesh(&self) -> ForgeMesh {
        let mut combined = ForgeMesh::new();

        // 1. Build primitive parts
        for part in &self.parts {
            let m = part.build_mesh();
            combined.merge(&m);

            if part.mirror_x {
                let mut mx_part = part.clone();
                mx_part.position[0] = -mx_part.position[0];
                mx_part.rotation_deg[1] = -mx_part.rotation_deg[1];
                mx_part.rotation_deg[2] = -mx_part.rotation_deg[2];
                let mx_mesh = mx_part.build_mesh();
                combined.merge(&mx_mesh);
            }

            if part.mirror_z {
                let mut mz_part = part.clone();
                mz_part.position[2] = -mz_part.position[2];
                mz_part.rotation_deg[0] = -mz_part.rotation_deg[0];
                let mz_mesh = mz_part.build_mesh();
                combined.merge(&mz_mesh);
            }
        }

        // 2. Build SDF mesh if any
        if !self.sdf_nodes.is_empty() {
            let sdf_mesh = crate::sdf::sdf_to_mesh(&self.sdf_nodes, 28, 2.5);
            combined.merge(&sdf_mesh);
        }

        // 3. Global modifiers
        for m in &self.global_modifiers {
            apply_modifier(&mut combined, m);
        }

        // 4. Normalize pivot
        if self.normalize_pivot {
            combined.normalize_pivot_bottom();
        }

        combined
    }
}

// ----------------------------------------------------------------------------
// Presets
// ----------------------------------------------------------------------------

pub fn preset_spaceship() -> ModelRecipe {
    ModelRecipe {
        name: "spaceship".to_string(),
        parts: vec![
            // Fuselage
            PartDef {
                name: "fuselage".to_string(),
                primitive: "cone".to_string(),
                points: None,
                position: [0.0, 0.25, 0.0],
                rotation_deg: [-90.0, 0.0, 0.0],
                scale: [1.2, 3.2, 0.8],
                color: [0.22, 0.35, 0.55, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![ModifierDef::Taper { factor: -0.6, axis: "y".to_string() }],
            },
            // Cockpit canopy
            PartDef {
                name: "cockpit".to_string(),
                primitive: "sphere".to_string(),
                points: None,
                position: [0.0, 0.5, 0.3],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.55, 0.4, 1.1],
                color: [0.15, 0.85, 1.0, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![],
            },
            // Swept Wings (mirrored across X)
            PartDef {
                name: "wing".to_string(),
                primitive: "prism".to_string(),
                points: None,
                position: [1.3, 0.1, -0.4],
                rotation_deg: [0.0, 20.0, 80.0],
                scale: [0.15, 2.2, 0.9],
                color: [0.18, 0.26, 0.42, 1.0],
                mirror_x: true,
                mirror_z: false,
                modifiers: vec![ModifierDef::Taper { factor: -0.5, axis: "y".to_string() }],
            },
            // Thrusters (mirrored)
            PartDef {
                name: "thruster".to_string(),
                primitive: "cylinder".to_string(),
                points: None,
                position: [0.45, 0.15, -1.5],
                rotation_deg: [90.0, 0.0, 0.0],
                scale: [0.35, 0.8, 0.35],
                color: [1.0, 0.45, 0.1, 1.0],
                mirror_x: true,
                mirror_z: false,
                modifiers: vec![],
            },
        ],
        sdf_nodes: Vec::new(),
        global_modifiers: vec![],
        normalize_pivot: true,
    }
}

pub fn preset_asteroid() -> ModelRecipe {
    ModelRecipe {
        name: "asteroid".to_string(),
        parts: vec![
            PartDef {
                name: "core".to_string(),
                primitive: "sphere".to_string(),
                points: None,
                position: [0.0, 1.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.8, 1.4, 1.9],
                color: [0.32, 0.30, 0.28, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![
                    ModifierDef::NoiseDisplace { frequency: 1.8, amplitude: 0.35, seed: 42 },
                    ModifierDef::FlatShading,
                ],
            },
        ],
        sdf_nodes: Vec::new(),
        global_modifiers: vec![],
        normalize_pivot: true,
    }
}

pub fn preset_turret() -> ModelRecipe {
    ModelRecipe {
        name: "scifi_turret".to_string(),
        parts: vec![
            // Base stand
            PartDef {
                name: "base".to_string(),
                primitive: "cylinder".to_string(),
                points: None,
                position: [0.0, 0.3, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.4, 0.6, 1.4],
                color: [0.2, 0.22, 0.26, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![ModifierDef::Taper { factor: -0.3, axis: "y".to_string() }],
            },
            // Swivel dome
            PartDef {
                name: "dome".to_string(),
                primitive: "sphere".to_string(),
                points: None,
                position: [0.0, 0.9, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.9, 0.75, 0.9],
                color: [0.35, 0.38, 0.45, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![],
            },
            // Twin Barrels (lathe profile, mirrored X)
            PartDef {
                name: "barrel".to_string(),
                primitive: "lathe".to_string(),
                points: Some(vec![
                    [0.0, 0.0],
                    [0.15, 0.1],
                    [0.12, 0.8],
                    [0.2, 1.2],
                    [0.14, 1.3],
                    [0.0, 1.3],
                ]),
                position: [0.28, 0.9, 0.5],
                rotation_deg: [90.0, 0.0, 0.0],
                scale: [0.6, 1.2, 0.6],
                color: [0.25, 0.85, 1.0, 1.0],
                mirror_x: true,
                mirror_z: false,
                modifiers: vec![],
            },
        ],
        sdf_nodes: Vec::new(),
        global_modifiers: vec![],
        normalize_pivot: true,
    }
}

pub fn preset_boss() -> ModelRecipe {
    ModelRecipe {
        name: "apophis_boss".to_string(),
        parts: vec![
            // Spiky mandible / armor plates
            PartDef {
                name: "mandible_top".to_string(),
                primitive: "cone".to_string(),
                points: None,
                position: [0.8, 1.8, 0.8],
                rotation_deg: [35.0, 45.0, -15.0],
                scale: [0.45, 2.2, 0.45],
                color: [0.95, 0.15, 0.1, 1.0],
                mirror_x: true,
                mirror_z: true,
                modifiers: vec![ModifierDef::Twist { angle_deg: 25.0, axis: "y".to_string() }],
            },
            // Outer containment ring
            PartDef {
                name: "containment_ring".to_string(),
                primitive: "torus".to_string(),
                points: None,
                position: [0.0, 1.2, 0.0],
                rotation_deg: [15.0, 0.0, 0.0],
                scale: [2.2, 0.25, 2.2],
                color: [0.9, 0.4, 0.05, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![],
            },
        ],
        // Organic menacing central core using SDF smooth union
        sdf_nodes: vec![
            crate::sdf::SdfNodeDef {
                name: "core_sphere".to_string(),
                primitive: crate::sdf::SdfPrimitive::Sphere { radius: 1.1 },
                op: crate::sdf::SdfOp::Union,
                position: [0.0, 1.2, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                color: [0.12, 0.08, 0.15, 1.0], // Dark cosmic obsidian
            },
            crate::sdf::SdfNodeDef {
                name: "glowing_eye".to_string(),
                primitive: crate::sdf::SdfPrimitive::Sphere { radius: 0.55 },
                op: crate::sdf::SdfOp::SmoothUnion { k: 0.3 },
                position: [0.0, 1.3, 0.8],
                rotation_deg: [0.0, 0.0, 0.0],
                color: [1.0, 0.18, 0.15, 1.0], // Crimson incandescent eye
            },
        ],
        global_modifiers: vec![],
        normalize_pivot: true,
    }
}

pub fn preset_dyson_ring() -> ModelRecipe {
    ModelRecipe {
        name: "dyson_ring".to_string(),
        parts: vec![
            PartDef {
                name: "main_ring".to_string(),
                primitive: "torus".to_string(),
                points: None,
                position: [0.0, 0.5, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [4.0, 0.5, 4.0],
                color: [0.1, 0.85, 0.65, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![],
            },
            PartDef {
                name: "inner_ring".to_string(),
                primitive: "torus".to_string(),
                points: None,
                position: [0.0, 0.5, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [3.2, 0.2, 3.2],
                color: [0.3, 0.95, 0.85, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![],
            },
            PartDef {
                name: "pylon_n".to_string(),
                primitive: "lathe".to_string(),
                points: Some(vec![
                    [0.0, 0.0],
                    [0.4, 0.1],
                    [0.25, 0.8],
                    [0.5, 1.4],
                    [0.1, 1.5],
                    [0.0, 1.5],
                ]),
                position: [0.0, 0.0, 3.8],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [0.8, 1.0, 0.8],
                color: [0.15, 0.22, 0.3, 1.0],
                mirror_x: false,
                mirror_z: true,
                modifiers: vec![],
            },
            PartDef {
                name: "pylon_e".to_string(),
                primitive: "lathe".to_string(),
                points: Some(vec![
                    [0.0, 0.0],
                    [0.4, 0.1],
                    [0.25, 0.8],
                    [0.5, 1.4],
                    [0.1, 1.5],
                    [0.0, 1.5],
                ]),
                position: [3.8, 0.0, 0.0],
                rotation_deg: [0.0, 90.0, 0.0],
                scale: [0.8, 1.0, 0.8],
                color: [0.15, 0.22, 0.3, 1.0],
                mirror_x: true,
                mirror_z: false,
                modifiers: vec![],
            },
        ],
        sdf_nodes: Vec::new(),
        global_modifiers: vec![],
        normalize_pivot: true,
    }
}

pub fn preset_comet_shard() -> ModelRecipe {
    ModelRecipe {
        name: "comet_shard".to_string(),
        parts: vec![
            PartDef {
                name: "crystal_body".to_string(),
                primitive: "prism".to_string(),
                points: None,
                position: [0.0, 1.0, 0.0],
                rotation_deg: [15.0, 20.0, 35.0],
                scale: [0.8, 2.2, 0.8],
                color: [0.2, 0.8, 1.0, 1.0],
                mirror_x: false,
                mirror_z: false,
                modifiers: vec![
                    ModifierDef::Taper { factor: 0.7, axis: "y".to_string() },
                    ModifierDef::Twist { angle_deg: 45.0, axis: "y".to_string() },
                    ModifierDef::NoiseDisplace { frequency: 2.2, amplitude: 0.15, seed: 77 },
                    ModifierDef::FlatShading,
                ],
            },
        ],
        sdf_nodes: Vec::new(),
        global_modifiers: vec![ModifierDef::FlatShading],
        normalize_pivot: true,
    }
}

pub fn bake_all_cosmos_assets() -> Result<(), String> {
    let models = [
        preset_boss(),
        preset_spaceship(),
        preset_dyson_ring(),
        preset_comet_shard(),
    ];
    let dir = Path::new("assets/models");
    let _ = std::fs::create_dir_all(dir);

    for m in &models {
        let mesh = m.build_mesh();
        let path = dir.join(format!("{}.glb", m.name));
        crate::glb_exporter::export_glb_to_file(&mesh, &m.name, &path)
            .map_err(|e| format!("Failed to export {}: {}", m.name, e))?;
    }

    // Also bake textures
    let _ = crate::texture_forge::save_procedural_texture("planet", "planet_blue", 256, 101, [0.15, 0.45, 0.95, 1.0]);
    let _ = crate::texture_forge::save_procedural_texture("planet", "planet_lava", 256, 202, [0.95, 0.25, 0.1, 1.0]);
    let _ = crate::texture_forge::save_procedural_texture("planet", "planet_gas", 256, 303, [0.8, 0.3, 0.85, 1.0]);
    let _ = crate::texture_forge::save_procedural_texture("grid_neon", "arena_grid", 512, 404, [0.1, 0.85, 0.95, 1.0]);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitives_generation() {
        let b = create_box(1.0, 1.0, 1.0, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(b.positions.len(), 24);
        assert_eq!(b.indices.len(), 36);

        let s = create_sphere(1.0, 12, 8, [0.0, 1.0, 0.0, 1.0]);
        assert!(!s.positions.is_empty());
        assert!(!s.indices.is_empty());

        let cyl = create_cylinder(0.5, 0.5, 1.0, 8, [0.0, 0.0, 1.0, 1.0]);
        assert!(!cyl.positions.is_empty());

        let lathe_pts = vec![[0.0, 0.0], [0.5, 0.5], [0.0, 1.0]];
        let lathe_mesh = create_lathe(&lathe_pts, 8, [1.0, 1.0, 0.0, 1.0]);
        assert!(!lathe_mesh.positions.is_empty());
    }

    #[test]
    fn test_modifiers_and_merge() {
        let mut m1 = create_box(1.0, 1.0, 1.0, [1.0, 1.0, 1.0, 1.0]);
        m1.apply_taper(-0.5, "y");
        m1.apply_twist(45.0, "y");
        m1.recompute_normals(true);

        let m2 = create_sphere(0.5, 8, 6, [0.5, 0.5, 0.5, 1.0]);
        m1.merge(&m2);
        assert!(m1.positions.len() > 24);
        assert!(m1.indices.len() > 36);

        m1.normalize_pivot_bottom();
        let min_y = m1.positions.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        assert!(min_y.abs() < 1e-4, "Bottom pivot should be 0.0");
    }

    #[test]
    fn test_recipe_build_and_presets() {
        let ship = preset_spaceship();
        let ship_mesh = ship.build_mesh();
        assert!(!ship_mesh.positions.is_empty());

        let asteroid = preset_asteroid();
        let asteroid_mesh = asteroid.build_mesh();
        assert!(!asteroid_mesh.positions.is_empty());

        let boss = preset_boss();
        let boss_mesh = boss.build_mesh();
        assert!(!boss_mesh.positions.is_empty());
    }

    #[test]
    fn test_bake_all_cosmos_assets() {
        let res = bake_all_cosmos_assets();
        assert!(res.is_ok(), "Failed to bake cosmos assets: {:?}", res.err());
        assert!(Path::new("assets/models/apophis_boss.glb").exists());
        assert!(Path::new("assets/models/spaceship.glb").exists());
        assert!(Path::new("assets/models/dyson_ring.glb").exists());
        assert!(Path::new("assets/models/comet_shard.glb").exists());
        assert!(Path::new("assets/textures/arena_grid.png").exists());
        assert!(Path::new("assets/textures/planet_blue.png").exists());
    }

    #[test]
    fn test_cylinder_and_lathe_outward_winding() {
        let cyl = create_cylinder(0.5, 0.5, 2.0, 12, [1.0, 1.0, 1.0, 1.0]);
        for tri in cyl.indices.chunks_exact(3) {
            let p0 = Vec3::from(cyl.positions[tri[0] as usize]);
            let p1 = Vec3::from(cyl.positions[tri[1] as usize]);
            let p2 = Vec3::from(cyl.positions[tri[2] as usize]);
            let face_n = (p1 - p0).cross(p2 - p0);
            let centroid = (p0 + p1 + p2) / 3.0;
            // For a convex cylinder centered at origin, face normal must point outward from origin
            assert!(face_n.dot(centroid) > 0.0, "Inward triangle on cylinder: {:?} dot {:?}", face_n, centroid);
        }

        for pts in [
            vec![[0.5, -1.0], [0.8, 0.0], [0.5, 1.0]],
            vec![[0.5, 1.0], [0.8, 0.0], [0.5, -1.0]],
        ] {
            let lathe = create_lathe(&pts, 12, [1.0, 1.0, 1.0, 1.0]);
            for tri in lathe.indices.chunks_exact(3) {
                let p0 = Vec3::from(lathe.positions[tri[0] as usize]);
                let p1 = Vec3::from(lathe.positions[tri[1] as usize]);
                let p2 = Vec3::from(lathe.positions[tri[2] as usize]);
                let face_n = (p1 - p0).cross(p2 - p0);
                let centroid_xz = Vec3::new((p0.x + p1.x + p2.x) / 3.0, 0.0, (p0.z + p1.z + p2.z) / 3.0);
                assert!(face_n.dot(centroid_xz) > 0.0, "Inward triangle on lathe: {:?} dot {:?}", face_n, centroid_xz);
            }
        }
    }
}
