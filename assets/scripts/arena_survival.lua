-- ============================================================================
-- ARENA SURVIVAL - ARCADE GAMEPLAY SCRIPT
-- Motor-dif Native Engine (Lua 5.4, game_ui, get_mouse_world_pos, VFX Forge, Audio Procedural)
-- ============================================================================

-- Game State
local lives = 3
local max_lives = 3
local score = 0
local game_over = false

-- Timers & Cooldowns
local spawn_timer = 0.0
local spawn_interval = 3.0
local shoot_cooldown = 0.0
local shoot_rate = 0.22

-- Arena Configuration (20x20 m -> boundaries -9.5 to +9.5)
local ARENA_HALF = 9.4
local move_speed = 7.0
local enemy_speed = 3.2

-- Dynamic Entities Tracking
local enemies_list = {}
local projectiles_list = {}
local uid_counter = 100

-- Aim Direction
local aim_x = 0.0
local aim_z = -1.0

-- Player Position Tracker for API queries
local current_player_x = 0.0
local current_player_z = 0.0

-- Start background ambient adaptive music
play_music_preset("cyber_action")
set_music_intensity(0.65)

local function get_next_uid()
    uid_counter = uid_counter + 1
    return tostring(uid_counter)
end

local function dist(x1, z1, x2, z2)
    local dx = x1 - x2
    local dz = z1 - z2
    return math.sqrt(dx * dx + dz * dz)
end

-- ============================================================================
-- RESTART / RESET LOGIC
-- ============================================================================
function restart_survival()
    -- 1. Clean all enemies from scene
    for _, enemy in ipairs(enemies_list) do
        if enemy.id then
            destroy_entity(enemy.id)
        end
    end
    enemies_list = {}

    -- 2. Clean all projectiles from scene
    for _, proj in ipairs(projectiles_list) do
        if proj.id then
            destroy_entity(proj.id)
        end
    end
    projectiles_list = {}

    -- 3. Reset Game State variables
    lives = max_lives
    score = 0
    spawn_timer = 0.0
    shoot_cooldown = 0.0
    game_over = false

    -- 4. Reset Player Position to center
    current_player_x = 0.0
    current_player_z = 0.0
    tween_position("player", 0.0, 0.5, 0.0, 0.0)

    -- 5. Clear Declarative UI to remove Game Over screen
    ui_clear()

    -- 6. Procedural Audio fanfare & VFX
    play_tone(440.0, 880.0, "sine", 0.22, 0.55)
    spawn_vfx("magic_portal", 0.0, 0.5, 0.0)
    spawn_floating_text(0.0, 2.2, 0.0, "ARENA REINICIADA", 0.25, 0.95, 0.45, 1.3)
    set_music_intensity(0.65)
end

-- ============================================================================
-- ENEMY SPAWNING (Every 3 seconds on a random border)
-- ============================================================================
local function spawn_border_enemy(custom_x, custom_z)
    local ex, ez = 0.0, 0.0
    if custom_x and custom_z then
        ex = custom_x
        ez = custom_z
    else
        local side = math.random(1, 4)
        local spread = (math.random() * 18.0) - 9.0
        if side == 1 then -- North border
            ex = spread
            ez = -ARENA_HALF
        elseif side == 2 then -- South border
            ex = spread
            ez = ARENA_HALF
        elseif side == 3 then -- West border
            ex = -ARENA_HALF
            ez = spread
        else -- East border
            ex = ARENA_HALF
            ez = spread
        end
    end

    local enemy_id = "enemy_" .. get_next_uid()

    spawn_entity({
        id = enemy_id,
        mesh = "cube",
        x = ex,
        y = 0.5,
        z = ez,
        sx = 0.85,
        sy = 0.85,
        sz = 0.85,
        color = {0.95, 0.22, 0.28, 1.0},
        emissive = 0.65
    })

    table.insert(enemies_list, {
        id = enemy_id,
        x = ex,
        z = ez,
        speed = enemy_speed
    })

    -- Procedural audio alert & VFX on spawn
    play_tone(180.0, 340.0, "sine", 0.12, 0.3)
    spawn_vfx("hit_spark", ex, 0.5, ez)
end

-- ============================================================================
-- SHOOTING LOGIC (Fires projectile towards aim)
-- ============================================================================
local function fire_projectile(dir_x, dir_z)
    local pid = "proj_" .. get_next_uid()
    local px = current_player_x + dir_x * 0.9
    local pz = current_player_z + dir_z * 0.9

    spawn_entity({
        id = pid,
        mesh = "sphere",
        x = px,
        y = 0.5,
        z = pz,
        sx = 0.35,
        sy = 0.35,
        sz = 0.35,
        color = {1.0, 0.92, 0.25, 1.0},
        emissive = 1.0
    })

    table.insert(projectiles_list, {
        id = pid,
        x = px,
        z = pz,
        vx = dir_x * 24.0,
        vz = dir_z * 24.0,
        life = 1.6
    })

    -- Procedural retro laser audio (sawtooth downsweep)
    play_tone(960.0, 220.0, "sawtooth", 0.08, 0.35)
    spawn_vfx("hit_spark", px, 0.5, pz)
end

-- ============================================================================
-- DAMAGE TO PLAYER LOGIC
-- ============================================================================
local function apply_damage_to_player()
    if game_over then return end

    lives = lives - 1

    -- Procedural damage buzz sound & explosion VFX
    play_tone(220.0, 70.0, "square", 0.22, 0.7)
    spawn_vfx("explosion", current_player_x, 0.5, current_player_z)
    camera_shake(0.45, 0.3)
    spawn_floating_text(current_player_x, 2.0, current_player_z, "-1 VIDA!", 1.0, 0.2, 0.2, 1.0)

    if lives <= 0 then
        lives = 0
        game_over = true
        -- Procedural Game Over tone
        play_tone(290.0, 50.0, "sawtooth", 0.65, 0.8)
        spawn_floating_text(current_player_x, 2.6, current_player_z, "GAME OVER", 1.0, 0.15, 0.15, 2.5)
        set_music_intensity(0.2)
    end
end

-- ============================================================================
-- UI HUD & GAME OVER RENDERING (game_ui)
-- ============================================================================
local function update_game_ui()
    -- Normal HUD Panel (Top-Left)
    ui_panel("hud_bg", "top_left", 20, 20, 240, 96, 0.06, 0.09, 0.16, 0.88)
    ui_label("hud_title", "top_left", 32, 28, "ARENA SURVIVAL", 13.0, 0.28, 0.85, 1.0)
    ui_label("hud_score", "top_left", 32, 48, "PUNTAJE: " .. tostring(score), 15.0, 1.0, 0.85, 0.2)
    
    local hp_ratio = math.max(0.0, lives / max_lives)
    local hp_color_r = (lives == 1) and 0.95 or 0.2
    local hp_color_g = (lives == 1) and 0.2 or 0.85
    local hp_color_b = 0.3
    ui_bar("hud_lives_bar", "top_left", 32, 74, 216, 18, hp_ratio, "VIDAS: " .. tostring(lives) .. " / 3", hp_color_r, hp_color_g, hp_color_b)

    -- Controls Info Panel (Top-Right)
    ui_panel("info_bg", "top_right", 20, 20, 210, 68, 0.06, 0.09, 0.16, 0.82)
    ui_label("info_t", "top_right", 28, 28, "CONTROLES", 12.0, 0.7, 0.82, 0.95)
    ui_label("info_move", "top_right", 28, 46, "WASD / Flechas: Moverse", 10.5, 0.6, 0.72, 0.85)
    ui_label("info_fire", "top_right", 28, 62, "Click / Espacio: Disparar", 10.5, 0.6, 0.72, 0.85)

    -- Game Over Screen Overlay (Center)
    if game_over then
        ui_panel("go_modal", "center", 0, 0, 380, 230, 0.09, 0.03, 0.04, 0.96)
        ui_label("go_header", "center", 0, -68, "GAME OVER", 26.0, 1.0, 0.2, 0.2)
        ui_label("go_reason", "center", 0, -28, "Tu nave ha sido consumida por la arena", 13.0, 0.82, 0.82, 0.88)
        ui_label("go_final_score", "center", 0, 6, "Puntaje Final: " .. tostring(score), 16.0, 1.0, 0.85, 0.2)
        ui_button("btn_restart", "center", 0, 56, 210, 42, "REINICIAR JUEGO", "restart_survival")
        ui_label("go_hint", "center", 0, 90, "o pulsa [R] para reiniciar", 11.0, 0.65, 0.65, 0.75)
    end
end

-- ============================================================================
-- MAIN ENGINE UPDATE LOOP (on_update)
-- ============================================================================
function on_update(entity, dt)
    if dt <= 0.0 then return end
    if dt > 0.1 then dt = 0.1 end

    current_player_x = entity.x
    current_player_z = entity.z

    -- R key quick restart
    if is_key_just_pressed("R") then
        restart_survival()
        return
    end

    -- Timers
    if shoot_cooldown > 0.0 then
        shoot_cooldown = shoot_cooldown - dt
    end

    -- 1. Player Movement (WASD / Arrows)
    if not game_over then
        local mx = 0.0
        local mz = 0.0

        if is_key_down("W") or is_key_down("ArrowUp") or is_action_down("move_up") then mz = mz - 1.0 end
        if is_key_down("S") or is_key_down("ArrowDown") or is_action_down("move_down") then mz = mz + 1.0 end
        if is_key_down("A") or is_key_down("ArrowLeft") or is_action_down("move_left") then mx = mx - 1.0 end
        if is_key_down("D") or is_key_down("ArrowRight") or is_action_down("move_right") then mx = mx + 1.0 end

        local mlen = math.sqrt(mx * mx + mz * mz)
        if mlen > 0.01 then
            entity.x = entity.x + (mx / mlen) * move_speed * dt
            entity.z = entity.z + (mz / mlen) * move_speed * dt
        end

        -- Strictly clamp inside closed 20x20 arena
        entity.x = math.max(-ARENA_HALF, math.min(ARENA_HALF, entity.x))
        entity.z = math.max(-ARENA_HALF, math.min(ARENA_HALF, entity.z))
        current_player_x = entity.x
        current_player_z = entity.z
    end

    -- 2. Aim towards Mouse World Position (plane_y = 0.5)
    local mouse = get_mouse_world_pos(0.5)
    if mouse and mouse.hit then
        local mdx = mouse.x - entity.x
        local mdz = mouse.z - entity.z
        local mdist = math.sqrt(mdx * mdx + mdz * mdz)
        if mdist > 0.15 then
            aim_x = mdx / mdist
            aim_z = mdz / mdist
            entity.ry = math.atan2(mdx, mdz)
        end
    end

    -- 3. Player Shooting (Click / Space / Fire action)
    if not game_over and shoot_cooldown <= 0.0 then
        local firing = is_action_just_pressed("fire") or is_key_just_pressed("Space") or is_action_down("fire")
        if firing then
            fire_projectile(aim_x, aim_z)
            shoot_cooldown = shoot_rate
        end
    end

    -- 4. Enemy Periodic Spawning (Every 3 seconds on a random border)
    if not game_over then
        spawn_timer = spawn_timer + dt
        if spawn_timer >= spawn_interval then
            spawn_timer = spawn_timer - spawn_interval
            spawn_border_enemy()
        end
    end

    -- 5. Enemy AI (Pursue Player & Collision Check)
    local surviving_enemies = {}
    for _, enemy in ipairs(enemies_list) do
        local edx = entity.x - enemy.x
        local edz = entity.z - enemy.z
        local edist = math.sqrt(edx * edx + edz * edz)
        local remove_enemy = false

        if edist < 0.85 then
            -- Enemy reaches player: damage dealt!
            apply_damage_to_player()
            destroy_entity(enemy.id)
            remove_enemy = true
        else
            if not game_over and edist > 0.05 then
                local step = enemy.speed * dt
                enemy.x = enemy.x + (edx / edist) * step
                enemy.z = enemy.z + (edz / edist) * step
                tween_position(enemy.id, enemy.x, 0.5, enemy.z, 0.0)
            end
        end

        if not remove_enemy then
            table.insert(surviving_enemies, enemy)
        end
    end
    enemies_list = surviving_enemies

    -- 6. Projectiles Movement & Enemy Destruction Check
    local surviving_projs = {}
    for _, proj in ipairs(projectiles_list) do
        proj.x = proj.x + proj.vx * dt
        proj.z = proj.z + proj.vz * dt
        proj.life = proj.life - dt
        tween_position(proj.id, proj.x, 0.5, proj.z, 0.0)

        local proj_destroyed = false

        -- Bounds & lifetime check
        if math.abs(proj.x) > (ARENA_HALF + 0.5) or math.abs(proj.z) > (ARENA_HALF + 0.5) or proj.life <= 0.0 then
            destroy_entity(proj.id)
            proj_destroyed = true
        else
            -- Check hit against all active enemies
            for idx = #enemies_list, 1, -1 do
                local enemy = enemies_list[idx]
                if dist(proj.x, proj.z, enemy.x, enemy.z) < 0.78 then
                    -- ENEMY DESTROYED BY PROJECTILE!
                    destroy_entity(enemy.id)
                    table.remove(enemies_list, idx)
                    
                    destroy_entity(proj.id)
                    proj_destroyed = true

                    score = score + 100
                    play_tone(420.0, 110.0, "noise", 0.12, 0.6)
                    spawn_vfx("explosion", enemy.x, 0.5, enemy.z)
                    spawn_floating_text(enemy.x, 1.8, enemy.z, "+100", 1.0, 0.85, 0.2, 0.8)
                    break
                end
            end
        end

        if not proj_destroyed then
            table.insert(surviving_projs, proj)
        end
    end
    projectiles_list = surviving_projs

    -- 7. Update Declarative HUD & Game Over UI
    update_game_ui()

    -- 8. Sync live telemetry to shared save_data store
    save_data("survival_telemetry", {
        lives = lives,
        max_lives = max_lives,
        score = score,
        enemies_count = #enemies_list,
        projectiles_count = #projectiles_list,
        game_over = game_over,
        player_x = current_player_x,
        player_z = current_player_z,
        spawn_timer = spawn_timer
    })
end

-- ============================================================================
-- EVENT LISTENERS (Cross-Context Signal Handlers via emit_event)
-- ============================================================================
function on_cmd_damage_player(payload)
    apply_damage_to_player()
end

function on_cmd_spawn_enemy(payload)
    local cx = payload and payload.x
    local cz = payload and payload.z
    spawn_border_enemy(cx, cz)
end

function on_cmd_fire_projectile(payload)
    local dx = (payload and payload.dx) or 0.0
    local dz = (payload and payload.dz) or -1.0
    local len = math.sqrt(dx * dx + dz * dz)
    if len > 0.001 then
        dx = dx / len
        dz = dz / len
    end
    fire_projectile(dx, dz)
end

on_event("cmd_damage_player", "on_cmd_damage_player")
on_event("cmd_spawn_enemy", "on_cmd_spawn_enemy")
on_event("cmd_fire_projectile", "on_cmd_fire_projectile")
on_event("cmd_restart", "restart_survival")

-- ============================================================================
-- GLOBAL TEST & INSPECTION HOOKS
-- ============================================================================
function get_survival_state()
    return {
        lives = lives,
        max_lives = max_lives,
        score = score,
        enemies_count = #enemies_list,
        projectiles_count = #projectiles_list,
        game_over = game_over,
        player_x = current_player_x,
        player_z = current_player_z,
        spawn_timer = spawn_timer
    }
end

function test_force_damage()
    apply_damage_to_player()
    return get_survival_state()
end

function test_spawn_enemy(x, z)
    spawn_border_enemy(x, z)
    return get_survival_state()
end

function test_fire_projectile(dir_x, dir_z)
    local dx = dir_x or 0.0
    local dz = dir_z or -1.0
    local len = math.sqrt(dx * dx + dz * dz)
    if len > 0.001 then
        dx = dx / len
        dz = dz / len
    end
    fire_projectile(dx, dz)
    return get_survival_state()
end
