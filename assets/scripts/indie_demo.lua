-- Indie Studio Showcase Script for Motor-dif (Fase 8)
local frame_timer = 0.0
local current_frame = 0
local initialized = false

function init()
    if initialized then return end
    initialized = true

    -- 1. Persistence test: save and load data
    save_data("high_score", 9999)
    save_data("player_name", "CyberKnight")
    local hs = load_data("high_score")
    local name = load_data("player_name")

    set_hud_text("PLAYER", tostring(name or "Knight"))
    set_hud_text("HIGH SCORE", tostring(hs or 9999))
    set_hud_text("ENGINE", "Motor-dif v0.8 - Low-PBR & Integrated GPU")

    -- 2. Interactive HUD button
    set_hud_button("btn_shake_scale", "[*] Trigger Shake & Tween", 25.0, 130.0, "on_trigger_fx")
    set_hud_button("btn_sound", "[>] Play Bounce SFX", 25.0, 175.0, "on_play_sfx")
end

-- Global callback invoked when the HUD button is clicked
function on_trigger_fx()
    camera_shake(0.4, 0.6)
    tween_scale("pedestal", 1.5, 1.5, 1.5, 0.35)
    spawn_particles(0.0, 1.5, 0.0, 1.0, 0.7, 0.2, 45, 5.5, 1.0)
    play_sound("assets/sfx/bounce.wav", 1.0)
    set_hud_text("EVENT", "Camera Shake + Tween Scale Triggered!")
end

function on_play_sfx()
    play_sound("assets/sfx/bounce.wav", 1.0)
    spawn_particles(0.0, 1.0, 0.0, 0.2, 0.9, 1.0, 25, 4.0, 0.8)
    set_hud_text("EVENT", "SFX Triggered!")
end

function on_update(self, dt)
    init()

    -- 1. Parent Node Rotation (Scene Graph Hierarchy test)
    self.ry = (self.ry or 0.0) + dt * 1.2

    -- 2. 2D Sprite Sheet Frame Cycling (4 frames: 2 cols x 2 rows)
    frame_timer = frame_timer + dt
    if frame_timer >= 0.3 then
        frame_timer = 0.0
        current_frame = (current_frame + 1) % 4
        set_sprite_frame("sprite_hero", 2, 2, current_frame)
    end
end
