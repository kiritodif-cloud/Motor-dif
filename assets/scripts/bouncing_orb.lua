-- Bouncing Orb script: plays bounce sfx and emits sparks on collision!
local bounce_count = 0

function on_update(entity, dt)
    -- Continuous rotation
    local pos = get_entity(entity.id)
end

function on_collision(entity, other_id)
    bounce_count = bounce_count + 1
    play_sound("assets/sfx/bounce.wav", 0.9)
    local pos = get_entity(entity.id)
    if pos then
        spawn_particles(pos.x, pos.y, pos.z, 1.0, 0.35, 0.85, 25, 4.5, 0.8)
    end
    set_hud_text("Orb Bounces", "Bounces: " .. tostring(bounce_count))
end
