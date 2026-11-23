-- Cosmos.io 3D: Plasma Projectile Script
local lifetime = 2.2
local speed = 48.0
local init_done = false
local dir_x = 0.0
local dir_z = -1.0

function on_update(entity, dt)
    if not init_done then
        init_done = true
        local player = get_entity("player_star")
        if player then
            local dx = entity.x - player.x
            local dz = entity.z - player.z
            local len = math.sqrt(dx * dx + dz * dz)
            if len > 0.001 then
                dir_x = dx / len
                dir_z = dz / len
            end
        end
    end

    lifetime = lifetime - dt
    if lifetime <= 0.0 then
        destroy_entity("plasma_shot")
        return
    end

    entity.x = entity.x + dir_x * speed * dt
    entity.z = entity.z + dir_z * speed * dt

    -- Small particle trail
    if math.random() > 0.4 then
        spawn_particles(entity.x, 0.0, entity.z, 0.0, 1.0, 0.85, 2, 1.0, 0.2)
    end

    -- Check collision with Boss
    local boss = get_entity("boss_apophis")
    if boss then
        local bx = boss.x - entity.x
        local bz = boss.z - entity.z
        local dist = math.sqrt(bx * bx + bz * bz)
        if dist < (boss.sx * 0.5) + 0.6 then
            -- Hit boss!
            spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.4, 0.1, 18, 5.0, 0.5)
            camera_shake(0.4, 0.15)
            destroy_entity("plasma_shot")
        end
    end
end
