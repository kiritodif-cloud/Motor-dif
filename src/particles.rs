use glam::{Mat4, Quat, Vec3};
use crate::renderer::InstanceRaw;

#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: [f32; 4],
    pub size: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub gravity: f32,
}

#[derive(Debug, Clone)]
pub struct ParticleSpawnRequest {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub count: usize,
    pub speed: f32,
    pub lifetime: f32,
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
        let base_gravity = 9.81;

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
                color,
                size,
                lifetime: p_life,
                max_lifetime: p_life,
                gravity: base_gravity,
            });
        }
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.lifetime -= dt;
            p.velocity.y -= p.gravity * dt;
            p.position += p.velocity * dt;
        }
        self.particles.retain(|p| p.lifetime > 0.0);
    }

    pub fn get_instances(&self) -> Vec<InstanceRaw> {
        let mut instances = Vec::with_capacity(self.particles.len());
        for p in &self.particles {
            let progress = (p.lifetime / p.max_lifetime).clamp(0.0, 1.0);
            let current_size = p.size * (0.3 + 0.7 * progress);
            let model = Mat4::from_scale_rotation_translation(
                Vec3::splat(current_size),
                Quat::IDENTITY,
                p.position,
            );

            let mut col = p.color;
            col[3] = progress; // Fade alpha smoothly

            instances.push(InstanceRaw {
                model: model.to_cols_array_2d(),
                color: col,
                extra: [1.0, 0.0, 0.0, 0.0], // Emissive glow = 1.0
                fresnel: [0.0, 0.0, 0.0, 0.0],
                uv_offset: [0.0, 0.0],
                uv_scale: [1.0, 1.0],
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
}
