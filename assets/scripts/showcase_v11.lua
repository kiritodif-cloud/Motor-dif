-- showcase_v11.lua
-- Motor-dif Fase 11: Produccion y Ecosistema Showcase Script

local time_acc = 0.0
local anim_playing = true
local particle_count = 0

-- 1. Initialize Declarative In-Game UI
ui_clear()

-- Top Header Panel
ui_panel("header_panel", "top_center", 0, 15, 420, 56, 0.08, 0.1, 0.15, 0.85)
ui_label("header_title", "top_center", 0, 22, "MOTOR-DIF FASE 11 * PRODUCTION SUITE", 15, 0.22, 0.75, 0.98)
ui_label("header_sub", "top_center", 0, 44, "Keyframes * Declarative UI * Profiler * 1-Click Exporter", 11, 0.55, 0.62, 0.72)

-- Core Telemetry HUD Card
ui_panel("telemetry_card", "top_left", 20, 20, 250, 130, 0.08, 0.1, 0.15, 0.88)
ui_label("telemetry_title", "top_left", 32, 30, "CELESTIAL CORE STATUS", 12, 1.0, 1.0, 1.0)
ui_bar("core_energy_bar", "top_left", 32, 52, 226, 22, 0.85, "ENERGY: 85%", 0.2, 0.8, 0.9)
ui_label("anim_label", "top_left", 32, 82, "Anim: Pulse Loop (Active)", 11, 0.35, 0.9, 0.45)
ui_label("controls_hint", "top_left", 32, 102, "F3: Profiler | Ctrl+1..4: Bookmarks", 10.5, 0.65, 0.72, 0.82)

-- Interactive Action Buttons
ui_button("btn_burst", "bottom_center", -110, 30, 200, 38, "[*] SPAWN PARTICLES", "trigger_burst")
ui_button("btn_toggle_anim", "bottom_center", 110, 30, 200, 38, "[>] TOGGLE ANIMATION", "toggle_core_anim")

-- Callbacks triggered by in-game UI buttons
function trigger_burst()
    particle_count = particle_count + 1
    play_tone(440, 880, "sine", 0.08, 0.4)
    spawn_particles(0.0, 2.5, 0.0, 45, 0.3, 0.85, 1.0, 1.0, 4.0, 1.2)
    spawn_floating_text(0.0, 3.2, 0.0, "+PARTICLE BURST", 0.3, 0.85, 1.0, 1.2)
end

function toggle_core_anim()
    anim_playing = not anim_playing
    if anim_playing then
        play_anim("celestial_core", "pulse_loop", true)
        ui_label("anim_label", "top_left", 32, 82, "Anim: Pulse Loop (Active)", 11, 0.35, 0.9, 0.45)
        spawn_floating_text(0.0, 3.0, 0.0, "PLAY ANIMATION", 0.35, 0.9, 0.45, 1.0)
    else
        stop_anim("celestial_core")
        ui_label("anim_label", "top_left", 32, 82, "Anim: Stopped (Paused)", 11, 0.95, 0.45, 0.45)
        spawn_floating_text(0.0, 3.0, 0.0, "STOP ANIMATION", 0.95, 0.45, 0.45, 1.0)
    end
    play_tone(330, 440, "sine", 0.06, 0.35)
end

function on_update(entity, dt)
    time_acc = time_acc + dt

    -- Dynamic smoothly oscillating Energy Bar in Declarative UI
    local energy = 0.5 + 0.45 * math.sin(time_acc * 1.8)
    local energy_pct = math.floor(energy * 100)
    local bar_color_r = 0.2 + (1.0 - energy) * 0.6
    local bar_color_g = 0.5 + energy * 0.4
    local bar_color_b = 0.9
    ui_bar("core_energy_bar", "top_left", 32, 52, 226, 22, energy, string.format("ENERGY: %d%%", energy_pct), bar_color_r, bar_color_g, bar_color_b)

    -- Ambient subtle particles around core
    if math.random() < 0.25 then
        spawn_particles(
            math.sin(time_acc * 3.0) * 2.5,
            2.2 + math.cos(time_acc * 2.0) * 0.5,
            math.cos(time_acc * 3.0) * 2.5,
            2,
            0.8,
            0.4,
            1.0,
            0.8,
            1.2,
            0.8
        )
    end
end
