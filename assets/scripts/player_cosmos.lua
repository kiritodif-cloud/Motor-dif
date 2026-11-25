-- Cosmos.io 3D - Player Cosmic Controller
-- Recreates the full 3D celestial agar.io gameplay for Motor-dif

local mass = 35.0
local vx = 0.0
local vz = 0.0
local base_speed = 19.0
local facing_x = 0.0
local facing_z = -1.0

local dash_cooldown = 0.0
local shot_cooldown = 0.0
local mine_cooldown = 0.0
local ability_cooldown = 0.0

local shield_timer = 0.0
local speed_boost_timer = 0.0

local shot_counter = 0
local mine_counter = 0
local moon_counter = 0

-- Orbiting celestial moons/planets
local orbiting_moons = {}

-- Food respawn queue
local respawn_timer = 0.0

-- Map boundaries
local MAP_LIMIT = 72.0
local SANCTUARY_RADIUS = 16.0

function get_era_info(m)
    if m < 80.0 then
        return "Proto-Planeta Rocoso", "Tipo 0.4", "Gris Ceniza / Rocoso"
    elseif m < 200.0 then
        return "Gigante Gaseoso", "Tipo 0.9", "Atmósfera de Tormenta"
    elseif m < 500.0 then
        return "Estrella Solar (Sol)", "Tipo 1.8", "Fusión Nuclear Brillante"
    elseif m < 900.0 then
        return "Supergigante Roja", "Tipo 2.4", "Corona Ígnea Expandida"
    else
        return "Singularidad / Hipernova", "Tipo 3.0", "Colapso Gravitacional"
    end
end

function on_update(entity, dt)
    -- 1. Timers & Cooldowns
    if dash_cooldown > 0.0 then dash_cooldown = dash_cooldown - dt end
    if shot_cooldown > 0.0 then shot_cooldown = shot_cooldown - dt end
    if mine_cooldown > 0.0 then mine_cooldown = mine_cooldown - dt end
    if ability_cooldown > 0.0 then ability_cooldown = ability_cooldown - dt end
    if shield_timer > 0.0 then shield_timer = shield_timer - dt end
    if speed_boost_timer > 0.0 then speed_boost_timer = speed_boost_timer - dt end

    -- 2. Calculate dynamic speed (larger mass = slightly more inertia)
    local mass_factor = math.max(0.45, 1.0 - (mass / 2000.0) * 0.4)
    local current_speed = base_speed * mass_factor
    if speed_boost_timer > 0.0 then
        current_speed = current_speed * 1.65
    end

    -- 3. WASD Movement Input
    local move_x = 0.0
    local move_z = 0.0

    if is_key_down("W") or is_key_down("ARROWUP") then
        move_z = move_z - 1.0
    end
    if is_key_down("S") or is_key_down("ARROWDOWN") then
        move_z = move_z + 1.0
    end
    if is_key_down("A") or is_key_down("ARROWLEFT") then
        move_x = move_x - 1.0
    end
    if is_key_down("D") or is_key_down("ARROWRIGHT") then
        move_x = move_x + 1.0
    end

    -- Normalize directional input
    local len = math.sqrt(move_x * move_x + move_z * move_z)
    if len > 0.001 then
        move_x = move_x / len
        move_z = move_z / len
        facing_x = move_x
        facing_z = move_z
    end

    -- Dash / Warp Boost (Shift)
    local is_dashing = false
    if (is_key_down("SHIFT") or is_key_down("LEFTSHIFT")) and dash_cooldown <= 0.0 and len > 0.1 then
        current_speed = current_speed * 2.4
        dash_cooldown = 1.8
        is_dashing = true
        mass = math.max(20.0, mass - 1.5)
        spawn_particles(entity.x, 0.0, entity.z, 0.1, 0.9, 1.0, 14, 5.0, 0.45)
        camera_shake(0.4, 0.15)
    end

    -- Inertia / Acceleration
    local accel = 45.0
    local target_vx = move_x * current_speed
    local target_vz = move_z * current_speed
    vx = vx + (target_vx - vx) * math.min(1.0, accel * dt)
    vz = vz + (target_vz - vz) * math.min(1.0, accel * dt)

    entity.x = entity.x + vx * dt
    entity.z = entity.z + vz * dt

    -- Clamp to Cosmic Map Boundary
    if entity.x < -MAP_LIMIT then entity.x = -MAP_LIMIT; vx = -vx * 0.5 end
    if entity.x > MAP_LIMIT then entity.x = MAP_LIMIT; vx = -vx * 0.5 end
    if entity.z < -MAP_LIMIT then entity.z = -MAP_LIMIT; vz = -vz * 0.5 end
    if entity.z > MAP_LIMIT then entity.z = MAP_LIMIT; vz = -vz * 0.5 end

    -- Star Self-Rotation
    entity.ry = entity.ry + 0.8 * dt

    -- Dynamic Scale based on Baryonic Mass
    local target_radius = 0.9 + math.sqrt(mass) * 0.16
    local current_scale = target_radius * 2.0
    entity.sx = current_scale
    entity.sy = current_scale
    entity.sz = current_scale

    -- Camera follows player smoothly
    local cam_dist = 14.0 + target_radius * 2.8
    follow_entity("player_star", 0.0, cam_dist * 0.85, cam_dist * 0.95, 0.14)

    -- 4. In Sanctuary Zone check (0,0)
    local dist_to_origin = math.sqrt(entity.x * entity.x + entity.z * entity.z)
    local in_sanctuary = (dist_to_origin <= SANCTUARY_RADIUS)

    -- 5. ABILITIES & ACTIONS

    -- [Space]: Plasma Ejection / Laser Shot
    if (is_key_down("SPACE") or is_key_just_pressed("SPACE")) and shot_cooldown <= 0.0 then
        shot_cooldown = 0.28
        shot_counter = shot_counter + 1
        local shot_id = "plasma_shot_" .. shot_counter
        local spawn_dist = target_radius + 0.8
        local shot_x = entity.x + facing_x * spawn_dist
        local shot_z = entity.z + facing_z * spawn_dist
        
        spawn_entity({
            id = shot_id,
            mesh = "sphere",
            x = shot_x,
            y = 0.0,
            z = shot_z,
            sx = 0.65,
            sy = 0.65,
            sz = 0.65,
            color = { 0.0, 1.0, 0.85, 1.0 },
            emissive = 1.0,
            script = "assets/scripts/plasma_shot.lua"
        })
        mass = math.max(15.0, mass - 1.2)
        spawn_particles(shot_x, 0.0, shot_z, 0.0, 1.0, 0.85, 8, 3.5, 0.3)
    end

    -- [F]: Antimatter Mine
    if is_key_just_pressed("F") and mine_cooldown <= 0.0 and mass > 35.0 then
        mine_cooldown = 1.5
        mine_counter = mine_counter + 1
        local mine_id = "mine_" .. mine_counter
        local spawn_dist = target_radius + 1.2
        local mine_x = entity.x - facing_x * spawn_dist
        local mine_z = entity.z - facing_z * spawn_dist

        spawn_entity({
            id = mine_id,
            mesh = "cylinder",
            x = mine_x,
            y = 0.0,
            z = mine_z,
            sx = 1.2,
            sy = 0.3,
            sz = 1.2,
            color = { 1.0, 0.2, 0.2, 1.0 },
            emissive = 1.0,
            script = "assets/scripts/antimatter_mine.lua"
        })
        mass = mass - 5.0
        spawn_particles(mine_x, 0.0, mine_z, 1.0, 0.2, 0.2, 10, 2.5, 0.4)
    end

    -- [C]: Gravity Well (Hold to pull nearby stardust matter)
    local pulling_gravity = false
    if is_key_down("C") then
        pulling_gravity = true
        spawn_particles(entity.x, 0.0, entity.z, 0.8, 0.1, 1.0, 4, 1.5, 0.25)
    end

    -- [E]: Form / Capture Orbiting Planet
    if is_key_just_pressed("E") and #orbiting_moons < 5 and mass >= 50.0 then
        moon_counter = moon_counter + 1
        local moon_id = "moon_" .. moon_counter
        local moon_angle = #orbiting_moons * (math.pi * 2 / 5)
        local orbit_dist = target_radius + 2.2 + #orbiting_moons * 0.8

        spawn_entity({
            id = moon_id,
            mesh = "sphere",
            x = entity.x + math.cos(moon_angle) * orbit_dist,
            y = 0.0,
            z = entity.z + math.sin(moon_angle) * orbit_dist,
            sx = 0.65,
            sy = 0.65,
            sz = 0.65,
            color = { 0.95, 0.8, 0.2, 1.0 },
            emissive = 0.9
        })
        table.insert(orbiting_moons, {
            id = moon_id,
            angle = moon_angle,
            dist = orbit_dist,
            speed = 2.4 - #orbiting_moons * 0.25
        })
        mass = mass - 8.0
        spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.8, 0.1, 12, 3.5, 0.5)
    end

    -- [Q]: Launch Orbiting Planet Missile
    if is_key_just_pressed("Q") and #orbiting_moons > 0 then
        local fired_moon = table.remove(orbiting_moons, 1)
        destroy_entity(fired_moon.id)

        shot_counter = shot_counter + 1
        local missile_id = "planet_missile_" .. shot_counter
        spawn_entity({
            id = missile_id,
            mesh = "sphere",
            x = entity.x + facing_x * (target_radius + 1.2),
            y = 0.0,
            z = entity.z + facing_z * (target_radius + 1.2),
            sx = 0.95,
            sy = 0.95,
            sz = 0.95,
            color = { 1.0, 0.5, 0.0, 1.0 },
            emissive = 1.0,
            script = "assets/scripts/plasma_shot.lua"
        })
        camera_shake(0.5, 0.2)
        spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.5, 0.0, 16, 6.0, 0.5)
    end

    -- 6. Update Orbiting Moons Positions
    for i, moon in ipairs(orbiting_moons) do
        moon.angle = moon.angle + moon.speed * dt
        local target_orbit_dist = target_radius + 2.0 + (i - 1) * 0.9
        local mx = entity.x + math.cos(moon.angle) * target_orbit_dist
        local mz = entity.z + math.sin(moon.angle) * target_orbit_dist
        local moon_ent = get_entity(moon.id)
        if moon_ent then
            tween_position(moon.id, mx, 0.0, mz, 0.05)
        end
    end

    -- 7. Interaction with World Entities (Food, Powerups, Wormholes, Bots, Boss)
    local all = get_all_entities()
    if all then
        for id, ent in pairs(all) do
            if id ~= "player_star" then
                local dx = ent.x - entity.x
                local dz = ent.z - entity.z
                local dist = math.sqrt(dx * dx + dz * dz)

                -- A. Stardust Food Pellets
                if string.sub(id, 1, 5) == "food_" then
                    -- Pull with Gravity Well [C]
                    if pulling_gravity and dist < 22.0 and dist > 0.1 then
                        local pull_speed = 18.0 * dt
                        local new_fx = ent.x - (dx / dist) * pull_speed
                        local new_fz = ent.z - (dz / dist) * pull_speed
                        tween_position(id, new_fx, 0.0, new_fz, 0.05)
                    end

                    -- Eat Food Pellet
                    if dist < target_radius + 0.65 then
                        destroy_entity(id)
                        mass = mass + 4.5
                        spawn_particles(ent.x, 0.0, ent.z, 0.2, 1.0, 0.8, 8, 2.5, 0.35)

                        -- Respawn food elsewhere
                        local rx = (math.random() - 0.5) * (MAP_LIMIT * 1.7)
                        local rz = (math.random() - 0.5) * (MAP_LIMIT * 1.7)
                        spawn_entity({
                            id = id,
                            mesh = "sphere",
                            x = rx,
                            y = 0.0,
                            z = rz,
                            sx = 0.42,
                            sy = 0.42,
                            sz = 0.42,
                            color = { math.random() * 0.8 + 0.2, math.random() * 0.8 + 0.2, 1.0, 1.0 },
                            emissive = 0.85
                        })
                    end

                -- B. Power-Ups
                elseif id == "powerup_speed" and dist < target_radius + 1.2 then
                    destroy_entity(id)
                    speed_boost_timer = 8.0
                    spawn_particles(ent.x, 0.0, ent.z, 1.0, 1.0, 0.0, 22, 6.0, 0.7)
                    camera_shake(0.6, 0.2)
                elseif id == "powerup_shield" and dist < target_radius + 1.2 then
                    destroy_entity(id)
                    shield_timer = 12.0
                    spawn_particles(ent.x, 0.0, ent.z, 0.0, 0.8, 1.0, 22, 6.0, 0.7)
                    camera_shake(0.6, 0.2)

                -- C. Wormholes (Teleportation)
                elseif id == "wormhole_a" and dist < 2.8 then
                    local wb = get_entity("wormhole_b")
                    if wb then
                        entity.x = wb.x + 4.0
                        entity.z = wb.z + 4.0
                        spawn_particles(entity.x, 0.0, entity.z, 0.0, 1.0, 0.9, 30, 8.0, 0.8)
                        camera_shake(1.0, 0.3)
                    end
                elseif id == "wormhole_b" and dist < 2.8 then
                    local wa = get_entity("wormhole_a")
                    if wa then
                        entity.x = wa.x + 4.0
                        entity.z = wa.z + 4.0
                        spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.1, 0.8, 30, 8.0, 0.8)
                        camera_shake(1.0, 0.3)
                    end

                -- D. Boss Leviathan Collision
                elseif id == "boss_apophis" and dist < target_radius + (ent.sx * 0.5) then
                    if shield_timer <= 0.0 and not in_sanctuary then
                        mass = math.max(10.0, mass - 25.0 * dt)
                        camera_shake(0.8, 0.1)
                        spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.1, 0.1, 4, 3.0, 0.2)
                    end
                end
            end
        end
    end

    -- 8. Real-Time HUD Overlay Display
    local era_name, era_rank, era_desc = get_era_info(mass)
    local sanctuary_status = in_sanctuary and "ESCUDO DYSON ACTIVO (ZONA SEGURA)" or "ESPACIO PROFUNDO ABIERTO"
    local shield_text = shield_timer > 0.0 and string.format("Escudo Cósmico (%.1fs)", shield_timer) or "Vulnerable"
    local speed_text = speed_boost_timer > 0.0 and string.format("Warp Acelerado (%.1fs)", speed_boost_timer) or "Propulsión Estándar"

    ui_panel("cosmos_hud_bg", "top_left", 16, 16, 620, 150, 0.05, 0.08, 0.12, 0.82)
    ui_label("01_TITULO", "top_left", 28, 26, "COSMOS.IO 3D — INCURSIÓN ESTELAR", 16, 1.0, 0.86, 0.35)
    ui_label("02_MASA", "top_left", 28, 46, string.format("%.1f TJ (Bariónica) | Radio Estelar: %.2f", mass, target_radius), 14, 0.55, 0.9, 1.0)
    ui_label("03_ERA", "top_left", 28, 66, string.format("%s [%s]", era_name, era_rank), 14, 0.75, 0.85, 1.0)
    ui_label("04_PLANETAS", "top_left", 28, 86, string.format("%d/5 Satélites en Órbita  [E: Forjar Planeta | Q: Lanzar Misil]", #orbiting_moons), 13, 0.8, 0.8, 0.85)
    ui_label("05_ESTADO", "top_left", 28, 106, string.format("%s | %s | %s", sanctuary_status, shield_text, speed_text), 13, 0.6, 1.0, 0.7)
    ui_label("06_CONTROLES", "top_left", 28, 126, "[WASD] Dirección | [Shift] Dash Turbo | [Espacio] Disparo Plasma | [C] Pozo Succión | [F] Mina", 12, 0.75, 0.78, 0.85)
    ui_label("07_MISIÓN", "top_left", 28, 146, "¡Consume materia estelar para evolucionar y destruir al Leviatán Apophis!", 12, 1.0, 0.55, 0.4)
end
