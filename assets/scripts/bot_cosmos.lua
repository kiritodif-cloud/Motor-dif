-- Cosmos.io 3D: AI Rival Star Controller
local bot_mass = 45.0
local vx = 0.0
local vz = 0.0
local base_speed = 15.0
local wander_angle = math.random() * math.pi * 2
local wander_timer = 0.0

function on_update(entity, dt)
    wander_timer = wander_timer + dt
    if wander_timer > 2.5 then
        wander_timer = 0.0
        wander_angle = wander_angle + (math.random() - 0.5) * 1.5
    end

    local target_dir_x = math.cos(wander_angle)
    local target_dir_z = math.sin(wander_angle)

    -- Detect player
    local player = get_entity("player_star")
    if player then
        local dx = player.x - entity.x
        local dz = player.z - entity.z
        local dist = math.sqrt(dx * dx + dz * dz)

        if dist < 24.0 and dist > 0.1 then
            local p_scale = player.sx
            local my_scale = entity.sx
            if p_scale > my_scale * 1.15 then
                -- Flee from player!
                target_dir_x = -(dx / dist)
                target_dir_z = -(dz / dist)
            elseif my_scale > p_scale * 1.15 then
                -- Chase player!
                target_dir_x = (dx / dist)
                target_dir_z = (dz / dist)
            end
        end
    end

    -- Movement and Inertia
    vx = vx + (target_dir_x * base_speed - vx) * math.min(1.0, 5.0 * dt)
    vz = vz + (target_dir_z * base_speed - vz) * math.min(1.0, 5.0 * dt)

    entity.x = entity.x + vx * dt
    entity.z = entity.z + vz * dt

    -- Bounce at map boundaries
    if entity.x < -68.0 or entity.x > 68.0 then vx = -vx; wander_angle = math.pi - wander_angle end
    if entity.z < -68.0 or entity.z > 68.0 then vz = -vz; wander_angle = -wander_angle end

    entity.ry = entity.ry + 0.6 * dt

    -- Dynamic scale based on mass
    local bot_radius = 0.8 + math.sqrt(bot_mass) * 0.14
    entity.sx = bot_radius * 2.0
    entity.sy = bot_radius * 2.0
    entity.sz = bot_radius * 2.0
end
