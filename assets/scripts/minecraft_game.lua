-- ============================================================================
-- MOTOR-DIF: REALISTIC & FAITHFUL MINECRAFT RECREATION (V2 - CLEAN & POLISHED)
-- Scene: assets/minecraft_world.json
-- Script: assets/scripts/minecraft_game.lua
-- Fixes:
-- 1. Jumping: Kinematic velocity integration (ZERO runtime errors!).
-- 2. 1st Person: Hides entire character body (scale = 0.0001) for unobstructed view.
-- 3. Steve Model: Modular parts parented to player_steve, animated in 3rd person.
-- ============================================================================

local initialized = false
local total_time = 0.0

-- Camera & Player State
local cam_mode = "1st" -- "1st" or "3rd"
local yaw = 0.0       -- Radians (0 = facing -Z)
local pitch = 0.0     -- Radians (0 = horizon)
local walk_speed = 5.2
local sprint_speed = 8.5
local is_sprinting = false

-- Kinematic Jumping & Gravity System
local vel_y = 0.0
local is_grounded = true
local jump_impulse = 7.8
local gravity = 22.0
local footstep_timer = 0.0

-- Safe sound helper
local function safe_play_tone(f_start, f_end, wave_type, duration, vol)
    if play_tone then
        pcall(play_tone, f_start, f_end, wave_type, duration, vol)
    end
end

-- Health, Hunger & Exp
local health = 20.0
local max_health = 20.0
local hunger = 18.0
local max_hunger = 20.0
local exp_progress = 0.72
local player_level = 30
local damage_flash = 0.0

-- Tool Swing Animation State
local swing_timer = 0.0
local is_swinging = false

-- Day / Night Cycle (120 seconds full cycle)
local day_night_cycle_len = 120.0
local time_of_day = 0.25

-- Hotbar Inventory (9 Slots)
local active_slot = 1
local hotbar = {
    { name = "Grass Block",     tex = "assets/textures/mc_grass.png",        count = 64, color = {1.0, 1.0, 1.0, 1.0}, is_block = true },
    { name = "Dirt",            tex = "assets/textures/mc_dirt.png",         count = 64, color = {1.0, 1.0, 1.0, 1.0}, is_block = true },
    { name = "Stone",           tex = "assets/textures/mc_stone.png",        count = 64, color = {1.0, 1.0, 1.0, 1.0}, is_block = true },
    { name = "Oak Wood",        tex = "assets/textures/mc_oak_log.png",      count = 64, color = {1.0, 1.0, 1.0, 1.0}, is_block = true },
    { name = "Oak Planks",      tex = "assets/textures/mc_oak_planks.png",   count = 64, color = {1.0, 1.0, 1.0, 1.0}, is_block = true },
    { name = "Cobblestone",     tex = "assets/textures/mc_cobblestone.png",  count = 64, color = {1.0, 1.0, 1.0, 1.0}, is_block = true },
    { name = "Torch",           tex = "assets/textures/mc_torch.png",        count = 32, color = {1.0, 0.9, 0.7, 1.0}, is_block = true, is_torch = true },
    { name = "Diamond Ore",     tex = "assets/textures/mc_diamond_ore.png",  count = 16, color = {1.0, 1.0, 1.0, 1.0}, is_block = true },
    { name = "Diamond Pickaxe", tex = "assets/textures/mc_diamond_ore.png",  count = 1,  color = {0.3, 0.9, 1.0, 1.0}, is_block = false }
}

-- Targeted Block info
local target_entity = nil
local target_pos = nil
local target_normal = nil
local place_cooldown = 0.0
local mine_cooldown = 0.0

-- UI State
local show_f3_debug = false
local show_crafting = false
local slot_msg_timer = 0.0
local slot_msg_text = ""

-- Music timer
local music_timer = 6.0
local chord_progression = {
    { 261.63, 329.63, 392.00 }, -- C Major
    { 349.23, 440.00, 523.25 }, -- F Major
    { 392.00, 493.88, 587.33 }, -- G Major
    { 220.00, 261.63, 329.63 }  -- A Minor
}
local chord_idx = 1

local function play_minecraft_chord()
    local chord = chord_progression[chord_idx]
    chord_idx = (chord_idx % #chord_progression) + 1
    safe_play_tone(chord[1], chord[1] * 0.99, "sine", 2.2, 0.18)
    if after then
        after(0.3, "play_sub_note1")
        after(0.6, "play_sub_note2")
    end
end

function play_sub_note1()
    local chord = chord_progression[chord_idx]
    safe_play_tone(chord[2], chord[2] * 0.99, "sine", 2.0, 0.15)
end

function play_sub_note2()
    local chord = chord_progression[chord_idx]
    safe_play_tone(chord[3], chord[3] * 0.99, "sine", 2.2, 0.15)
end

-- Outline drawing for targeted voxel block
local function draw_block_outline(bx, by, bz)
    local min_x, max_x = bx - 0.505, bx + 0.505
    local min_y, max_y = by - 0.505, by + 0.505
    local min_z, max_z = bz - 0.505, bz + 0.505
    local r, g, b, a = 0.1, 0.1, 0.1, 0.95

    if draw_line then
        -- Bottom square
        draw_line(min_x, min_y, min_z, max_x, min_y, min_z, r, g, b, a)
        draw_line(max_x, min_y, min_z, max_x, min_y, max_z, r, g, b, a)
        draw_line(max_x, min_y, max_z, min_x, min_y, max_z, r, g, b, a)
        draw_line(min_x, min_y, max_z, min_x, min_y, min_z, r, g, b, a)

        -- Top square
        draw_line(min_x, max_y, min_z, max_x, max_y, min_z, r, g, b, a)
        draw_line(max_x, max_y, min_z, max_x, max_y, max_z, r, g, b, a)
        draw_line(max_x, max_y, max_z, min_x, max_y, max_z, r, g, b, a)
        draw_line(min_x, max_y, max_z, min_x, max_y, min_z, r, g, b, a)

        -- Vertical pillars
        draw_line(min_x, min_y, min_z, min_x, max_y, min_z, r, g, b, a)
        draw_line(max_x, min_y, min_z, max_x, max_y, min_z, r, g, b, a)
        draw_line(max_x, min_y, max_z, max_x, max_y, max_z, r, g, b, a)
        draw_line(min_x, min_y, max_z, min_x, max_y, max_z, r, g, b, a)
    end
end

function on_test_jump_event()
    vel_y = jump_impulse
    is_grounded = false
    slot_msg_text = "Jump Test Triggered!"
    slot_msg_timer = 3.0
    safe_play_tone(240, 360, "sine", 0.12, 0.40)
end

function on_test_cam_3rd_event()
    cam_mode = "3rd"
    slot_msg_text = "Camera: Third-Person (Steve)"
    slot_msg_timer = 3.0
end

function on_test_cam_1st_event()
    cam_mode = "1st"
    slot_msg_text = "Camera: First-Person"
    slot_msg_timer = 3.0
end

-- ============================================================================
-- INITIALIZATION
-- ============================================================================
local function on_init(entity)
    yaw = 0.0
    pitch = 0.0
    vel_y = 0.0
    is_grounded = true
    cam_mode = "1st"

    -- Initial position at ground level
    entity.x = 0.0
    entity.y = 3.5
    entity.z = 0.0

    -- Hide character in 1st person
    entity.sx = 0.0001
    entity.sy = 0.0001
    entity.sz = 0.0001

    if set_environment then
        set_environment("gradient", 0.75, 0.88, 1.0, 0.003, {0.35, 0.65, 0.95}, {0.75, 0.88, 1.0})
    end

    slot_msg_text = "Welcome to Minecraft (Motor-dif)!"
    slot_msg_timer = 4.0

    -- Register Test Events using string callback names
    if on_event then
        on_event("test_jump", "on_test_jump_event")
        on_event("test_camera_3rd", "on_test_cam_3rd_event")
        on_event("test_camera_1st", "on_test_cam_1st_event")
    end
end

-- ============================================================================
-- MAIN FRAME UPDATE
-- ============================================================================
function on_update(entity, dt)
    if not initialized then
        initialized = true
        on_init(entity)
    end

    total_time = total_time + dt
    if place_cooldown > 0 then place_cooldown = place_cooldown - dt end
    if mine_cooldown > 0 then mine_cooldown = mine_cooldown - dt end
    if slot_msg_timer > 0 then slot_msg_timer = slot_msg_timer - dt end
    if damage_flash > 0 then damage_flash = damage_flash - dt end

    -- 1. Camera Look (Mouse Delta + Arrow Keys)
    local m_delta = get_mouse_delta and get_mouse_delta() or nil
    local mouse_sens = 0.0035
    if m_delta and (m_delta.dx ~= 0 or m_delta.dy ~= 0) then
        yaw = yaw - m_delta.dx * mouse_sens
        pitch = pitch - m_delta.dy * mouse_sens
    end

    local key_look_speed = 1.8 * dt
    if is_key_down("LEFT") then yaw = yaw + key_look_speed end
    if is_key_down("RIGHT") then yaw = yaw - key_look_speed end
    if is_key_down("UP") then pitch = pitch + key_look_speed end
    if is_key_down("DOWN") then pitch = pitch - key_look_speed end

    if pitch > 1.42 then pitch = 1.42 end
    if pitch < -1.42 then pitch = -1.42 end
    entity.ry = yaw

    -- 2. Movement Physics
    is_sprinting = is_key_down("SHIFT") or is_key_down("LSHIFT")
    local cur_speed = is_sprinting and sprint_speed or walk_speed

    local fwd_x = math.sin(yaw)
    local fwd_z = -math.cos(yaw)
    local right_x = math.cos(yaw)
    local right_z = math.sin(yaw)

    local move_x, move_z = 0, 0
    if is_key_down("W") then move_x = move_x + fwd_x; move_z = move_z + fwd_z end
    if is_key_down("S") then move_x = move_x - fwd_x; move_z = move_z - fwd_z end
    if is_key_down("A") then move_x = move_x - right_x; move_z = move_z - right_z end
    if is_key_down("D") then move_x = move_x + right_x; move_z = move_z + right_z end

    local len = math.sqrt(move_x * move_x + move_z * move_z)
    local is_moving = len > 0.01

    if is_moving then
        move_x = (move_x / len) * cur_speed * dt
        move_z = (move_z / len) * cur_speed * dt

        -- Auto-step up 1-block terrain elevation
        if raycast then
            local step_hit = raycast(entity.x, entity.y + 0.3, entity.z, fwd_x, 0, fwd_z, 0.75)
            local head_hit = raycast(entity.x, entity.y + 1.2, entity.z, fwd_x, 0, fwd_z, 0.75)
            if step_hit and step_hit.hit and (not head_hit or not head_hit.hit) then
                entity.y = entity.y + 0.35
                vel_y = 0.0
                is_grounded = true
            end
        end

        entity.x = entity.x + move_x
        entity.z = entity.z + move_z

        footstep_timer = footstep_timer + dt
        local step_interval = is_sprinting and 0.28 or 0.40
        if is_grounded and footstep_timer > step_interval then
            footstep_timer = 0.0
            safe_play_tone(180 + math.random(0, 30), 120, "triangle", 0.06, 0.25)
        end
    else
        footstep_timer = 0.0
    end

    -- 3. Jumping (Kinematic Impulse - Zero Lua Errors!)
    if is_key_just_pressed("SPACE") and is_grounded then
        vel_y = jump_impulse
        is_grounded = false
        safe_play_tone(240, 360, "sine", 0.12, 0.40)
    end

    -- Gravity Integration
    vel_y = vel_y - gravity * dt
    entity.y = entity.y + vel_y * dt

    -- Solid Ground Surface Detection (Top of surface blocks = 3.5)
    local floor_y = 3.5
    if raycast then
        local g_hit = raycast(entity.x, entity.y + 1.0, entity.z, 0, -1, 0, 4.0)
        if g_hit and g_hit.hit and g_hit.entity and string.sub(g_hit.entity, 1, 5) ~= "steve" and g_hit.entity ~= "player_steve" then
            local hit_floor = (entity.y + 1.0) - g_hit.dist
            if hit_floor > floor_y then
                floor_y = hit_floor
            end
        end
    end

    if entity.y <= floor_y and vel_y <= 0.0 then
        if vel_y < -14.0 then
            safe_play_tone(150, 90, "sawtooth", 0.12, 0.6)
            damage_flash = 0.35
        end
        entity.y = floor_y
        vel_y = 0.0
        is_grounded = true
    else
        is_grounded = false
    end

    -- 4. Toggle Camera Mode (V key)
    if is_key_just_pressed("V") then
        cam_mode = (cam_mode == "1st") and "3rd" or "1st"
        safe_play_tone(500, 700, "square", 0.08, 0.35)
        slot_msg_text = "Camera: " .. (cam_mode == "1st" and "First-Person" or "Third-Person (Steve)")
        slot_msg_timer = 2.5
    end

    if is_key_just_pressed("C") then
        show_f3_debug = not show_f3_debug
        safe_play_tone(600, 750, "sine", 0.06, 0.3)
    end

    if is_key_just_pressed("E") then
        show_crafting = not show_crafting
        safe_play_tone(400, 550, "triangle", 0.08, 0.35)
    end

    -- Hotbar selection (Keys 1-9)
    for i = 1, 9 do
        local key_str = tostring(i)
        local digit_str = "DIGIT" .. i
        if is_key_just_pressed(key_str) or is_key_just_pressed(digit_str) then
            if active_slot ~= i then
                active_slot = i
                safe_play_tone(350 + i * 25, 450 + i * 25, "sine", 0.08, 0.4)
                slot_msg_text = hotbar[i].name
                slot_msg_timer = 2.5
            end
        end
    end

    -- Tool Swing Animation
    if is_swinging then
        swing_timer = swing_timer + dt * 10.0
        if swing_timer >= 3.14159 then
            is_swinging = false
            swing_timer = 0.0
        end
    end

    -- 5. Camera & Steve Body Visibility (1st Person vs 3rd Person)
    local cos_p = math.cos(pitch)
    local dir_x = math.sin(yaw) * cos_p
    local dir_y = math.sin(pitch)
    local dir_z = -math.cos(yaw) * cos_p

    local eye_x = entity.x
    local eye_y = entity.y + 1.62 -- Standard Minecraft eye level
    local eye_z = entity.z

    if cam_mode == "1st" then
        -- --------------------------------------------------------------------
        -- FIRST PERSON: Hide player body completely!
        -- Setting scale of player_steve to near-zero scales all parented child
        -- parts (head, torso, arms, legs) to zero, giving a 100% clean view.
        -- --------------------------------------------------------------------
        entity.sx = 0.0001
        entity.sy = 0.0001
        entity.sz = 0.0001
        entity.ry = yaw

        set_camera(eye_x, eye_y, eye_z, eye_x + dir_x, eye_y + dir_y, eye_z + dir_z)
    else
        -- --------------------------------------------------------------------
        -- THIRD PERSON: Steve is fully visible at 1.0 scale!
        -- The engine's ProceduralRig biped automatically animates leg & arm swing.
        -- --------------------------------------------------------------------
        entity.sx = 1.0
        entity.sy = 1.0
        entity.sz = 1.0
        entity.ry = yaw

        local cam_dist = 3.6
        local cx = eye_x - dir_x * cam_dist
        local cy = eye_y - dir_y * cam_dist + 0.6
        local cz = eye_z - dir_z * cam_dist
        set_camera(cx, cy, cz, eye_x, eye_y, eye_z)
    end

    -- 6. Raycast & Voxel Targeting
    if raycast then
        local hit = raycast(eye_x, eye_y, eye_z, dir_x, dir_y, dir_z, 5.2)
        if hit and hit.hit and hit.entity and string.sub(hit.entity, 1, 5) ~= "steve" and hit.entity ~= "player_steve" then
            target_entity = hit.entity
            local b_ent = get_entity and get_entity(target_entity) or nil
            if b_ent then
                target_pos = { x = math.floor(b_ent.x + 0.5), y = math.floor(b_ent.y + 0.5), z = math.floor(b_ent.z + 0.5) }
                target_normal = { nx = hit.nx or 0, ny = hit.ny or 1, nz = hit.nz or 0 }
                draw_block_outline(target_pos.x, target_pos.y, target_pos.z)
            else
                target_pos = nil
            end
        else
            target_entity = nil
            target_pos = nil
        end
    end

    -- 7. Mining
    local do_mine = is_key_down("F") or (is_action_down and is_action_down("fire"))
    if do_mine and mine_cooldown <= 0 and target_entity and target_pos then
        mine_cooldown = (active_slot == 9) and 0.18 or 0.28
        is_swinging = true
        swing_timer = 0.0

        safe_play_tone(170 + math.random(0, 40), 100, "sawtooth", 0.10, 0.65)
        if spawn_particles then
            spawn_particles(target_pos.x, target_pos.y, target_pos.z, 0.45, 0.85, 0.35, 18, 2.2, 0.5)
        end
        if destroy_entity then
            destroy_entity(target_entity)
        end

        hotbar[active_slot].count = hotbar[active_slot].count + 1
        if spawn_floating_text then
            spawn_floating_text(target_pos.x, target_pos.y + 0.7, target_pos.z, "+1 " .. hotbar[active_slot].name, 0.4, 1.0, 0.5, 1.0)
        end

        target_entity = nil
        target_pos = nil
    end

    -- 8. Placing
    local do_place = is_key_just_pressed("R") or (is_action_just_pressed and is_action_just_pressed("alt_fire"))
    if do_place and place_cooldown <= 0 and target_pos and target_normal then
        local slot_item = hotbar[active_slot]
        if slot_item.is_block and slot_item.count > 0 then
            place_cooldown = 0.22
            is_swinging = true
            swing_timer = 0.0

            local px = target_pos.x + math.floor(target_normal.nx + (target_normal.nx > 0 and 0.5 or -0.5))
            local py = target_pos.y + math.floor(target_normal.ny + (target_normal.ny > 0 and 0.5 or -0.5))
            local pz = target_pos.z + math.floor(target_normal.nz + (target_normal.nz > 0 and 0.5 or -0.5))

            local p_dist = math.sqrt((entity.x - px)^2 + (entity.y - py)^2 + (entity.z - pz)^2)
            if p_dist > 0.85 and spawn_entity then
                local new_id = "placed_block_" .. px .. "_" .. py .. "_" .. pz .. "_" .. math.floor(total_time * 100)
                local emiss = slot_item.is_torch and 0.5 or 0.0

                spawn_entity({
                    id = new_id,
                    mesh = "cube",
                    x = px, y = py, z = pz,
                    rx = 0, ry = 0, rz = 0,
                    sx = 1.0, sy = 1.0, sz = 1.0,
                    texture = slot_item.tex,
                    color = slot_item.color,
                    emissive = emiss,
                    physics = {
                        body = "fixed",
                        collider = "cuboid",
                        mass = 0.0,
                        restitution = 0.0
                    }
                })

                if slot_item.is_torch and set_light then
                    set_light(new_id, 1.0, 0.75, 0.35, 4.0, 12.0)
                    if spawn_vfx then spawn_vfx("fire", px, py + 0.3, pz) end
                end

                safe_play_tone(320, 480, "square", 0.08, 0.55)
                if spawn_particles then
                    spawn_particles(px, py, pz, 0.9, 0.9, 0.9, 8, 1.2, 0.3)
                end
                slot_item.count = slot_item.count - 1
            end
        end
    end

    -- 9. Day / Night Sky Progression
    time_of_day = (total_time % day_night_cycle_len) / day_night_cycle_len
    local sun_angle = time_of_day * 6.2831853
    local sun_dist = 45.0
    local sun_x = math.cos(sun_angle) * sun_dist
    local sun_y = math.sin(sun_angle) * sun_dist
    local sun_z = -15.0

    if set_environment then
        if sun_y > 10.0 then
            set_environment("gradient", 0.75, 0.88, 1.0, 0.003, {0.35, 0.65, 0.95}, {0.75, 0.88, 1.0})
        elseif sun_y > -5.0 then
            set_environment("gradient", 0.85, 0.45, 0.25, 0.006, {0.25, 0.15, 0.35}, {0.92, 0.48, 0.18})
        else
            set_environment("gradient", 0.04, 0.06, 0.12, 0.007, {0.02, 0.03, 0.08}, {0.06, 0.08, 0.15})
        end
    end

    -- 10. Declarative HUD (lua_ui)
    if ui_clear then
        ui_clear()

        -- Crosshair
        ui_label("mc_crosshair", "center", -4, -10, "+", 18, 1.0, 1.0, 1.0)

        -- Action / item selected notification
        if slot_msg_timer > 0 then
            ui_label("mc_slot_msg", "bottom_center", -180, -115, slot_msg_text, 14, 1.0, 0.95, 0.5)
        end

        -- 10 Hearts (Health Bar)
        local heart_start_x = -195
        for h = 1, 10 do
            local is_flash = (damage_flash > 0)
            local hr = is_flash and 1.0 or 0.88
            local hg = is_flash and 0.8 or 0.12
            local hb = is_flash and 0.8 or 0.16
            ui_panel("mc_heart_bg_" .. h, "bottom_center", heart_start_x + (h - 1) * 18, -95, 14, 14, 0.15, 0.05, 0.05, 0.9)
            ui_panel("mc_heart_fg_" .. h, "bottom_center", heart_start_x + (h - 1) * 18 + 2, -93, 10, 10, hr, hg, hb, 1.0)
        end

        -- 10 Hunger (Hunger Bar)
        local hunger_start_x = 35
        for g = 1, 10 do
            ui_panel("mc_hunger_bg_" .. g, "bottom_center", hunger_start_x + (g - 1) * 18, -95, 14, 14, 0.2, 0.12, 0.04, 0.9)
            ui_panel("mc_hunger_fg_" .. g, "bottom_center", hunger_start_x + (g - 1) * 18 + 2, -93, 10, 10, 0.85, 0.55, 0.18, 1.0)
        end

        -- Experience Bar & Level
        ui_bar("mc_exp_bar", "bottom_center", -195, -75, 390, 8, exp_progress, "", 0.35, 0.95, 0.25)
        ui_label("mc_exp_lvl", "bottom_center", -8, -84, tostring(player_level), 15, 0.35, 1.0, 0.3)

        -- 9 Hotbar Slots
        local slot_size = 40
        local slot_gap = 4
        local total_hotbar_w = 9 * slot_size + 8 * slot_gap
        local hotbar_start_x = -total_hotbar_w / 2

        for i = 1, 9 do
            local sx = hotbar_start_x + (i - 1) * (slot_size + slot_gap)
            local is_active = (i == active_slot)

            if is_active then
                ui_panel("slot_border_" .. i, "bottom_center", sx - 2, -58, slot_size + 4, slot_size + 4, 1.0, 0.88, 0.25, 0.95)
                ui_panel("slot_bg_" .. i, "bottom_center", sx, -56, slot_size, slot_size, 0.22, 0.22, 0.24, 0.95)
            else
                ui_panel("slot_border_" .. i, "bottom_center", sx - 1, -57, slot_size + 2, slot_size + 2, 0.3, 0.3, 0.32, 0.85)
                ui_panel("slot_bg_" .. i, "bottom_center", sx, -56, slot_size, slot_size, 0.12, 0.12, 0.14, 0.85)
            end

            local item = hotbar[i]
            local init_letter = string.sub(item.name, 1, 2)
            ui_label("slot_num_" .. i, "bottom_center", sx + 4, -53, tostring(i), 11, 0.7, 0.7, 0.7)
            ui_label("slot_name_" .. i, "bottom_center", sx + 12, -42, init_letter, 13, item.color[1], item.color[2], item.color[3])

            if item.is_block and item.count > 0 then
                ui_label("slot_cnt_" .. i, "bottom_center", sx + 18, -26, tostring(item.count), 11, 1.0, 1.0, 1.0)
            end
        end

        -- Top-Left Mini HUD
        ui_panel("mc_top_panel", "top_left", 14, 14, 250, 48, 0.08, 0.09, 0.12, 0.85)
        ui_label("mc_top_title", "top_left", 24, 22, "Minecraft v1.20 - Motor-dif", 13, 0.4, 0.9, 1.0)
        local tod_str = (sun_y > 0) and "Day" or "Night"
        ui_label("mc_top_info", "top_left", 24, 40, "Time: " .. tod_str .. " | Mode: " .. cam_mode .. " | [V] Cam [E] Craft", 11, 0.8, 0.85, 0.9)

        -- F3 Debug Screen
        if show_f3_debug then
            ui_panel("f3_panel", "top_right", -320, 14, 306, 150, 0.05, 0.05, 0.08, 0.9)
            ui_label("f3_l1", "top_right", -308, 22, "Minecraft (Motor-dif Engine 2.0)", 12, 0.95, 0.95, 0.95)
            local px_str = string.format("XYZ: %.2f / %.2f / %.2f", entity.x, entity.y, entity.z)
            ui_label("f3_l2", "top_right", -308, 42, px_str, 11, 0.8, 0.8, 0.8)
            local facing_str = (math.abs(fwd_z) > math.abs(fwd_x)) and ((fwd_z < 0) and "North (-Z)" or "South (+Z)") or ((fwd_x > 0) and "East (+X)" or "West (-X)")
            ui_label("f3_l3", "top_right", -308, 62, "Facing: " .. facing_str, 11, 0.8, 0.8, 0.8)
            ui_label("f3_l4", "top_right", -308, 82, "Biome: Plains / Oak Forest", 11, 0.5, 0.95, 0.5)
            local target_name = target_entity and target_entity or "none"
            ui_label("f3_l5", "top_right", -308, 102, "Target: " .. target_name, 11, 0.8, 0.8, 0.8)
            ui_label("f3_l6", "top_right", -308, 122, "Ground: " .. (is_grounded and "YES" or "AIR"), 11, is_grounded and 0.4 or 1.0, is_grounded and 1.0 or 0.4, 0.4)
        end
    end
end
