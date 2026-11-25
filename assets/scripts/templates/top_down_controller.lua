-- Top-Down / Isometric Controller Template
-- Features: WASD movement on XZ plane, character turns towards movement or cursor, follow camera

local speed = 9.0
local cam_height = 14.0
local cam_back = 8.0

local initialized = false

function on_init(entity)
    print("[Top-Down Controller] Initialized")
end

function on_update(entity, dt)
    if not initialized then
        initialized = true
        on_init(entity)
    end
    local dx = 0.0
    local dz = 0.0

    if is_key_down("W") or is_key_down("Up") then
        dz = dz - 1.0
    end
    if is_key_down("S") or is_key_down("Down") then
        dz = dz + 1.0
    end
    if is_key_down("A") or is_key_down("Left") then
        dx = dx - 1.0
    end
    if is_key_down("D") or is_key_down("Right") then
        dx = dx + 1.0
    end

    local x, y, z = entity.x, entity.y, entity.z

    local len = math.sqrt(dx * dx + dz * dz)
    if len > 0.001 then
        dx = (dx / len) * speed * dt
        dz = (dz / len) * speed * dt

        local angle = math.atan2(-dx, -dz)
        entity.ry = angle
    else
        dx = 0.0
        dz = 0.0
    end

    local th = get_terrain_height(x, z)
    if th then
        y = th + 1.0
    else
        y = 1.0
    end

    local new_x = x + dx
    local new_z = z + dz
    entity.x, entity.y, entity.z = new_x, y, new_z

    local cam_x = new_x
    local cam_y = y + cam_height
    local cam_z = new_z + cam_back
    set_camera(cam_x, cam_y, cam_z, new_x, y, new_z)
end
