-- ============================================================================
-- COSMOS.IO 3D — COSMIC LEVIATHAN BOSS UPDATE
-- Complete Game Script for Motor-dif
-- ============================================================================

-- Game State Variables
local player_mass = 50.0
local player_scale = 1.2
local player_vx = 0.0
local player_vz = 0.0
local facing_x = 0.0
local facing_z = -1.0

-- Cooldowns & Timers
local dash_cooldown = 0.0
local mine_cooldown = 0.0
local shoot_cooldown = 0.0
local emp_cooldown = 0.0
local game_time = 0.0

-- Evolution
local evolution = "None"
local evolved = false
local planets_spawned = false
local planet_orbit_angle = 0.0

-- Persistent Data
local record_tj = 50.0
local dark_matter = 50.0

-- World Entities tracking
local initialized = false
local food_list = {}
local bots_list = {}
local mines_list = {}
local projectiles_list = {}
local uid_counter = 100

-- Gamma Radiation Storm
local storm_interval = 35.0
local storm_duration = 10.0
local storm_timer = 0.0
local storm_active = false
local storm_warning = false
local SANCTUARY_RADIUS = 35.0

-- Boss Apophis
local boss_spawned = false
local boss_alive = false
local boss_id = "apophis_boss"
local boss_hp = 600.0
local boss_max_hp = 600.0
local boss_x = 0.0
local boss_z = -55.0
local boss_vx = 0.0
local boss_vz = 0.0
local boss_mine_timer = 0.0

-- Initial load of persistent data
local saved_record = load_data("cosmos_record")
if saved_record and type(saved_record) == "number" then
    record_tj = saved_record
end

local saved_dm = load_data("cosmos_dm")
if saved_dm and type(saved_dm) == "number" then
    dark_matter = saved_dm
end

-- ============================================================================
-- HUD Button Callbacks
-- ============================================================================
function restart_game()
    player_mass = 50.0
    player_vx = 0.0
    player_vz = 0.0
    dash_cooldown = 0.0
    mine_cooldown = 0.0
    shoot_cooldown = 0.0
    emp_cooldown = 0.0
    evolution = "None"
    evolved = false
    planets_spawned = false
    storm_timer = 0.0
    storm_active = false
    storm_warning = false
    boss_spawned = false
    boss_alive = false
    boss_hp = boss_max_hp
    play_sound("assets/sfx/bounce.wav", 0.8)
    camera_shake(0.5, 0.4)
end

function buy_starting_mass()
    if dark_matter >= 50 then
        dark_matter = dark_matter - 50
        player_mass = player_mass + 80.0
        save_data("cosmos_dm", dark_matter)
        play_sound("assets/sfx/bounce.wav", 0.9)
        spawn_particles(0.0, 1.0, 0.0, 0.2, 1.0, 0.5, 40, 6.0, 0.8)
        camera_shake(0.4, 0.3)
    else
        play_sound("assets/sfx/bounce.wav", 0.2)
    end
end

-- ============================================================================
-- Helper Functions
-- ============================================================================
local function get_next_uid()
    uid_counter = uid_counter + 1
    return tostring(uid_counter)
end

local function dist(x1, z1, x2, z2)
    local dx = x1 - x2
    local dz = z1 - z2
    return math.sqrt(dx * dx + dz * dz)
end

local function clamp(val, min_v, max_v)
    if val < min_v then return min_v end
    if val > max_v then return max_v end
    return val
end

-- Spawn food orb at random position
local function spawn_single_food()
    local angle = math.random() * math.pi * 2.0
    local r = 8.0 + math.random() * 65.0
    local fx = math.cos(angle) * r
    local fz = math.sin(angle) * r
    local fid = "food_" .. get_next_uid()

    spawn_entity({
        id = fid,
        mesh = "sphere",
        x = fx,
        y = 0.4,
        z = fz,
        sx = 0.35,
        sy = 0.35,
        sz = 0.35,
        color = {1.0, 0.95, 0.35, 1.0},
        emissive = 0.85
    })

    table.insert(food_list, { id = fid, x = fx, z = fz })
end

-- Spawn a rival AI bot
local function spawn_single_bot(custom_id)
    local angle = math.random() * math.pi * 2.0
    local r = 25.0 + math.random() * 50.0
    local bx = math.cos(angle) * r
    local bz = math.sin(angle) * r
    local bid = custom_id or ("bot_" .. get_next_uid())
    local b_mass = 25.0 + math.random() * 60.0
    local b_scale = 0.8 + (b_mass / 100.0) * 0.5

    local bot_data = {
        id = bid,
        x = bx,
        z = bz,
        vx = 0.0,
        vz = 0.0,
        mass = b_mass,
        scale = b_scale
    }

    spawn_entity({
        id = bid,
        mesh = "sphere",
        x = bx,
        y = b_scale * 0.5,
        z = bz,
        sx = b_scale,
        sy = b_scale,
        sz = b_scale,
        color = {0.92, 0.28, 0.38, 1.0},
        emissive = 0.45
    })

    table.insert(bots_list, bot_data)
end

-- Initial World Population
local function init_world()
    food_list = {}
    bots_list = {}
    mines_list = {}
    projectiles_list = {}

    -- Spawn 45 food orbs
    for _ = 1, 45 do
        spawn_single_food()
    end

    -- Spawn 10 AI bots
    for _ = 1, 10 do
        spawn_single_bot()
    end

    -- Setup persistent HUD buttons
    set_hud_button("btn_restart", "[Restart]", 280, 18, "restart_game")
    set_hud_button("btn_shop", "[+Masa Inicial (50 DM)]", 365, 18, "buy_starting_mass")

    initialized = true
end

-- ============================================================================
-- MAIN GAME LOOP (on_update)
-- ============================================================================
function on_update(entity, dt)
    if dt <= 0.0 then return end
    if dt > 0.1 then dt = 0.1 end

    game_time = game_time + dt
    if not initialized then
        init_world()
    end

    -- 1. Cooldowns Countdown
    if dash_cooldown > 0.0 then dash_cooldown = dash_cooldown - dt end
    if mine_cooldown > 0.0 then mine_cooldown = mine_cooldown - dt end
    if shoot_cooldown > 0.0 then shoot_cooldown = shoot_cooldown - dt end
    if emp_cooldown > 0.0 then emp_cooldown = emp_cooldown - dt end

    -- 2. Player Input & Movement (W, A, S, D)
    local move_x = 0.0
    local move_z = 0.0

    if is_key_down("W") then move_z = move_z - 1.0 end
    if is_key_down("S") then move_z = move_z + 1.0 end
    if is_key_down("A") then move_x = move_x - 1.0 end
    if is_key_down("D") then move_x = move_x + 1.0 end

    local len = math.sqrt(move_x * move_x + move_z * move_z)
    if len > 0.01 then
        move_x = move_x / len
        move_z = move_z / len
        facing_x = move_x
        facing_z = move_z
    end

    -- Base speed modified by mass and Pulsar evolution
    local speed_mult = 1.0
    if evolution == "Pulsar" then
        speed_mult = 1.35
    end
    local accel = (16.0 / math.max(1.0, (player_mass / 50.0) ^ 0.3)) * speed_mult

    player_vx = player_vx + move_x * accel * dt
    player_vz = player_vz + move_z * accel * dt

    -- Damping / Friction
    local friction = math.max(0.0, 1.0 - 4.0 * dt)
    player_vx = player_vx * friction
    player_vz = player_vz * friction

    -- 3. Dash Ability (Shift)
    if is_key_down("Shift") and dash_cooldown <= 0.0 and len > 0.01 then
        player_vx = player_vx + move_x * 24.0
        player_vz = player_vz + move_z * 24.0
        dash_cooldown = 2.0
        camera_shake(0.5, 0.35)
        spawn_particles(entity.x, entity.y, entity.z, 0.2, 0.8, 1.0, 25, 5.0, 0.45)
        play_sound("assets/sfx/bounce.wav", 0.6)
    end

    -- 4. Gravitational Well (C - Hold)
    local suction_radius = (evolution == "BlackHole") and 32.0 or 18.0
    local suction_force = (evolution == "BlackHole") and 18.0 or 9.0

    if is_key_down("C") then
        -- Particle visual cue
        if math.random() < 0.4 then
            local p_ang = math.random() * math.pi * 2.0
            local p_rad = suction_radius * (0.4 + math.random() * 0.6)
            local px = entity.x + math.cos(p_ang) * p_rad
            local pz = entity.z + math.sin(p_ang) * p_rad
            spawn_particles(px, 0.5, pz, 0.7, 0.2, 1.0, 4, 3.0, 0.35)
        end

        -- Pull Food
        for _, food in ipairs(food_list) do
            local d = dist(entity.x, entity.z, food.x, food.z)
            if d < suction_radius and d > 0.5 then
                local pull = (suction_force / d) * dt
                food.x = food.x + (entity.x - food.x) * pull
                food.z = food.z + (entity.z - food.z) * pull
                tween_position(food.id, food.x, 0.4, food.z, 0.0)
            end
        end

        -- Pull smaller Bots
        for _, bot in ipairs(bots_list) do
            if bot.mass < player_mass then
                local d = dist(entity.x, entity.z, bot.x, bot.z)
                if d < suction_radius and d > 1.0 then
                    local pull = (suction_force * 0.7 / d) * dt
                    bot.vx = bot.vx + (entity.x - bot.x) * pull
                    bot.vz = bot.vz + (entity.z - bot.z) * pull
                end
            end
        end
    end

    -- 5. Shoot / Eject Mass (Space)
    if is_key_just_pressed("Space") and shoot_cooldown <= 0.0 and player_mass > 20.0 then
        player_mass = player_mass - 4.0
        shoot_cooldown = 0.22
        local pid = "proj_" .. get_next_uid()
        local proj_vx = facing_x * 32.0
        local proj_vz = facing_z * 32.0
        local spawn_dist = player_scale * 0.8

        spawn_entity({
            id = pid,
            mesh = "sphere",
            x = entity.x + facing_x * spawn_dist,
            y = 0.6,
            z = entity.z + facing_z * spawn_dist,
            sx = 0.4,
            sy = 0.4,
            sz = 0.4,
            color = {0.2, 0.9, 1.0, 1.0},
            emissive = 1.0
        })

        table.insert(projectiles_list, {
            id = pid,
            x = entity.x + facing_x * spawn_dist,
            z = entity.z + facing_z * spawn_dist,
            vx = proj_vx,
            vz = proj_vz,
            life = 2.5
        })

        play_sound("assets/sfx/bounce.wav", 0.45)
        spawn_particles(entity.x, entity.y, entity.z, 0.2, 0.8, 1.0, 8, 3.5, 0.25)
    end

    -- 6. Antimatter Mine (F)
    if is_key_just_pressed("F") and mine_cooldown <= 0.0 then
        mine_cooldown = 4.0
        local mid = "mine_" .. get_next_uid()
        spawn_prefab("cosmos_mine", entity.x, 0.6, entity.z)
        table.insert(mines_list, {
            id = mid,
            x = entity.x,
            z = entity.z,
            life = 15.0
        })
        play_sound("assets/sfx/bounce.wav", 0.5)
        spawn_particles(entity.x, 0.6, entity.z, 1.0, 0.2, 0.2, 12, 4.0, 0.3)
    end

    -- 7. EMP Pulse Ability (E - Pulsar Only)
    if evolution == "Pulsar" and is_key_just_pressed("E") and emp_cooldown <= 0.0 then
        emp_cooldown = 5.0
        camera_shake(0.9, 0.5)
        spawn_particles(entity.x, entity.y, entity.z, 0.2, 1.0, 0.9, 80, 12.0, 0.85)
        play_sound("assets/sfx/bounce.wav", 0.8)

        -- Repel bots
        for _, bot in ipairs(bots_list) do
            local d = dist(entity.x, entity.z, bot.x, bot.z)
            if d < 26.0 and d > 0.1 then
                local push = 30.0 / d
                bot.vx = bot.vx + (bot.x - entity.x) * push
                bot.vz = bot.vz + (bot.z - entity.z) * push
            end
        end

        -- Damage boss if nearby
        if boss_alive then
            local bd = dist(entity.x, entity.z, boss_x, boss_z)
            if bd < 28.0 then
                boss_hp = boss_hp - 65.0
                spawn_particles(boss_x, 3.5, boss_z, 1.0, 0.5, 0.1, 40, 8.0, 0.6)
            end
        end
    end

    -- Apply player movement
    entity.x = entity.x + player_vx * dt
    entity.z = entity.z + player_vz * dt

    -- Arena boundary clamp
    local map_bound = 84.0
    entity.x = clamp(entity.x, -map_bound, map_bound)
    entity.z = clamp(entity.z, -map_bound, map_bound)

    -- Dynamic Player Scale from Mass (TJ)
    player_scale = 1.0 + (player_mass / 100.0) ^ 0.5 * 0.7
    entity.sx = player_scale
    entity.sy = player_scale
    entity.sz = player_scale
    entity.y = player_scale * 0.5

    -- Dynamic Camera follow smoothly adjusting distance by mass
    local cam_dist = 11.0 + player_scale * 4.5
    local cam_height = 8.0 + player_scale * 3.8
    follow_entity("player_cosmos", 0.0, cam_height, cam_dist, 0.16)

    -- 8. Food Absorption (Eating Food)
    local player_r = player_scale * 0.5
    local i_f = 1
    while i_f <= #food_list do
        local food = food_list[i_f]
        if dist(entity.x, entity.z, food.x, food.z) < (player_r + 0.35) then
            player_mass = player_mass + 2.5
            spawn_particles(food.x, 0.4, food.z, 1.0, 0.95, 0.35, 12, 3.2, 0.4)
            play_sound("assets/sfx/bounce.wav", 0.35)
            destroy_entity(food.id)
            table.remove(food_list, i_f)
            -- Instant respawn elsewhere
            spawn_single_food()
        else
            i_f = i_f + 1
        end
    end

    -- 9. Update Projectiles & Check Collisions
    local i_p = 1
    while i_p <= #projectiles_list do
        local p = projectiles_list[i_p]
        p.x = p.x + p.vx * dt
        p.z = p.z + p.vz * dt
        p.life = p.life - dt
        tween_position(p.id, p.x, 0.6, p.z, 0.0)

        local hit = false

        -- Check hit with Boss
        if boss_alive and dist(p.x, p.z, boss_x, boss_z) < 3.5 then
            boss_hp = boss_hp - 32.0
            spawn_particles(p.x, 2.0, p.z, 1.0, 0.4, 0.1, 20, 5.0, 0.5)
            play_sound("assets/sfx/bounce.wav", 0.5)
            camera_shake(0.3, 0.2)
            hit = true
        end

        -- Check hit with Bots
        if not hit then
            for _, bot in ipairs(bots_list) do
                if dist(p.x, p.z, bot.x, bot.z) < (bot.scale * 0.5 + 0.4) then
                    bot.mass = bot.mass - 15.0
                    bot.vx = bot.vx + p.vx * 0.2
                    bot.vz = bot.vz + p.vz * 0.2
                    spawn_particles(p.x, 0.6, p.z, 0.9, 0.3, 0.4, 15, 4.0, 0.4)
                    hit = true
                    break
                end
            end
        end

        if hit or p.life <= 0.0 or math.abs(p.x) > 86.0 or math.abs(p.z) > 86.0 then
            destroy_entity(p.id)
            table.remove(projectiles_list, i_p)
        else
            i_p = i_p + 1
        end
    end

    -- 10. Update Mines & Check Trigger
    local i_m = 1
    while i_m <= #mines_list do
        local m = mines_list[i_m]
        m.life = m.life - dt
        local exploded = false

        -- Explode on Boss
        if boss_alive and dist(m.x, m.z, boss_x, boss_z) < 4.0 then
            boss_hp = boss_hp - 85.0
            exploded = true
        end

        -- Explode on Bots
        if not exploded then
            for _, bot in ipairs(bots_list) do
                if dist(m.x, m.z, bot.x, bot.z) < (bot.scale * 0.5 + 0.8) then
                    bot.mass = bot.mass - 40.0
                    bot.vx = bot.vx * 1.5
                    bot.vz = bot.vz * 1.5
                    exploded = true
                    break
                end
            end
        end

        if exploded or m.life <= 0.0 then
            spawn_particles(m.x, 0.6, m.z, 1.0, 0.15, 0.1, 40, 7.0, 0.75)
            camera_shake(0.6, 0.35)
            play_sound("assets/sfx/bounce.wav", 0.7)
            destroy_entity(m.id)
            table.remove(mines_list, i_m)
        else
            i_m = i_m + 1
        end
    end

    -- 11. Bots AI, Movement & Collisions
    local i_b = 1
    while i_b <= #bots_list do
        local bot = bots_list[i_b]

        -- Dead bot check
        if bot.mass <= 5.0 then
            spawn_particles(bot.x, 0.8, bot.z, 0.9, 0.3, 0.4, 25, 4.5, 0.6)
            destroy_entity(bot.id)
            table.remove(bots_list, i_b)
            spawn_single_bot()
        else
            local b_target_x = 0.0
            local b_target_z = 0.0
            local b_urge = 1.0

            -- Flee to sanctuary if Gamma storm active
            if storm_warning or storm_active then
                b_target_x = 0.0
                b_target_z = 0.0
                b_urge = 2.5
            else
                -- Interact with Player
                local d_player = dist(bot.x, bot.z, entity.x, entity.z)
                if d_player < 22.0 then
                    if bot.mass > player_mass * 1.15 then
                        -- Chase Player
                        b_target_x = entity.x
                        b_target_z = entity.z
                        b_urge = 1.8
                    elseif bot.mass < player_mass * 0.9 then
                        -- Flee from Player
                        b_target_x = bot.x + (bot.x - entity.x)
                        b_target_z = bot.z + (bot.z - entity.z)
                        b_urge = 2.0
                    end
                else
                    -- Move towards closest food
                    local min_fd = 999.0
                    local close_fx = 0.0
                    local close_fz = 0.0
                    for _, food in ipairs(food_list) do
                        local fd = dist(bot.x, bot.z, food.x, food.z)
                        if fd < min_fd then
                            min_fd = fd
                            close_fx = food.x
                            close_fz = food.z
                        end
                    end
                    b_target_x = close_fx
                    b_target_z = close_fz
                    b_urge = 1.0
                end
            end

            local b_dx = b_target_x - bot.x
            local b_dz = b_target_z - bot.z
            local b_len = math.sqrt(b_dx * b_dx + b_dz * b_dz)
            if b_len > 0.1 then
                local b_accel = 12.0 * b_urge
                bot.vx = bot.vx + (b_dx / b_len) * b_accel * dt
                bot.vz = bot.vz + (b_dz / b_len) * b_accel * dt
            end

            -- Bot friction
            bot.vx = bot.vx * (1.0 - 3.5 * dt)
            bot.vz = bot.vz * (1.0 - 3.5 * dt)
            bot.x = bot.x + bot.vx * dt
            bot.z = bot.z + bot.vz * dt
            bot.x = clamp(bot.x, -map_bound, map_bound)
            bot.z = clamp(bot.z, -map_bound, map_bound)

            -- Update bot scale
            bot.scale = 0.8 + (bot.mass / 100.0) * 0.5
            tween_position(bot.id, bot.x, bot.scale * 0.5, bot.z, 0.0)

            -- Bot eats food
            for f_idx, food in ipairs(food_list) do
                if dist(bot.x, bot.z, food.x, food.z) < (bot.scale * 0.5 + 0.35) then
                    bot.mass = bot.mass + 2.0
                    destroy_entity(food.id)
                    table.remove(food_list, f_idx)
                    spawn_single_food()
                    break
                end
            end

            -- Bot vs Player Collision
            local p_dist = dist(entity.x, entity.z, bot.x, bot.z)
            if p_dist < (player_r + bot.scale * 0.5) then
                if player_mass >= bot.mass then
                    -- Player eats Bot!
                    player_mass = player_mass + bot.mass * 0.65
                    spawn_particles(bot.x, 0.8, bot.z, 0.9, 0.3, 0.4, 35, 6.0, 0.7)
                    play_sound("assets/sfx/bounce.wav", 0.7)
                    camera_shake(0.4, 0.25)
                    destroy_entity(bot.id)
                    table.remove(bots_list, i_b)
                    spawn_single_bot()
                else
                    -- Bot hurts Player
                    player_mass = math.max(10.0, player_mass - 18.0)
                    local rep_x = (entity.x - bot.x) / (p_dist + 0.01)
                    local rep_z = (entity.z - bot.z) / (p_dist + 0.01)
                    player_vx = player_vx + rep_x * 18.0
                    player_vz = player_vz + rep_z * 18.0
                    camera_shake(0.8, 0.4)
                    play_sound("assets/sfx/bounce.wav", 0.8)
                    spawn_particles(entity.x, entity.y, entity.z, 1.0, 0.2, 0.2, 20, 5.0, 0.5)
                    i_b = i_b + 1
                end
            else
                i_b = i_b + 1
            end
        end
    end

    -- 12. Gamma Radiation Storm Cycle
    storm_timer = storm_timer + dt
    local cycle_time = storm_timer % (storm_interval + storm_duration)

    if cycle_time >= (storm_interval - 5.0) and cycle_time < storm_interval then
        storm_warning = true
        storm_active = false
    elseif cycle_time >= storm_interval then
        storm_warning = false
        storm_active = true
    else
        storm_warning = false
        storm_active = false
    end

    if storm_active then
        local p_dist_origin = math.sqrt(entity.x * entity.x + entity.z * entity.z)
        if p_dist_origin > SANCTUARY_RADIUS then
            -- Radiation drains mass!
            player_mass = math.max(10.0, player_mass - 14.0 * dt)
            if math.random() < 0.25 then
                spawn_particles(entity.x, entity.y, entity.z, 1.0, 0.1, 0.1, 5, 2.5, 0.3)
            end
        else
            -- Safe inside sanctuary!
            if math.random() < 0.15 then
                spawn_particles(entity.x, 0.2, entity.z, 0.0, 1.0, 0.5, 3, 2.0, 0.4)
            end
        end
    end

    -- 13. Evolution Stage 1 (at 250 TJ)
    if player_mass >= 250.0 and not evolved then
        if is_key_just_pressed("1") then
            evolution = "Sol"
            evolved = true
            set_light("player_cosmos", 1.0, 0.85, 0.25, 8.0, 26.0)
            camera_shake(0.8, 0.5)
            spawn_particles(entity.x, entity.y, entity.z, 1.0, 0.85, 0.2, 60, 8.0, 0.8)
            play_sound("assets/sfx/bounce.wav", 0.9)
        elseif is_key_just_pressed("2") then
            evolution = "Pulsar"
            evolved = true
            set_light("player_cosmos", 0.2, 1.0, 0.9, 10.0, 28.0)
            camera_shake(0.8, 0.5)
            spawn_particles(entity.x, entity.y, entity.z, 0.2, 1.0, 0.9, 60, 8.0, 0.8)
            play_sound("assets/sfx/bounce.wav", 0.9)
        elseif is_key_just_pressed("3") then
            evolution = "Agujero Negro"
            evolved = true
            set_light("player_cosmos", 0.72, 0.16, 1.0, 10.0, 32.0)
            camera_shake(0.8, 0.5)
            spawn_particles(entity.x, entity.y, entity.z, 0.72, 0.16, 1.0, 60, 8.0, 0.8)
            play_sound("assets/sfx/bounce.wav", 0.9)
        end
    end

    -- Sol Orbiting Planets (set_parent)
    if evolution == "Sol" and not planets_spawned then
        planets_spawned = true
        for p_idx = 1, 3 do
            local pid = "orbit_planet_" .. p_idx
            local p_ang = (p_idx - 1) * (2.0 * math.pi / 3.0)
            local p_dist_orb = 2.2 + p_idx * 0.7
            spawn_entity({
                id = pid,
                mesh = "sphere",
                x = math.cos(p_ang) * p_dist_orb,
                y = 0.0,
                z = math.sin(p_ang) * p_dist_orb,
                sx = 0.45,
                sy = 0.45,
                sz = 0.45,
                parent = "player_cosmos",
                color = {0.3 + p_idx * 0.2, 0.8, 1.0, 1.0},
                emissive = 0.7
            })
            set_parent(pid, "player_cosmos")
        end
    end

    -- Rotate orbiting planets
    if evolution == "Sol" and planets_spawned then
        planet_orbit_angle = planet_orbit_angle + 2.5 * dt
        for p_idx = 1, 3 do
            local pid = "orbit_planet_" .. p_idx
            local p_ang = planet_orbit_angle + (p_idx - 1) * (2.0 * math.pi / 3.0)
            local p_dist_orb = 2.2 + p_idx * 0.7
            local lpx = math.cos(p_ang) * p_dist_orb
            local lpz = math.sin(p_ang) * p_dist_orb
            tween_position(pid, lpx, 0.0, lpz, 0.0)
        end
    end

    -- 14. Boss Apophis Spawning & Behavior
    if (game_time >= 20.0 or player_mass >= 180.0) and not boss_spawned then
        boss_spawned = true
        boss_alive = true
        boss_hp = boss_max_hp
        boss_x = 0.0
        boss_z = -55.0
        spawn_prefab("cosmos_boss", boss_x, 3.5, boss_z)
        camera_shake(1.2, 1.0)
        spawn_particles(boss_x, 3.5, boss_z, 1.0, 0.2, 0.05, 70, 9.0, 1.0)
        play_sound("assets/sfx/bounce.wav", 1.0)
    end

    if boss_alive then
        boss_mine_timer = boss_mine_timer + dt

        -- Boss chases player outside sanctuary
        local b_to_p_dist = dist(boss_x, boss_z, entity.x, entity.z)
        local p_to_center = math.sqrt(entity.x * entity.x + entity.z * entity.z)

        if p_to_center > SANCTUARY_RADIUS then
            -- Player outside: pursue player
            local b_dir_x = (entity.x - boss_x) / (b_to_p_dist + 0.01)
            local b_dir_z = (entity.z - boss_z) / (b_to_p_dist + 0.01)
            boss_vx = boss_vx + b_dir_x * 9.0 * dt
            boss_vz = boss_vz + b_dir_z * 9.0 * dt
        else
            -- Player inside sanctuary: patrol perimeter
            local angle_patrol = game_time * 0.5
            local target_px = math.cos(angle_patrol) * 48.0
            local target_pz = math.sin(angle_patrol) * 48.0
            local d_patrol = dist(boss_x, boss_z, target_px, target_pz)
            boss_vx = boss_vx + ((target_px - boss_x) / (d_patrol + 0.01)) * 10.0 * dt
            boss_vz = boss_vz + ((target_pz - boss_z) / (d_patrol + 0.01)) * 10.0 * dt
        end

        boss_vx = boss_vx * (1.0 - 2.5 * dt)
        boss_vz = boss_vz * (1.0 - 2.5 * dt)
        boss_x = boss_x + boss_vx * dt
        boss_z = boss_z + boss_vz * dt
        tween_position(boss_id, boss_x, 3.5, boss_z, 0.0)

        -- Boss drops mines every 6s
        if boss_mine_timer >= 6.0 then
            boss_mine_timer = 0.0
            local mid = "mine_" .. get_next_uid()
            spawn_prefab("cosmos_mine", boss_x, 0.6, boss_z)
            table.insert(mines_list, { id = mid, x = boss_x, z = boss_z, life = 18.0 })
            spawn_particles(boss_x, 1.0, boss_z, 1.0, 0.1, 0.0, 15, 4.0, 0.4)
        end

        -- Boss collision with player
        if b_to_p_dist < (player_r + 3.0) then
            player_mass = math.max(10.0, player_mass - 35.0 * dt)
            player_vx = player_vx + (entity.x - boss_x) * 4.0 * dt
            player_vz = player_vz + (entity.z - boss_z) * 4.0 * dt
            camera_shake(0.5, 0.2)
        end

        -- Boss Defeat Check
        if boss_hp <= 0.0 then
            boss_alive = false
            spawn_particles(boss_x, 3.5, boss_z, 1.0, 0.3, 0.0, 120, 12.0, 1.8)
            camera_shake(1.5, 1.5)
            play_sound("assets/sfx/bounce.wav", 1.0)
            destroy_entity(boss_id)
            player_mass = player_mass + 500.0
            dark_matter = dark_matter + 150.0
            save_data("cosmos_dm", dark_matter)
        end
    end

    -- 15. Record Update & Save
    if player_mass > record_tj then
        record_tj = player_mass
        save_data("cosmos_record", record_tj)
    end

    -- 16. Rich HUD Rendering
    set_hud_text("1_MASA", string.format("%.1f TJ", player_mass))
    set_hud_text("2_RECORD", string.format("%.1f TJ", record_tj))
    set_hud_text("3_MATERIA_OSCURA", string.format("%d DM", math.floor(dark_matter)))

    local evo_str = (evolution == "None") and "Proto-Estrella" or evolution
    set_hud_text("4_EVOLUCION", evo_str)

    local skills_str = "[Shift] Dash | [C] Succión | [Espacio] Disparo | [F] Mina"
    if evolution == "Pulsar" then
        skills_str = skills_str .. " | [E] EMP"
    end
    set_hud_text("5_HABILIDADES", skills_str)

    if boss_alive then
        set_hud_text("6_JEFE", string.format("APOPHIS HP: %d / %d", math.max(0, math.floor(boss_hp)), math.floor(boss_max_hp)))
    else
        set_hud_text("6_JEFE", boss_spawned and "APOPHIS DERROTADO (+150 DM)" or "En acecho...")
    end

    if storm_warning then
        set_hud_text("7_EVENTO", "⚠️ TORMENTA GAMMA EN 5s — VE AL SANTUARIO (0,0)")
    elseif storm_active then
        set_hud_text("7_EVENTO", "⚡ TORMENTA GAMMA ACTIVA — QUÉDATE EN EL SANTUARIO")
    else
        set_hud_text("7_EVENTO", "Espacio Estable")
    end

    if player_mass >= 250.0 and not evolved then
        set_hud_text("8_UPGRADE", "⭐ EVOLUCIÓN: [1] Sol | [2] Púlsar | [3] Agujero Negro")
    else
        set_hud_text("8_UPGRADE", "")
    end
end
