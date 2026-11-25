-- Vehicle Controller Template
-- Features: Forward/reverse acceleration, steering with yaw accumulation, natural drag, follow camera with lag

local speed = 0.0
local max_speed = 22.0
local max_reverse = -8.0
local acceleration = 18.0
local reverse_accel = 12.0
local drag = 4.0
local steer_speed = 2.4
local yaw = 0.0

local initialized = false

function on_init(entity)
    print("[Vehicle Controller] Initialized")
    yaw = entity.ry or 0.0
end

function on_update(entity, dt)
    if not initialized then
        initialized = true
        on_init(entity)
    end
    -- Steering (steers more effectively when moving)
    local steer_factor = math.min(1.0, math.abs(speed) / 4.0)
    if speed < 0.0 then
        steer_factor = -steer_factor
    end

    if is_key_down("A") or is_key_down("Left") then
        yaw = yaw + steer_speed * steer_factor * dt
    end
    if is_key_down("D") or is_key_down("Right") then
        yaw = yaw - steer_speed * steer_factor * dt
    end

    -- Throttle / Brakes
    if is_key_down("W") or is_key_down("Up") then
        speed = speed + acceleration * dt
        if speed > max_speed then speed = max_speed end
    elseif is_key_down("S") or is_key_down("Down") then
        speed = speed - reverse_accel * dt
        if speed < max_reverse then speed = max_reverse end
    else
        -- Natural drag / friction deceleration
        if speed > 0.0 then
            speed = speed - drag * dt
            if speed < 0.0 then speed = 0.0 end
        elseif speed < 0.0 then
            speed = speed + drag * dt
            if speed > 0.0 then speed = 0.0 end
        end
    end

    -- Handbrake
    if is_key_down("Space") then
        if speed > 0.0 then
            speed = speed - drag * 3.0 * dt
            if speed < 0.0 then speed = 0.0 end
        elseif speed < 0.0 then
            speed = speed + drag * 3.0 * dt
            if speed > 0.0 then speed = 0.0 end
        end
    end

    -- Movement along vehicle yaw heading
    local forward_x = -math.sin(yaw)
    local forward_z = -math.cos(yaw)

    local x, y, z = entity.x, entity.y, entity.z
    local new_x = x + forward_x * speed * dt
    local new_z = z + forward_z * speed * dt

    local th = get_terrain_height(new_x, new_z)
    if th then
        y = th + 0.6
    else
        y = 0.6
    end

    entity.x, entity.y, entity.z = new_x, y, new_z
    entity.ry = yaw

    -- Dynamic follow camera with lag behind the vehicle
    local cam_dist = 8.0 + (math.abs(speed) / max_speed) * 3.0
    local cam_x = new_x + math.sin(yaw) * cam_dist
    local cam_y = y + 3.2 + (math.abs(speed) / max_speed) * 0.8
    local cam_z = new_z + math.cos(yaw) * cam_dist

    local target_x = new_x - math.sin(yaw) * 4.0
    local target_y = y + 1.0
    local target_z = new_z - math.cos(yaw) * 4.0

    set_camera(cam_x, cam_y, cam_z, target_x, target_y, target_z)
end
