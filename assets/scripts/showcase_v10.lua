-- Showcase Visual Fase 10: HDR Bloom, Fresnel Rim, Trails, Raycast, Triggers & SFX Synth
local orbit_angle = 0.0
local has_initialized = false
local last_trigger_time = 0.0

function on_init(entity)
    -- Setup floating world labels
    set_world_label("plasma_core", "HYPERSPACE CORE [v10 HDR]", 1.6, 0.35, 0.9, 1.0)
    set_world_label("orbiting_scout", "PATROL SCOUT [TRAIL ACTIVE]", 0.8, 1.0, 0.85, 0.3)
    set_world_label("sensor_gateway", "[TRIGGER SENSOR GATE]", 1.3, 1.0, 0.3, 0.45)

    -- Setup dynamic additive trail behind the scout
    set_trail("orbiting_scout", 1.0, 0.75, 0.2, 0.9, 1.4, 0.25)

    -- Initial welcome floating text
    spawn_floating_text(0.0, 3.2, 0.0, "MOTOR-DIF FASE 10 ACTIVE", 0.3, 0.95, 1.0, 4.5)
    
    -- Play startup tone
    play_tone(300, 900, "sine", 0.2, 0.4)
end

function on_update(entity, dt)
    if not has_initialized then
        has_initialized = true
        on_init(entity)
    end

    -- 1. Orbit calculation around the central core
    orbit_angle = orbit_angle + dt * 1.6
    local radius = 3.5
    local ox = math.cos(orbit_angle) * radius
    local oz = math.sin(orbit_angle) * radius
    local oy = 1.8 + math.sin(orbit_angle * 2.5) * 0.45

    entity.x = ox
    entity.y = oy
    entity.z = oz
    entity.ry = -math.deg(orbit_angle) - 90.0
    entity.rz = math.sin(orbit_angle * 2.5) * 15.0

    -- 2. Immediate 3D vector drawing for boundary and gravity field
    draw_ring_3d(0, 0.05, 0, 3.5, 0.2, 0.7, 1.0, 0.6)
    draw_ring_3d(0, 0.05, 0, 7.0, 0.6, 0.25, 0.9, 0.45)
    draw_ring_3d(0, 0.05, 0, 10.5, 0.1, 0.5, 0.8, 0.3)

    -- 3. 3D Raycasting from scout towards the central core
    local to_core_x = -ox
    local to_core_y = 1.8 - oy
    local to_core_z = -oz
    local hit = raycast(entity.x, entity.y, entity.z, to_core_x, to_core_y, to_core_z, 6.0)

    if hit.hit then
        -- Draw connection beam when ray hits
        draw_line_3d(entity.x, entity.y, entity.z, 0, 1.8, 0, 0.2, 1.0, 0.6, 0.85)
    else
        -- Draw faint scan line
        draw_line_3d(entity.x, entity.y, entity.z, 0, 1.8, 0, 0.3, 0.6, 1.0, 0.4)
    end
end

function on_trigger_enter(other_id)
    -- Trigger feedback: Sound synthesizer + floating combat text
    play_tone(650, 1300, "square", 0.15, 0.6)
    spawn_floating_text(0, 2.6, 3.5, "SENSOR GATE ACTIVATED!", 1.0, 0.3, 0.5, 1.8)
end
