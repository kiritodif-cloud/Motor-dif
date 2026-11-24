-- Third-Person Controller Template
-- Features: Smooth chase camera, WASD relative movement, character turning, jump, 'V' perspective toggle

local yaw = 0.0
local pitch = 0.25
local speed = 8.0
local jump_force = 7.0
local mouse_sensitivity = 0.0025
local cam_distance = 6.0
local is_grounded = true
local vy = 0.0
local gravity = -20.0
local first_person = false

function on_init()
    print("[Third-Person Controller] Initialized")
    set_pointer_lock(true)
end

function on_update(dt)
    -- Click to lock pointer
    if is_mouse_button_just_pressed(1) then
        set_pointer_lock(true)
    end

    -- Toggle 1st / 3rd person with 'V'
    if is_key_just_pressed("V") then
        first_person = not first_person
    end

    -- Camera Orbit
    if is_pointer_locked() then
        local mdx, mdy = get_mouse_delta()
        yaw = yaw - mdx * mouse_sensitivity
        pitch = pitch + mdy * mouse_sensitivity
        if pitch > 1.2 then pitch = 1.2 end
        if pitch < -0.3 then pitch = -0.3 end
    end

    local forward_x = -math.sin(yaw)
    local forward_z = -math.cos(yaw)
    local right_x = math.cos(yaw)
    local right_z = -math.sin(yaw)

    local input_x = 0.0
    local input_z = 0.0

    if is_key_down("W") then
        input_x = input_x + forward_x
        input_z = input_z + forward_z
    end
    if is_key_down("S") then
        input_x = input_x - forward_x
        input_z = input_z - forward_z
    end
    if is_key_down("D") then
        input_x = input_x + right_x
        input_z = input_z + right_z
    end
    if is_key_down("A") then
        input_x = input_x - right_x
        input_z = input_z - right_z
    end

    local x, y, z = get_position()

    -- Move and rotate character facing direction
    local move_len = math.sqrt(input_x * input_x + input_z * input_z)
    if move_len > 0.001 then
        input_x = (input_x / move_len) * speed * dt
        input_z = (input_z / move_len) * speed * dt
        local target_rot_y = math.atan2(-input_x, -input_z)
        set_rotation(0.0, target_rot_y, 0.0)
    else
        input_x = 0.0
        input_z = 0.0
    end

    -- Ground & Gravity
    local ground_h = get_terrain_height(x, z) or 0.0
    if y <= ground_h + 1.0 then
        is_grounded = true
        y = ground_h + 1.0
        vy = 0.0
    else
        is_grounded = false
        vy = vy + gravity * dt
    end

    if is_grounded and is_key_just_pressed("Space") then
        vy = jump_force
        is_grounded = false
    end
    y = y + vy * dt

    local new_x = x + input_x
    local new_z = z + input_z
    set_position(new_x, y, new_z)

    -- Camera Position
    if first_person then
        local eye_y = y + 0.8
        local tx = new_x - math.sin(yaw) * math.cos(pitch) * 10.0
        local ty = eye_y - math.sin(pitch) * 10.0
        local tz = new_z - math.cos(yaw) * math.cos(pitch) * 10.0
        set_camera(new_x, eye_y, new_z, tx, ty, tz)
    else
        local target_y = y + 1.2
        local cam_x = new_x + math.sin(yaw) * math.cos(pitch) * cam_distance
        local cam_y = target_y + math.sin(pitch) * cam_distance + 0.8
        local cam_z = new_z + math.cos(yaw) * math.cos(pitch) * cam_distance
        set_camera(cam_x, cam_y, cam_z, new_x, target_y, new_z)
    end
end
