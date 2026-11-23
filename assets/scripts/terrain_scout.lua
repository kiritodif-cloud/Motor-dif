-- Showcase Script: Terrain Scout for Level Forge (Fase 18)
local orbit_angle = 0.0
local has_initialized = false
local time = 0.0

function on_init(entity)
    set_world_label("plasma_beacon", "ALIEN SANCTUARY CORE [Fase 18]", 2.2, 0.2, 0.9, 1.0)
    set_world_label("terrain_scout", "TERRAIN ROVER [HEIGHTMAP TRACKING]", 1.0, 0.95, 0.25, 0.85)

    -- Dynamic trail behind the scout
    set_trail("terrain_scout", 1.2, 0.85, 0.2, 0.95, 1.5, 0.3)

    -- Initial welcome floating text
    spawn_floating_text(0.0, 15.5, 0.0, "LEVEL FORGE: PROCEDURAL WORLDS ACTIVE", 0.35, 0.9, 1.0, 5.0)

    -- Start adaptive space ambient music
    play_music_preset("space_ambient")
    set_music_intensity(0.5)
end

function on_update(entity, dt)
    if not has_initialized then
        has_initialized = true
        on_init(entity)
    end

    time = time + dt
    orbit_angle = orbit_angle + dt * 0.45

    -- Patrol path over the terrain
    local radius_x = 22.0
    local radius_z = 18.0
    local ox = math.cos(orbit_angle) * radius_x
    local oz = math.sin(orbit_angle) * radius_z

    -- Sample procedural terrain height directly from Rust engine Level Forge
    local ground_h = get_terrain_height(ox, oz)
    local hover_y = ground_h + 2.2 + math.sin(time * 3.5) * 0.35

    entity.x = ox
    entity.y = hover_y
    entity.z = oz

    -- Orient scout towards travel direction
    local dir_x = -math.sin(orbit_angle) * radius_x
    local dir_z = math.cos(orbit_angle) * radius_z
    local yaw = math.deg(math.atan2(dir_x, dir_z))
    entity.ry = yaw
    entity.rz = math.sin(time * 3.5) * 8.0

    -- 3D scan rings & line to sanctuary core
    draw_ring_3d(ox, ground_h + 0.1, oz, 1.8, 0.2, 0.9, 1.0, 0.6)
    draw_line_3d(ox, hover_y, oz, 0.0, 12.8, 0.0, 0.85, 0.25, 0.95, 0.4)

    -- Dynamic music intensity based on terrain elevation
    local intensity = 0.35 + (ground_h / 14.0) * 0.55
    set_music_intensity(intensity)
end
