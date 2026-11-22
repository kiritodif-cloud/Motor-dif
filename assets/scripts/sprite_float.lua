-- Floating 2D sprite billboard animation
local elapsed = 0.0

function on_update(entity, dt)
    elapsed = elapsed + dt
    local target_y = 3.5 + math.sin(elapsed * 2.0) * 0.4
    set_hud_text("Sprite Status", "Floating Billboard Active")
end
