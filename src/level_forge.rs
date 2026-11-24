use std::f32::consts::PI;
use std::path::Path;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use crate::modeler::ForgeMesh;
use crate::texture_forge::fbm_2d;

// ============================================================================
// 1. TERRAIN FORGE — Heightmap 3D Terrain Generator
// ============================================================================

fn default_terrain_width() -> f32 { 100.0 }
fn default_terrain_depth() -> f32 { 100.0 }
fn default_terrain_res() -> usize { 64 }
fn default_terrain_max_h() -> f32 { 16.0 }
fn default_terrain_scale() -> f32 { 0.035 }
fn default_terrain_octaves() -> u32 { 4 }
fn default_terrain_seed() -> u32 { 1337 }
fn default_true() -> bool { true }
fn default_palette() -> String { "alpine".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainRecipe {
    pub name: String,
    #[serde(default = "default_terrain_width")]
    pub width: f32,
    #[serde(default = "default_terrain_depth")]
    pub depth: f32,
    #[serde(default = "default_terrain_res")]
    pub resolution: usize, // e.g. 32 to 96
    #[serde(default = "default_terrain_max_h")]
    pub max_height: f32,
    #[serde(default = "default_terrain_scale")]
    pub noise_scale: f32,
    #[serde(default = "default_terrain_octaves")]
    pub octaves: u32,
    #[serde(default = "default_terrain_seed")]
    pub seed: u32,
    #[serde(default = "default_true")]
    pub island_falloff: bool,
    #[serde(default)]
    pub flat_shading: bool,
    #[serde(default = "default_palette")]
    pub palette: String, // "alpine" | "desert" | "alien" | "volcanic"
}

impl Default for TerrainRecipe {
    fn default() -> Self {
        Self {
            name: "terrain_default".to_string(),
            width: default_terrain_width(),
            depth: default_terrain_depth(),
            resolution: default_terrain_res(),
            max_height: default_terrain_max_h(),
            noise_scale: default_terrain_scale(),
            octaves: default_terrain_octaves(),
            seed: default_terrain_seed(),
            island_falloff: true,
            flat_shading: false,
            palette: default_palette(),
        }
    }
}

/// Evaluates terrain height deterministically at any (x, z) world coordinates
pub fn sample_terrain_height(recipe: &TerrainRecipe, x: f32, z: f32) -> f32 {
    let half_w = recipe.width * 0.5;
    let half_d = recipe.depth * 0.5;

    let nx = x / half_w.max(0.1);
    let nz = z / half_d.max(0.1);
    let r_sq = nx * nx + nz * nz;

    if recipe.island_falloff && r_sq >= 1.0 {
        return 0.0;
    }

    let falloff = if recipe.island_falloff {
        let f = (1.0 - r_sq).clamp(0.0, 1.0);
        // Smoothstep falloff
        f * f * (3.0 - 2.0 * f)
    } else {
        1.0
    };

    let n = fbm_2d(
        x * recipe.noise_scale,
        z * recipe.noise_scale,
        recipe.octaves as usize,
        0.5,
        2.0,
        recipe.seed,
    );

    // Bias to have natural valleys and peaks
    let h_normalized = (n * 0.5 + 0.5).clamp(0.0, 1.0);
    h_normalized * recipe.max_height * falloff
}

/// Computes surface normal at (x, z) using central differences
pub fn sample_terrain_normal(recipe: &TerrainRecipe, x: f32, z: f32) -> [f32; 3] {
    let delta = 0.5;
    let h_l = sample_terrain_height(recipe, x - delta, z);
    let h_r = sample_terrain_height(recipe, x + delta, z);
    let h_d = sample_terrain_height(recipe, x, z - delta);
    let h_u = sample_terrain_height(recipe, x, z + delta);

    let dx = (h_r - h_l) / (2.0 * delta);
    let dz = (h_u - h_d) / (2.0 * delta);

    let len = (dx * dx + 1.0 + dz * dz).sqrt();
    [-dx / len, 1.0 / len, -dz / len]
}

/// Computes vertex color based on height ratio and surface slope
pub fn evaluate_terrain_color(palette: &str, height_ratio: f32, slope: f32) -> [f32; 4] {
    let h = height_ratio.clamp(0.0, 1.0);
    let s = slope.clamp(0.0, 1.0); // 0 = flat horizontal, 1 = vertical wall

    match palette.trim().to_lowercase().as_str() {
        "desert" => {
            // Golden sand -> red canyon rock
            if s > 0.45 {
                [0.65, 0.38, 0.22, 1.0] // Canyon wall
            } else if h < 0.35 {
                [0.85, 0.74, 0.48, 1.0] // Low sand
            } else if h < 0.70 {
                [0.78, 0.60, 0.36, 1.0] // Dune mid
            } else {
                [0.72, 0.45, 0.28, 1.0] // High ridge
            }
        }
        "alien" => {
            // Bioluminescent purple/cyan
            if s > 0.40 {
                [0.15, 0.85, 0.90, 1.0] // Glowing cyan cliffs
            } else if h < 0.25 {
                [0.22, 0.10, 0.35, 1.0] // Dark purple abyss
            } else if h < 0.65 {
                [0.55, 0.20, 0.75, 1.0] // Magenta plains
            } else {
                [0.88, 0.28, 0.95, 1.0] // Neon crest
            }
        }
        "volcanic" => {
            // Dark basalt, ash crevices and glowing magma
            if h < 0.18 && s < 0.40 {
                [0.98, 0.36, 0.05, 1.0] // Magma pool / crevice
            } else if s > 0.48 {
                [0.18, 0.16, 0.19, 1.0] // Steep ash cliff
            } else if h > 0.70 {
                [0.14, 0.13, 0.15, 1.0] // Dark basalt peak
            } else {
                [0.09, 0.09, 0.11, 1.0] // Deep basalt ground
            }
        }
        _ => {
            // "alpine": Water/beach -> grass -> mountain rock -> snow
            if s > 0.45 {
                [0.46, 0.48, 0.52, 1.0] // Rock slope
            } else if h < 0.12 {
                [0.76, 0.70, 0.50, 1.0] // Beach sand
            } else if h < 0.55 {
                [0.28, 0.65, 0.24, 1.0] // Lush green grass
            } else if h < 0.75 {
                [0.48, 0.50, 0.54, 1.0] // Mountain rock
            } else {
                [0.92, 0.94, 0.98, 1.0] // Snow peak
            }
        }
    }
}

impl TerrainRecipe {
    /// Generates a complete 3D ForgeMesh with vertex colors and normals
    pub fn build_mesh(&self) -> ForgeMesh {
        let res = self.resolution.clamp(8, 128);
        let w = self.width;
        let d = self.depth;
        let dx = w / res as f32;
        let dz = d / res as f32;

        let num_verts = (res + 1) * (res + 1);
        let mut heights = vec![0.0f32; num_verts];
        let mut normals = vec![[0.0f32, 1.0, 0.0]; num_verts];
        let mut colors = vec![[1.0f32, 1.0, 1.0, 1.0]; num_verts];

        // 1. Compute heights, normals and colors on grid
        for rz in 0..=res {
            let z = -d * 0.5 + rz as f32 * dz;
            for rx in 0..=res {
                let x = -w * 0.5 + rx as f32 * dx;
                let idx = rz * (res + 1) + rx;

                let h = sample_terrain_height(self, x, z);
                let norm = sample_terrain_normal(self, x, z);
                let slope = 1.0 - norm[1]; // slope factor
                let h_ratio = if self.max_height > 0.001 { h / self.max_height } else { 0.0 };
                let col = evaluate_terrain_color(&self.palette, h_ratio, slope);

                heights[idx] = h;
                normals[idx] = norm;
                colors[idx] = col;
            }
        }

        // 2. Build Mesh Geometry
        let mut mesh = ForgeMesh::new();

        if self.flat_shading {
            // Flat shading: unshared vertices per triangle
            for rz in 0..res {
                let z0 = -d * 0.5 + rz as f32 * dz;
                let z1 = z0 + dz;

                for rx in 0..res {
                    let x0 = -w * 0.5 + rx as f32 * dx;
                    let x1 = x0 + dx;

                    let idx00 = rz * (res + 1) + rx;
                    let idx10 = idx00 + 1;
                    let idx01 = (rz + 1) * (res + 1) + rx;
                    let idx11 = idx01 + 1;

                    let p00 = [x0, heights[idx00], z0];
                    let p10 = [x1, heights[idx10], z0];
                    let p01 = [x0, heights[idx01], z1];
                    let p11 = [x1, heights[idx11], z1];

                    // Tri 1: p00, p01, p10
                    let face_norm_1 = compute_face_normal(&p00, &p01, &p10);
                    let c00 = colors[idx00];
                    let base_idx_1 = mesh.positions.len() as u32;
                    mesh.positions.extend_from_slice(&[p00, p01, p10]);
                    mesh.normals.extend_from_slice(&[face_norm_1, face_norm_1, face_norm_1]);
                    mesh.uvs.extend_from_slice(&[[0.0, 0.0], [0.0, 1.0], [1.0, 0.0]]);
                    mesh.colors.extend_from_slice(&[c00, c00, c00]);
                    mesh.indices.extend_from_slice(&[base_idx_1, base_idx_1 + 1, base_idx_1 + 2]);

                    // Tri 2: p10, p01, p11
                    let face_norm_2 = compute_face_normal(&p10, &p01, &p11);
                    let c11 = colors[idx11];
                    let base_idx_2 = mesh.positions.len() as u32;
                    mesh.positions.extend_from_slice(&[p10, p01, p11]);
                    mesh.normals.extend_from_slice(&[face_norm_2, face_norm_2, face_norm_2]);
                    mesh.uvs.extend_from_slice(&[[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]);
                    mesh.colors.extend_from_slice(&[c11, c11, c11]);
                    mesh.indices.extend_from_slice(&[base_idx_2, base_idx_2 + 1, base_idx_2 + 2]);
                }
            }
        } else {
            // Smooth shading: shared indexed vertices
            for rz in 0..=res {
                let z = -d * 0.5 + rz as f32 * dz;
                let v = rz as f32 / res as f32;
                for rx in 0..=res {
                    let x = -w * 0.5 + rx as f32 * dx;
                    let u = rx as f32 / res as f32;
                    let idx = rz * (res + 1) + rx;

                    mesh.positions.push([x, heights[idx], z]);
                    mesh.normals.push(normals[idx]);
                    mesh.uvs.push([u, v]);
                    mesh.colors.push(colors[idx]);
                }
            }

            for rz in 0..res {
                for rx in 0..res {
                    let idx00 = (rz * (res + 1) + rx) as u32;
                    let idx10 = idx00 + 1;
                    let idx01 = ((rz + 1) * (res + 1) + rx) as u32;
                    let idx11 = idx01 + 1;

                    // CCW winding order
                    mesh.indices.push(idx00);
                    mesh.indices.push(idx01);
                    mesh.indices.push(idx10);

                    mesh.indices.push(idx10);
                    mesh.indices.push(idx01);
                    mesh.indices.push(idx11);
                }
            }
        }

        mesh
    }

    /// Converts this recipe into a regular sampled TerrainHeightmap
    pub fn to_heightmap(&self) -> TerrainHeightmap {
        let res = self.resolution.clamp(8, 128);
        let w = self.width;
        let d = self.depth;
        let dx = w / res as f32;
        let dz = d / res as f32;
        let num_verts = (res + 1) * (res + 1);
        let mut heights = Vec::with_capacity(num_verts);
        let mut min_h = f32::MAX;
        let mut max_h = f32::MIN;

        for rz in 0..=res {
            let z = -d * 0.5 + rz as f32 * dz;
            for rx in 0..=res {
                let x = -w * 0.5 + rx as f32 * dx;
                let h = sample_terrain_height(self, x, z);
                if h < min_h { min_h = h; }
                if h > max_h { max_h = h; }
                heights.push(h);
            }
        }

        if min_h == f32::MAX { min_h = 0.0; }
        if max_h == f32::MIN { max_h = 0.0; }

        TerrainHeightmap {
            width: w,
            depth: d,
            resolution: res as u32,
            heights,
            min_height: min_h,
            max_height: max_h,
            offset: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            rotation_y: 0.0,
        }
    }

    /// Exports the terrain mesh to binary glTF 2.0 (.glb) and saves a .heightmap.json sidecar
    pub fn bake_to_glb(&self, output_path: &str) -> Result<String, String> {
        let mesh = self.build_mesh();
        crate::glb_exporter::export_glb_to_file(&mesh, &self.name, output_path)
            .map_err(|e| e.to_string())?;

        // Automatically write sidecar .heightmap.json
        let hm = self.to_heightmap();
        let hm_path = heightmap_path_for_glb(output_path);
        let _ = hm.save_to_file(&hm_path);

        Ok(output_path.to_string())
    }
}

/// Helper returning the companion .heightmap.json path for any .glb model
pub fn heightmap_path_for_glb(glb_path: &str) -> String {
    let lower = glb_path.to_lowercase();
    if lower.ends_with(".glb") {
        let prefix = &glb_path[..glb_path.len() - 4];
        format!("{}.heightmap.json", prefix)
    } else {
        format!("{}.heightmap.json", glb_path)
    }
}

fn default_heightmap_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

/// Heightmap data structure providing persistent terrain height sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainHeightmap {
    pub width: f32,
    pub depth: f32,
    pub resolution: u32,
    pub heights: Vec<f32>,
    #[serde(default)]
    pub min_height: f32,
    #[serde(default)]
    pub max_height: f32,
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default = "default_heightmap_scale")]
    pub scale: [f32; 3],
    #[serde(default)]
    pub rotation_y: f32,
}

impl TerrainHeightmap {
    /// Bilinearly interpolates terrain height in local space (centered at origin)
    pub fn sample_local_height(&self, lx: f32, lz: f32) -> f32 {
        if self.width <= 0.0 || self.depth <= 0.0 || self.resolution == 0 || self.heights.is_empty() {
            return 0.0;
        }

        let half_w = self.width * 0.5;
        let half_d = self.depth * 0.5;

        // Check if outside horizontal terrain bounds
        if lx < -half_w || lx > half_w || lz < -half_d || lz > half_d {
            return 0.0;
        }

        let u = (lx + half_w) / self.width;
        let v = (lz + half_d) / self.depth;

        let res = self.resolution as f32;
        let gx = (u * res).clamp(0.0, res);
        let gz = (v * res).clamp(0.0, res);

        let x0 = (gx.floor() as usize).min(self.resolution as usize);
        let z0 = (gz.floor() as usize).min(self.resolution as usize);
        let x1 = (x0 + 1).min(self.resolution as usize);
        let z1 = (z0 + 1).min(self.resolution as usize);

        let fx = gx - (x0 as f32);
        let fz = gz - (z0 as f32);

        let row_stride = (self.resolution + 1) as usize;

        let idx00 = z0 * row_stride + x0;
        let idx10 = z0 * row_stride + x1;
        let idx01 = z1 * row_stride + x0;
        let idx11 = z1 * row_stride + x1;

        if idx11 >= self.heights.len() {
            return 0.0;
        }

        let h00 = self.heights[idx00];
        let h10 = self.heights[idx10];
        let h01 = self.heights[idx01];
        let h11 = self.heights[idx11];

        let h_top = h00 * (1.0 - fx) + h10 * fx;
        let h_bot = h01 * (1.0 - fx) + h11 * fx;

        h_top * (1.0 - fz) + h_bot * fz
    }

    /// Evaluates world height accounting for entity world position, scale, and yaw rotation
    pub fn sample_world_height(&self, wx: f32, wz: f32) -> f32 {
        let sx = if self.scale[0].abs() < 1e-5 { 1.0 } else { self.scale[0] };
        let sy = self.scale[1];
        let sz = if self.scale[2].abs() < 1e-5 { 1.0 } else { self.scale[2] };

        let mut lx = (wx - self.offset[0]) / sx;
        let mut lz = (wz - self.offset[2]) / sz;

        if self.rotation_y.abs() > 1e-4 {
            let rad = -self.rotation_y.to_radians();
            let cos_r = rad.cos();
            let sin_r = rad.sin();
            let rx = lx * cos_r - lz * sin_r;
            let rz = lx * sin_r + lz * cos_r;
            lx = rx;
            lz = rz;
        }

        let local_h = self.sample_local_height(lx, lz);
        self.offset[1] + local_h * sy
    }

    /// Evaluates world surface normal using central differences
    pub fn sample_world_normal(&self, wx: f32, wz: f32) -> [f32; 3] {
        let delta = 0.25;
        let hl = self.sample_world_height(wx - delta, wz);
        let hr = self.sample_world_height(wx + delta, wz);
        let hd = self.sample_world_height(wx, wz - delta);
        let hu = self.sample_world_height(wx, wz + delta);

        let dx = (hr - hl) / (2.0 * delta);
        let dz = (hu - hd) / (2.0 * delta);

        let len = (dx * dx + 1.0 + dz * dz).sqrt();
        [-dx / len, 1.0 / len, -dz / len]
    }

    /// Saves the heightmap as JSON to disk
    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize TerrainHeightmap: {}", e))?;
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, json_str)
            .map_err(|e| format!("Failed to write TerrainHeightmap to '{}': {}", path, e))
    }

    /// Loads the heightmap from a JSON file
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read TerrainHeightmap from '{}': {}", path, e))?;
        serde_json::from_str::<TerrainHeightmap>(&content)
            .map_err(|e| format!("Failed to deserialize TerrainHeightmap from '{}': {}", path, e))
    }

    /// Reconstructs a TerrainHeightmap from a 3D binary GLB mesh
    pub fn from_glb_file(glb_path: &str) -> Result<Self, String> {
        let p = Path::new(glb_path);
        let (document, buffers, _) = gltf::import(p)
            .map_err(|e| format!("Failed to import GLB '{}': {}", glb_path, e))?;

        let mut positions: Vec<[f32; 3]> = Vec::new();
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|b| Some(&buffers[b.index()]));
                if let Some(pos_iter) = reader.read_positions() {
                    positions.extend(pos_iter);
                }
            }
        }

        if positions.is_empty() {
            return Err(format!("No vertex positions found in GLB '{}'", glb_path));
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for pt in &positions {
            min_x = min_x.min(pt[0]);
            max_x = max_x.max(pt[0]);
            min_y = min_y.min(pt[1]);
            max_y = max_y.max(pt[1]);
            min_z = min_z.min(pt[2]);
            max_z = max_z.max(pt[2]);
        }

        let width = (max_x - min_x).max(1.0);
        let depth = (max_z - min_z).max(1.0);
        let res: u32 = 48;
        let num_verts = ((res + 1) * (res + 1)) as usize;

        let mut grid_heights: Vec<Option<f32>> = vec![None; num_verts];

        // Bin vertices into grid cells (keeping highest point for terrain surface)
        for pt in &positions {
            let u = ((pt[0] - min_x) / width).clamp(0.0, 1.0);
            let v = ((pt[2] - min_z) / depth).clamp(0.0, 1.0);

            let rx = (u * (res as f32)).round() as usize;
            let rz = (v * (res as f32)).round() as usize;
            let idx = rz.min(res as usize) * ((res + 1) as usize) + rx.min(res as usize);

            match grid_heights[idx] {
                Some(prev_h) => {
                    if pt[1] > prev_h {
                        grid_heights[idx] = Some(pt[1]);
                    }
                }
                None => {
                    grid_heights[idx] = Some(pt[1]);
                }
            }
        }

        // Fill missing cells from nearest populated cell
        let mut final_heights = Vec::with_capacity(num_verts);
        let row_stride = (res + 1) as usize;

        for rz in 0..=res as usize {
            for rx in 0..=res as usize {
                let idx = rz * row_stride + rx;
                if let Some(h) = grid_heights[idx] {
                    final_heights.push(h);
                } else {
                    let mut closest_h = 0.0;
                    let mut min_dist_sq = usize::MAX;
                    for rad in 1..=16 {
                        let rz_min = rz.saturating_sub(rad);
                        let rz_max = (rz + rad).min(res as usize);
                        let rx_min = rx.saturating_sub(rad);
                        let rx_max = (rx + rad).min(res as usize);

                        for nz in rz_min..=rz_max {
                            for nx in rx_min..=rx_max {
                                let n_idx = nz * row_stride + nx;
                                if let Some(h) = grid_heights[n_idx] {
                                    let dz = nz.abs_diff(rz);
                                    let dx = nx.abs_diff(rx);
                                    let d_sq = dx * dx + dz * dz;
                                    if d_sq < min_dist_sq {
                                        min_dist_sq = d_sq;
                                        closest_h = h;
                                    }
                                }
                            }
                        }
                        if min_dist_sq < usize::MAX {
                            break;
                        }
                    }
                    final_heights.push(closest_h);
                }
            }
        }

        let hm = TerrainHeightmap {
            width,
            depth,
            resolution: res,
            heights: final_heights,
            min_height: min_y,
            max_height: max_y,
            offset: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            rotation_y: 0.0,
        };

        Ok(hm)
    }

    /// Loads sidecar .heightmap.json if it exists, or reconstructs from the .glb and caches the sidecar
    pub fn load_or_reconstruct(glb_path: &str) -> Result<Self, String> {
        let hm_path = heightmap_path_for_glb(glb_path);
        if Path::new(&hm_path).exists() {
            if let Ok(hm) = Self::load_from_file(&hm_path) {
                return Ok(hm);
            }
        }

        if Path::new(glb_path).exists() {
            let hm = Self::from_glb_file(glb_path)?;
            let _ = hm.save_to_file(&hm_path);
            return Ok(hm);
        }

        Err(format!("Terrain model not found: neither '{}' nor '{}' exist", hm_path, glb_path))
    }
}

/// Detects and loads active terrain heightmap from scene entities
pub fn sync_active_terrain_from_scene(
    scene: &crate::assets::SceneDef,
    active_terrain: &Arc<RwLock<Option<TerrainHeightmap>>>,
) {
    let mut found_terrain = None;

    for entity in &scene.entities {
        let mesh = &entity.mesh;
        let id = &entity.id;

        let hm_path = heightmap_path_for_glb(mesh);
        let has_sidecar = Path::new(&hm_path).exists();
        let mesh_lower = mesh.to_lowercase();
        let id_lower = id.to_lowercase();
        let is_glb = mesh_lower.ends_with(".glb");
        let name_match = mesh_lower.contains("terrain") || mesh_lower.contains("arena")
            || id_lower.contains("terrain") || id_lower.contains("arena");

        if is_glb && (has_sidecar || name_match) {
            match TerrainHeightmap::load_or_reconstruct(mesh) {
                Ok(mut hm) => {
                    hm.offset = entity.position;
                    hm.scale = entity.scale;
                    hm.rotation_y = entity.rotation[1];
                    println!(
                        "[Level Forge] Loaded active terrain heightmap from '{}' for entity '{}' (pos: {:?}, scale: {:?})",
                        mesh, id, entity.position, entity.scale
                    );
                    found_terrain = Some(hm);
                    break;
                }
                Err(e) => {
                    eprintln!("[Level Forge] Could not load/reconstruct terrain from '{}': {}", mesh, e);
                }
            }
        }
    }

    if let Ok(mut guard) = active_terrain.write() {
        *guard = found_terrain;
    }
}

fn compute_face_normal(p0: &[f32; 3], p1: &[f32; 3], p2: &[f32; 3]) -> [f32; 3] {
    let v0 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let v1 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

    let cx = v0[1] * v1[2] - v0[2] * v1[1];
    let cy = v0[2] * v1[0] - v0[0] * v1[2];
    let cz = v0[0] * v1[1] - v0[1] * v1[0];

    let len = (cx * cx + cy * cy + cz * cz).sqrt().max(1e-6);
    [cx / len, cy / len, cz / len]
}


// ============================================================================
// 2. DUNGEON FORGE — Interior Level & Room Generator
// ============================================================================

fn default_dungeon_grid_w() -> usize { 24 }
fn default_dungeon_grid_h() -> usize { 24 }
fn default_dungeon_cell_size() -> f32 { 3.0 }
fn default_dungeon_room_count() -> usize { 8 }
fn default_dungeon_seed() -> u32 { 42 }
fn default_dungeon_wall_h() -> f32 { 3.0 }
fn default_theme() -> String { "stone".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonRecipe {
    pub name: String,
    #[serde(default = "default_dungeon_grid_w")]
    pub grid_width: usize,
    #[serde(default = "default_dungeon_grid_h")]
    pub grid_height: usize,
    #[serde(default = "default_dungeon_cell_size")]
    pub cell_size: f32,
    #[serde(default = "default_dungeon_room_count")]
    pub room_count: usize,
    #[serde(default = "default_dungeon_seed")]
    pub seed: u32,
    #[serde(default = "default_dungeon_wall_h")]
    pub wall_height: f32,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for DungeonRecipe {
    fn default() -> Self {
        Self {
            name: "dungeon_default".to_string(),
            grid_width: default_dungeon_grid_w(),
            grid_height: default_dungeon_grid_h(),
            cell_size: default_dungeon_cell_size(),
            room_count: default_dungeon_room_count(),
            seed: default_dungeon_seed(),
            wall_height: default_dungeon_wall_h(),
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonRoom {
    pub id: usize,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub center_x: usize,
    pub center_y: usize,
}

impl DungeonRoom {
    pub fn intersects(&self, other: &DungeonRoom, margin: usize) -> bool {
        let x1 = self.x.saturating_sub(margin);
        let y1 = self.y.saturating_sub(margin);
        let x2 = self.x + self.width + margin;
        let y2 = self.y + self.height + margin;

        let ox1 = other.x.saturating_sub(margin);
        let oy1 = other.y.saturating_sub(margin);
        let ox2 = other.x + other.width + margin;
        let oy2 = other.y + other.height + margin;

        !(x2 <= ox1 || ox2 <= x1 || y2 <= oy1 || oy2 <= y1)
    }
}

#[derive(Debug, Clone)]
pub struct DungeonResult {
    pub recipe: DungeonRecipe,
    pub rooms: Vec<DungeonRoom>,
    pub spawn_point: [f32; 3],
    pub boss_point: [f32; 3],
    pub loot_points: Vec<[f32; 3]>,
    pub floor_cells: Vec<(usize, usize)>,
    pub wall_cells: Vec<(usize, usize)>,
}

// Simple deterministic Linear Congruential Generator
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self { state: (seed as u64).wrapping_add(1442695040888963407) }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_range(&mut self, min: usize, max: usize) -> usize {
        if min >= max { return min; }
        let diff = max - min + 1;
        min + (self.next_u32() as usize % diff)
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() & 0x00FFFFFF) as f32 / 16777216.0
    }
}

pub fn generate_dungeon(recipe: &DungeonRecipe) -> DungeonResult {
    let gw = recipe.grid_width.max(12);
    let gh = recipe.grid_height.max(12);
    let target_rooms = recipe.room_count.clamp(2, 20);

    let mut rng = SimpleRng::new(recipe.seed);
    let mut rooms: Vec<DungeonRoom> = Vec::new();

    // 1. Generate Rooms with rejection sampling
    let max_attempts = target_rooms * 25;
    for _ in 0..max_attempts {
        if rooms.len() >= target_rooms {
            break;
        }

        let rw = rng.next_range(3, 6);
        let rh = rng.next_range(3, 6);
        if rw + 2 >= gw || rh + 2 >= gh { continue; }

        let rx = rng.next_range(1, gw - rw - 1);
        let ry = rng.next_range(1, gh - rh - 1);

        let candidate = DungeonRoom {
            id: rooms.len(),
            x: rx,
            y: ry,
            width: rw,
            height: rh,
            center_x: rx + rw / 2,
            center_y: ry + rh / 2,
        };

        if !rooms.iter().any(|existing| candidate.intersects(existing, 1)) {
            rooms.push(candidate);
        }
    }

    // 2. Mark Grid: 0 = Empty, 1 = Floor, 2 = Wall
    let mut grid = vec![vec![0u8; gw]; gh];

    // Carve room floors
    for r in &rooms {
        for y in r.y..(r.y + r.height) {
            for x in r.x..(r.x + r.width) {
                grid[y][x] = 1;
            }
        }
    }

    // Connect rooms sequentially with L-shaped corridors
    for i in 0..rooms.len().saturating_sub(1) {
        let (x1, y1) = (rooms[i].center_x, rooms[i].center_y);
        let (x2, y2) = (rooms[i + 1].center_x, rooms[i + 1].center_y);

        // Horizontal segment
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        for x in min_x..=max_x {
            grid[y1][x] = 1;
        }

        // Vertical segment
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);
        for y in min_y..=max_y {
            grid[y][x2] = 1;
        }
    }

    // Identify Wall cells: any empty cell adjacent (orthogonal or diagonal) to a floor cell
    let mut floor_cells = Vec::new();
    let mut wall_cells = Vec::new();

    for y in 0..gh {
        for x in 0..gw {
            if grid[y][x] == 1 {
                floor_cells.push((x, y));
            } else {
                let mut is_adjacent_to_floor = false;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < gw as i32 && ny >= 0 && ny < gh as i32 {
                            if grid[ny as usize][nx as usize] == 1 {
                                is_adjacent_to_floor = true;
                                break;
                            }
                        }
                    }
                    if is_adjacent_to_floor { break; }
                }

                if is_adjacent_to_floor {
                    grid[y][x] = 2;
                    wall_cells.push((x, y));
                }
            }
        }
    }

    // 3. Compute Coordinates in 3D World space
    let cell = recipe.cell_size;
    let ox = -((gw as f32) * cell) * 0.5;
    let oz = -((gh as f32) * cell) * 0.5;

    let world_pos = |gx: usize, gy: usize, y_h: f32| -> [f32; 3] {
        [
            ox + (gx as f32 + 0.5) * cell,
            y_h,
            oz + (gy as f32 + 0.5) * cell,
        ]
    };

    let spawn_point = if let Some(r0) = rooms.first() {
        world_pos(r0.center_x, r0.center_y, 0.5)
    } else {
        [0.0, 0.5, 0.0]
    };

    let boss_point = if let Some(r_last) = rooms.last() {
        world_pos(r_last.center_x, r_last.center_y, 0.5)
    } else {
        [0.0, 0.5, 0.0]
    };

    let mut loot_points = Vec::new();
    if rooms.len() > 2 {
        for r in &rooms[1..rooms.len() - 1] {
            loot_points.push(world_pos(r.center_x, r.center_y, 0.5));
        }
    }

    DungeonResult {
        recipe: recipe.clone(),
        rooms,
        spawn_point,
        boss_point,
        loot_points,
        floor_cells,
        wall_cells,
    }
}

// ============================================================================
// 3. SCATTER SYSTEM — Intelligent Procedural Prop Placement
// ============================================================================

fn default_scatter_count() -> usize { 30 }
fn default_scatter_radius() -> f32 { 40.0 }
fn default_min_spacing() -> f32 { 3.5 }
fn default_scale_min() -> f32 { 0.8 }
fn default_scale_max() -> f32 { 1.3 }
fn default_max_slope() -> f32 { 35.0 }
fn default_scatter_seed() -> u32 { 999 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatterRecipe {
    pub name: String,
    pub prefab_or_mesh: String,
    #[serde(default = "default_scatter_count")]
    pub count: usize,
    #[serde(default = "default_scatter_radius")]
    pub area_radius: f32,
    #[serde(default = "default_min_spacing")]
    pub min_spacing: f32,
    #[serde(default = "default_scale_min")]
    pub scale_min: f32,
    #[serde(default = "default_scale_max")]
    pub scale_max: f32,
    #[serde(default = "default_true")]
    pub random_yaw: bool,
    #[serde(default = "default_max_slope")]
    pub max_slope_deg: f32,
    #[serde(default = "default_scatter_seed")]
    pub seed: u32,
}

impl Default for ScatterRecipe {
    fn default() -> Self {
        Self {
            name: "scatter_props".to_string(),
            prefab_or_mesh: "cube".to_string(),
            count: default_scatter_count(),
            area_radius: default_scatter_radius(),
            min_spacing: default_min_spacing(),
            scale_min: default_scale_min(),
            scale_max: default_scale_max(),
            random_yaw: true,
            max_slope_deg: default_max_slope(),
            seed: default_scatter_seed(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatterInstance {
    pub id: String,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
    pub mesh: String,
    pub color: [f32; 4],
    #[serde(default)]
    pub emissive: Option<f32>,
}

pub fn generate_scatter_instances(
    recipe: &ScatterRecipe,
    terrain: Option<&TerrainHeightmap>,
) -> Vec<ScatterInstance> {
    let mut rng = SimpleRng::new(recipe.seed);
    let mut instances: Vec<ScatterInstance> = Vec::new();

    let max_attempts = recipe.count * 30;
    let min_spacing_sq = recipe.min_spacing * recipe.min_spacing;

    for _ in 0..max_attempts {
        if instances.len() >= recipe.count {
            break;
        }

        // Uniform disk sampling
        let u = rng.next_f32();
        let v = rng.next_f32();
        let r = recipe.area_radius * u.sqrt();
        let theta = v * 2.0 * PI;

        let x = r * theta.cos();
        let z = r * theta.sin();

        // 1. Spacing check (Poisson-disc minimum spacing rejection)
        let too_close = instances.iter().any(|inst| {
            let dx = inst.position[0] - x;
            let dz = inst.position[2] - z;
            dx * dx + dz * dz < min_spacing_sq
        });

        if too_close {
            continue;
        }

        // 2. Terrain height and slope check
        let (y, slope_deg) = if let Some(t) = terrain {
            let h = t.sample_world_height(x, z);
            let n = t.sample_world_normal(x, z);
            // Slope in degrees from vertical (Y=1 is 0 degrees slope)
            let cos_theta = n[1].clamp(0.0, 1.0);
            let s_deg = cos_theta.acos() * (180.0 / PI);
            (h, s_deg)
        } else {
            (0.0, 0.0)
        };

        if slope_deg > recipe.max_slope_deg {
            continue;
        }

        // 3. Scale and Yaw calculation
        let s_ratio = rng.next_f32();
        let scale_val = recipe.scale_min + s_ratio * (recipe.scale_max - recipe.scale_min);
        let yaw = if recipe.random_yaw {
            rng.next_f32() * 360.0
        } else {
            0.0
        };

        let id = format!("{}_{}", recipe.name, instances.len() + 1);

        let (color, emissive) = match recipe.prefab_or_mesh.as_str() {
            "basalt_pillar" => ([0.14, 0.14, 0.16, 1.0], None),
            "jagged_rock" => ([0.25, 0.23, 0.22, 1.0], None),
            "crystal_cluster" => ([0.20, 0.85, 0.95, 1.0], Some(0.6)),
            "sci_fi_crate" => ([0.22, 0.28, 0.32, 1.0], Some(0.15)),
            _ => ([0.85, 0.85, 0.90, 1.0], None),
        };

        instances.push(ScatterInstance {
            id,
            position: [x, y, z],
            rotation_deg: [0.0, yaw, 0.0],
            scale: [scale_val, scale_val, scale_val],
            mesh: recipe.prefab_or_mesh.clone(),
            color,
            emissive,
        });
    }

    instances
}

/// Convenience helper to generate scatter props directly from a TerrainRecipe
pub fn generate_scatter_instances_from_recipe(
    recipe: &ScatterRecipe,
    terrain: Option<&TerrainRecipe>,
) -> Vec<ScatterInstance> {
    let hm = terrain.map(|t| t.to_heightmap());
    generate_scatter_instances(recipe, hm.as_ref())
}


// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_terrain_height_and_mesh_generation_no_nan() {
        let recipe = TerrainRecipe {
            name: "test_terrain".to_string(),
            width: 80.0,
            depth: 80.0,
            resolution: 32,
            max_height: 12.0,
            noise_scale: 0.04,
            octaves: 3,
            seed: 777,
            island_falloff: true,
            flat_shading: false,
            palette: "alpine".to_string(),
        };

        // Check center height
        let h_center = sample_terrain_height(&recipe, 0.0, 0.0);
        assert!(!h_center.is_nan() && !h_center.is_infinite());
        assert!(h_center >= 0.0 && h_center <= recipe.max_height);

        // Check island falloff at extreme boundary
        let h_edge = sample_terrain_height(&recipe, 100.0, 100.0);
        assert_eq!(h_edge, 0.0);

        // Check mesh generation
        let mesh = recipe.build_mesh();
        assert!(!mesh.positions.is_empty());
        assert_eq!(mesh.positions.len(), mesh.normals.len());
        assert_eq!(mesh.positions.len(), mesh.colors.len());
        assert!(!mesh.indices.is_empty());

        for pos in &mesh.positions {
            assert!(!pos[0].is_nan() && !pos[1].is_nan() && !pos[2].is_nan());
        }
        for n in &mesh.normals {
            assert!(!n[0].is_nan() && !n[1].is_nan() && !n[2].is_nan());
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-2, "Normal not normalized: {}", len);
        }
    }

    #[test]
    fn test_bake_alien_terrain_demo_glb() {
        let recipe = TerrainRecipe {
            name: "terrain_demo".to_string(),
            width: 100.0,
            depth: 100.0,
            resolution: 64,
            max_height: 14.0,
            noise_scale: 0.035,
            octaves: 4,
            seed: 888,
            island_falloff: true,
            flat_shading: false,
            palette: "alien".to_string(),
        };

        let path = "assets/models/terrain_demo.glb";
        let res = recipe.bake_to_glb(path);
        assert!(res.is_ok(), "Bake failed: {:?}", res.err());
        assert!(std::path::Path::new(path).exists());
    }

    #[test]
    fn test_bake_volcanic_terrain_glb() {
        let recipe = TerrainRecipe {
            name: "terrain_volcanic".to_string(),
            width: 80.0,
            depth: 80.0,
            resolution: 48,
            max_height: 10.0,
            noise_scale: 0.04,
            octaves: 4,
            seed: 666,
            island_falloff: true,
            flat_shading: false,
            palette: "volcanic".to_string(),
        };

        let path = "assets/models/terrain_volcanic.glb";
        let res = recipe.bake_to_glb(path);
        assert!(res.is_ok(), "Bake failed: {:?}", res.err());
        assert!(std::path::Path::new(path).exists());
        let hm_path = heightmap_path_for_glb(path);
        assert!(std::path::Path::new(&hm_path).exists());
    }

    #[test]
    fn test_dungeon_generation_and_connectivity() {
        let recipe = DungeonRecipe {
            name: "test_dungeon".to_string(),
            grid_width: 24,
            grid_height: 24,
            cell_size: 3.0,
            room_count: 6,
            seed: 1234,
            wall_height: 3.0,
            theme: "stone".to_string(),
        };

        let result = generate_dungeon(&recipe);
        assert!(result.rooms.len() >= 4, "Expected at least 4 rooms generated, got {}", result.rooms.len());
        assert!(!result.floor_cells.is_empty());
        assert!(!result.wall_cells.is_empty());

        // Check spawn and boss points
        assert_ne!(result.spawn_point, result.boss_point, "Spawn and boss point should be distinct rooms");
        assert!(!result.spawn_point[0].is_nan() && !result.boss_point[0].is_nan());
    }

    #[test]
    fn test_scatter_min_spacing_and_slope_rejection() {
        let terrain = TerrainRecipe {
            name: "test_scatter_terrain".to_string(),
            width: 100.0,
            depth: 100.0,
            resolution: 32,
            max_height: 15.0,
            noise_scale: 0.03,
            octaves: 3,
            seed: 42,
            island_falloff: true,
            flat_shading: false,
            palette: "desert".to_string(),
        };

        let recipe = ScatterRecipe {
            name: "tree_prop".to_string(),
            prefab_or_mesh: "cone".to_string(),
            count: 25,
            area_radius: 35.0,
            min_spacing: 4.0,
            scale_min: 0.8,
            scale_max: 1.2,
            random_yaw: true,
            max_slope_deg: 40.0,
            seed: 888,
        };

        let instances = generate_scatter_instances_from_recipe(&recipe, Some(&terrain));
        assert!(!instances.is_empty(), "Should generate scatter instances");
        assert!(instances.len() <= recipe.count);

        // Verify min spacing between any two instances
        let min_spacing_sq = (recipe.min_spacing - 0.01) * (recipe.min_spacing - 0.01);
        for i in 0..instances.len() {
            for j in (i + 1)..instances.len() {
                let dx = instances[i].position[0] - instances[j].position[0];
                let dz = instances[i].position[2] - instances[j].position[2];
                let dist_sq = dx * dx + dz * dz;
                assert!(dist_sq >= min_spacing_sq, "Instances too close: {} and {} dist_sq={}", i, j, dist_sq);
            }
        }

        // Verify y coordinates correspond to terrain heightmap
        let hm = terrain.to_heightmap();
        for inst in &instances {
            let h = hm.sample_world_height(inst.position[0], inst.position[2]);
            assert!((inst.position[1] - h).abs() < 1e-4, "Height mismatch against heightmap: {} vs {}", inst.position[1], h);
            let h_fbm = sample_terrain_height(&terrain, inst.position[0], inst.position[2]);
            assert!((inst.position[1] - h_fbm).abs() < 1.0, "Height mismatch against fbm: {} vs {}", inst.position[1], h_fbm);
        }
    }

    #[test]
    fn test_generate_showcase_v18_terrain_scene() {
        let terrain_recipe = TerrainRecipe {
            name: "terrain_demo".to_string(),
            width: 100.0,
            depth: 100.0,
            resolution: 64,
            max_height: 14.0,
            noise_scale: 0.035,
            octaves: 4,
            seed: 888,
            island_falloff: true,
            flat_shading: false,
            palette: "alien".to_string(),
        };

        let scatter_recipe = ScatterRecipe {
            name: "crystal_monolith".to_string(),
            prefab_or_mesh: "cylinder".to_string(),
            count: 30,
            area_radius: 38.0,
            min_spacing: 4.5,
            scale_min: 0.6,
            scale_max: 1.6,
            random_yaw: true,
            max_slope_deg: 35.0,
            seed: 777,
        };

        let instances = generate_scatter_instances_from_recipe(&scatter_recipe, Some(&terrain_recipe));
        assert!(!instances.is_empty());

        let mut entities = Vec::new();

        // 1. Terrain Entity
        entities.push(serde_json::json!({
            "id": "alien_terrain",
            "name": "Alien Terrain",
            "mesh": "assets/models/terrain_demo.glb",
            "position": [0.0, 0.0, 0.0],
            "rotation": [0.0, 0.0, 0.0],
            "scale": [1.0, 1.0, 1.0],
            "color": [1.0, 1.0, 1.0, 1.0],
            "emissive": 0.0,
            "wireframe": false,
            "blend_mode": "opaque"
        }));

        // 2. Plasma Beacon Core at center peak
        let peak_h = sample_terrain_height(&terrain_recipe, 0.0, 0.0);
        entities.push(serde_json::json!({
            "id": "plasma_beacon",
            "name": "Alien Sanctuary Beacon",
            "mesh": "icosphere",
            "position": [0.0, peak_h + 1.2, 0.0],
            "rotation": [0.0, 0.0, 0.0],
            "scale": [1.8, 1.8, 1.8],
            "color": [0.15, 0.9, 1.0, 1.0],
            "emissive": 4.5,
            "fresnel_color": [0.4, 0.95, 1.0],
            "fresnel_power": 2.5,
            "blend_mode": "opaque",
            "point_light": {
                "color": [0.2, 0.85, 1.0],
                "intensity": 6.0,
                "radius": 30.0
            }
        }));

        // 3. Terrain Scout Drone
        entities.push(serde_json::json!({
            "id": "terrain_scout",
            "name": "Terrain Rover Scout",
            "mesh": "cone",
            "position": [22.0, 10.0, 0.0],
            "rotation": [0.0, 0.0, 0.0],
            "scale": [0.9, 0.9, 1.2],
            "color": [0.95, 0.2, 0.85, 1.0],
            "emissive": 3.0,
            "fresnel_color": [1.0, 0.4, 0.9],
            "fresnel_power": 2.0,
            "script": "assets/scripts/terrain_scout.lua"
        }));

        // 4. Scattered Crystal Monoliths
        for (idx, inst) in instances.iter().enumerate() {
            let col = if idx % 2 == 0 {
                [0.15, 0.85, 0.95, 1.0]
            } else {
                [0.85, 0.2, 0.9, 1.0]
            };
            let h = inst.scale[0] * 2.2;
            entities.push(serde_json::json!({
                "id": format!("crystal_{}", idx + 1),
                "name": format!("Alien Monolith {}", idx + 1),
                "mesh": "cylinder",
                "position": [inst.position[0], inst.position[1] + h * 0.5, inst.position[2]],
                "rotation": [0.0, inst.rotation_deg[1], 0.0],
                "scale": [inst.scale[0] * 0.6, h, inst.scale[2] * 0.6],
                "color": col,
                "emissive": 2.2,
                "fresnel_color": [col[0] * 1.2, col[1] * 1.2, col[2] * 1.2],
                "fresnel_power": 2.2,
                "blend_mode": "opaque"
            }));
        }

        let scene = serde_json::json!({
            "camera": {
                "mode": "3d",
                "position": [0.0, 22.0, 38.0],
                "target": [0.0, 7.0, 0.0],
                "ortho_size": 15.0
            },
            "post_process": {
                "bloom_enabled": true,
                "bloom_intensity": 1.25,
                "bloom_threshold": 1.0,
                "vignette_intensity": 0.35,
                "exposure": 1.15
            },
            "environment": {
                "sky_mode": "space",
                "zenith_color": [0.05, 0.08, 0.18],
                "horizon_color": [0.02, 0.02, 0.08],
                "fog_color": [0.03, 0.04, 0.12],
                "fog_density": 0.008
            },
            "entities": entities
        });

        let scene_json = serde_json::to_string_pretty(&scene).unwrap();
        std::fs::write("assets/showcase_v18_terrain.json", scene_json).unwrap();
        assert!(std::path::Path::new("assets/showcase_v18_terrain.json").exists());
    }

    #[test]
    fn test_stylized_scatter_presets_and_volcanic_palette() {
        let presets = ["basalt_pillar", "jagged_rock", "crystal_cluster", "sci_fi_crate"];
        for preset in presets {
            let recipe = ScatterRecipe {
                name: format!("prop_{}", preset),
                prefab_or_mesh: preset.to_string(),
                count: 10,
                area_radius: 20.0,
                min_spacing: 2.0,
                scale_min: 0.8,
                scale_max: 1.2,
                random_yaw: true,
                max_slope_deg: 45.0,
                seed: 42,
            };
            let instances = generate_scatter_instances(&recipe, None);
            assert_eq!(instances.len(), 10);
            assert_eq!(instances[0].mesh, preset);
        }

        // Test volcanic palette colors are valid non-NaN and within basalt/magma range
        let volcanic_low = evaluate_terrain_color("volcanic", 0.05, 0.1);
        assert_eq!(volcanic_low, [0.98, 0.36, 0.05, 1.0]); // Magma crevice

        let volcanic_cliff = evaluate_terrain_color("volcanic", 0.5, 0.6);
        assert_eq!(volcanic_cliff, [0.18, 0.16, 0.19, 1.0]); // Steep ash cliff

        let volcanic_ground = evaluate_terrain_color("volcanic", 0.5, 0.1);
        assert_eq!(volcanic_ground, [0.09, 0.09, 0.11, 1.0]); // Deep basalt ground
    }

    #[test]
    fn test_terrain_heightmap_sidecar_serialization_roundtrip() {
        let mut recipe = TerrainRecipe::default();
        recipe.width = 80.0;
        recipe.depth = 80.0;
        recipe.resolution = 32;
        recipe.max_height = 12.0;
        recipe.noise_scale = 0.04;
        recipe.octaves = 3;
        recipe.seed = 555;

        let hm = recipe.to_heightmap();
        let temp_hm_path = "assets/models/temp_test_terrain.heightmap.json";

        let save_res = hm.save_to_file(temp_hm_path);
        assert!(save_res.is_ok(), "Failed to save heightmap: {:?}", save_res.err());
        assert!(Path::new(temp_hm_path).exists());

        let loaded = TerrainHeightmap::load_from_file(temp_hm_path).expect("Failed to load heightmap");
        assert_eq!(loaded.width, hm.width);
        assert_eq!(loaded.depth, hm.depth);
        assert_eq!(loaded.resolution, hm.resolution);
        assert_eq!(loaded.heights.len(), hm.heights.len());

        // Check sampling accuracy
        let test_points = [(0.0, 0.0), (12.0, -18.0), (-25.0, 15.0), (35.0, 35.0)];
        for (x, z) in test_points {
            let h_orig = hm.sample_local_height(x, z);
            let h_load = loaded.sample_local_height(x, z);
            assert!((h_orig - h_load).abs() < 1e-5, "Mismatch at ({}, {}): orig={} loaded={}", x, z, h_orig, h_load);
        }

        let _ = std::fs::remove_file(temp_hm_path);
    }

    #[test]
    fn test_terrain_heightmap_glb_fallback_reconstruction() {
        let mut recipe = TerrainRecipe::default();
        recipe.name = "test_reconstruction".to_string();
        recipe.width = 60.0;
        recipe.depth = 60.0;
        recipe.resolution = 24;
        recipe.max_height = 10.0;
        recipe.seed = 321;

        let temp_glb = "assets/models/temp_reconstruct_test.glb";
        let temp_hm = "assets/models/temp_reconstruct_test.heightmap.json";

        // 1. Bake terrain GLB (which also writes sidecar)
        let bake_res = recipe.bake_to_glb(temp_glb);
        assert!(bake_res.is_ok());
        assert!(Path::new(temp_glb).exists());
        assert!(Path::new(temp_hm).exists());

        // 2. Delete sidecar to test GLB fallback reconstruction
        let _ = std::fs::remove_file(temp_hm);
        assert!(!Path::new(temp_hm).exists(), "Sidecar should be deleted");

        // 3. Load or reconstruct: must parse GLB and recreate the sidecar
        let reconstructed = TerrainHeightmap::load_or_reconstruct(temp_glb)
            .expect("Fallback reconstruction from GLB failed");

        assert!(Path::new(temp_hm).exists(), "Sidecar should be automatically cached upon reconstruction");
        assert!(reconstructed.heights.len() > 0);

        // Verify height at center is reasonably close to recipe height
        let h_recipe = sample_terrain_height(&recipe, 0.0, 0.0);
        let h_recon = reconstructed.sample_local_height(0.0, 0.0);
        assert!((h_recipe - h_recon).abs() < 1.0, "Reconstructed height mismatch: recipe={} recon={}", h_recipe, h_recon);

        // Cleanup
        let _ = std::fs::remove_file(temp_glb);
        let _ = std::fs::remove_file(temp_hm);
    }

    #[test]
    fn test_terrain_heightmap_world_transform_and_rotation() {
        let mut recipe = TerrainRecipe::default();
        recipe.width = 50.0;
        recipe.depth = 50.0;
        recipe.resolution = 20;
        recipe.max_height = 8.0;

        let mut hm = recipe.to_heightmap();
        hm.offset = [10.0, 5.0, -20.0];
        hm.scale = [2.0, 1.5, 2.0];

        // At entity origin (world x=10, z=-20), local coordinates are (0, 0)
        let local_center_h = hm.sample_local_height(0.0, 0.0);
        let world_h = hm.sample_world_height(10.0, -20.0);
        let expected_world_h = 5.0 + local_center_h * 1.5;
        assert!((world_h - expected_world_h).abs() < 1e-4, "World height at center mismatch: {} vs {}", world_h, expected_world_h);

        // Test normal is normalized
        let normal = hm.sample_world_normal(10.0, -20.0);
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-3, "Normal is not normalized: length = {}", len);
    }
}
