-- showcase_v19_rig.lua
-- Motor-dif Fase 19: Auto-Rig & Animacion Procedural Showcase Script

local orbit_angle = 0.0
local has_initialized = false
local total_time = 0.0
local recoil_timer = 0.0
local pulse_count = 0

function on_init(entity)
    set_world_label("rig_beacon", "CENTRAL TRACKING BEACON", 2.0, 0.2, 0.85, 1.0)
    set_world_label("mech_root", "APEX BIPED TITAN [PROCEDURAL RIG]", 1.8, 0.3, 1.0, 0.4)
    set_world_label("spider_root", "ARACHNID DRONE [QUADRUPED TROT]", 1.2, 1.0, 0.6, 0.2)
    set_world_label("turret_root", "SENTINEL CANNON [AZIMUTH/PITCH TRACKING]", 1.5, 0.9, 0.2, 0.2)

    -- Dynamic trail behind the biped mech feet/torso
    set_trail("mech_root", 0.4, 0.85, 1.0, 0.8, 1.8, 0.4)

    -- Declarative In-Game HUD
    ui_clear()
    ui_panel("header_card", "top_center", 0, 15, 520, 58, 0.08, 0.1, 0.16, 0.90)
    ui_label("header_title", "top_center", 0, 22, "MOTOR-DIF FASE 19: AUTO-RIG & PROCEDURAL ANIMATION", 14.0, 0.22, 0.85, 1.0)
    ui_label("header_sub", "top_center", 0, 44, "Analytical 2-Bone IK * Biped Locomotion * Head Tracking * Recoil Decay", 10.5, 0.58, 0.65, 0.78)

    ui_panel("telem_card", "top_left", 20, 20, 290, 160, 0.08, 0.1, 0.16, 0.90)
    ui_label("telem_title", "top_left", 32, 30, "BIOMECHANICAL RIG TELEMETRY", 12.0, 1.0, 1.0, 1.0)
    ui_label("mech_status", "top_left", 32, 54, "Titan Mech: Locomotion Loop Active", 11.0, 0.35, 0.9, 0.45)
    ui_label("ik_status", "top_left", 32, 74, "2-Bone IK Solver: Analytical Law of Cosines", 11.0, 0.55, 0.8, 1.0)
    ui_label("look_status", "top_left", 32, 94, "Head Tracking Target: Beacon (0, 3, 0)", 11.0, 1.0, 0.85, 0.3)
    ui_label("recoil_status", "top_left", 32, 114, "Recoil Pulses: 0", 11.0, 0.9, 0.4, 0.3)
    ui_label("fps_target", "top_left", 32, 134, "FPS Target: 60+ Pure Rust ECS Transforms", 10.5, 0.65, 0.72, 0.82)

    -- Floating text notification
    spawn_floating_text(0.0, 6.0, 0.0, "PROCEDURAL RIG ENGINE INITIALIZED", 0.3, 0.95, 1.0, 4.0)

    -- Play dynamic synthwave music from Music Forge
    play_music_preset("synthwave")
    set_music_intensity(0.65)
end

function on_update(entity, dt)
    if not has_initialized then
        has_initialized = true
        on_init(entity)
    end

    total_time = total_time + dt
    recoil_timer = recoil_timer + dt

    -- Circular patrol orbit around the central beacon (radius = 9.0)
    local move_speed = 0.55
    orbit_angle = orbit_angle + dt * move_speed
    local radius = 8.5
    local ox = math.cos(orbit_angle) * radius
    local oz = math.sin(orbit_angle) * radius

    entity.x = ox
    entity.y = 0.0
    entity.z = oz

    -- Orient mech forward tangent to the circle
    local forward_x = -math.sin(orbit_angle)
    local forward_z = math.cos(orbit_angle)
    local yaw_deg = math.deg(math.atan2(forward_x, forward_z))
    entity.ry = yaw_deg

    -- Direct head tracking towards central beacon (0.0, 3.0, 0.0)
    rig_set_look_target(entity.id, 0.0, 3.0, 0.0)

    -- Periodically trigger weapon recoil every 2.5 seconds
    if recoil_timer >= 2.5 then
        recoil_timer = 0.0
        pulse_count = pulse_count + 1
        rig_trigger_recoil(entity.id, 1.35)
        spawn_floating_text(entity.x, entity.y + 3.8, entity.z, "* HEAVY WEAPON RECOIL *", 1.0, 0.3, 0.2, 1.4)
        ui_set_text("recoil_status", "Recoil Pulses: " .. tostring(pulse_count))
    end

    -- Verify 2-bone IK analytical solver dynamically in Lua
    local test_ik = solve_ik_2bone(0.0, 2.0, 0.0, ox, 1.0, oz, 1.2, 1.2)
    if test_ik and test_ik.jy then
        -- IK solution evaluated without errors
    end
end
