-- ============================================================================
-- MOTOR-DIF: URBAN SANDBOX V2 SCRIPT
-- Scene: assets/urban_sandbox_v2.json
-- Script: assets/scripts/urban_sandbox_v2.lua
-- Compliance: Declarative lua_ui HUD (NEVER set_hud_text), Safe Lighting,
--             Full Vehicle Physics, Character E Enter/Exit, 5 Pedestrians AI,
--             30s Mission with Neon Marker.
-- ============================================================================

-- Game & Mission State
local mission_timer = 30.0
local mission_active = true
local mission_won = false
local mission_lost = false
local total_time = 0.0
local initialized = false

-- Vehicle State & Kinematics
-- Initial heading: 1.5707963 rad (90 deg yaw, facing +X along main street)
local INIT_HEADING = 1.5707963
local in_vehicle = false
local last_in_vehicle = false
local veh_pos = { x = 0.0, y = 0.5, z = -12.0 }
local veh_heading = INIT_HEADING
local veh_speed = 0.0     -- m/s
local veh_accel = 22.0    -- m/s^2
local veh_brake = 30.0    -- m/s^2
local veh_max_fwd = 26.0  -- m/s (~94 km/h)
local veh_max_rev = -8.0  -- m/s
local veh_drag = 3.5      -- m/s^2
local steer_speed = 1.8   -- rad/s

-- Player On-Foot State
local walk_speed = 6.5    -- m/s

-- Pedestrians State & Sidewalk Patrol
-- Central sidewalk perimeter loop waypoints (clockwise):
local sidewalk_wps = {
    { x = -7.8, z = -7.8 }, -- WP 1: NW
    { x =  7.8, z = -7.8 }, -- WP 2: NE
    { x =  7.8, z =  7.8 }, -- WP 3: SE
    { x = -7.8, z =  7.8 }  -- WP 4: SW
}

local pedestrians = {
    { id = "ped_1", x = -4.5, z = -7.8, wp_idx = 2, speed = 2.4, anim = 0.0 },
    { id = "ped_2", x =  4.5, z = -7.8, wp_idx = 2, speed = 2.1, anim = 1.2 },
    { id = "ped_3", x =  7.8, z =  1.5, wp_idx = 3, speed = 2.6, anim = 2.4 },
    { id = "ped_4", x =  3.5, z =  7.8, wp_idx = 4, speed = 2.3, anim = 3.6 },
    { id = "ped_5", x = -7.8, z =  3.0, wp_idx = 1, speed = 2.5, anim = 4.8 }
}

-- Target Marker Coordinates
local TARGET_X = 12.0
local TARGET_Z = 12.0

-- Relative offsets for vehicle sub-parts
local subparts = {
    { id = "vehicle_cabin",   rx =  0.00, ry =  0.45, rz = -0.20 },
    { id = "vehicle_light_l", rx = -0.70, ry =  0.02, rz = -2.15 },
    { id = "vehicle_light_r", rx =  0.70, ry =  0.02, rz = -2.15 },
    { id = "wheel_fl",        rx = -1.15, ry = -0.15, rz = -1.30 },
    { id = "wheel_fr",        rx =  1.15, ry = -0.15, rz = -1.30 },
    { id = "wheel_rl",        rx = -1.15, ry = -0.15, rz =  1.30 },
    { id = "wheel_rr",        rx =  1.15, ry = -0.15, rz =  1.30 }
}

-- Distance helper
local function dist(x1, z1, x2, z2)
    local dx = x1 - x2
    local dz = z1 - z2
    return math.sqrt(dx * dx + dz * dz)
end

-- Test event listeners for programmatic flow validation
function on_test_enter()
    in_vehicle = true
    spawn_particles(veh_pos.x, 1.0, veh_pos.z, 0.2, 0.8, 1.0, 16, 2.0, 0.6)
    play_tone(350, 700, "sine", 0.2, 0.6)
end

function on_test_drive()
    in_vehicle = true
    veh_pos.x = TARGET_X - 1.5
    veh_pos.z = TARGET_Z - 1.5
    veh_speed = 14.0
end

local function register_events()
    on_event("test_enter_vehicle", "on_test_enter")
    on_event("test_drive_to_target", "on_test_drive")
end

-- Global Reset / Restart function
function reset_sandbox()
    ui_clear()
    clear_timers()
    clear_events()
    register_events()

    mission_timer = 30.0
    mission_active = true
    mission_won = false
    mission_lost = false

    in_vehicle = false
    last_in_vehicle = false
    veh_pos.x = 0.0
    veh_pos.y = 0.5
    veh_pos.z = -12.0
    veh_heading = INIT_HEADING
    veh_speed = 0.0

    -- Reset vehicle entity and subparts
    local fwd_x =  math.sin(veh_heading)
    local fwd_z = -math.cos(veh_heading)
    local rgt_x =  math.cos(veh_heading)
    local rgt_z =  math.sin(veh_heading)

    tween_position("vehicle", veh_pos.x, veh_pos.y, veh_pos.z, 0.0)
    for _, sp in ipairs(subparts) do
        local fwd_dist = -sp.rz
        local rgt_dist = sp.rx
        local wx = veh_pos.x + rgt_dist * rgt_x + fwd_dist * fwd_x
        local wy = veh_pos.y + sp.ry
        local wz = veh_pos.z + rgt_dist * rgt_z + fwd_dist * fwd_z
        tween_position(sp.id, wx, wy, wz, 0.0)
    end

    spawn_particles(veh_pos.x, 1.5, veh_pos.z, 0.2, 0.8, 1.0, 20, 2.0, 0.8)
    camera_shake(0.2, 0.3)
end

-- Global UI Button Callback
function on_reset_click()
    reset_sandbox()
end

-- ============================================================================
-- MAIN ENGINE UPDATE LOOP
-- ============================================================================
function on_update(entity, dt)
    if dt <= 0.0 then return end
    if dt > 0.1 then dt = 0.1 end

    -- Init Idiom
    if not initialized then
        initialized = true
        reset_sandbox()
        entity.x = 0.0
        entity.y = 0.85
        entity.z = -8.8
    end

    total_time = total_time + dt

    -- Quick Restart with R key
    if is_key_just_pressed("R") then
        reset_sandbox()
        entity.x = 0.0
        entity.y = 0.85
        entity.z = -8.8
        return
    end

    -- ------------------------------------------------------------------------
    -- 1. VEHICLE CONTROLS & KINEMATICS
    -- ------------------------------------------------------------------------
    local fwd_x =  math.sin(veh_heading)
    local fwd_z = -math.cos(veh_heading)
    local rgt_x =  math.cos(veh_heading)
    local rgt_z =  math.sin(veh_heading)

    if in_vehicle then
        -- Accelerate forward (W / Up)
        local accel_input = is_key_down("W") or is_key_down("UP") or is_key_down("ArrowUp")
        local brake_input = is_key_down("S") or is_key_down("DOWN") or is_key_down("ArrowDown")
        local handbrake   = is_key_down("SPACE") or is_key_down("Space")

        if accel_input then
            veh_speed = math.min(veh_max_fwd, veh_speed + veh_accel * dt)
            spawn_particles(veh_pos.x - fwd_x * 1.8, 0.3, veh_pos.z - fwd_z * 1.8, 0.5, 0.5, 0.55, 1, 0.8, 0.35)
        end

        if brake_input then
            if veh_speed > 0.5 then
                veh_speed = math.max(0.0, veh_speed - veh_brake * dt)
                spawn_particles(veh_pos.x, 0.2, veh_pos.z, 1.0, 0.35, 0.15, 2, 0.8, 0.25)
            else
                veh_speed = math.max(veh_max_rev, veh_speed - veh_accel * 0.7 * dt)
            end
        end

        if handbrake then
            veh_speed = veh_speed * math.max(0.0, 1.0 - 7.5 * dt)
            spawn_particles(veh_pos.x, 0.2, veh_pos.z, 0.75, 0.75, 0.8, 3, 1.2, 0.3)
        end

        -- Passive aerodynamic and ground drag
        if not accel_input and not brake_input and not handbrake then
            if veh_speed > 0.0 then
                veh_speed = math.max(0.0, veh_speed - veh_drag * dt)
            elseif veh_speed < 0.0 then
                veh_speed = math.min(0.0, veh_speed + veh_drag * dt)
            end
        end

        -- Dynamic Turning (A / D / Left / Right)
        local turn_factor = math.min(1.0, math.abs(veh_speed) / 3.0)
        if veh_speed < -0.1 then turn_factor = -turn_factor end -- reverse steering invert

        if is_key_down("A") or is_key_down("LEFT") or is_key_down("ArrowLeft") then
            veh_heading = veh_heading - steer_speed * turn_factor * dt
        end
        if is_key_down("D") or is_key_down("RIGHT") or is_key_down("ArrowRight") then
            veh_heading = veh_heading + steer_speed * turn_factor * dt
        end

        -- Integrate vehicle position
        veh_pos.x = veh_pos.x + fwd_x * veh_speed * dt
        veh_pos.z = veh_pos.z + fwd_z * veh_speed * dt

        -- Boundary limits within city grid
        veh_pos.x = math.max(-32.0, math.min(32.0, veh_pos.x))
        veh_pos.z = math.max(-32.0, math.min(32.0, veh_pos.z))

        -- Keep player entity aligned inside vehicle (hidden below surface)
        entity.x = veh_pos.x
        entity.y = -10.0
        entity.z = veh_pos.z

        -- Smooth camera follow vehicle with elevated city overview angle
        follow_entity("vehicle", -20.0, 24.0, -20.0, 0.15)
        set_world_label("vehicle", "", 0.0, 0.0, 0.0, 0.0)
    else
        -- Unattended vehicle slows down to zero
        if math.abs(veh_speed) > 0.1 then
            veh_speed = veh_speed * math.max(0.0, 1.0 - 4.0 * dt)
            veh_pos.x = veh_pos.x + fwd_x * veh_speed * dt
            veh_pos.z = veh_pos.z + fwd_z * veh_speed * dt
        else
            veh_speed = 0.0
        end

        -- --------------------------------------------------------------------
        -- 2. PLAYER ON-FOOT CONTROLS
        -- --------------------------------------------------------------------
        local mx = 0.0
        local mz = 0.0

        if is_key_down("W") or is_key_down("UP") or is_key_down("ArrowUp") then mz = mz - 1.0 end
        if is_key_down("S") or is_key_down("DOWN") or is_key_down("ArrowDown") then mz = mz + 1.0 end
        if is_key_down("A") or is_key_down("LEFT") or is_key_down("ArrowLeft") then mx = mx - 1.0 end
        if is_key_down("D") or is_key_down("RIGHT") or is_key_down("ArrowRight") then mx = mx + 1.0 end

        local mlen = math.sqrt(mx * mx + mz * mz)
        if mlen > 0.01 then
            entity.x = entity.x + (mx / mlen) * walk_speed * dt
            entity.z = entity.z + (mz / mlen) * walk_speed * dt
            entity.y = 0.85 + math.abs(math.sin(total_time * 8.0)) * 0.08
        else
            entity.y = 0.85
        end

        entity.x = math.max(-32.0, math.min(32.0, entity.x))
        entity.z = math.max(-32.0, math.min(32.0, entity.z))

        -- Smooth camera follow player with elevated city overview angle
        follow_entity("player", -20.0, 24.0, -20.0, 0.15)
        set_world_label("vehicle", "VEHICULO [E]", 1.6, 0.3, 0.8, 1.0)
    end

    -- ------------------------------------------------------------------------
    -- 3. ENTER / EXIT VEHICLE (KEY E)
    -- ------------------------------------------------------------------------
    local d_to_veh = dist(entity.x, entity.z, veh_pos.x, veh_pos.z)
    local can_enter = (not in_vehicle) and (d_to_veh <= 3.6)

    if is_key_just_pressed("E") then
        if in_vehicle then
            -- Exit vehicle: step out to driver side
            in_vehicle = false
            entity.x = veh_pos.x - 2.4 * rgt_x
            entity.y = 0.85
            entity.z = veh_pos.z - 2.4 * rgt_z
            spawn_particles(entity.x, 0.8, entity.z, 0.3, 1.0, 0.5, 12, 1.5, 0.5)
            play_tone(700, 350, "sine", 0.2, 0.6)
            camera_shake(0.12, 0.15)
        elseif can_enter then
            -- Enter vehicle
            in_vehicle = true
            spawn_particles(veh_pos.x, 1.0, veh_pos.z, 0.2, 0.8, 1.0, 16, 2.0, 0.6)
            play_tone(350, 700, "sine", 0.2, 0.6)
            camera_shake(0.15, 0.2)
        end
    end

    -- Clear UI widgets on mode change to avoid label overlap
    if last_in_vehicle ~= in_vehicle then
        last_in_vehicle = in_vehicle
        ui_clear()
    end

    -- ------------------------------------------------------------------------
    -- 4. UPDATE VEHICLE TRANSFORM & SUBPARTS
    -- ------------------------------------------------------------------------
    tween_position("vehicle", veh_pos.x, veh_pos.y, veh_pos.z, 0.0)

    for _, sp in ipairs(subparts) do
        local fwd_dist = -sp.rz
        local rgt_dist = sp.rx
        local wx = veh_pos.x + rgt_dist * rgt_x + fwd_dist * fwd_x
        local wy = veh_pos.y + sp.ry
        local wz = veh_pos.z + rgt_dist * rgt_z + fwd_dist * fwd_z
        tween_position(sp.id, wx, wy, wz, 0.0)
    end

    -- ------------------------------------------------------------------------
    -- 5. 5 PEDESTRIANS PATROL & OBSTACLE AVOIDANCE
    -- ------------------------------------------------------------------------
    for _, ped in ipairs(pedestrians) do
        local target_wp = sidewalk_wps[ped.wp_idx]
        local to_wp_x = target_wp.x - ped.x
        local to_wp_z = target_wp.z - ped.z
        local d_wp = math.sqrt(to_wp_x * to_wp_x + to_wp_z * to_wp_z)

        if d_wp < 0.8 then
            ped.wp_idx = (ped.wp_idx % 4) + 1
            target_wp = sidewalk_wps[ped.wp_idx]
            to_wp_x = target_wp.x - ped.x
            to_wp_z = target_wp.z - ped.z
            d_wp = math.sqrt(to_wp_x * to_wp_x + to_wp_z * to_wp_z)
        end

        local dir_x = to_wp_x / math.max(0.01, d_wp)
        local dir_z = to_wp_z / math.max(0.01, d_wp)

        -- Avoidance calculations
        local avoid_x = 0.0
        local avoid_z = 0.0

        -- Avoid Vehicle
        local dv = dist(ped.x, ped.z, veh_pos.x, veh_pos.z)
        if dv < 4.0 then
            local away_vx = ped.x - veh_pos.x
            local away_vz = ped.z - veh_pos.z
            local safe_d = math.max(0.1, math.sqrt(away_vx * away_vx + away_vz * away_vz))
            local force = (4.0 - dv) / 4.0 * 5.0
            avoid_x = avoid_x + (away_vx / safe_d) * force
            avoid_z = avoid_z + (away_vz / safe_d) * force
            spawn_particles(ped.x, 1.2, ped.z, 1.0, 0.85, 0.2, 1, 0.4, 0.2)
        end

        -- Avoid Player on foot
        if not in_vehicle then
            local dp = dist(ped.x, ped.z, entity.x, entity.z)
            if dp < 2.5 then
                local away_px = ped.x - entity.x
                local away_pz = ped.z - entity.z
                local safe_dp = math.max(0.1, math.sqrt(away_px * away_px + away_pz * away_pz))
                local force_p = (2.5 - dp) / 2.5 * 3.5
                avoid_x = avoid_x + (away_px / safe_dp) * force_p
                avoid_z = avoid_z + (away_pz / safe_dp) * force_p
            end
        end

        -- Combine desired heading with avoidance
        local move_fx = dir_x * ped.speed + avoid_x
        local move_fz = dir_z * ped.speed + avoid_z
        local m_mag = math.sqrt(move_fx * move_fx + move_fz * move_fz)

        if m_mag > 0.01 then
            ped.x = ped.x + (move_fx / m_mag) * ped.speed * dt
            ped.z = ped.z + (move_fz / m_mag) * ped.speed * dt
        end

        -- Keep on sidewalk perimeter
        ped.x = math.max(-8.5, math.min(8.5, ped.x))
        ped.z = math.max(-8.5, math.min(8.5, ped.z))

        ped.anim = ped.anim + dt
        local ped_bob = 0.8 + math.abs(math.sin(ped.anim * 6.5)) * 0.08
        tween_position(ped.id, ped.x, ped_bob, ped.z, 0.0)
    end

    -- ------------------------------------------------------------------------
    -- 6. MISSION LOGIC & GLOWING MARKER ANIMATION
    -- ------------------------------------------------------------------------
    local beacon_y = 2.0 + math.sin(total_time * 4.0) * 0.35
    tween_position("target_beacon", TARGET_X, beacon_y, TARGET_Z, 0.0)

    local marker_pulse = 2.6 + math.sin(total_time * 5.0) * 0.4
    tween_scale("target_marker", marker_pulse, 0.3, marker_pulse, 0.0)

    draw_ring_3d(TARGET_X, 0.25, TARGET_Z, 2.5, 0.2, 1.0, 0.5, 0.8)
    set_world_label("target_beacon", "OBJETIVO", 2.0, 0.2, 1.0, 0.5)

    -- Active player/vehicle position for mission proximity
    local cur_x = in_vehicle and veh_pos.x or entity.x
    local cur_z = in_vehicle and veh_pos.z or entity.z
    local dist_target = dist(cur_x, cur_z, TARGET_X, TARGET_Z)

    if mission_active then
        mission_timer = mission_timer - dt

        if dist_target <= 3.8 then
            mission_active = false
            mission_won = true
            spawn_particles(TARGET_X, 1.8, TARGET_Z, 0.2, 1.0, 0.5, 60, 4.0, 1.5)
            play_tone(440, 880, "triangle", 0.5, 0.8)
            camera_shake(0.35, 0.5)
        elseif mission_timer <= 0.0 then
            mission_timer = 0.0
            mission_active = false
            mission_lost = true
            spawn_particles(cur_x, 1.5, cur_z, 1.0, 0.2, 0.2, 35, 2.5, 1.0)
            play_tone(280, 140, "sawtooth", 0.6, 0.8)
            camera_shake(0.4, 0.4)
        end
    end

    -- ------------------------------------------------------------------------
    -- 7. DECLARATIVE IN-GAME HUD (lua_ui) - STRICTLY NO set_hud_text
    -- ------------------------------------------------------------------------
    -- Mission Card (top_left)
    ui_panel("pnl_mission", "top_left", 16, 16, 320, 108, 0.08, 0.10, 0.16, 0.88)
    ui_label("lbl_m_title", "top_left", 28, 26, "MISIÓN: RUTA URBANA 30s", 14, 0.3, 0.85, 1.0)

    local timer_pct = math.max(0.0, math.min(1.0, mission_timer / 30.0))
    local timer_str = string.format("%.1fs / 30.0s", mission_timer)
    local bar_r = (timer_pct > 0.5) and 0.2 or (timer_pct > 0.25 and 0.95 or 0.95)
    local bar_g = (timer_pct > 0.5) and 0.85 or (timer_pct > 0.25 and 0.75 or 0.25)
    local bar_b = (timer_pct > 0.5) and 0.4 or (timer_pct > 0.25 and 0.2 or 0.2)
    ui_bar("bar_timer", "top_left", 28, 50, 296, 20, timer_pct, timer_str, bar_r, bar_g, bar_b)

    local m_status_text = mission_won and "¡OBJETIVO ALCANZADO!" or (mission_lost and "¡TIEMPO AGOTADO!" or string.format("Distancia: %.1f m", dist_target))
    ui_label("lbl_m_dist", "top_left", 28, 78, m_status_text, 12, 0.92, 0.94, 0.98)

    -- Telemetry Card (top_left, below mission card)
    ui_panel("pnl_telemetry", "top_left", 16, 132, 320, 125, 0.08, 0.10, 0.16, 0.88)

    if in_vehicle then
        local kmh = math.abs(veh_speed * 3.6)
        local speed_pct = math.min(1.0, math.abs(veh_speed) / veh_max_fwd)
        ui_label("lbl_t_mode", "top_left", 28, 142, "MODO: VEHÍCULO DEPORTIVO", 13, 1.0, 0.75, 0.2)
        ui_bar("bar_speed", "top_left", 28, 166, 296, 20, speed_pct, string.format("Velocidad: %.0f km/h", kmh), 0.2, 0.7, 1.0)
        ui_label("lbl_t_c1", "top_left", 28, 196, "W/S: Acelerar/Frenar | A/D: Girar", 11, 0.82, 0.85, 0.9)
        ui_label("lbl_t_c2", "top_left", 28, 218, "[E]: Salir  |  [ESPACIO]: Freno de Mano", 11, 0.75, 0.9, 1.0)
    else
        ui_label("lbl_t_mode", "top_left", 28, 142, "MODO: A PIE (PEATÓN)", 13, 0.25, 0.9, 0.45)
        ui_label("lbl_t_c1", "top_left", 28, 168, "WASD / Flechas: Desplazarse", 12, 0.85, 0.88, 0.92)
        if can_enter then
            ui_label("lbl_t_c2", "top_left", 28, 196, ">> [E] ENTRAR AL VEHÍCULO <<", 13, 0.3, 1.0, 0.6)
            ui_label("lbl_t_c3", "top_left", 28, 218, "Pulsa E para subir y conducir", 11, 0.75, 0.9, 1.0)
        else
            ui_label("lbl_t_c2", "top_left", 28, 196, string.format("Vehículo a %.1f m (acércate)", d_to_veh), 12, 0.7, 0.75, 0.8)
            ui_label("lbl_t_c3", "top_left", 28, 218, "Acércate a menos de 3.5m para subir", 11, 0.6, 0.65, 0.7)
        end
    end

    -- Victory / Defeat Modal Panels
    if mission_won then
        ui_panel("pnl_modal", "center", 0, -25, 420, 130, 0.05, 0.22, 0.12, 0.94)
        ui_label("lbl_modal_1", "center", -160, -60, "¡¡VICTORIA!! OBJETIVO ALCANZADO", 16, 0.25, 1.0, 0.45)
        ui_label("lbl_modal_2", "center", -140, -30, string.format("Completado con %.1fs restantes", mission_timer), 13, 0.9, 0.95, 1.0)
        ui_button("btn_reset", "center", 0, 15, 160, 32, "Reiniciar [R]", "on_reset_click")
    elseif mission_lost then
        ui_panel("pnl_modal", "center", 0, -25, 420, 130, 0.26, 0.06, 0.06, 0.94)
        ui_label("lbl_modal_1", "center", -160, -60, "¡¡TIEMPO AGOTADO!! MISIÓN FALLIDA", 16, 1.0, 0.3, 0.3)
        ui_label("lbl_modal_2", "center", -140, -30, "No lograste llegar a la esquina a tiempo", 13, 0.95, 0.9, 0.9)
        ui_button("btn_reset", "center", 0, 15, 160, 32, "Reintentar [R]", "on_reset_click")
    end
end
