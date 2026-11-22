-- Gameplay Demo Script for Motor-dif
-- Controls: WASD or Arrow keys to move, Space to jump

local score = 0
local jumps = 0

function on_update(entity, dt)
    local move_speed = 0.35
    local jump_power = 0.6

    -- Input handling
    if is_key_down("W") or is_key_down("UP") then
        apply_impulse(0.0, 0.0, -move_speed)
    end
    if is_key_down("S") or is_key_down("DOWN") then
        apply_impulse(0.0, 0.0, move_speed)
    end
    if is_key_down("A") or is_key_down("LEFT") then
        apply_impulse(-move_speed, 0.0, 0.0)
    end
    if is_key_down("D") or is_key_down("RIGHT") then
        apply_impulse(move_speed, 0.0, 0.0)
    end
    if is_key_down("SPACE") then
        apply_impulse(0.0, jump_power, 0.0)
        jumps = jumps + 1
    end

    -- In-Game HUD overlay
    set_hud_text("1. Player State", string.format("Pos(%.1f, %.1f, %.1f)", entity.x, entity.y, entity.z))
    set_hud_text("2. Controls", "WASD / Arrows to Move | SPACE to Jump")
    set_hud_text("3. Physics Engine", "Rapier3D 60Hz Deterministic")
    set_hud_text("4. Total Jumps", tostring(jumps))
    set_hud_text("5. Score", tostring(score))
end

function on_collision(entity, other_id)
    score = score + 10
    set_hud_text("Collision Event", string.format("Entity '%s' collided with '%s'!", entity.id, other_id))
end
