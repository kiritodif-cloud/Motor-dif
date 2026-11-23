-- showcase_v12.lua
-- Motor-dif Fase 12: Arquitectura de Gameplay Pro Showcase Script

local time_acc = 0.0
local path_points = {}
local waypoint_idx = 1
local move_speed = 6.0
local is_moving = false
local current_goal = { x = 9.0, z = 9.0 }
local goal_count = 0
local sfx_muted = false
local pulse_ticks = 0

-- 1. Initialize Navigation Grid (XZ Plane)
nav_bake(0.8, 16.0)

-- 2. Register Signal Listeners (Event Bus)
on_event("goal_reached", "on_goal_reached")
on_event("path_calculated", "on_path_calculated")

-- 3. Set up Periodic Managed Timer
every(1.0, "on_pulse_timer")

-- 4. Initial path from start (-9, -9) to initial beacon (9, 9)
local initial_path = find_path(-9.0, -9.0, 9.0, 9.0)
if initial_path and #initial_path > 0 then
    path_points = initial_path
    waypoint_idx = 1
    is_moving = true
end

-- 5. Build Declarative In-Game UI (game_ui)
ui_clear()

-- Header Panel
ui_panel("header_card", "top_center", 0, 15, 460, 56, 0.08, 0.1, 0.16, 0.88)
ui_label("header_title", "top_center", 0, 22, "MOTOR-DIF FASE 12 * GAMEPLAY ARCHITECTURE", 14.5, 0.22, 0.85, 1.0)
ui_label("header_sub", "top_center", 0, 44, "A* Pathfinding * Signals * Timers * Input Map * 3D Audio", 11.0, 0.58, 0.65, 0.78)

-- Left Telemetry HUD Card
ui_panel("telemetry_card", "top_left", 20, 20, 260, 165, 0.08, 0.1, 0.16, 0.90)
ui_label("telem_title", "top_left", 32, 30, "TACTICAL NAVIGATION HUD", 12.0, 1.0, 1.0, 1.0)
ui_label("agent_status_lbl", "top_left", 32, 54, "Agent: In Transit", 11.0, 0.35, 0.9, 0.45)
ui_label("path_info_lbl", "top_left", 32, 74, "Waypoints: 0 active", 11.0, 0.55, 0.8, 1.0)
ui_label("goals_lbl", "top_left", 32, 94, "Goals Reached: 0", 11.0, 1.0, 0.85, 0.3)
ui_label("bus_lbl", "top_left", 32, 114, "Audio Buses: Master 1.0 | SFX 1.0", 10.5, 0.65, 0.72, 0.82)
ui_label("pulse_lbl", "top_left", 32, 134, "Heartbeat Timer: 0s", 10.5, 0.5, 0.75, 0.9)

-- Bottom Interaction Panel
ui_panel("controls_panel", "bottom_center", 0, 20, 520, 80, 0.08, 0.1, 0.16, 0.90)
ui_label("controls_hint", "bottom_center", 0, 32, "Click ANYWHERE on the floor to navigate agent with A* Pathfinding", 11.5, 0.9, 0.92, 0.98)
ui_button("btn_rebake", "bottom_center", -120, 56, 200, 32, "[R] RE-BAKE NAVGRID", "action_rebake")
ui_button("btn_mute_sfx", "bottom_center", 120, 56, 200, 32, "[!] TOGGLE SFX BUS", "action_toggle_sfx")

-- Callbacks for UI Buttons
function action_rebake()
    nav_bake(0.8, 16.0)
    spawn_floating_text(0.0, 3.5, 0.0, "NAVGRID RE-BAKED!", 0.2, 0.85, 1.0, 1.2)
    play_sound_3d("assets/sfx/bounce.wav", 0.0, 1.0, 0.0, 30.0, 1.0, "sfx")
end

function action_toggle_sfx()
    sfx_muted = not sfx_muted
    if sfx_muted then
        set_bus_volume("sfx", 0.0)
        ui_label("bus_lbl", "top_left", 32, 114, "Audio Buses: Master 1.0 | SFX 0.0 (MUTED)", 10.5, 0.95, 0.45, 0.45)
        spawn_floating_text(0.0, 2.5, 0.0, "SFX BUS MUTED", 0.95, 0.45, 0.45, 1.0)
    else
        set_bus_volume("sfx", 1.0)
        ui_label("bus_lbl", "top_left", 32, 114, "Audio Buses: Master 1.0 | SFX 1.0 (ACTIVE)", 10.5, 0.35, 0.9, 0.45)
        spawn_floating_text(0.0, 2.5, 0.0, "SFX BUS UNMUTED", 0.35, 0.9, 0.45, 1.0)
        play_sound_3d("assets/sfx/bounce.wav", 0.0, 1.0, 0.0, 30.0, 0.8, "sfx")
    end
end

-- Timer callback (runs every 1 second)
function on_pulse_timer()
    pulse_ticks = pulse_ticks + 1
    ui_label("pulse_lbl", "top_left", 32, 134, string.format("Heartbeat Timer: %ds", pulse_ticks), 10.5, 0.5, 0.75, 0.9)
end

-- Event Signal Callbacks
function on_path_calculated(payload)
    if payload and payload.points then
        ui_label("path_info_lbl", "top_left", 32, 74, string.format("Waypoints: %d active", payload.points), 11.0, 0.35, 0.85, 1.0)
        ui_label("agent_status_lbl", "top_left", 32, 54, "Agent: Navigating Path", 11.0, 0.35, 0.9, 0.45)
    end
end

function on_goal_reached(payload)
    goal_count = goal_count + 1
    ui_label("goals_lbl", "top_left", 32, 94, string.format("Goals Reached: %d", goal_count), 11.0, 1.0, 0.85, 0.3)
    ui_label("agent_status_lbl", "top_left", 32, 54, "Agent: Goal Reached (Idle)", 11.0, 1.0, 0.85, 0.3)
    ui_label("path_info_lbl", "top_left", 32, 74, "Waypoints: 0 active", 11.0, 0.55, 0.65, 0.75)

    local gx = (payload and payload.x) or 0.0
    local gz = (payload and payload.z) or 0.0

    -- 3D Spatial Audio & Floating Text
    play_sound_3d("assets/sfx/bounce.wav", gx, 1.0, gz, 35.0, 1.0, "sfx")
    spawn_floating_text(gx, 2.2, gz, "GOAL REACHED!", 0.2, 1.0, 0.45, 1.4)
    spawn_particles(gx, 1.2, gz, 1.0, 0.85, 0.2, 40, 4.0, 1.2)

    -- Managed one-shot timer for celebratory sound & fanfare
    after(0.25, "on_celebration_fanfare")
end

function on_celebration_fanfare()
    play_tone(523.25, 783.99, "sine", 0.12, 0.35) -- C5 to G5 fanfare
    spawn_particles(current_goal.x, 1.5, current_goal.z, 0.3, 0.9, 1.0, 25, 2.5, 0.9)
end

function on_update(entity, dt)
    time_acc = time_acc + dt

    -- 1. Check Input for Mouse Raycasting onto Floor Plane (plane_y = 0.0)
    if is_action_just_pressed("fire") then
        local mouse = get_mouse_world_pos(0.0)
        if mouse and mouse.hit then
            -- Verify target point is within arena boundaries (-13 to +13)
            local target_x = math.max(-12.5, math.min(12.5, mouse.x))
            local target_z = math.max(-12.5, math.min(12.5, mouse.z))

            local new_path = find_path(entity.x, entity.z, target_x, target_z)
            if new_path and #new_path > 0 then
                path_points = new_path
                waypoint_idx = 1
                is_moving = true
                current_goal = { x = target_x, z = target_z }

                -- Emit Event Signal that path was calculated
                emit_event("path_calculated", { points = #path_points })

                -- 3D spatial click audio
                play_sound_3d("assets/sfx/bounce.wav", target_x, 0.5, target_z, 30.0, 0.7, "sfx")
                spawn_floating_text(target_x, 1.2, target_z, "TARGET", 1.0, 0.85, 0.2, 0.8)
            end
        end
    end

    -- 2. Follow Waypoints along A* Path
    if is_moving and #path_points > 0 and waypoint_idx <= #path_points then
        local target = path_points[waypoint_idx]
        local dx = target.x - entity.x
        local dz = target.z - entity.z
        local dist = math.sqrt(dx * dx + dz * dz)

        if dist < 0.25 then
            waypoint_idx = waypoint_idx + 1
            if waypoint_idx > #path_points then
                is_moving = false
                path_points = {}
                emit_event("goal_reached", { x = entity.x, z = entity.z })
            end
        else
            local step = move_speed * dt
            entity.x = entity.x + (dx / dist) * step
            entity.z = entity.z + (dz / dist) * step
            entity.ry = math.atan2(dx, dz)
        end
    end

    -- 3. Draw Active 3D Path and Destination Rings
    if #path_points > 1 then
        for i = 1, #path_points - 1 do
            local p1 = path_points[i]
            local p2 = path_points[i + 1]
            draw_line_3d(p1.x, 0.15, p1.z, p2.x, 0.15, p2.z, 0.2, 0.85, 1.0, 0.95)
            draw_ring_3d(p1.x, 0.12, p1.z, 0.2, 0.2, 0.85, 1.0, 0.6)
        end
    end

    -- Destination Marker Ring
    local ring_radius = 0.7 + 0.15 * math.sin(time_acc * 4.0)
    draw_ring_3d(current_goal.x, 0.12, current_goal.z, ring_radius, 1.0, 0.82, 0.2, 0.9)

    -- Ambient subtle trail behind hero
    if is_moving and math.random() > 0.65 then
        spawn_particles(entity.x, 0.3, entity.z, 0.2, 0.8, 1.0, 2, 0.5, 0.4)
    end
end
