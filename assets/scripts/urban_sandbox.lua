-- ============================================================================
-- MOTOR-DIF: URBAN SANDBOX PROTOTYPE SCRIPT
-- Scene: assets/urban_sandbox.json
-- Script: assets/scripts/urban_sandbox.lua
-- ============================================================================

-- Game & Mission State
local mission_timer = 30.0
local mission_active = true
local mission_won = false
local mission_lost = false
local total_time = 0.0

-- Vehicle State & Kinematics
local in_vehicle = false
local veh_pos = { x = 0.0, y = 0.5, z = -12.0 }
local veh_heading = 0.0   -- 0 rad = facing -Z (along main street)
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
-- Central sidewalk perimeter loop waypoints:
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

-- Reset / Restart function
local function reset_sandbox(entity)
    mission_timer = 30.0
    mission_active = true
    mission_won = false
    mission_lost = false

    in_vehicle = false
    veh_pos.x = 0.0
    veh_pos.y = 0.5
    veh_pos.z = -12.0
    veh_heading = 0.0
    veh_speed = 0.0

    entity.x = 0.0
    entity.y = 0.85
    entity.z = -8.5

    tween_position("vehicle", veh_pos.x, veh_pos.y, veh_pos.z, 0.0)
    for _, sp in ipairs(subparts) do
        tween_position(sp.id, veh_pos.x + sp.rx, veh_pos.y + sp.ry, veh_pos.z + sp.rz, 0.0)
    end

    spawn_particles(entity.x, 1.5, entity.z, 0.2, 0.8, 1.0, 20, 2.0, 0.8)
    camera_shake(0.2, 0.3)
end

-- ============================================================================
-- MAIN ENGINE UPDATE LOOP
-- ============================================================================
function on_update(entity, dt)
    if dt <= 0.0 then return end
    if dt > 0.1 then dt = 0.1 end

    total_time = total_time + dt

    -- Quick Restart with R key
    if is_key_just_pressed("R") then
        reset_sandbox(entity)
        return
    end

    -- ------------------------------------------------------------------------
    -- 1. VEHICLE CONTROLS & PHYSICS
    -- ------------------------------------------------------------------------
    local fwd_x =  math.sin(veh_heading)
    local fwd_z = -math.cos(veh_heading)
    local rgt_x =  math.cos(veh_heading)
    local rgt_z =  math.sin(veh_heading)

    if in_vehicle then
        -- Accelerate forward (W / Up)
        local accel_input = is_key_down("W") or is_key_down("UP") or is_key_down("ArrowUp")
        local brake_input = is_key_down("S") or is_key_down("DOWN") or is_key_down("ArrowDown")
        local handbrake = is_key_down("SPACE") or is_key_down("Space")

        if accel_input then
            veh_speed = math.min(veh_max_fwd, veh_speed + veh_accel * dt)
            spawn_particles(veh_pos.x, 0.2, veh_pos.z, 0.4, 0.4, 0.45, 1, 0.6, 0.3)
        end

        if brake_input then
            if veh_speed > 0.5 then
                veh_speed = math.max(0.0, veh_speed - veh_brake * dt)
                spawn_particles(veh_pos.x, 0.2, veh_pos.z, 1.0, 0.3, 0.15, 2, 0.8, 0.25)
            else
                veh_speed = math.max(veh_max_rev, veh_speed - veh_accel * 0.7 * dt)
            end
        end

        if handbrake then
            veh_speed = veh_speed * math.max(0.0, 1.0 - 7.5 * dt)
            spawn_particles(veh_pos.x, 0.2, veh_pos.z, 0.7, 0.7, 0.7, 3, 1.2, 0.3)
        end

        -- Passive drag when no pedal applied
        if not accel_input and not brake_input and not handbrake then
            if veh_speed > 0.0 then
                veh_speed = math.max(0.0, veh_speed - veh_drag * dt)
            elseif veh_speed < 0.0 then
                veh_speed = math.min(0.0, veh_speed + veh_drag * dt)
            end
        end

        -- Turning (A / D / Left / Right)
        local turn_factor = math.min(1.0, math.abs(veh_speed) / 3.5)
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

        -- Boundary limits
        veh_pos.x = math.max(-32.0, math.min(32.0, veh_pos.x))
        veh_pos.z = math.max(-32.0, math.min(32.0, veh_pos.z))

        -- Keep player entity aligned inside vehicle
        entity.x = veh_pos.x
        entity.y = -5.0
        entity.z = veh_pos.z

        -- Smooth camera follow vehicle
        follow_entity("vehicle", 0.0, 8.5, 14.0, 0.12)
    else
        -- Vehicle slows down smoothly if unattended
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

        -- Smooth camera follow player
        follow_entity("player", 0.0, 8.0, 12.0, 0.12)
    end

    -- ------------------------------------------------------------------------
    -- 3. ENTER / EXIT VEHICLE (KEY E)
    -- ------------------------------------------------------------------------
    local d_to_veh = dist(entity.x, entity.z, veh_pos.x, veh_pos.z)
    local can_enter = (not in_vehicle) and (d_to_veh <= 3.6)

    if is_key_just_pressed("E") then
        if in_vehicle then
            -- Exit vehicle: step out to the left side
            in_vehicle = false
            entity.x = veh_pos.x - 2.4 * rgt_x
            entity.y = 0.85
            entity.z = veh_pos.z - 2.4 * rgt_z
            spawn_particles(entity.x, 0.8, entity.z, 0.3, 1.0, 0.5, 12, 1.5, 0.5)
            camera_shake(0.12, 0.15)
        elseif can_enter then
            -- Enter vehicle
            in_vehicle = true
            spawn_particles(veh_pos.x, 1.0, veh_pos.z, 0.2, 0.8, 1.0, 16, 2.0, 0.6)
            camera_shake(0.15, 0.2)
        end
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
        if dv < 3.8 then
            local away_vx = ped.x - veh_pos.x
            local away_vz = ped.z - veh_pos.z
            local safe_d = math.max(0.1, math.sqrt(away_vx * away_vx + away_vz * away_vz))
            local force = (3.8 - dv) / 3.8 * 4.5
            avoid_x = avoid_x + (away_vx / safe_d) * force
            avoid_z = avoid_z + (away_vz / safe_d) * force
            spawn_particles(ped.x, 1.2, ped.z, 1.0, 0.85, 0.2, 1, 0.4, 0.2)
        end

        -- Avoid Player on foot
        if not in_vehicle then
            local dp = dist(ped.x, ped.z, entity.x, entity.z)
            if dp < 2.2 then
                local away_px = ped.x - entity.x
                local away_pz = ped.z - entity.z
                local safe_dp = math.max(0.1, math.sqrt(away_px * away_px + away_pz * away_pz))
                local force_p = (2.2 - dp) / 2.2 * 3.0
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
        local ped_bob = 0.8 + math.abs(math.sin(ped.anim * 6.8)) * 0.08
        tween_position(ped.id, ped.x, ped_bob, ped.z, 0.0)
    end

    -- ------------------------------------------------------------------------
    -- 6. MISSION LOGIC & GLOWING MARKER ANIMATION
    -- ------------------------------------------------------------------------
    -- Animate glowing target marker
    local beacon_y = 2.0 + math.sin(total_time * 4.0) * 0.35
    tween_position("target_beacon", TARGET_X, beacon_y, TARGET_Z, 0.0)

    local marker_pulse = 2.6 + math.sin(total_time * 5.0) * 0.4
    tween_scale("target_marker", marker_pulse, 0.4, marker_pulse, 0.0)

    -- Active player/vehicle position for mission proximity
    local cur_x = in_vehicle and veh_pos.x or entity.x
    local cur_z = in_vehicle and veh_pos.z or entity.z
    local dist_target = dist(cur_x, cur_z, TARGET_X, TARGET_Z)

    if mission_active then
        mission_timer = mission_timer - dt

        if dist_target <= 3.8 then
            mission_active = false
            mission_won = true
            spawn_particles(TARGET_X, 1.5, TARGET_Z, 0.2, 1.0, 0.5, 60, 4.0, 1.5)
            camera_shake(0.35, 0.5)
        elseif mission_timer <= 0.0 then
            mission_timer = 0.0
            mission_active = false
            mission_lost = true
            spawn_particles(cur_x, 1.5, cur_z, 1.0, 0.2, 0.2, 35, 2.5, 1.0)
            camera_shake(0.4, 0.4)
        end
    end

    -- ------------------------------------------------------------------------
    -- 7. IN-GAME HUD TELEMETRY & CONTROLS
    -- ------------------------------------------------------------------------
    set_hud_text("0. CABECERA", "=== MOTOR-DIF: URBAN SANDBOX PROTOTYPE ===")

    local mission_status_str = "EN PROGRESO"
    if mission_won then
        mission_status_str = "¡¡VICTORIA!! OBJETIVO ALCANZADO A TIEMPO"
    elseif mission_lost then
        mission_status_str = "¡¡TIEMPO AGOTADO!! PULSA [R] PARA REINTENTAR"
    end
    set_hud_text("1. MISION", string.format("ESTADO: %s | Distancia: %.1f m", mission_status_str, dist_target))
    set_hud_text("2. CRONOMETRO", string.format("Tiempo Restante: %.1f s / 30.0 s", mission_timer))

    local mode_str = in_vehicle and string.format("VEHICULO (Velocidad: %.0f km/h)", math.abs(veh_speed * 3.6)) or "A PIE"
    set_hud_text("3. MODO", mode_str)

    local prompt_str = "Ninguna accion cercana"
    if in_vehicle then
        prompt_str = "Pulsa [E] para SALIR del vehiculo"
    elseif can_enter then
        prompt_str = ">> [E] ENTRAR AL VEHICULO <<"
    else
        prompt_str = string.format("Vehiculo a %.1f m (acercate para entrar)", d_to_veh)
    end
    set_hud_text("4. ACCION", prompt_str)

    set_hud_text("5. CONTROLES", "WASD / Flechas: Mover/Conducir | E: Entrar/Salir | ESPACIO: Freno | R: Reiniciar")
    set_hud_text("6. PEATONES", "5 Peatones activos en aceras esquivando auto y jugador")
end
