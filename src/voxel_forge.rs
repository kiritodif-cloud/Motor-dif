use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::modeler::ForgeMesh;

pub const CHUNK_SIZE_X: usize = 16;
pub const CHUNK_SIZE_Y: usize = 32;
pub const CHUNK_SIZE_Z: usize = 16;
pub const CHUNK_VOXELS: usize = CHUNK_SIZE_X * CHUNK_SIZE_Y * CHUNK_SIZE_Z; // 8192

pub const SEA_LEVEL: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlockId {
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
    Wood = 4,
    Leaves = 5,
    Sand = 6,
    Water = 7,
    Bedrock = 8,
    Cobblestone = 9,
    Planks = 10,
    Glass = 11,
}

impl BlockId {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => BlockId::Grass,
            2 => BlockId::Dirt,
            3 => BlockId::Stone,
            4 => BlockId::Wood,
            5 => BlockId::Leaves,
            6 => BlockId::Sand,
            7 => BlockId::Water,
            8 => BlockId::Bedrock,
            9 => BlockId::Cobblestone,
            10 => BlockId::Planks,
            11 => BlockId::Glass,
            _ => BlockId::Air,
        }
    }

    #[inline]
    pub fn is_solid(self) -> bool {
        self != BlockId::Air && self != BlockId::Water
    }

    #[inline]
    pub fn is_opaque(self) -> bool {
        match self {
            BlockId::Air | BlockId::Water | BlockId::Leaves | BlockId::Glass => false,
            _ => true,
        }
    }

    #[inline]
    pub fn is_translucent(self) -> bool {
        self == BlockId::Water || self == BlockId::Glass
    }

    #[inline]
    pub fn is_cutout(self) -> bool {
        self == BlockId::Leaves
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub blocks: Vec<u8>,
}

impl Chunk {
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            chunk_x,
            chunk_z,
            blocks: vec![0u8; CHUNK_VOXELS],
        }
    }

    #[inline]
    pub fn block_index(lx: usize, ly: usize, lz: usize) -> usize {
        lx + lz * CHUNK_SIZE_X + ly * (CHUNK_SIZE_X * CHUNK_SIZE_Z)
    }

    #[inline]
    pub fn get(&self, lx: usize, ly: usize, lz: usize) -> BlockId {
        if lx < CHUNK_SIZE_X && ly < CHUNK_SIZE_Y && lz < CHUNK_SIZE_Z {
            BlockId::from_u8(self.blocks[Self::block_index(lx, ly, lz)])
        } else {
            BlockId::Air
        }
    }

    #[inline]
    pub fn set(&mut self, lx: usize, ly: usize, lz: usize, id: BlockId) {
        if lx < CHUNK_SIZE_X && ly < CHUNK_SIZE_Y && lz < CHUNK_SIZE_Z {
            let idx = Self::block_index(lx, ly, lz);
            self.blocks[idx] = id as u8;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelWorld {
    pub chunks: HashMap<(i32, i32), Chunk>,
    pub min_chunk: [i32; 2],
    pub max_chunk: [i32; 2],
}

impl Default for VoxelWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelWorld {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            min_chunk: [0, 0],
            max_chunk: [0, 0],
        }
    }

    #[inline]
    pub fn world_to_chunk_coords(x: i32, z: i32) -> ((i32, i32), (usize, usize)) {
        let cx = x.div_euclid(CHUNK_SIZE_X as i32);
        let cz = z.div_euclid(CHUNK_SIZE_Z as i32);
        let lx = x.rem_euclid(CHUNK_SIZE_X as i32) as usize;
        let lz = z.rem_euclid(CHUNK_SIZE_Z as i32) as usize;
        ((cx, cz), (lx, lz))
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> BlockId {
        if y < 0 || y >= CHUNK_SIZE_Y as i32 {
            return BlockId::Air;
        }
        let (c_coord, (lx, lz)) = Self::world_to_chunk_coords(x, z);
        if let Some(chunk) = self.chunks.get(&c_coord) {
            chunk.get(lx, y as usize, lz)
        } else {
            BlockId::Air
        }
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: BlockId) {
        if y < 0 || y >= CHUNK_SIZE_Y as i32 {
            return;
        }
        let (c_coord, (lx, lz)) = Self::world_to_chunk_coords(x, z);
        let chunk = self.chunks.entry(c_coord).or_insert_with(|| {
            Chunk::new(c_coord.0, c_coord.1)
        });
        chunk.set(lx, y as usize, lz, id);

        self.min_chunk[0] = self.min_chunk[0].min(c_coord.0);
        self.min_chunk[1] = self.min_chunk[1].min(c_coord.1);
        self.max_chunk[0] = self.max_chunk[0].max(c_coord.0);
        self.max_chunk[1] = self.max_chunk[1].max(c_coord.1);
    }

    #[inline]
    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z).is_solid()
    }

    #[inline]
    pub fn is_opaque(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z).is_opaque()
    }

    pub fn total_blocks(&self) -> usize {
        let mut count = 0;
        for chunk in self.chunks.values() {
            for &b in &chunk.blocks {
                if b > 0 {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn save_to_json(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, json)
    }

    pub fn load_from_json(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// ---------------------------------------------------------------------------
// DDA 3D Raycasting
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelRaycastHit {
    pub block_pos: [i32; 3],
    pub normal: [i32; 3],
    pub block_id: u8,
    pub distance: f32,
}

pub fn voxel_raycast(
    world: &VoxelWorld,
    ox: f32,
    oy: f32,
    oz: f32,
    dx: f32,
    dy: f32,
    dz: f32,
    max_dist: f32,
) -> Option<VoxelRaycastHit> {
    let len = (dx * dx + dy * dy + dz * dz).sqrt();
    if len < 1e-6 {
        return None;
    }
    let (ndx, ndy, ndz) = (dx / len, dy / len, dz / len);

    let mut map_x = ox.floor() as i32;
    let mut map_y = oy.floor() as i32;
    let mut map_z = oz.floor() as i32;

    let step_x = if ndx >= 0.0 { 1 } else { -1 };
    let step_y = if ndy >= 0.0 { 1 } else { -1 };
    let step_z = if ndz >= 0.0 { 1 } else { -1 };

    let delta_dist_x = if ndx.abs() < 1e-6 { 1e30 } else { (1.0 / ndx).abs() };
    let delta_dist_y = if ndy.abs() < 1e-6 { 1e30 } else { (1.0 / ndy).abs() };
    let delta_dist_z = if ndz.abs() < 1e-6 { 1e30 } else { (1.0 / ndz).abs() };

    let mut side_dist_x = if ndx >= 0.0 {
        ((map_x as f32 + 1.0) - ox) * delta_dist_x
    } else {
        (ox - map_x as f32) * delta_dist_x
    };
    let mut side_dist_y = if ndy >= 0.0 {
        ((map_y as f32 + 1.0) - oy) * delta_dist_y
    } else {
        (oy - map_y as f32) * delta_dist_y
    };
    let mut side_dist_z = if ndz >= 0.0 {
        ((map_z as f32 + 1.0) - oz) * delta_dist_z
    } else {
        (oz - map_z as f32) * delta_dist_z
    };

    let mut normal;
    let mut dist = 0.0;

    // Check if initial voxel is solid
    let initial_block = world.get_block(map_x, map_y, map_z);
    if initial_block.is_solid() {
        return Some(VoxelRaycastHit {
            block_pos: [map_x, map_y, map_z],
            normal: [0, 1, 0],
            block_id: initial_block as u8,
            distance: 0.0,
        });
    }

    while dist <= max_dist {
        if side_dist_x < side_dist_y && side_dist_x < side_dist_z {
            dist = side_dist_x;
            side_dist_x += delta_dist_x;
            map_x += step_x;
            normal = [-step_x, 0, 0];
        } else if side_dist_y < side_dist_z {
            dist = side_dist_y;
            side_dist_y += delta_dist_y;
            map_y += step_y;
            normal = [0, -step_y, 0];
        } else {
            dist = side_dist_z;
            side_dist_z += delta_dist_z;
            map_z += step_z;
            normal = [0, 0, -step_z];
        }

        if dist > max_dist {
            break;
        }

        let b = world.get_block(map_x, map_y, map_z);
        if b.is_solid() {
            return Some(VoxelRaycastHit {
                block_pos: [map_x, map_y, map_z],
                normal,
                block_id: b as u8,
                distance: dist,
            });
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Atlas UV & Texture Generation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFace {
    Top,
    Bottom,
    North, // +Z
    South, // -Z
    East,  // +X
    West,  // -X
}

pub fn get_block_face_uv_rect(block: BlockId, face: BlockFace) -> (f32, f32, f32, f32) {
    // Atlas is 4x4 tiles of 16x16 pixels
    let (col, row) = match block {
        BlockId::Grass => match face {
            BlockFace::Top => (0, 0),
            BlockFace::Bottom => (2, 0),
            _ => (1, 0),
        },
        BlockId::Dirt => (2, 0),
        BlockId::Stone => (3, 0),
        BlockId::Wood => match face {
            BlockFace::Top | BlockFace::Bottom => (1, 1),
            _ => (0, 1),
        },
        BlockId::Leaves => (2, 1),
        BlockId::Sand => (3, 1),
        BlockId::Water => (0, 2),
        BlockId::Bedrock => (1, 2),
        BlockId::Cobblestone => (2, 2),
        BlockId::Planks => (3, 2),
        BlockId::Glass => (0, 3),
        BlockId::Air => (0, 0),
    };

    let tile_w = 0.25;
    let tile_h = 0.25;
    let u0 = col as f32 * tile_w;
    let v0 = row as f32 * tile_h;
    let u1 = u0 + tile_w;
    let v1 = v0 + tile_h;

    (u0, v0, u1, v1)
}

pub fn ensure_voxel_atlas(path: impl AsRef<Path>) -> Result<(), String> {
    let p = path.as_ref();
    if p.exists() {
        return Ok(());
    }
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let img_w = 64u32;
    let img_h = 64u32;
    let mut img = image::RgbaImage::new(img_w, img_h);

    // Procedural 16x16 pixel art tiles
    for ty in 0..4 {
        for tx in 0..4 {
            let tile_idx = (tx, ty);
            for py in 0..16 {
                for px in 0..16 {
                    let gx = tx * 16 + px;
                    let gy = ty * 16 + py;

                    // Pseudo-random noise per pixel
                    let hash = ((px * 37 + py * 17 + tx * 101 + ty * 59) % 25) as i32 - 12;

                    let color = match tile_idx {
                        // (0, 0) Grass Top
                        (0, 0) => {
                            let g = (168 + hash).clamp(120, 210) as u8;
                            let r = (85 + hash / 2).clamp(60, 115) as u8;
                            let b = (48 + hash / 3).clamp(30, 70) as u8;
                            [r, g, b, 255]
                        }
                        // (1, 0) Grass Side
                        (1, 0) => {
                            if py < 4 || (py == 4 && (px % 3 == 0 || px % 5 == 0)) {
                                [90, 165, 52, 255]
                            } else {
                                let c = (128 + hash).clamp(90, 155) as u8;
                                [c, (c as f32 * 0.65) as u8, (c as f32 * 0.40) as u8, 255]
                            }
                        }
                        // (2, 0) Dirt
                        (2, 0) => {
                            let c = (130 + hash).clamp(95, 160) as u8;
                            [c, (c as f32 * 0.67) as u8, (c as f32 * 0.42) as u8, 255]
                        }
                        // (3, 0) Stone
                        (3, 0) => {
                            let v = (138 + hash).clamp(100, 175) as u8;
                            [v, v, v, 255]
                        }
                        // (0, 1) Wood Bark Side
                        (0, 1) => {
                            let bark = if px % 4 == 0 { 80 } else { 105 + hash };
                            let v = bark.clamp(65, 135) as u8;
                            [v, (v as f32 * 0.65) as u8, (v as f32 * 0.38) as u8, 255]
                        }
                        // (1, 1) Wood Rings Top
                        (1, 1) => {
                            let dx = px as i32 - 8;
                            let dy = py as i32 - 8;
                            let dist = ((dx * dx + dy * dy) as f32).sqrt() as i32;
                            if dist >= 7 {
                                [85, 55, 30, 255] // dark bark ring
                            } else if dist % 3 == 0 {
                                [160, 125, 75, 255]
                            } else {
                                [185, 145, 95, 255]
                            }
                        }
                        // (2, 1) Leaves (Cutout foliage)
                        (2, 1) => {
                            // Leave scattered transparent holes for pixel art cutout look
                            if (px * 7 + py * 11) % 9 == 0 {
                                [0, 0, 0, 0]
                            } else {
                                let g = (145 + hash).clamp(100, 185) as u8;
                                let r = (45 + hash / 2).clamp(25, 70) as u8;
                                let b = (35 + hash / 3).clamp(20, 55) as u8;
                                [r, g, b, 255]
                            }
                        }
                        // (3, 1) Sand
                        (3, 1) => {
                            let r = (220 + hash).clamp(185, 245) as u8;
                            let g = (205 + hash).clamp(170, 230) as u8;
                            let b = (145 + hash).clamp(115, 170) as u8;
                            [r, g, b, 255]
                        }
                        // (0, 2) Water (Vibrant azure translucent)
                        (0, 2) => {
                            let b = (230 + hash).clamp(195, 255) as u8;
                            let g = (135 + hash / 2).clamp(100, 165) as u8;
                            [40, g, b, 170]
                        }
                        // (1, 2) Bedrock
                        (1, 2) => {
                            let v = (55 + hash * 2).clamp(20, 95) as u8;
                            [v, v, v, 255]
                        }
                        // (2, 2) Cobblestone
                        (2, 2) => {
                            let is_mortar = (px % 8 == 0) || (py % 8 == 0);
                            if is_mortar {
                                [60, 60, 65, 255]
                            } else {
                                let v = (120 + hash * 2).clamp(85, 160) as u8;
                                [v, v, v, 255]
                            }
                        }
                        // (3, 2) Planks
                        (3, 2) => {
                            let is_gap = py % 4 == 0;
                            if is_gap {
                                [110, 75, 40, 255]
                            } else {
                                let r = (185 + hash).clamp(150, 215) as u8;
                                let g = (135 + hash).clamp(105, 165) as u8;
                                let b = (78 + hash / 2).clamp(55, 105) as u8;
                                [r, g, b, 255]
                            }
                        }
                        // (0, 3) Glass (Translucent border & glint)
                        (0, 3) => {
                            let is_border = px == 0 || px == 15 || py == 0 || py == 15;
                            let is_glint = (px + py == 8) || (px + py == 9) || (px + py == 16);
                            if is_border {
                                [200, 230, 245, 200]
                            } else if is_glint {
                                [255, 255, 255, 220]
                            } else {
                                [210, 240, 255, 80]
                            }
                        }
                        _ => [128, 128, 128, 255],
                    };

                    img.put_pixel(gx, gy, image::Rgba(color));
                }
            }
        }
    }

    img.save(p).map_err(|e| format!("Failed to save voxel atlas: {}", e))
}

// ---------------------------------------------------------------------------
// Chunk Mesher with Hidden-Face Culling and Corner Vertex AO
// ---------------------------------------------------------------------------

#[inline]
fn compute_vertex_ao(s1: bool, s2: bool, corner: bool) -> f32 {
    if s1 && s2 {
        0.46
    } else {
        match (s1 as u8) + (s2 as u8) + (corner as u8) {
            0 => 1.0,
            1 => 0.82,
            2 => 0.65,
            _ => 0.46,
        }
    }
}

pub fn mesh_voxel_world(world: &VoxelWorld) -> (ForgeMesh, ForgeMesh) {
    let mut opaque = ForgeMesh::new();
    let mut translucent = ForgeMesh::new();

    for chunk in world.chunks.values() {
        mesh_chunk_into(world, chunk, &mut opaque, &mut translucent);
    }

    (opaque, translucent)
}

pub fn mesh_single_chunk(world: &VoxelWorld, chunk_x: i32, chunk_z: i32) -> (ForgeMesh, ForgeMesh) {
    let mut opaque = ForgeMesh::new();
    let mut translucent = ForgeMesh::new();

    if let Some(chunk) = world.chunks.get(&(chunk_x, chunk_z)) {
        mesh_chunk_into(world, chunk, &mut opaque, &mut translucent);
    }

    (opaque, translucent)
}

fn mesh_chunk_into(
    world: &VoxelWorld,
    chunk: &Chunk,
    opaque: &mut ForgeMesh,
    translucent: &mut ForgeMesh,
) {
    let base_x = chunk.chunk_x * CHUNK_SIZE_X as i32;
    let base_z = chunk.chunk_z * CHUNK_SIZE_Z as i32;

    for ly in 0..CHUNK_SIZE_Y {
        let wy = ly as i32;
        for lz in 0..CHUNK_SIZE_Z {
            let wz = base_z + lz as i32;
            for lx in 0..CHUNK_SIZE_X {
                let wx = base_x + lx as i32;

                let block = chunk.get(lx, ly, lz);
                if block == BlockId::Air {
                    continue;
                }

                let is_water = block == BlockId::Water;
                let target_mesh = if block.is_translucent() {
                    &mut *translucent
                } else {
                    &mut *opaque
                };

                // Check 6 faces
                // 1. Top Face (+Y)
                let top_block = world.get_block(wx, wy + 1, wz);
                let show_top = if is_water {
                    top_block == BlockId::Air
                } else {
                    !top_block.is_opaque() && (top_block != block || !block.is_cutout())
                };

                if show_top {
                    emit_face(
                        world,
                        target_mesh,
                        block,
                        BlockFace::Top,
                        wx, wy, wz,
                        is_water,
                    );
                }

                // 2. Bottom Face (-Y)
                let bot_block = world.get_block(wx, wy - 1, wz);
                let show_bot = if is_water {
                    bot_block == BlockId::Air
                } else {
                    !bot_block.is_opaque() && (bot_block != block || !block.is_cutout())
                };

                if show_bot {
                    emit_face(
                        world,
                        target_mesh,
                        block,
                        BlockFace::Bottom,
                        wx, wy, wz,
                        is_water,
                    );
                }

                // 3. North Face (+Z)
                let north_block = world.get_block(wx, wy, wz + 1);
                let show_north = if is_water {
                    north_block == BlockId::Air
                } else {
                    !north_block.is_opaque() && (north_block != block || !block.is_cutout())
                };

                if show_north {
                    emit_face(
                        world,
                        target_mesh,
                        block,
                        BlockFace::North,
                        wx, wy, wz,
                        is_water,
                    );
                }

                // 4. South Face (-Z)
                let south_block = world.get_block(wx, wy, wz - 1);
                let show_south = if is_water {
                    south_block == BlockId::Air
                } else {
                    !south_block.is_opaque() && (south_block != block || !block.is_cutout())
                };

                if show_south {
                    emit_face(
                        world,
                        target_mesh,
                        block,
                        BlockFace::South,
                        wx, wy, wz,
                        is_water,
                    );
                }

                // 5. East Face (+X)
                let east_block = world.get_block(wx + 1, wy, wz);
                let show_east = if is_water {
                    east_block == BlockId::Air
                } else {
                    !east_block.is_opaque() && (east_block != block || !block.is_cutout())
                };

                if show_east {
                    emit_face(
                        world,
                        target_mesh,
                        block,
                        BlockFace::East,
                        wx, wy, wz,
                        is_water,
                    );
                }

                // 6. West Face (-X)
                let west_block = world.get_block(wx - 1, wy, wz);
                let show_west = if is_water {
                    west_block == BlockId::Air
                } else {
                    !west_block.is_opaque() && (west_block != block || !block.is_cutout())
                };

                if show_west {
                    emit_face(
                        world,
                        target_mesh,
                        block,
                        BlockFace::West,
                        wx, wy, wz,
                        is_water,
                    );
                }
            }
        }
    }
}

fn emit_face(
    world: &VoxelWorld,
    mesh: &mut ForgeMesh,
    block: BlockId,
    face: BlockFace,
    x: i32,
    y: i32,
    z: i32,
    is_water: bool,
) {
    let (u0, v0, u1, v1) = get_block_face_uv_rect(block, face);

    let fx = x as f32;
    let fy = y as f32;
    let fz = z as f32;

    // Water level is slightly sunken on top face for natural look
    let top_y = if is_water && face == BlockFace::Top { fy + 0.88 } else { fy + 1.0 };

    let (pos, normal, ao_vals) = match face {
        BlockFace::Top => {
            let n = [0.0, 1.0, 0.0];
            let p0 = [fx, top_y, fz];
            let p1 = [fx + 1.0, top_y, fz];
            let p2 = [fx + 1.0, top_y, fz + 1.0];
            let p3 = [fx, top_y, fz + 1.0];

            let ao0 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y + 1, z),
                    world.is_opaque(x, y + 1, z - 1),
                    world.is_opaque(x - 1, y + 1, z - 1),
                )
            };
            let ao1 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y + 1, z),
                    world.is_opaque(x, y + 1, z - 1),
                    world.is_opaque(x + 1, y + 1, z - 1),
                )
            };
            let ao2 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y + 1, z),
                    world.is_opaque(x, y + 1, z + 1),
                    world.is_opaque(x + 1, y + 1, z + 1),
                )
            };
            let ao3 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y + 1, z),
                    world.is_opaque(x, y + 1, z + 1),
                    world.is_opaque(x - 1, y + 1, z + 1),
                )
            };

            ([p0, p1, p2, p3], n, [ao0, ao1, ao2, ao3])
        }
        BlockFace::Bottom => {
            let n = [0.0, -1.0, 0.0];
            let p0 = [fx, fy, fz + 1.0];
            let p1 = [fx + 1.0, fy, fz + 1.0];
            let p2 = [fx + 1.0, fy, fz];
            let p3 = [fx, fy, fz];

            let ao0 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y - 1, z),
                    world.is_opaque(x, y - 1, z + 1),
                    world.is_opaque(x - 1, y - 1, z + 1),
                )
            };
            let ao1 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y - 1, z),
                    world.is_opaque(x, y - 1, z + 1),
                    world.is_opaque(x + 1, y - 1, z + 1),
                )
            };
            let ao2 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y - 1, z),
                    world.is_opaque(x, y - 1, z - 1),
                    world.is_opaque(x + 1, y - 1, z - 1),
                )
            };
            let ao3 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y - 1, z),
                    world.is_opaque(x, y - 1, z - 1),
                    world.is_opaque(x - 1, y - 1, z - 1),
                )
            };

            ([p0, p1, p2, p3], n, [ao0, ao1, ao2, ao3])
        }
        BlockFace::North => {
            let n = [0.0, 0.0, 1.0];
            let p0 = [fx, fy, fz + 1.0];
            let p1 = [fx + 1.0, fy, fz + 1.0];
            let p2 = [fx + 1.0, top_y, fz + 1.0];
            let p3 = [fx, top_y, fz + 1.0];

            let ao0 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z + 1),
                    world.is_opaque(x, y - 1, z + 1),
                    world.is_opaque(x - 1, y - 1, z + 1),
                )
            };
            let ao1 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z + 1),
                    world.is_opaque(x, y - 1, z + 1),
                    world.is_opaque(x + 1, y - 1, z + 1),
                )
            };
            let ao2 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z + 1),
                    world.is_opaque(x, y + 1, z + 1),
                    world.is_opaque(x + 1, y + 1, z + 1),
                )
            };
            let ao3 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z + 1),
                    world.is_opaque(x, y + 1, z + 1),
                    world.is_opaque(x - 1, y + 1, z + 1),
                )
            };

            ([p0, p1, p2, p3], n, [ao0, ao1, ao2, ao3])
        }
        BlockFace::South => {
            let n = [0.0, 0.0, -1.0];
            let p0 = [fx + 1.0, fy, fz];
            let p1 = [fx, fy, fz];
            let p2 = [fx, top_y, fz];
            let p3 = [fx + 1.0, top_y, fz];

            let ao0 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z - 1),
                    world.is_opaque(x, y - 1, z - 1),
                    world.is_opaque(x + 1, y - 1, z - 1),
                )
            };
            let ao1 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z - 1),
                    world.is_opaque(x, y - 1, z - 1),
                    world.is_opaque(x - 1, y - 1, z - 1),
                )
            };
            let ao2 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z - 1),
                    world.is_opaque(x, y + 1, z - 1),
                    world.is_opaque(x - 1, y + 1, z - 1),
                )
            };
            let ao3 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z - 1),
                    world.is_opaque(x, y + 1, z - 1),
                    world.is_opaque(x + 1, y + 1, z - 1),
                )
            };

            ([p0, p1, p2, p3], n, [ao0, ao1, ao2, ao3])
        }
        BlockFace::East => {
            let n = [1.0, 0.0, 0.0];
            let p0 = [fx + 1.0, fy, fz + 1.0];
            let p1 = [fx + 1.0, fy, fz];
            let p2 = [fx + 1.0, top_y, fz];
            let p3 = [fx + 1.0, top_y, fz + 1.0];

            let ao0 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z + 1),
                    world.is_opaque(x + 1, y - 1, z),
                    world.is_opaque(x + 1, y - 1, z + 1),
                )
            };
            let ao1 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z - 1),
                    world.is_opaque(x + 1, y - 1, z),
                    world.is_opaque(x + 1, y - 1, z - 1),
                )
            };
            let ao2 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z - 1),
                    world.is_opaque(x + 1, y + 1, z),
                    world.is_opaque(x + 1, y + 1, z - 1),
                )
            };
            let ao3 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x + 1, y, z + 1),
                    world.is_opaque(x + 1, y + 1, z),
                    world.is_opaque(x + 1, y + 1, z + 1),
                )
            };

            ([p0, p1, p2, p3], n, [ao0, ao1, ao2, ao3])
        }
        BlockFace::West => {
            let n = [-1.0, 0.0, 0.0];
            let p0 = [fx, fy, fz];
            let p1 = [fx, fy, fz + 1.0];
            let p2 = [fx, top_y, fz + 1.0];
            let p3 = [fx, top_y, fz];

            let ao0 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z - 1),
                    world.is_opaque(x - 1, y - 1, z),
                    world.is_opaque(x - 1, y - 1, z - 1),
                )
            };
            let ao1 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z + 1),
                    world.is_opaque(x - 1, y - 1, z),
                    world.is_opaque(x - 1, y - 1, z + 1),
                )
            };
            let ao2 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z + 1),
                    world.is_opaque(x - 1, y + 1, z),
                    world.is_opaque(x - 1, y + 1, z + 1),
                )
            };
            let ao3 = if is_water { 1.0 } else {
                compute_vertex_ao(
                    world.is_opaque(x - 1, y, z - 1),
                    world.is_opaque(x - 1, y + 1, z),
                    world.is_opaque(x - 1, y + 1, z - 1),
                )
            };

            ([p0, p1, p2, p3], n, [ao0, ao1, ao2, ao3])
        }
    };

    let start_idx = mesh.positions.len() as u32;

    mesh.positions.push(pos[0]);
    mesh.positions.push(pos[1]);
    mesh.positions.push(pos[2]);
    mesh.positions.push(pos[3]);

    mesh.normals.push(normal);
    mesh.normals.push(normal);
    mesh.normals.push(normal);
    mesh.normals.push(normal);

    mesh.uvs.push([u0, v1]);
    mesh.uvs.push([u1, v1]);
    mesh.uvs.push([u1, v0]);
    mesh.uvs.push([u0, v0]);

    mesh.colors.push([ao_vals[0], ao_vals[0], ao_vals[0], 1.0]);
    mesh.colors.push([ao_vals[1], ao_vals[1], ao_vals[1], 1.0]);
    mesh.colors.push([ao_vals[2], ao_vals[2], ao_vals[2], 1.0]);
    mesh.colors.push([ao_vals[3], ao_vals[3], ao_vals[3], 1.0]);

    // Flip triangulation diagonal if needed to prevent AO interpolation distortion
    if ao_vals[0] + ao_vals[2] < ao_vals[1] + ao_vals[3] {
        mesh.indices.push(start_idx);
        mesh.indices.push(start_idx + 1);
        mesh.indices.push(start_idx + 2);

        mesh.indices.push(start_idx);
        mesh.indices.push(start_idx + 2);
        mesh.indices.push(start_idx + 3);
    } else {
        mesh.indices.push(start_idx + 1);
        mesh.indices.push(start_idx + 2);
        mesh.indices.push(start_idx + 3);

        mesh.indices.push(start_idx + 1);
        mesh.indices.push(start_idx + 3);
        mesh.indices.push(start_idx);
    }
}

// ---------------------------------------------------------------------------
// Procedural World Generator
// ---------------------------------------------------------------------------

pub fn generate_voxel_world(
    width_chunks: usize,
    depth_chunks: usize,
    seed: u64,
    _terrain_type: &str,
) -> VoxelWorld {
    let mut world = VoxelWorld::new();
    let seed_f = seed as f32 * 0.1718;

    let w_chunks = width_chunks.max(1);
    let d_chunks = depth_chunks.max(1);

    for cz in 0..d_chunks as i32 {
        for cx in 0..w_chunks as i32 {
            world.chunks.insert((cx, cz), Chunk::new(cx, cz));
        }
    }
    world.min_chunk = [0, 0];
    world.max_chunk = [(w_chunks - 1) as i32, (d_chunks - 1) as i32];

    let total_w = (w_chunks * CHUNK_SIZE_X) as i32;
    let total_d = (d_chunks * CHUNK_SIZE_Z) as i32;

    // 1. Terrain Base Height & Layers
    for z in 0..total_d {
        for x in 0..total_w {
            let fx = x as f32;
            let fz = z as f32;

            // Multi-octave sinusoidal terrain noise
            let n1 = (fx * 0.06 + seed_f).sin() * (fz * 0.06 + seed_f * 0.7).cos();
            let n2 = (fx * 0.14 - seed_f * 0.4).cos() * (fz * 0.12 + seed_f * 0.3).sin();
            let n3 = (fx * 0.28 + fz * 0.28).sin() * 0.5;

            let combined_noise = (n1 * 0.6 + n2 * 0.3 + n3 * 0.1) * 0.5 + 0.5; // 0..1
            let height = 6 + (combined_noise * 18.0) as i32;
            let height = height.clamp(4, (CHUNK_SIZE_Y - 5) as i32);

            // Bedrock at y = 0
            world.set_block(x, 0, z, BlockId::Bedrock);

            // Stone layer
            let stone_top = (height - 3).max(1);
            for y in 1..=stone_top {
                // Simple 3D cave carving
                let cave_n = ((fx * 0.25).sin() * (y as f32 * 0.35).cos() * (fz * 0.25).sin()).abs();
                if cave_n > 0.75 && y > 2 && y < stone_top - 1 {
                    world.set_block(x, y, z, BlockId::Air);
                } else {
                    world.set_block(x, y, z, BlockId::Stone);
                }
            }

            // Dirt layer
            for y in (stone_top + 1)..height {
                world.set_block(x, y, z, BlockId::Dirt);
            }

            // Surface block
            if height <= SEA_LEVEL {
                world.set_block(x, height, z, BlockId::Sand);
                // Water from height+1 up to SEA_LEVEL
                for wy in (height + 1)..=SEA_LEVEL {
                    world.set_block(x, wy, z, BlockId::Water);
                }
            } else {
                world.set_block(x, height, z, BlockId::Grass);
            }
        }
    }

    // 2. Oak Trees placement
    for z in 4..(total_d - 4) {
        for x in 4..(total_w - 4) {
            // Pseudo-random deterministic tree placement
            let tree_hash = ((x * 7919 + z * 6271 + (seed as i32 * 31)) % 100).abs();
            if tree_hash < 3 {
                // Find surface y
                let mut surf_y = -1;
                for y in (SEA_LEVEL + 1)..(CHUNK_SIZE_Y as i32 - 7) {
                    if world.get_block(x, y, z) == BlockId::Grass && world.get_block(x, y + 1, z) == BlockId::Air {
                        surf_y = y;
                        break;
                    }
                }

                if surf_y > SEA_LEVEL {
                    let trunk_h = 4 + (tree_hash % 2);

                    // Trunk
                    for ty in 1..=trunk_h {
                        world.set_block(x, surf_y + ty, z, BlockId::Wood);
                    }

                    // Leaves canopy
                    let leaf_center_y = surf_y + trunk_h;
                    for ldy in -1..=1 {
                        let radius: i32 = if ldy == 1 { 1 } else { 2 };
                        for ldz in -radius..=radius {
                            for ldx in -radius..=radius {
                                if ldy == 1 && (ldx.abs() == 2 && ldz.abs() == 2) {
                                    continue;
                                }
                                let lx = x + ldx;
                                let ly = leaf_center_y + ldy;
                                let lz = z + ldz;
                                if world.get_block(lx, ly, lz) == BlockId::Air {
                                    world.set_block(lx, ly, lz, BlockId::Leaves);
                                }
                            }
                        }
                    }
                    // Very top leaves
                    world.set_block(x, leaf_center_y + 2, z, BlockId::Leaves);
                    world.set_block(x + 1, leaf_center_y + 2, z, BlockId::Leaves);
                    world.set_block(x - 1, leaf_center_y + 2, z, BlockId::Leaves);
                    world.set_block(x, leaf_center_y + 2, z + 1, BlockId::Leaves);
                    world.set_block(x, leaf_center_y + 2, z - 1, BlockId::Leaves);
                }
            }
        }
    }

    world
}

// ---------------------------------------------------------------------------
// Scene Sync and Persistence Helpers
// ---------------------------------------------------------------------------

pub fn voxels_path_for_model(model_name_or_path: &str) -> String {
    let clean = model_name_or_path
        .strip_suffix(".glb")
        .unwrap_or(model_name_or_path);
    format!("{}.voxels.json", clean)
}

pub fn sync_active_voxel_world_from_scene(
    scene: &crate::assets::SceneDef,
    active_world: &std::sync::Arc<std::sync::RwLock<Option<VoxelWorld>>>,
) {
    for entity in &scene.entities {
        let path = voxels_path_for_model(&entity.mesh);
        if Path::new(&path).exists() {
            if let Ok(w) = VoxelWorld::load_from_json(&path) {
                if let Ok(mut lock) = active_world.write() {
                    *lock = Some(w);
                }
                return;
            }
        }
    }

    // Fallback: check if assets/models/voxel_world.voxels.json exists
    let default_path = "assets/models/voxel_world.voxels.json";
    if Path::new(default_path).exists() {
        if let Ok(w) = VoxelWorld::load_from_json(default_path) {
            if let Ok(mut lock) = active_world.write() {
                *lock = Some(w);
            }
        }
    }
}
