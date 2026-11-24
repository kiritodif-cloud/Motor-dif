use glam::{Mat4, Quat, Vec3};
use crate::renderer::InstanceRaw;

#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub emissive_boost: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub gravity: Vec3,
    pub drag: f32,
    pub vortex_strength: f32,
    pub emitter_center: Vec3,
}

#[derive(Debug, Clone)]
pub struct ParticleSpawnRequest {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub count: usize,
    pub speed: f32,
    pub lifetime: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParticleEmitter {
    pub preset: String,
    pub position: [f32; 3],
    pub rate: f32,
    pub enabled: bool,
    pub looping: bool,
    pub burst_count: usize,
    #[serde(skip)]
    pub accumulator: f32,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            preset: "sparks".to_string(),
            position: [0.0, 0.0, 0.0],
            rate: 25.0,
            enabled: true,
            looping: true,
            burst_count: 35,
            accumulator: 0.0,
        }
    }
}

impl ParticleEmitter {
    pub fn new(preset: impl Into<String>, position: [f32; 3], rate: f32) -> Self {
        Self {
            preset: preset.into(),
            position,
            rate,
            enabled: true,
            looping: true,
            burst_count: 35,
            accumulator: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32, ps: &mut ParticleSystem) {
        if !self.enabled {
            return;
        }
        if self.rate > 0.0 {
            self.accumulator += self.rate * dt;
            let to_spawn = self.accumulator.floor() as usize;
            if to_spawn > 0 {
                self.accumulator -= to_spawn as f32;
                ps.spawn_preset(Vec3::from(self.position), &self.preset, Some(to_spawn));
            }
        }
    }

    pub fn burst(&mut self, ps: &mut ParticleSystem) {
        if self.enabled {
            let count = if self.burst_count > 0 { self.burst_count } else { 35 };
            ps.spawn_preset(Vec3::from(self.position), &self.preset, Some(count));
        }
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    max_particles: usize,
    rng_seed: u32,
}

impl ParticleSystem {
    pub const DEFAULT_MAX_PARTICLES: usize = 2000;

    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(Self::DEFAULT_MAX_PARTICLES),
            max_particles: Self::DEFAULT_MAX_PARTICLES,
            rng_seed: 123456789,
        }
    }

    /// Fast lightweight pseudo-random float in range [0.0, 1.0)
    fn next_float(&mut self) -> f32 {
        self.rng_seed = self.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        ((self.rng_seed >> 8) & 0x00ffffff) as f32 / 16777216.0
    }

    /// Fast float in range [-1.0, 1.0]
    fn next_bilateral(&mut self) -> f32 {
        self.next_float() * 2.0 - 1.0
    }

    pub fn spawn(
        &mut self,
        origin: Vec3,
        color: [f32; 4],
        count: usize,
        speed: f32,
        lifetime: f32,
    ) {
        let count_to_spawn = count.min(self.max_particles.saturating_sub(self.particles.len()));
        let base_gravity = Vec3::new(0.0, -9.81, 0.0);

        for _ in 0..count_to_spawn {
            // Generate a random 3D direction
            let mut dir = Vec3::new(
                self.next_bilateral(),
                self.next_float() * 1.5 + 0.2, // bias upwards
                self.next_bilateral(),
            );
            if dir.length_squared() < 0.001 {
                dir = Vec3::Y;
            } else {
                dir = dir.normalize();
            }

            let spd = speed * (0.6 + self.next_float() * 0.8);
            let p_life = (lifetime * (0.7 + self.next_float() * 0.6)).max(0.05);
            let size = 0.12 * (0.8 + self.next_float() * 0.5);

            self.particles.push(Particle {
                position: origin,
                velocity: dir * spd,
                size_start: size,
                size_end: size * 0.3,
                color_start: color,
                color_end: [color[0], color[1], color[2], 0.0],
                emissive_boost: 1.0,
                lifetime: p_life,
                max_lifetime: p_life,
                gravity: base_gravity,
                drag: 0.0,
                vortex_strength: 0.0,
                emitter_center: origin,
            });
        }
    }

    /// Spawns particles configured via a procedural VfxRecipe
    pub fn spawn_vfx(
        &mut self,
        origin: Vec3,
        recipe: &crate::vfx_forge::VfxRecipe,
        count: usize,
    ) {
        let count_to_spawn = count.min(self.max_particles.saturating_sub(self.particles.len()));
        let grav = Vec3::from(recipe.gravity);

        for _ in 0..count_to_spawn {
            let (pos, vel, life) = crate::vfx_forge::sample_particle_spawn(recipe, origin, &mut self.rng_seed);
            self.particles.push(Particle {
                position: pos,
                velocity: vel,
                size_start: recipe.size_start,
                size_end: recipe.size_end,
                color_start: recipe.color_start,
                color_end: recipe.color_end,
                emissive_boost: recipe.emissive_boost,
                lifetime: life,
                max_lifetime: life,
                gravity: grav,
                drag: recipe.drag,
                vortex_strength: recipe.vortex_strength,
                emitter_center: origin,
            });
        }
    }

    /// Spawns particles using one of the built-in presets: "explosion", "fire", "smoke", "sparks", "rain"
    pub fn spawn_preset(&mut self, origin: Vec3, preset: &str, count: Option<usize>) -> bool {
        if let Some(recipe) = crate::vfx_forge::get_vfx_preset(preset) {
            let n = count.unwrap_or(recipe.burst_count.max(25));
            self.spawn_vfx(origin, &recipe, n);
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.lifetime -= dt;

            // Apply 3D gravity vector
            p.velocity += p.gravity * dt;

            // Apply aerodynamic drag
            if p.drag > 0.0 {
                p.velocity *= 1.0 - (p.drag * dt).min(0.99);
            }

            // Apply vortex force (swirling in XZ plane around emitter_center)
            if p.vortex_strength.abs() > 0.001 {
                let r_xz = Vec3::new(p.position.x - p.emitter_center.x, 0.0, p.position.z - p.emitter_center.z);
                let dist = r_xz.length();
                if dist > 0.001 {
                    let tangent = Vec3::new(-r_xz.z, 0.0, r_xz.x) / dist;
                    p.velocity += tangent * (p.vortex_strength * dt);
                }
            }

            p.position += p.velocity * dt;
        }
        self.particles.retain(|p| p.lifetime > 0.0);
    }

    pub fn get_instances(&self) -> Vec<InstanceRaw> {
        let mut instances = Vec::with_capacity(self.particles.len());
        for p in &self.particles {
            let u = (1.0 - p.lifetime / p.max_lifetime).clamp(0.0, 1.0);
            let current_size = p.size_start + (p.size_end - p.size_start) * u;
            let model = Mat4::from_scale_rotation_translation(
                Vec3::splat(current_size.max(0.001)),
                Quat::IDENTITY,
                p.position,
            );

            // Interpolate color and alpha with smooth fade
            let col = [
                p.color_start[0] + (p.color_end[0] - p.color_start[0]) * u,
                p.color_start[1] + (p.color_end[1] - p.color_start[1]) * u,
                p.color_start[2] + (p.color_end[2] - p.color_start[2]) * u,
                (p.color_start[3] + (p.color_end[3] - p.color_start[3]) * u) * (1.0 - u * u),
            ];

            let emissive = p.emissive_boost.max(1.0);

            instances.push(InstanceRaw {
                model: model.to_cols_array_2d(),
                color: col,
                extra: [emissive, 0.0, 0.0, 0.0], // Emissive glow for HDR Bloom
                fresnel: [0.0, 0.0, 0.0, 0.0],
                uv_offset: [0.0, 0.0],
                uv_scale: [1.0, 1.0],
                pbr_params: [0.0, 1.0, 1.0, emissive],
            });
        }
        instances
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }

    pub fn active_count(&self) -> usize {
        self.particles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_system_lifecycle() {
        let mut ps = ParticleSystem::new();
        assert_eq!(ps.active_count(), 0);

        ps.spawn(Vec3::ZERO, [1.0, 0.5, 0.2, 1.0], 50, 5.0, 1.0);
        assert_eq!(ps.active_count(), 50);

        let instances = ps.get_instances();
        assert_eq!(instances.len(), 50);
        assert_eq!(instances[0].extra[0], 1.0); // Emissive check

        ps.update(0.5);
        assert_eq!(ps.active_count(), 50);

        // Advance beyond lifetime
        ps.update(1.0);
        assert_eq!(ps.active_count(), 0);

        ps.spawn(Vec3::ZERO, [0.0, 1.0, 0.5, 1.0], 20, 2.0, 0.5);
        assert_eq!(ps.active_count(), 20);
        ps.clear();
        assert_eq!(ps.active_count(), 0);
    }

    #[test]
    fn test_particle_system_vfx_recipe_and_forces() {
        let mut ps = ParticleSystem::new();
        let portal = crate::vfx_forge::preset_magic_portal();
        ps.spawn_vfx(Vec3::ZERO, &portal, 30);
        assert_eq!(ps.active_count(), 30);

        let instances = ps.get_instances();
        assert_eq!(instances.len(), 30);
        assert!((instances[0].extra[0] - 4.0).abs() < 1e-4, "Expected emissive boost of 4.0");

        // Update with drag and vortex
        ps.update(0.1);
        assert_eq!(ps.active_count(), 30);
        for p in &ps.particles {
            assert!(!p.position.is_nan());
            assert!(!p.velocity.is_nan());
        }
    }

    #[test]
    fn test_particle_emitter_and_all_presets() {
        let mut ps = ParticleSystem::new();

        // Test all required presets
        for preset in &["explosion", "fire", "smoke", "sparks", "rain"] {
            let ok = ps.spawn_preset(Vec3::new(1.0, 2.0, 3.0), preset, Some(20));
            assert!(ok, "Expected preset '{}' to spawn successfully", preset);
        }
        assert!(ps.active_count() >= 100);

        // Test ParticleEmitter continuous update
        let mut emitter = ParticleEmitter::new("fire", [0.0, 1.0, 0.0], 50.0);
        ps.clear();
        assert_eq!(ps.active_count(), 0);

        // Advance 0.1s at 50 particles/sec -> 5 particles
        emitter.update(0.1, &mut ps);
        assert!(ps.active_count() >= 5);

        // Test ParticleEmitter burst
        emitter.burst(&mut ps);
        assert!(ps.active_count() >= 35);
    }
}
