use std::f32::consts::PI;
use std::io::Cursor;
use std::path::Path;
use image::{ImageFormat, Rgba, RgbaImage};

// ----------------------------------------------------------------------------
// Noise Primitives (Deterministic Pure Rust)
// ----------------------------------------------------------------------------

#[inline]
pub fn hash2d(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x85ebca6b)
        ^ (y as u32).wrapping_mul(0xc2b2ae35)
        ^ seed;
    h = (h ^ (h >> 16)).wrapping_mul(0x7feb352d);
    h = (h ^ (h >> 15)).wrapping_mul(0x846ca68b);
    h = h ^ (h >> 16);
    (h as f32) / (u32::MAX as f32)
}

pub fn value_noise_2d(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);

    let n00 = hash2d(xi, yi, seed);
    let n10 = hash2d(xi + 1, yi, seed);
    let n01 = hash2d(xi, yi + 1, seed);
    let n11 = hash2d(xi + 1, yi + 1, seed);

    let nx0 = n00 * (1.0 - u) + n10 * u;
    let nx1 = n01 * (1.0 - u) + n11 * u;
    nx0 * (1.0 - v) + nx1 * v
}

pub fn fbm_2d(x: f32, y: f32, octaves: usize, persistence: f32, lacunarity: f32, seed: u32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_val = 0.0;

    for i in 0..octaves {
        total += value_noise_2d(x * frequency, y * frequency, seed.wrapping_add((i * 1013) as u32)) * amplitude;
        max_val += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }
    total / max_val.max(1e-4)
}

pub fn worley_2d(x: f32, y: f32, seed: u32) -> (f32, f32) {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let mut f1 = 1000.0f32;
    let mut f2 = 1000.0f32;

    for dy in -1..=1 {
        for dx in -1..=1 {
            let cell_x = xi + dx;
            let cell_y = yi + dy;
            let p_x = cell_x as f32 + hash2d(cell_x, cell_y, seed);
            let p_y = cell_y as f32 + hash2d(cell_x, cell_y, seed.wrapping_add(777));
            let dist = ((p_x - x).powi(2) + (p_y - y).powi(2)).sqrt();
            if dist < f1 {
                f2 = f1;
                f1 = dist;
            } else if dist < f2 {
                f2 = dist;
            }
        }
    }
    (f1, f2)
}

// ----------------------------------------------------------------------------
// Procedural Style Generators
// ----------------------------------------------------------------------------

fn lerp_color(c0: [f32; 4], c1: [f32; 4], t: f32) -> [f32; 4] {
    let factor = t.clamp(0.0, 1.0);
    [
        c0[0] + (c1[0] - c0[0]) * factor,
        c0[1] + (c1[1] - c0[1]) * factor,
        c0[2] + (c1[2] - c0[2]) * factor,
        c0[3] + (c1[3] - c0[3]) * factor,
    ]
}

/// Generates a planetary surface texture (bands, turbulence, craters, spherical lighting)
pub fn generate_planet(width: u32, height: u32, seed: u32, tint: [f32; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let inv_w = 1.0 / width as f32;
    let inv_h = 1.0 / height as f32;

    let deep_color = [tint[0] * 0.15, tint[1] * 0.15, tint[2] * 0.25, 1.0];
    let mid_color = tint;
    let high_color = [
        (tint[0] * 1.5).min(1.0),
        (tint[1] * 1.5).min(1.0),
        (tint[2] * 1.5).min(1.0),
        1.0,
    ];
    let cloud_color = [1.0, 1.0, 1.0, 1.0];

    for y in 0..height {
        let v = y as f32 * inv_h;
        for x in 0..width {
            let u = x as f32 * inv_w;

            // Equirectangular / spherical seamless noise coordinates
            let nx = (u * 2.0 * PI).cos();
            let nz = (u * 2.0 * PI).sin();
            let ny = v * 2.0 - 1.0;

            // Atmospheric bands with sinusoidal turbulence
            let turbulence = fbm_2d(nx * 3.0 + 10.0, ny * 6.0 + 10.0, 4, 0.5, 2.0, seed);
            let band = (ny * 12.0 + turbulence * 6.0).sin() * 0.5 + 0.5;

            // Surface terrain / craters via Worley
            let (w1, _) = worley_2d(nx * 8.0 + 20.0, (ny + nz) * 8.0 + 20.0, seed.wrapping_add(55));
            let crater_rim = if w1 < 0.35 {
                (1.0 - (w1 - 0.25).abs() * 10.0).clamp(0.0, 1.0) * 0.3
            } else {
                0.0
            };

            // Cloud swirl layer
            let clouds = fbm_2d(nx * 5.0 + 33.0, (ny + turbulence * 0.3) * 5.0, 4, 0.55, 2.1, seed.wrapping_add(199));
            let cloud_mask = ((clouds - 0.52) * 3.5).clamp(0.0, 1.0);

            // Blend colors
            let base_t = (band * 0.7 + turbulence * 0.3 + crater_rim).clamp(0.0, 1.0);
            let mut col = if base_t < 0.5 {
                lerp_color(deep_color, mid_color, base_t * 2.0)
            } else {
                lerp_color(mid_color, high_color, (base_t - 0.5) * 2.0)
            };

            col = lerp_color(col, cloud_color, cloud_mask * 0.85);

            img.put_pixel(
                x,
                y,
                Rgba([
                    (col[0] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[1] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[2] * 255.0).clamp(0.0, 255.0) as u8,
                    255,
                ]),
            );
        }
    }
    img
}

/// Generates sci-fi metallic paneling with beveled seams, corner rivets, and neon light circuits
pub fn generate_scifi_panel(width: u32, height: u32, seed: u32, tint: [f32; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let cells_x = 4u32;
    let cells_y = 4u32;
    let cell_w = width / cells_x;
    let cell_h = height / cells_y;

    let base_plate = [0.12, 0.14, 0.18, 1.0];
    let highlight_edge = [0.28, 0.32, 0.38, 1.0];
    let seam_dark = [0.03, 0.04, 0.05, 1.0];
    let rivet_color = [0.45, 0.48, 0.55, 1.0];
    let neon_color = tint;

    for y in 0..height {
        let cy = y / cell_h;
        let ly = (y % cell_h) as f32 / cell_h as f32; // 0..1 inside cell

        for x in 0..width {
            let cx = x / cell_w;
            let lx = (x % cell_w) as f32 / cell_w as f32; // 0..1 inside cell

            // Seam distance from border (0 at border, 0.5 at center)
            let border_dist_x = lx.min(1.0 - lx);
            let border_dist_y = ly.min(1.0 - ly);
            let border_dist = border_dist_x.min(border_dist_y);

            // Fine brushed metal noise
            let brushed = (hash2d(x as i32, y as i32, seed) - 0.5) * 0.04;

            // Bevel edge logic
            let mut col = if border_dist < 0.02 {
                seam_dark
            } else if border_dist < 0.06 {
                let t = (border_dist - 0.02) / 0.04;
                lerp_color(highlight_edge, base_plate, t)
            } else {
                base_plate
            };

            // Rivets at corners: inset by 0.12
            let rivet_radius = 0.035;
            let corner_offsets = [(0.12, 0.12), (0.88, 0.12), (0.12, 0.88), (0.88, 0.88)];
            for (rx, ry) in corner_offsets {
                let dist = ((lx - rx).powi(2) + (ly - ry).powi(2)).sqrt();
                if dist < rivet_radius {
                    let r_shade = 1.0 - (dist / rivet_radius).powf(2.0) * 0.5;
                    col = [rivet_color[0] * r_shade, rivet_color[1] * r_shade, rivet_color[2] * r_shade, 1.0];
                    break;
                }
            }

            // Neon accent strip running across horizontal or vertical borders on select panels
            let is_neon_line = (cx + cy) % 3 == 0 && (border_dist_y > 0.015 && border_dist_y < 0.035);
            if is_neon_line {
                col = lerp_color(col, neon_color, 0.9);
            }

            col[0] = (col[0] + brushed).clamp(0.0, 1.0);
            col[1] = (col[1] + brushed).clamp(0.0, 1.0);
            col[2] = (col[2] + brushed).clamp(0.0, 1.0);

            img.put_pixel(
                x,
                y,
                Rgba([
                    (col[0] * 255.0) as u8,
                    (col[1] * 255.0) as u8,
                    (col[2] * 255.0) as u8,
                    255,
                ]),
            );
        }
    }
    img
}

/// Generates turbulent energetic plasma swirls with multi-octave domain warping
pub fn generate_plasma(width: u32, height: u32, seed: u32, tint: [f32; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let inv_w = 1.0 / width as f32;
    let inv_h = 1.0 / height as f32;

    let dark_core = [0.03, 0.01, 0.05, 1.0];
    let mid_plasma = tint;
    let hot_glow = [
        (tint[0] * 2.0).min(1.0),
        (tint[1] * 2.0).min(1.0),
        (tint[2] * 2.0).min(1.0),
        1.0,
    ];
    let core_white = [1.0, 1.0, 1.0, 1.0];

    for y in 0..height {
        let v = y as f32 * inv_h * 4.0;
        for x in 0..width {
            let u = x as f32 * inv_w * 4.0;

            // Domain warping (turbulent flow)
            let qx = fbm_2d(u, v, 3, 0.5, 2.0, seed);
            let qy = fbm_2d(u + 5.2, v + 1.3, 3, 0.5, 2.0, seed.wrapping_add(13));

            let rx = fbm_2d(u + 4.0 * qx + 1.7, v + 4.0 * qy + 9.2, 3, 0.5, 2.0, seed.wrapping_add(27));
            let ry = fbm_2d(u + 4.0 * qx + 8.3, v + 4.0 * qy + 2.8, 3, 0.5, 2.0, seed.wrapping_add(41));

            let val = fbm_2d(u + 4.0 * rx, v + 4.0 * ry, 4, 0.5, 2.0, seed.wrapping_add(99));

            // Plasma gradient mapping
            let col = if val < 0.35 {
                lerp_color(dark_core, mid_plasma, val / 0.35)
            } else if val < 0.75 {
                lerp_color(mid_plasma, hot_glow, (val - 0.35) / 0.40)
            } else {
                lerp_color(hot_glow, core_white, (val - 0.75) / 0.25)
            };

            img.put_pixel(
                x,
                y,
                Rgba([
                    (col[0] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[1] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[2] * 255.0).clamp(0.0, 255.0) as u8,
                    255,
                ]),
            );
        }
    }
    img
}

/// Generates cosmic synthwave neon gridlines with glowing intersection nodes
pub fn generate_grid_neon(width: u32, height: u32, seed: u32, tint: [f32; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let grid_step = 32u32;
    let inv_w = 1.0 / width as f32;
    let inv_h = 1.0 / height as f32;

    let dark_floor = [0.03, 0.04, 0.06, 1.0];
    let neon_color = tint;
    let node_white = [1.0, 1.0, 1.0, 1.0];

    for y in 0..height {
        let dist_y = ((y % grid_step) as i32 - 0).abs().min(((y % grid_step) as i32 - grid_step as i32).abs()) as f32;
        let u_v = y as f32 * inv_h;

        for x in 0..width {
            let dist_x = ((x % grid_step) as i32 - 0).abs().min(((x % grid_step) as i32 - grid_step as i32).abs()) as f32;
            let u_u = x as f32 * inv_w;

            // Distance to closest grid line
            let line_dist = dist_x.min(dist_y);
            let line_glow = (1.0 - (line_dist / 4.0)).clamp(0.0, 1.0).powf(2.0);

            // Intersection point distance
            let node_dist = (dist_x * dist_x + dist_y * dist_y).sqrt();
            let node_glow = (1.0 - (node_dist / 6.0)).clamp(0.0, 1.0).powf(1.8);

            // Faint background circuit texture
            let bg_circuit = fbm_2d(u_u * 8.0, u_v * 8.0, 2, 0.5, 2.0, seed) * 0.06;

            let mut col = dark_floor;
            col[0] += bg_circuit;
            col[1] += bg_circuit;
            col[2] += bg_circuit;

            col = lerp_color(col, neon_color, line_glow * 0.95);
            col = lerp_color(col, node_white, node_glow * 0.9);

            img.put_pixel(
                x,
                y,
                Rgba([
                    (col[0] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[1] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[2] * 255.0).clamp(0.0, 255.0) as u8,
                    255,
                ]),
            );
        }
    }
    img
}

/// Generates craggy rock / asteroid surface with multi-octave fBm and fracture lines
pub fn generate_rock(width: u32, height: u32, seed: u32, tint: [f32; 4]) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let inv_w = 1.0 / width as f32;
    let inv_h = 1.0 / height as f32;

    let shadow_color = [0.06, 0.06, 0.07, 1.0];
    let rock_base = tint;
    let highlight_color = [
        (tint[0] * 1.6).min(1.0),
        (tint[1] * 1.6).min(1.0),
        (tint[2] * 1.6).min(1.0),
        1.0,
    ];

    for y in 0..height {
        let v = y as f32 * inv_h * 6.0;
        for x in 0..width {
            let u = x as f32 * inv_w * 6.0;

            // Multi-octave fBm for rock surface height
            let height_val = fbm_2d(u, v, 6, 0.55, 2.05, seed);

            // Finite difference slopes for directional pseudo-normal shading
            let eps = 0.05;
            let h_r = fbm_2d(u + eps, v, 6, 0.55, 2.05, seed);
            let h_u = fbm_2d(u, v + eps, 6, 0.55, 2.05, seed);
            let slope = (h_r - height_val) * 0.707 + (h_u - height_val) * 0.707;
            let light = (0.5 + slope * 3.5).clamp(0.0, 1.0);

            // Worley fracture lines
            let (w1, w2) = worley_2d(u * 1.5, v * 1.5, seed.wrapping_add(88));
            let crack = ((w2 - w1) * 8.0).clamp(0.0, 1.0);

            let shaded = height_val * light * crack;

            let col = if shaded < 0.4 {
                lerp_color(shadow_color, rock_base, shaded / 0.4)
            } else {
                lerp_color(rock_base, highlight_color, (shaded - 0.4) / 0.6)
            };

            img.put_pixel(
                x,
                y,
                Rgba([
                    (col[0] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[1] * 255.0).clamp(0.0, 255.0) as u8,
                    (col[2] * 255.0).clamp(0.0, 255.0) as u8,
                    255,
                ]),
            );
        }
    }
    img
}

// ----------------------------------------------------------------------------
// Public API Functions
// ----------------------------------------------------------------------------

/// Generates a procedural texture as an `RgbaImage` based on style name
pub fn generate_texture_image(
    style: &str,
    width: u32,
    height: u32,
    seed: u32,
    color_tint: [f32; 4],
) -> RgbaImage {
    let w = width.max(16);
    let h = height.max(16);
    let s = style.to_lowercase();

    match s.as_str() {
        "planet" => generate_planet(w, h, seed, color_tint),
        "scifi_panel" | "panel" => generate_scifi_panel(w, h, seed, color_tint),
        "plasma" => generate_plasma(w, h, seed, color_tint),
        "grid_neon" | "grid" | "neon" => generate_grid_neon(w, h, seed, color_tint),
        "rock" | "asteroid" => generate_rock(w, h, seed, color_tint),
        _ => generate_plasma(w, h, seed, color_tint),
    }
}

/// Generates a procedural texture and returns the encoded PNG bytes in memory
pub fn generate_texture(
    style: &str,
    width: u32,
    height: u32,
    seed: u32,
    color_tint: [f32; 4],
) -> Result<Vec<u8>, String> {
    let img = generate_texture_image(style, width, height, seed, color_tint);
    let mut bytes = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    img.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode texture as PNG: {}", e))?;
    Ok(bytes)
}

/// Saves a procedural texture to `assets/textures/<filename>.png`
pub fn save_procedural_texture(
    style: &str,
    filename: &str,
    size: u32,
    seed: u32,
    color_tint: [f32; 4],
) -> Result<String, String> {
    let clean_name = if filename.ends_with(".png") {
        filename.to_string()
    } else {
        format!("{}.png", filename)
    };

    let dir = Path::new("assets/textures");
    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create textures dir: {}", e))?;
    }

    let full_path = dir.join(&clean_name);
    let img = generate_texture_image(style, size, size, seed, color_tint);

    img.save_with_format(&full_path, ImageFormat::Png)
        .map_err(|e| format!("Failed to save texture '{}': {}", full_path.display(), e))?;

    Ok(full_path.to_string_lossy().to_string().replace('\\', "/"))
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_determinism_and_bounds() {
        let val1 = hash2d(15, 28, 42);
        let val2 = hash2d(15, 28, 42);
        assert_eq!(val1, val2);
        assert!(val1 >= 0.0 && val1 <= 1.0);

        let noise = fbm_2d(2.5, 3.7, 4, 0.5, 2.0, 99);
        assert!(noise >= 0.0 && noise <= 1.0);

        let (f1, f2) = worley_2d(1.5, 2.5, 77);
        assert!(f1 <= f2);
    }

    #[test]
    fn test_all_texture_forge_styles_generate_valid_png() {
        let styles = ["planet", "scifi_panel", "plasma", "grid_neon", "rock"];
        for style in styles {
            let bytes = generate_texture(style, 64, 64, 123, [0.3, 0.7, 0.9, 1.0])
                .expect(&format!("Failed to generate style '{}'", style));
            assert!(bytes.len() > 100);
            // Verify PNG magic bytes: 0x89 0x50 0x4E 0x47 0x0D 0x0A 0x1A 0x0A
            assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        }
    }

    #[test]
    fn test_save_procedural_texture() {
        let test_file = "test_forge_sample";
        let path_res = save_procedural_texture("planet", test_file, 64, 555, [0.2, 0.6, 0.8, 1.0]);
        assert!(path_res.is_ok());
        let path = path_res.unwrap();
        assert!(Path::new(&path).exists());
        let _ = std::fs::remove_file(path);
    }
}
