-- Cosmos.io 3D: Black Hole Singularity Script
local spin_speed = 3.2
local pull_radius = 24.0
local pull_strength = 20.0

function on_update(entity, dt)
    entity.ry = entity.ry + spin_speed * dt

    -- Ambient vortex particles
    if math.random() > 0.65 then
        local p_angle = math.random() * math.pi * 2
        local p_dist = 4.0 + math.random() * 6.0
        spawn_particles(
            entity.x + math.cos(p_angle) * p_dist,
            0.0,
            entity.z + math.sin(p_angle) * p_dist,
            0.6, 0.1, 0.9, 1, 1.5, 0.3
        )
    end
end
