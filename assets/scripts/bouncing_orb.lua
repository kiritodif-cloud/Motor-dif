-- Bouncing Orb script: plays bounce sfx and emits sparks on collision!
local bounce_count = 0

function on_update(entity, dt)
    set_hud_text("Orb Tracking", string.format("Y: %.2f | Bounces: %d", entity.y, bounce_count))
end

function on_collision(entity, other_id)
    bounce_count = bounce_count + 1
    play_sound("assets/sfx/bounce.wav", 0.9)
    local pos = get_entity(entity.id)
    if pos then
        spawn_particles(pos.x, pos.y, pos.z, 1.0, 0.35, 0.85, 30, 5.0, 0.9)
    end
    set_hud_text("Last Collision", "Bounced on " .. other_id .. " (SFX + Sparks)")
end
