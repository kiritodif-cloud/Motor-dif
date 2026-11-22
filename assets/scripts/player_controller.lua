-- Motor-dif Player Controller Script
-- Controls player_cube using WASD / Arrow keys with physical forces and impulses

local move_force = 50.0
local jump_cooldown = 0.0

function on_update(entity, dt)
    -- Handle jump cooldown
    if jump_cooldown > 0.0 then
        jump_cooldown = jump_cooldown - dt
    end

    -- Boost with Shift
    local current_force = move_force
    if is_key_down("SHIFT") or is_key_down("LEFTSHIFT") or is_key_down("RIGHTSHIFT") then
        current_force = move_force * 2.2
    end

    local fx = 0.0
    local fz = 0.0

    -- Forward / Backward (Camera looks from +Z towards -Z)
    if is_key_down("W") or is_key_down("ARROWUP") then
        fz = fz - current_force
    end
    if is_key_down("S") or is_key_down("ARROWDOWN") then
        fz = fz + current_force
    end

    -- Left / Right
    if is_key_down("A") or is_key_down("ARROWLEFT") then
        fx = fx - current_force
    end
    if is_key_down("D") or is_key_down("ARROWRIGHT") then
        fx = fx + current_force
    end

    -- Apply directional physics force
    if fx ~= 0.0 or fz ~= 0.0 then
        apply_force(fx, 0.0, fz)
    end

    -- Jump / Vertical impulse with Space
    if is_key_down("SPACE") and jump_cooldown <= 0.0 then
        apply_impulse(0.0, 15.0, 0.0)
        jump_cooldown = 0.6
    end
end
