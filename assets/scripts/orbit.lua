-- Motor-dif Lua Script: Orbit and Rotate
-- Controls entity in 3D: orbits around origin (0, 0, 0) and rotates

local angle = 0.0
local speed = 2.5
local radius = 3.5

function on_update(entity, dt)
    -- Speed boost if W is pressed, reverse if S is pressed
    local current_speed = speed
    if is_key_down("W") then
        current_speed = speed * 2.5
    elseif is_key_down("S") then
        current_speed = -speed
    end

    angle = angle + current_speed * dt

    -- Orbit in XZ plane around origin
    entity.x = math.cos(angle) * radius
    entity.z = math.sin(angle) * radius
    entity.y = math.sin(angle * 2.0) * 0.75 -- Gentle vertical wave

    -- Continuous self-rotation
    entity.ry = entity.ry + 2.5 * dt
    entity.rx = entity.rx + 1.2 * dt
end
