-- Cosmos.io 3D: Antimatter Mine Script
local pulse_time = 0.0
local lifetime = 25.0

function on_update(entity, dt)
    lifetime = lifetime - dt
    if lifetime <= 0.0 then
        destroy_entity("antimatter_mine")
        return
    end

    pulse_time = pulse_time + dt * 4.0
    local pulse_scale = 1.0 + math.sin(pulse_time) * 0.25
    entity.sx = pulse_scale
    entity.sz = pulse_scale
    entity.ry = entity.ry + 2.0 * dt

    -- Check collision with Boss
    local boss = get_entity("boss_apophis")
    if boss then
        local bx = boss.x - entity.x
        local bz = boss.z - entity.z
        local dist = math.sqrt(bx * bx + bz * bz)
        if dist < (boss.sx * 0.5) + 1.2 then
            spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.1, 0.2, 40, 8.0, 0.8)
            camera_shake(1.2, 0.35)
            destroy_entity("antimatter_mine")
        end
    end
end
