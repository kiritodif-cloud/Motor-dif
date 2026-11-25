-- FPS Controller Template
-- Features: Pointer lock, WASD movement, mouse look (yaw/pitch), jump, optional voxel collision

local yaw = 0.0
local pitch = 0.0
local speed = 10.0
local jump_force = 7.0
local mouse_sensitivity = 0.0025
local is_grounded = false
local vy = 0.0
local gravity = -20.0
local voxel_collision = false

local initialized = false

function on_init(entity)
    print("[FPS Controller] Initialized for entity")
    set_pointer_lock(true)
end

function on_update(entity, dt)
    if not initialized then
        initialized = true
        on_init(entity)
    end
    -- Click to lock pointer if unlocked
    if is_mouse_button_just_pressed(1) then
        set_pointer_lock(true)
    end

    -- Mouse look (only when pointer locked)
    if is_pointer_locked() then
        local mouse_delta = get_mouse_delta()
        yaw = yaw - mouse_delta.dx * mouse_sensitivity
        pitch = pitch - mouse_delta.dy * mouse_sensitivity

        -- Clamp pitch between ~ -85 and +85 degrees
        if pitch > 1.48 then pitch = 1.48 end
        if pitch < -1.48 then pitch = -1.48 end
    end

    -- Calculate camera direction vectors on XZ plane
    local forward_x = -math.sin(yaw)
    local forward_z = -math.cos(yaw)
    local right_x = math.cos(yaw)
    local right_z = -math.sin(yaw)

    -- Movement input
    local move_x = 0.0
    local move_z = 0.0

    if is_key_down("W") then
        move_x = move_x + forward_x
        move_z = move_z + forward_z
    end
    if is_key_down("S") then
        move_x = move_x - forward_x
        move_z = move_z - forward_z
    end
    if is_key_down("D") then
        move_x = move_x + right_x
        move_z = move_z + right_z
    end
    if is_key_down("A") then
        move_x = move_x - right_x
        move_z = move_z - right_z
    end

    -- Normalize movement
    local len = math.sqrt(move_x * move_x + move_z * move_z)
    if len > 0.001 then
        move_x = (move_x / len) * speed * dt
        move_z = (move_z / len) * speed * dt
    else
        move_x = 0.0
        move_z = 0.0
    end

    local x, y, z = entity.x, entity.y, entity.z

    -- Ground check & jumping
    if voxel_collision then
        local hit = voxel_raycast(x, y + 0.1, z, 0.0, -1.0, 0.0, 1.3)
        if hit and hit.hit then
            is_grounded = true
            vy = 0.0
            if y < (hit.y + 1.0) then
                y = hit.y + 1.0
            end
        else
            is_grounded = false
            vy = vy + gravity * dt
        end

        if is_grounded and is_key_just_pressed("Space") then
            vy = jump_force
            is_grounded = false
        end
        y = y + vy * dt
    else
        local ground_h = get_terrain_height(x, z)
        if ground_h and (y - 1.0 <= ground_h + 0.05) then
            is_grounded = true
            y = ground_h + 1.0
            vy = 0.0
        elseif y <= 1.0 then
            is_grounded = true
            y = 1.0
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
    end

    -- Update position and rotation
    local new_x = x + move_x
    local new_z = z + move_z
    entity.x, entity.y, entity.z = new_x, y, new_z
    entity.ry = yaw

    -- Update camera to eye level
    local eye_x = new_x
    local eye_y = y + 0.7
    local eye_z = new_z

    local target_x = eye_x - math.sin(yaw) * math.cos(pitch) * 10.0
    local target_y = eye_y + math.sin(pitch) * 10.0
    local target_z = eye_z - math.cos(yaw) * math.cos(pitch) * 10.0

    set_camera(eye_x, eye_y, eye_z, target_x, target_y, target_z)
end
