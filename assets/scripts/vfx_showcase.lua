-- vfx_showcase.lua
-- Motor-dif Fase 20: VFX Forge * Procedural Visual Effects Showcase Script

local total_time = 0.0
local has_initialized = false
local burst_timer = 0.0
local levelup_timer = 0.0
local spark_timer = 0.0
local burst_count = 0
local orb_angle = 0.0

function on_init(entity)
    -- World Space Floating Landmark Labels
    set_world_label("portal_arch", "ARCANE VORTEX GATEWAY [PORTAL VFX]", 0.35, 0.85, 1.0, 1.0)
    set_world_label("altar_core", "CELESTIAL NEXUS CORE", 0.45, 1.0, 0.75, 1.0)
    set_world_label("torch_l", "PYRO BRAZIER LEFT [FIRE VFX]", 1.0, 0.55, 0.2, 0.95)
    set_world_label("torch_r", "PYRO BRAZIER RIGHT [FIRE VFX]", 1.0, 0.55, 0.2, 0.95)

    -- Dynamic trail behind the central floating orb
    set_trail("altar_core", 0.3, 0.85, 1.0, 0.8, 1.5, 0.35)

    -- Declarative In-Game HUD
    ui_clear()

    -- Top center header card
    ui_panel("header_card", "top_center", 0, 15, 540, 58, 0.06, 0.09, 0.15, 0.92)
    ui_label("header_title", "top_center", 0, 22, "MOTOR-DIF FASE 20: VFX FORGE * PROCEDURAL VISUAL EFFECTS", 13.5, 0.25, 0.88, 1.0)
    ui_label("header_sub", "top_center", 0, 44, "Dynamic Physics * Vortex Swirl * HDR Bloom * Instanced GPU Quad Batching", 10.0, 0.55, 0.68, 0.80)

    -- Top left telemetry card
    ui_panel("telem_card", "top_left", 20, 20, 310, 175, 0.06, 0.09, 0.15, 0.92)
    ui_label("telem_title", "top_left", 32, 30, "VFX FORGE ENGINE TELEMETRY", 12.0, 1.0, 1.0, 1.0)
    ui_label("emitters_status", "top_left", 32, 54, "Persistent Emitters: 3 (Portal + Torches L/R)", 10.5, 0.35, 0.9, 0.45)
    ui_label("presets_status", "top_left", 32, 74, "Active Presets: magic_portal, fire, explosion, level_up", 10.0, 0.55, 0.8, 1.0)
    ui_label("vortex_status", "top_left", 32, 94, "Vortex Dynamics: XZ Helical Angular Acceleration", 10.0, 1.0, 0.85, 0.3)
    ui_label("burst_status", "top_left", 32, 114, "Dynamic VFX Bursts: 0", 10.5, 0.95, 0.45, 0.35)
    ui_label("bloom_status", "top_left", 32, 134, "HDR Pipeline: Multi-Pass Gaussian Bloom Active", 10.0, 0.35, 0.95, 0.85)
    ui_label("fps_target", "top_left", 32, 154, "Target FPS: 60+ (Instanced Zero-Alloc Per-Particle)", 10.0, 0.65, 0.75, 0.85)

    -- Top right control shortcuts card
    ui_panel("ctrl_card", "top_right", 20, 20, 240, 110, 0.06, 0.09, 0.15, 0.92)
    ui_label("ctrl_title", "top_right", 32, 30, "FORGE TOOLS & SHORTCUTS", 11.5, 1.0, 1.0, 1.0)
    ui_label("ctrl_vfx", "top_right", 32, 54, "[File -> VFX Forge] Editor GUI", 10.5, 0.3, 0.85, 1.0)
    ui_label("ctrl_palette", "top_right", 32, 74, "[Ctrl+K] Quick Preset Spawner", 10.5, 0.85, 0.65, 1.0)
    ui_label("ctrl_profiler", "top_right", 32, 94, "[F3] Real-Time GPU Profiler", 10.5, 0.95, 0.85, 0.4)

    -- Floating text notification
    spawn_floating_text(0.0, 4.5, 0.0, "VFX FORGE SYSTEM ONLINE", 0.3, 0.95, 1.0, 3.5)

    -- Ambient music from Music Forge
    play_music_preset("synthwave")
    set_music_intensity(0.65)
end

function on_update(entity, dt)
    if not has_initialized then
        has_initialized = true
        on_init(entity)
    end

    total_time = total_time + dt
    burst_timer = burst_timer + dt
    levelup_timer = levelup_timer + dt
    spark_timer = spark_timer + dt

    -- Smooth floating bobbing and slow rotation for the Nexus Core
    orb_angle = (orb_angle + dt * 45.0) % 360.0
    entity.ry = orb_angle
    entity.y = 2.0 + math.sin(total_time * 2.2) * 0.18

    -- Periodic Explosion VFX burst on left platform every 3.2 seconds
    if burst_timer >= 3.2 then
        burst_timer = 0.0
        burst_count = burst_count + 1
        spawn_vfx("explosion", -4.0, 1.0, -1.0)
        spawn_floating_text(-4.0, 2.5, -1.0, "* EXPLOSION BURST *", 1.0, 0.45, 0.2, 1.5)
        ui_set_text("burst_status", "Dynamic VFX Bursts: " .. tostring(burst_count))
    end

    -- Periodic Level-Up helical VFX burst on central altar every 4.8 seconds
    if levelup_timer >= 4.8 then
        levelup_timer = 0.0
        burst_count = burst_count + 1
        spawn_vfx("level_up", 0.0, 0.4, 0.0)
        spawn_floating_text(0.0, 3.2, 0.0, "* NEXUS LEVEL UP *", 0.35, 1.0, 0.6, 1.8)
        ui_set_text("burst_status", "Dynamic VFX Bursts: " .. tostring(burst_count))
    end

    -- Periodic Hit-Sparks on right platform every 2.1 seconds
    if spark_timer >= 2.1 then
        spark_timer = 0.0
        spawn_vfx("hit_spark", 4.0, 1.2, -1.0)
    end
end
