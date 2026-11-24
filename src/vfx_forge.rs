use std::f32::consts::PI;
use glam::Vec3;
use serde::{Deserialize, Serialize};

// ============================================================================
// 1. VFX RECIPE DATA STRUCTURE
// ============================================================================

fn default_emitter_shape() -> String { "point".to_string() }
fn default_shape_radius() -> f32 { 0.2 }
fn default_burst_count() -> usize { 30 }
fn default_rate_per_sec() -> f32 { 0.0 }
fn default_lifetime_min() -> f32 { 0.5 }
fn default_lifetime_max() -> f32 { 1.2 }
fn default_speed_min() -> f32 { 2.0 }
fn default_speed_max() -> f32 { 5.0 }
fn default_direction() -> [f32; 3] { [0.0, 1.0, 0.0] }
fn default_spread_deg() -> f32 { 45.0 }
fn default_size_start() -> f32 { 0.2 }
fn default_size_end() -> f32 { 0.04 }
fn default_color_start() -> [f32; 4] { [1.0, 0.8, 0.3, 1.0] }
fn default_color_end() -> [f32; 4] { [1.0, 0.2, 0.0, 0.0] }
fn default_emissive_boost() -> f32 { 2.5 }
fn default_gravity() -> [f32; 3] { [0.0, -9.81, 0.0] }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VfxRecipe {
    pub name: String,

    #[serde(default = "default_emitter_shape")]
    pub emitter_shape: String, // "point" | "sphere" | "ring" | "cone"

    #[serde(default = "default_shape_radius")]
    pub shape_radius: f32,

    #[serde(default = "default_burst_count")]
    pub burst_count: usize,

    #[serde(default = "default_rate_per_sec")]
    pub rate_per_sec: f32,

    #[serde(default = "default_lifetime_min")]
    pub lifetime_min: f32,

    #[serde(default = "default_lifetime_max")]
    pub lifetime_max: f32,

    #[serde(default = "default_speed_min")]
    pub speed_min: f32,

    #[serde(default = "default_speed_max")]
    pub speed_max: f32,

    #[serde(default = "default_direction")]
    pub direction: [f32; 3],

    #[serde(default = "default_spread_deg")]
    pub spread_deg: f32,

    #[serde(default = "default_size_start")]
    pub size_start: f32,

    #[serde(default = "default_size_end")]
    pub size_end: f32,

    #[serde(default = "default_color_start")]
    pub color_start: [f32; 4],

    #[serde(default = "default_color_end")]
    pub color_end: [f32; 4],

    #[serde(default = "default_emissive_boost")]
    pub emissive_boost: f32,

    #[serde(default = "default_gravity")]
    pub gravity: [f32; 3],

    #[serde(default)]
    pub drag: f32,

    #[serde(default)]
    pub vortex_strength: f32,
}

impl Default for VfxRecipe {
    fn default() -> Self {
        Self {
            name: "default_vfx".to_string(),
            emitter_shape: default_emitter_shape(),
            shape_radius: default_shape_radius(),
            burst_count: default_burst_count(),
            rate_per_sec: default_rate_per_sec(),
            lifetime_min: default_lifetime_min(),
            lifetime_max: default_lifetime_max(),
            speed_min: default_speed_min(),
            speed_max: default_speed_max(),
            direction: default_direction(),
            spread_deg: default_spread_deg(),
            size_start: default_size_start(),
            size_end: default_size_end(),
            color_start: default_color_start(),
            color_end: default_color_end(),
            emissive_boost: default_emissive_boost(),
            gravity: default_gravity(),
            drag: 0.0,
            vortex_strength: 0.0,
        }
    }
}

// ============================================================================
// 2. CORE VFX PRESETS
// ============================================================================

pub fn preset_explosion() -> VfxRecipe {
    VfxRecipe {
        name: "explosion".to_string(),
        emitter_shape: "sphere".to_string(),
        shape_radius: 0.35,
        burst_count: 90,
        rate_per_sec: 0.0,
        lifetime_min: 0.6,
        lifetime_max: 1.4,
        speed_min: 5.0,
        speed_max: 12.0,
        direction: [0.0, 1.0, 0.0],
        spread_deg: 180.0, // Full radial sphere
        size_start: 0.38,
        size_end: 0.06,
        color_start: [1.0, 0.85, 0.2, 1.0], // Intense hot yellow-orange
        color_end: [0.25, 0.06, 0.02, 0.0], // Dark smoky ember fading out
        emissive_boost: 3.5,                // Triggers HDR Bloom
        gravity: [0.0, -4.0, 0.0],
        drag: 2.2,                          // Aerodynamic deceleration
        vortex_strength: 0.0,
    }
}

pub fn preset_fire() -> VfxRecipe {
    VfxRecipe {
        name: "fire".to_string(),
        emitter_shape: "cone".to_string(),
        shape_radius: 0.25,
        burst_count: 0,
        rate_per_sec: 45.0, // Continuous fire emitter
        lifetime_min: 0.45,
        lifetime_max: 0.95,
        speed_min: 1.5,
        speed_max: 3.5,
        direction: [0.0, 1.0, 0.0],
        spread_deg: 25.0,
        size_start: 0.24,
        size_end: 0.04,
        color_start: [1.0, 0.9, 0.3, 0.95], // Golden core
        color_end: [0.85, 0.15, 0.0, 0.0],  // Flame tip red fade
        emissive_boost: 2.8,
        gravity: [0.0, 3.2, 0.0],           // Positive convective lift
        drag: 0.8,
        vortex_strength: 0.6,               // Light flame flicker
    }
}

pub fn preset_magic_portal() -> VfxRecipe {
    VfxRecipe {
        name: "magic_portal".to_string(),
        emitter_shape: "ring".to_string(),
        shape_radius: 1.6,
        burst_count: 0,
        rate_per_sec: 65.0,
        lifetime_min: 0.8,
        lifetime_max: 1.6,
        speed_min: 0.8,
        speed_max: 2.2,
        direction: [0.0, 0.0, 1.0],
        spread_deg: 35.0,
        size_start: 0.16,
        size_end: 0.02,
        color_start: [0.2, 0.9, 1.0, 0.9],  // Electric Cyan HDR
        color_end: [0.75, 0.2, 1.0, 0.0],   // Deep Violet/Magenta
        emissive_boost: 4.0,                 // Intense cosmic glow
        gravity: [0.0, 0.0, 0.0],            // Weightless floating
        drag: 0.4,
        vortex_strength: 6.8,                // High spiral swirl around the ring
    }
}

pub fn preset_hit_spark() -> VfxRecipe {
    VfxRecipe {
        name: "hit_spark".to_string(),
        emitter_shape: "point".to_string(),
        shape_radius: 0.05,
        burst_count: 35,
        rate_per_sec: 0.0,
        lifetime_min: 0.2,
        lifetime_max: 0.5,
        speed_min: 6.0,
        speed_max: 15.0,
        direction: [0.0, 1.0, 0.0],
        spread_deg: 80.0,
        size_start: 0.08,
        size_end: 0.01,
        color_start: [1.0, 0.98, 0.75, 1.0], // White-hot electric spark
        color_end: [1.0, 0.4, 0.0, 0.0],     // Orange cooling ember
        emissive_boost: 3.2,
        gravity: [0.0, -14.0, 0.0],          // Rapid gravitational fall
        drag: 3.0,                           // Fast spark deceleration
        vortex_strength: 0.0,
    }
}

pub fn preset_level_up() -> VfxRecipe {
    VfxRecipe {
        name: "level_up".to_string(),
        emitter_shape: "ring".to_string(),
        shape_radius: 1.1,
        burst_count: 75,
        rate_per_sec: 0.0,
        lifetime_min: 1.0,
        lifetime_max: 2.2,
        speed_min: 2.5,
        speed_max: 5.5,
        direction: [0.0, 1.0, 0.0],
        spread_deg: 18.0,
        size_start: 0.18,
        size_end: 0.03,
        color_start: [1.0, 0.85, 0.2, 0.95], // Radiant golden pillar
        color_end: [0.4, 1.0, 0.6, 0.0],     // Emerald sparkle fade
        emissive_boost: 3.5,
        gravity: [0.0, 1.5, 0.0],            // Ascending lift
        drag: 0.5,
        vortex_strength: 3.4,                // Elegant ascending spiral
    }
}

pub fn preset_smoke() -> VfxRecipe {
    VfxRecipe {
        name: "smoke".to_string(),
        emitter_shape: "point".to_string(),
        shape_radius: 0.25,
        burst_count: 0,
        rate_per_sec: 25.0,
        lifetime_min: 1.5,
        lifetime_max: 2.8,
        speed_min: 0.8,
        speed_max: 2.0,
        direction: [0.0, 1.0, 0.0],
        spread_deg: 40.0,
        size_start: 0.18,
        size_end: 0.65,
        color_start: [0.35, 0.35, 0.35, 0.6],
        color_end: [0.1, 0.1, 0.1, 0.0],
        emissive_boost: 0.0,
        gravity: [0.0, 0.4, 0.0], // Gentle buoyant rising
        drag: 1.8,
        vortex_strength: 0.5,
    }
}

pub fn preset_rain() -> VfxRecipe {
    VfxRecipe {
        name: "rain".to_string(),
        emitter_shape: "box".to_string(),
        shape_radius: 12.0,
        burst_count: 0,
        rate_per_sec: 120.0,
        lifetime_min: 0.6,
        lifetime_max: 0.9,
        speed_min: 22.0,
        speed_max: 28.0,
        direction: [0.0, -1.0, 0.0],
        spread_deg: 3.0,
        size_start: 0.06,
        size_end: 0.05,
        color_start: [0.75, 0.88, 1.0, 0.8],
        color_end: [0.6, 0.8, 1.0, 0.1],
        emissive_boost: 0.5,
        gravity: [0.0, -9.81, 0.0],
        drag: 0.0,
        vortex_strength: 0.0,
    }
}

pub fn get_vfx_preset(name: &str) -> Option<VfxRecipe> {
    match name.to_lowercase().as_str() {
        "explosion" => Some(preset_explosion()),
        "fire" => Some(preset_fire()),
        "smoke" => Some(preset_smoke()),
        "sparks" | "spark" | "hit_spark" => Some(preset_hit_spark()),
        "rain" => Some(preset_rain()),
        "magic_portal" | "portal" => Some(preset_magic_portal()),
        "level_up" | "levelup" => Some(preset_level_up()),
        _ => None,
    }
}

pub fn list_vfx_presets() -> Vec<&'static str> {
    vec!["explosion", "fire", "magic_portal", "hit_spark", "level_up", "smoke", "rain"]
}

// ============================================================================
// 2.1. RUNTIME VFX EMITTER COMPONENT
// ============================================================================

#[derive(Debug, Clone)]
pub struct VfxEmitterComponent {
    pub recipe: VfxRecipe,
    pub enabled: bool,
    pub looping: bool,
    pub offset: [f32; 3],
    pub accumulator: f32,
    pub burst_fired: bool,
}

impl VfxEmitterComponent {
    pub fn new(recipe: VfxRecipe) -> Self {
        Self {
            recipe,
            enabled: true,
            looping: true,
            offset: [0.0, 0.0, 0.0],
            accumulator: 0.0,
            burst_fired: false,
        }
    }

    pub fn from_preset(name: &str) -> Option<Self> {
        get_vfx_preset(name).map(Self::new)
    }
}

// ============================================================================
// 3. DETERMINISTIC RANDOM SAMPLER FOR PARTICLE SPAWNING
// ============================================================================

/// Fast LCG float in [0.0, 1.0)
#[inline]
pub fn next_float(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    ((*seed >> 8) & 0x00ffffff) as f32 / 16777216.0
}

/// Fast LCG float in [-1.0, 1.0]
#[inline]
pub fn next_bilateral(seed: &mut u32) -> f32 {
    next_float(seed) * 2.0 - 1.0
}

/// Generates a single particle's initial position offset, velocity, and lifetime from a VfxRecipe.
/// Guaranteed free of NaN and zero division.
pub fn sample_particle_spawn(
    recipe: &VfxRecipe,
    origin: Vec3,
    seed: &mut u32,
) -> (Vec3, Vec3, f32) {
    let r = recipe.shape_radius.max(0.001);

    // 1. Position offset by emitter shape
    let offset = match recipe.emitter_shape.to_lowercase().as_str() {
        "sphere" => {
            let u = next_bilateral(seed);
            let v = next_bilateral(seed);
            let w = next_bilateral(seed);
            let mut p = Vec3::new(u, v, w);
            if p.length_squared() < 0.0001 {
                p = Vec3::Y;
            } else {
                p = p.normalize();
            }
            let rad = r * next_float(seed).cbrt();
            p * rad
        }
        "ring" => {
            let theta = next_float(seed) * 2.0 * PI;
            Vec3::new(theta.cos() * r, 0.0, theta.sin() * r)
        }
        "cone" => {
            let theta = next_float(seed) * 2.0 * PI;
            let rad = r * next_float(seed).sqrt();
            Vec3::new(theta.cos() * rad, 0.0, theta.sin() * rad)
        }
        _ => {
            // "point" or unknown
            Vec3::ZERO
        }
    };
    let spawn_pos = origin + offset;

    // 2. Velocity and direction with spread
    let base_dir = {
        let d = Vec3::from(recipe.direction);
        if d.length_squared() < 0.0001 {
            Vec3::Y
        } else {
            d.normalize()
        }
    };

    let dir = if recipe.spread_deg >= 179.0 {
        // Uniform radial sphere distribution
        let d = Vec3::new(
            next_bilateral(seed),
            next_bilateral(seed),
            next_bilateral(seed),
        );
        if d.length_squared() < 0.0001 {
            Vec3::Y
        } else {
            d.normalize()
        }
    } else {
        let spread_rad = (recipe.spread_deg.to_radians() * 0.5).clamp(0.0, PI);
        let cos_max = spread_rad.cos();
        let cos_theta = cos_max + next_float(seed) * (1.0 - cos_max);
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let phi = next_float(seed) * 2.0 * PI;

        // Construct orthonormal basis around base_dir
        let up = if base_dir.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let right = base_dir.cross(up).normalize();
        let forward = right.cross(base_dir).normalize();

        (base_dir * cos_theta + right * (sin_theta * phi.cos()) + forward * (sin_theta * phi.sin())).normalize()
    };

    let min_spd = recipe.speed_min.max(0.0);
    let max_spd = recipe.speed_max.max(min_spd);
    let speed = min_spd + next_float(seed) * (max_spd - min_spd);
    let velocity = dir * speed;

    // 3. Lifetime
    let min_life = recipe.lifetime_min.max(0.05);
    let max_life = recipe.lifetime_max.max(min_life);
    let lifetime = min_life + next_float(seed) * (max_life - min_life);

    (spawn_pos, velocity, lifetime)
}

// ============================================================================
// 4. UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_vfx_presets_generation() {
        let presets = list_vfx_presets();
        assert_eq!(presets.len(), 7);

        for name in presets {
            let p = get_vfx_preset(name);
            assert!(p.is_some(), "Preset '{}' should exist", name);
            let recipe = p.unwrap();
            assert_eq!(recipe.name, name);
            assert!(recipe.lifetime_max >= recipe.lifetime_min);
            assert!(recipe.speed_max >= recipe.speed_min);
            assert!(recipe.emissive_boost >= 0.0);
            assert!(recipe.size_start > 0.0);
        }
    }

    #[test]
    fn test_vfx_recipe_json_roundtrip() {
        let exp = preset_explosion();
        let json = serde_json::to_string_pretty(&exp).unwrap();
        assert!(json.contains("\"name\": \"explosion\""));
        assert!(json.contains("\"emissive_boost\": 3.5"));

        let deserialized: VfxRecipe = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, exp.name);
        assert_eq!(deserialized.burst_count, exp.burst_count);
        assert_eq!(deserialized.color_start, exp.color_start);
    }

    #[test]
    fn test_sample_particle_spawn_no_nan() {
        let mut seed = 42;
        let presets = ["explosion", "fire", "magic_portal", "hit_spark", "level_up"];

        for name in &presets {
            let recipe = get_vfx_preset(name).unwrap();
            for _ in 0..100 {
                let (pos, vel, life) = sample_particle_spawn(&recipe, Vec3::new(1.0, 2.0, 3.0), &mut seed);
                assert!(!pos.is_nan(), "pos NaN in preset {}", name);
                assert!(!vel.is_nan(), "vel NaN in preset {}", name);
                assert!(!life.is_nan(), "life NaN in preset {}", name);
                assert!(life > 0.0);
            }
        }
    }
}
