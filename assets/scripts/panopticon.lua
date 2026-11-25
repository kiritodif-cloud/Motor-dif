-- ============================================================================
-- PROYECTO "PANOPTICON" - Motor-dif v0.3.0 (RTT CCTV + Multijugador UDP Renet)
-- Escena: assets/panopticon.json
-- Script: assets/scripts/panopticon.lua
-- Hito 1 (Torre/CCTV) + Hito 2 (Campo/Sabotajes) + Hito 3 (Red UDP y Roles)
-- ============================================================================

local MATCH_DURATION = 720.0 -- 12 minutos (4 min por fase)
local ANTENNA_HOLD_GOAL = 35.0
local SERVER_HACK_GOAL = 15.0

-- Estado global de partida
local initialized = false
local game_state = "MENU" -- "MENU" | "PLAYING"
local menu_tab = "MAIN"   -- "MAIN" | "CONTROLS"
local match_time = MATCH_DURATION
local elapsed_time = 0.0
local phase = 1
local game_over = false
local winner = ""
local win_reason = ""

-- Recursos y alertas del Sistema
local energy = 100.0
local max_energy = 100.0
local lockdown_timer = 0.0
local comms_jam_timer = 0.0
local server_hack_progress = 0.0
local antenna_hold_timer = 0.0
local alert_banner = "SISTEMA EN LINEA - VIGILANCIA RUTINARIA"
local alert_banner_timer = 4.0

-- Estado del Operador y Camaras
local active_cam = 0
local selected_npc_idx = 1
local scan_progress = 0.0
local scanning = false
local cam_micro_installed = false
local show_rtt_matrix = true
local op_walk_phase = 0.0

-- Puerta con contraseña de la Sala de Camaras (Operador, Policia y Tecnico)
local DOOR_PIN = "4826"
local cam_door_open_timer = 0.0
local cam_door_is_open = false
local keypad_open = false
local keypad_input = ""
local keypad_status = "INGRESE PIN (4 DIGITOS)"

-- Estado de Red Multijugador (Renet UDP)
local my_role = "OPERADOR" -- "OPERADOR", "POLICIA", "REBELDE", "TECNICO"
local my_controlled_npc = 0
local net_players = {} -- [client_id] = { role, npc_id, mx, mz, sprint, action_queued }
local net_sync_accum = 0.0
local client_input_accum = 0.0
local client_action_pending = false
local synced_jailed = 0
local synced_hacked = 0

-- Definicion de camaras
local cameras = {
    [0] = {
        name = "SALA DE CONTROL",
        x = 0.0, y = 6.85, z = -20.6,
        yaw = 0.0, pitch = -0.15, zoom = 0.0,
        yaw_min = -6.283, yaw_max = 6.283,
        blind_timer = 0.0, loop_timer = 0.0, sector = 0, fixed = true
    },
    [1] = {
        name = "CAM-01 PLAZA N",
        x = -12.0, y = 8.2, z = -10.0,
        yaw = 0.78, pitch = -0.52, zoom = 0.0,
        yaw_min = -0.2, yaw_max = 1.75,
        blind_timer = 0.0, loop_timer = 0.0, sector = 1, fixed = true,
        head_id = "cctv_1_head", rtt_id = "rtt_cam_1"
    },
    [2] = {
        name = "CAM-02 PLAZA S",
        x = 12.0, y = 8.2, z = 13.0,
        yaw = -2.35, pitch = -0.52, zoom = 0.0,
        yaw_min = -3.3, yaw_max = -1.4,
        blind_timer = 0.0, loop_timer = 0.0, sector = 2, fixed = true,
        head_id = "cctv_2_head", rtt_id = "rtt_cam_2"
    },
    [3] = {
        name = "CAM-03 MERCADO",
        x = -16.5, y = 7.2, z = -4.0,
        yaw = 2.35, pitch = -0.48, zoom = 0.0,
        yaw_min = 1.1, yaw_max = 3.5,
        blind_timer = 0.0, loop_timer = 0.0, sector = 1, fixed = true,
        head_id = "cctv_3_head", rtt_id = "rtt_cam_3"
    },
    [4] = {
        name = "CAM-04 SUBEST.",
        x = 16.5, y = 7.2, z = -11.0,
        yaw = -0.85, pitch = -0.48, zoom = 0.0,
        yaw_min = -2.0, yaw_max = 0.25,
        blind_timer = 0.0, loop_timer = 0.0, sector = 2, fixed = true,
        head_id = "cctv_4_head", rtt_id = "rtt_cam_4"
    },
    [5] = {
        name = "CAM-05 MICRO",
        x = -22.5, y = 5.4, z = 9.5,
        yaw = -1.2, pitch = -0.45, zoom = 0.0,
        yaw_min = -3.14, yaw_max = 3.14,
        blind_timer = 0.0, loop_timer = 0.0, sector = 1, fixed = false,
        head_id = "cctv_5_head"
    }
}

-- Infraestructura
local fuse_boxes = {
    { id = "fuse_box_1", sector = 1, x = -19.5, z = -4.5, down_timer = 0.0, name = "Sector Oeste" },
    { id = "fuse_box_2", sector = 2, x = 19.5,  z = 5.5,  down_timer = 0.0, name = "Sector Este" }
}

local antennas = {
    { id = "antenna_1", tip = "antenna_1_tip", x = -8.0,  z = 15.0,  hacked = false, hack_prog = 0.0 },
    { id = "antenna_2", tip = "antenna_2_tip", x = -25.5, z = 11.0,  hacked = false, hack_prog = 0.0 },
    { id = "antenna_3", tip = "antenna_3_tip", x = 24.5,  z = -16.5, hacked = false, hack_prog = 0.0 }
}

local keycards = {
    { id = "keycard_1", x = 22.5,  z = -17.5, stolen = false },
    { id = "keycard_2", x = -22.5, z = 5.5,   stolen = false }
}

local plaza_wps = {
    { x = -6.0,  z = -9.5 },  { x =  6.0,  z = -9.5 },
    { x =  11.5, z =  2.0 },  { x =  6.0,  z =  13.2 },
    { x = -6.0,  z =  13.2 }, { x = -11.5, z =  2.0 },
    { x = -17.5, z = -3.0 },  { x = -18.5, z =  8.5 },
    { x =  17.2, z = -3.0 },  { x =  17.2, z =  6.5 }
}

local npcs = {}
local rebel_ids = {}
local NUM_NPCS = 12

local police = {
    x = 5.0, z = -8.0,
    speed_patrol = 3.4, speed_sprint = 8.4,
    state = "PATRULLA",
    target_npc = nil,
    ping_x = nil, ping_z = nil,
    patrol_idx = 1,
    anim = 0.0
}

local tech = {
    x = -5.0, z = -7.0,
    speed = 3.6,
    state = "INSTALAR_MICRO",
    repair_kind = nil,
    repair_idx = nil,
    repair_prog = 0.0,
    patrol_idx = 4,
    anim = 0.0
}

-- ============================================================================
-- UTILIDADES
-- ============================================================================
local function dist2d(x1, z1, x2, z2)
    local dx = x1 - x2
    local dz = z1 - z2
    return math.sqrt(dx * dx + dz * dz)
end

local function clamp(v, lo, hi)
    if v < lo then return lo end
    if v > hi then return hi end
    return v
end

local function split_pipe(str)
    local out = {}
    for part in string.gmatch(str, "([^|]+)") do
        table.insert(out, part)
    end
    return out
end

local function key_pressed_any(k1, k2)
    if k2 then
        return is_key_just_pressed(k1) or is_key_just_pressed(k2)
    end
    return is_key_just_pressed(k1)
end

local function set_alert(msg, dur)
    alert_banner = msg
    alert_banner_timer = dur or 4.0
end

local function is_on_main_road(x, z)
    if math.abs(z - (-13.0)) < 2.2 and math.abs(x) < 30.0 then return true end
    if math.abs(z - 17.0) < 2.2 and math.abs(x) < 30.0 then return true end
    if math.abs(x - (-14.5)) < 2.2 and z > -15.0 and z < 19.0 then return true end
    if math.abs(x - 14.5) < 2.2 and z > -15.0 and z < 19.0 then return true end
    return false
end

local function is_sector_powered(sector)
    for _, fb in ipairs(fuse_boxes) do
        if fb.sector == sector and fb.down_timer > 0.0 then
            return false
        end
    end
    return true
end

local function is_cam_operational(idx)
    local c = cameras[idx]
    if not c then return false end
    if idx == 5 and not cam_micro_installed then return false end
    if c.blind_timer > 0.0 then return false end
    if c.sector > 0 and not is_sector_powered(c.sector) then return false end
    return true
end

local function get_cam_eye_pos(idx)
    local c = cameras[idx]
    if idx == 0 then
        return c.x, c.y, c.z
    end
    local cp = math.cos(c.pitch)
    local fx = math.sin(c.yaw) * cp
    local fy = math.sin(c.pitch)
    local fz = math.cos(c.yaw) * cp
    local dist_ahead = 0.95 + c.zoom * 5.0
    return c.x + fx * dist_ahead, c.y - 0.15 + fy * dist_ahead, c.z + fz * dist_ahead
end

local function get_cam_target_point(idx)
    local c = cameras[idx]
    local cp = math.cos(c.pitch)
    local fx = math.sin(c.yaw) * cp
    local fy = math.sin(c.pitch)
    local fz = math.cos(c.yaw) * cp
    if idx == 0 then
        return c.x + fx * 10.0, c.y + fy * 10.0, c.z + fz * 10.0
    end
    local ex, ey, ez = get_cam_eye_pos(idx)
    return ex + fx * 14.0, ey + fy * 14.0, ez + fz * 14.0
end

local function is_point_watched_by_any_cam(px, pz)
    for idx = 1, 5 do
        if is_cam_operational(idx) and cameras[idx].loop_timer <= 0.0 then
            local c = cameras[idx]
            local d = dist2d(px, pz, c.x, c.z)
            if d < 18.0 then
                local dx = px - c.x
                local dz = pz - c.z
                local ang = math.atan2(dx, dz)
                local diff = math.abs(ang - c.yaw)
                if diff > 3.14159 then diff = 6.28318 - diff end
                if diff < 0.72 then
                    return true
                end
            end
        end
    end
    return false
end

local function setup_rtt_monitors()
    for i = 1, 4 do
        local rtt_id = "rtt_cam_" .. i
        create_render_target(rtt_id, 320, 180)
        attach_camera_rtt("cctv_" .. i .. "_lens", rtt_id, 68.0, 12.0)
        set_material_rtt("tower_mon_" .. i, rtt_id, true)
    end
end

local function update_building_window_pbr(glow)
    local brick_ids = { "ent_w1_upper", "ent_w3_upper", "ent_s1_upper" }
    for _, id in ipairs(brick_ids) do
        set_pbr_material(id, {
            normal_map = "assets/textures/bldg_brick_normal.png",
            metallic_roughness_map = "assets/textures/bldg_brick_mr.png",
            emissive_map = "assets/textures/bldg_brick_emissive.png",
            metallic = 0.35, roughness = 0.88, normal_scale = 1.2, emissive_intensity = glow
        })
    end
    local conc_ids = { "ent_w2n_upper", "ent_w2s_upper", "ent_e1_upper", "ent_e2n_upper", "ent_e2s_upper", "ent_s2_upper" }
    for _, id in ipairs(conc_ids) do
        set_pbr_material(id, {
            normal_map = "assets/textures/bldg_concrete_normal.png",
            metallic_roughness_map = "assets/textures/bldg_concrete_mr.png",
            emissive_map = "assets/textures/bldg_concrete_emissive.png",
            metallic = 0.45, roughness = 0.85, normal_scale = 1.2, emissive_intensity = glow
        })
    end
end

local function setup_pbr_architecture()
    update_building_window_pbr(0.15)
    set_pbr_material("plaza_floor", {
        normal_map = "assets/textures/urban_plaza_normal.png",
        metallic_roughness_map = "assets/textures/urban_plaza_mr.png",
        metallic = 0.15, roughness = 0.85, normal_scale = 1.15, emissive_intensity = 0.0
    })
    for _, sw in ipairs({ "sidewalk_nw", "sidewalk_w", "sidewalk_e", "sidewalk_s", "sidewalk_n", "tree_1_box", "tree_2_box", "tree_3_box", "tree_4_box" }) do
        set_pbr_material(sw, {
            normal_map = "assets/textures/sidewalk_normal.png",
            metallic_roughness_map = "assets/textures/sidewalk_mr.png",
            metallic = 0.05, roughness = 0.88, normal_scale = 1.15, emissive_intensity = 0.0
        })
    end
    for tidx = 1, 4 do
        for _, tp in ipairs({ "flare", "trunk", "br1", "br2", "br3" }) do
            set_pbr_material("tree_" .. tidx .. "_" .. tp, {
                normal_map = "assets/textures/tree_bark_normal.png",
                metallic_roughness_map = "assets/textures/tree_bark_mr.png",
                metallic = 0.02, roughness = 0.92, normal_scale = 1.35, emissive_intensity = 0.0
            })
        end
        for _, tc in ipairs({ "leaf", "crown_1", "crown_2", "crown_3", "crown_4" }) do
            set_pbr_material("tree_" .. tidx .. "_" .. tc, {
                normal_map = "assets/textures/tree_foliage_normal.png",
                metallic_roughness_map = "assets/textures/tree_foliage_mr.png",
                metallic = 0.02, roughness = 0.80, normal_scale = 1.25, emissive_intensity = 0.0
            })
        end
    end
    for _, wf in ipairs({ "ent_w2n_floor", "ent_w2s_floor", "ent_w3_floor", "ent_e2s_floor", "ent_s1_floor" }) do
        set_pbr_material(wf, {
            normal_map = "assets/textures/wood_floor_normal.png",
            metallic_roughness_map = "assets/textures/wood_floor_mr.png",
            metallic = 0.05, roughness = 0.56, normal_scale = 1.1, emissive_intensity = 0.0
        })
    end
    for _, rf in ipairs({ "ent_w1_floor", "ent_e1_floor", "ent_e2n_floor", "ent_s2_floor", "tower_upper_floor" }) do
        set_pbr_material(rf, {
            normal_map = "assets/textures/room_floor_normal.png",
            metallic_roughness_map = "assets/textures/room_floor_mr.png",
            metallic = 0.28, roughness = 0.58, normal_scale = 1.15, emissive_intensity = 0.0
        })
    end
    local plaster_ids = {
        "ent_w1_ceil", "ent_w1_wn", "ent_w1_ws", "ent_w1_ww", "ent_w1_we_a", "ent_w1_we_b",
        "ent_w2n_ceil", "ent_w2n_wn", "ent_w2n_ws", "ent_w2n_ww", "ent_w2n_we_a", "ent_w2n_we_b",
        "ent_w2s_ceil", "ent_w2s_wn", "ent_w2s_ws", "ent_w2s_ww", "ent_w2s_we_a", "ent_w2s_we_b",
        "ent_w3_ceil", "ent_w3_wn_l", "ent_w3_wn_r", "ent_w3_ws", "ent_w3_ww", "ent_w3_we",
        "ent_e1_ceil", "ent_e1_wn", "ent_e1_ws", "ent_e1_ww_a", "ent_e1_ww_b", "ent_e1_we",
        "ent_e2n_ceil", "ent_e2n_wn", "ent_e2n_ws", "ent_e2n_ww_a", "ent_e2n_ww_b", "ent_e2n_we",
        "ent_e2s_ceil", "ent_e2s_wn", "ent_e2s_ws", "ent_e2s_ww_a", "ent_e2s_ww_b", "ent_e2s_we",
        "ent_s1_ceil", "ent_s1_wn_l", "ent_s1_wn_r", "ent_s1_ws", "ent_s1_ww", "ent_s1_we",
        "ent_s2_ceil", "ent_s2_wn_l", "ent_s2_wn_r", "ent_s2_ws", "ent_s2_ww", "ent_s2_we"
    }
    for _, pl in ipairs(plaster_ids) do
        set_pbr_material(pl, {
            normal_map = "assets/textures/interior_plaster_normal.png",
            metallic_roughness_map = "assets/textures/interior_plaster_mr.png",
            metallic = 0.04, roughness = 0.82, normal_scale = 1.0, emissive_intensity = 0.0
        })
    end
    local roads = { "ground_asphalt", "road_north", "road_south", "road_west", "road_east" }
    for _, r in ipairs(roads) do
        set_pbr_material(r, {
            normal_map = "assets/textures/urban_asphalt_normal.png",
            metallic_roughness_map = "assets/textures/urban_asphalt_mr.png",
            metallic = 0.1, roughness = 0.88, normal_scale = 1.1, emissive_intensity = 0.0
        })
    end
    local sub_ids = { "substation_core", "substation_trans_1", "substation_trans_2", "tower_desk", "tower_desk_front", "prop_side_desk", "tower_srv_console", "tower_rack_l", "tower_rack_r" }
    for _, s in ipairs(sub_ids) do
        set_pbr_material(s, {
            normal_map = "assets/textures/substation_panel_normal.png",
            metallic_roughness_map = "assets/textures/substation_panel_mr.png",
            metallic = 0.65, roughness = 0.55, normal_scale = 1.0, emissive_intensity = 0.0
        })
    end
    set_pbr_material("tower_upper_floor", {
        normal_map = "assets/textures/room_floor_normal.png",
        metallic_roughness_map = "assets/textures/room_floor_mr.png",
        metallic = 0.28, roughness = 0.58, normal_scale = 1.15, emissive_intensity = 0.0
    })
    for _, w in ipairs({ "tower_int_wall_w", "tower_int_wall_n" }) do
        set_pbr_material(w, {
            normal_map = "assets/textures/room_wall_normal.png",
            metallic_roughness_map = "assets/textures/room_wall_mr.png",
            metallic = 0.22, roughness = 0.72, normal_scale = 1.15, emissive_intensity = 0.0
        })
    end
    local tower_ids = {
        "tower_wall_back", "tower_wall_left", "tower_wall_right", "tower_roof",
        "tower_front_lower_c", "tower_stair_landing", "tower_room_wall_n", "tower_room_wall_s"
    }
    for _, t in ipairs(tower_ids) do
        set_pbr_material(t, {
            normal_map = "assets/textures/tower_concrete_normal.png",
            metallic_roughness_map = "assets/textures/tower_concrete_mr.png",
            metallic = 0.15, roughness = 0.9, normal_scale = 1.1, emissive_intensity = 0.0
        })
    end
    for i = 0, 9 do
        set_pbr_material("tower_stair_" .. i, {
            normal_map = "assets/textures/substation_panel_normal.png",
            metallic_roughness_map = "assets/textures/substation_panel_mr.png",
            metallic = 0.55, roughness = 0.65, normal_scale = 1.0, emissive_intensity = 0.0
        })
    end
end

-- ============================================================================
-- INICIALIZACION Y RESET
-- ============================================================================
function reset_panopticon()
    ui_clear()
    clear_timers()
    clear_events()

    if net_is_client() and net_client_id() == 0 then
        net_disconnect()
        my_role = "OPERADOR"
        my_controlled_npc = 0
    end

    match_time = MATCH_DURATION
    elapsed_time = 0.0
    phase = 1
    game_over = false
    winner = ""
    win_reason = ""
    energy = 100.0
    lockdown_timer = 0.0
    comms_jam_timer = 0.0
    cam_door_open_timer = 0.0
    cam_door_is_open = false
    keypad_open = false
    keypad_input = ""
    keypad_status = "INGRESE PIN (4 DIGITOS)"
    tween_position("cam_room_door", 5.4, 6.6, -24.1, 0.0)
    set_material("cam_room_door_led_in",  { color = { 0.95, 0.2, 0.2, 1.0 }, emissive = 0.7 })
    set_material("cam_room_door_led_out", { color = { 0.95, 0.2, 0.2, 1.0 }, emissive = 0.7 })
    server_hack_progress = 0.0
    antenna_hold_timer = 0.0
    active_cam = 0
    selected_npc_idx = 1
    scan_progress = 0.0
    scanning = false
    cam_micro_installed = false

    setup_rtt_monitors()
    setup_pbr_architecture()
    set_time_of_day(12.0)
    set_alert("TURNO INICIADO: VIGILA LA PLAZA Y DETECTA INFRACTORES", 5.0)

    cameras[0].x = 0.0; cameras[0].y = 6.85; cameras[0].z = -20.6
    cameras[0].yaw = 0.0; op_walk_phase = 0.0
    cameras[1].yaw = 0.78
    cameras[2].yaw = -2.35
    cameras[3].yaw = 2.35
    cameras[4].yaw = -0.85
    cameras[5].yaw = -1.2
    for i = 0, 5 do
        cameras[i].pitch = (i == 0) and -0.15 or -0.50
        cameras[i].zoom = 0.0
        cameras[i].blind_timer = 0.0
        cameras[i].loop_timer = 0.0
    end
    tween_position("cctv_5_head", 0.0, -15.0, 0.0, 0.0)

    for _, fb in ipairs(fuse_boxes) do
        fb.down_timer = 0.0
        set_material(fb.id, { color = { 0.9, 0.6, 0.15, 1.0 }, emissive = 0.12 })
    end
    for _, a in ipairs(antennas) do
        a.hacked = false
        a.hack_prog = 0.0
        set_material(a.tip, { color = { 0.2, 0.85, 0.4, 1.0 }, emissive = 0.6 })
        set_light(a.tip, 0.2, 0.9, 0.4, 2.0, 10.0)
    end
    for _, kc in ipairs(keycards) do
        kc.stolen = false
        tween_position(kc.id, kc.x, 0.9, kc.z, 0.0)
    end
    tween_position("sec_door_west", -6.0, 4.2, -16.0, 0.0)
    tween_position("sec_door_east",  6.0, 4.2, -16.0, 0.0)

    math.randomseed(math.floor(elapsed_time * 1000) + 7919)
    local indices = { 1,2,3,4,5,6,7,8,9,10,11,12 }
    for i = 12, 2, -1 do
        local j = math.random(1, i)
        indices[i], indices[j] = indices[j], indices[i]
    end
    rebel_ids = { indices[1], indices[2], indices[3] }
    local rebel_set = {}
    rebel_set[indices[1]] = true
    rebel_set[indices[2]] = true
    rebel_set[indices[3]] = true

    npcs = {}
    for i = 1, NUM_NPCS do
        local ang = (i - 1) * (6.28318 / NUM_NPCS)
        local sx = math.cos(ang) * 7.5
        local sz = 2.0 + math.sin(ang) * 7.5
        local wp = ((i - 1) % #plaza_wps) + 1
        npcs[i] = {
            id = i,
            root = "npc_" .. i .. "_root",
            head = "npc_" .. i .. "_head",
            item = "npc_" .. i .. "_item",
            x = sx, z = sz,
            wp_idx = wp,
            is_rebel = rebel_set[i] == true,
            state = "CAMINAR",
            task_kind = nil,
            task_idx = nil,
            task_timer = 0.0,
            cooldown = 5.0 + math.random() * 8.0,
            speed = 2.1 + math.random() * 0.4,
            sprinting = false,
            has_contraband = false,
            has_keycard = false,
            warrant = false,
            scanned = false,
            infraction = nil,
            crowding_timer = 0.0,
            false_arrest_timer = 0.0,
            anim = i * 0.6
        }
        if not net_is_client() then
            tween_position(npcs[i].root, sx, 0.0, sz, 0.0)
            tween_scale(npcs[i].item, 0.001, 0.001, 0.001, 0.0)
        end
        set_world_label(npcs[i].head, "", 0.55, 1.0, 1.0, 1.0)
    end

    police.x = 5.0; police.z = -8.0
    police.state = "PATRULLA"; police.target_npc = nil
    police.ping_x = nil; police.ping_z = nil
    if not net_is_client() then
        tween_position("police_root", police.x, 0.0, police.z, 0.0)
    end

    tech.x = -5.0; tech.z = -7.0
    tech.state = "INSTALAR_MICRO"; tech.repair_kind = nil; tech.repair_prog = 0.0
    if not net_is_client() then
        tween_position("tech_root", tech.x, 0.0, tech.z, 0.0)
    end

    play_music_preset("cyber_action")
    set_music_intensity(0.25)
    play_tone(320, 640, "sine", 0.25, 0.5)
end

-- ============================================================================
-- CALLBACKS Y PROTOCOLO DE RED UDP (Renet)
-- ============================================================================
local function find_human_for_role(role_name)
    for cid, p in pairs(net_players) do
        if p.role == role_name then
            return cid, p
        end
    end
    return nil, nil
end

local function assign_role_to_client(cid, requested_role)
    local chosen = requested_role
    if not chosen then
        local _, has_pol = find_human_for_role("POLICIA")
        local _, has_tec = find_human_for_role("TECNICO")
        if not has_pol then
            chosen = "POLICIA"
        elseif not has_tec then
            chosen = "TECNICO"
        else
            chosen = "REBELDE"
        end
    end
    local npc_id = 0
    if chosen == "REBELDE" then
        npc_id = rebel_ids[1] or 1
        for _, rid in ipairs(rebel_ids) do
            local used = false
            for ocid, op in pairs(net_players) do
                if ocid ~= cid and op.npc_id == rid then used = true; break end
            end
            if not used then npc_id = rid; break end
        end
    end
    net_players[cid] = {
        role = chosen,
        npc_id = npc_id,
        mx = 0.0, mz = 0.0,
        sprint = false,
        action_queued = false,
        jump_queued = false
    }
    net_broadcast(1, string.format("ASSIGN|%d|%s|%d", cid, chosen, npc_id))
    set_alert(string.format("JUGADOR #%d UNIDO COMO %s", cid, chosen), 4.0)
end

function on_net_connect(client_id)
    if net_is_server() then
        assign_role_to_client(client_id, nil)
    end
end

function on_net_disconnect(client_id)
    if net_is_server() then
        net_players[client_id] = nil
        set_alert("JUGADOR #" .. tostring(client_id) .. " DESCONECTADO (IA RETOMA CONTROL)", 3.5)
    end
end

function on_net_message(sender, channel, message)
    if channel ~= 1 or type(message) ~= "string" then return end
    local parts = split_pipe(message)
    local tag = parts[1]
    if not tag then return end

    if net_is_server() then
        if tag == "IN" and #parts >= 6 then
            local cid = tonumber(parts[2]) or sender
            local p = net_players[cid]
            if p then
                p.mx = tonumber(parts[3]) or 0.0
                p.mz = tonumber(parts[4]) or 0.0
                p.sprint = (parts[5] == "1")
                if parts[6] == "1" then p.action_queued = true end
                if parts[7] == "1" then p.jump_queued = true end
            end
        elseif tag == "CLAIM" and #parts >= 3 then
            local cid = tonumber(parts[2]) or sender
            assign_role_to_client(cid, parts[3])
        elseif tag == "PIN" and #parts >= 3 then
            local cid = tonumber(parts[2]) or sender
            local code = parts[3]
            local p = net_players[cid]
            local who = p and p.role or ("CLI#" .. cid)
            if code == DOOR_PIN then
                cam_door_open_timer = 8.0
                cam_door_is_open = true
                tween_position("cam_room_door", 5.4, 6.6, -21.6, 0.35)
                set_material("cam_room_door_led_in",  { color = { 0.2, 0.95, 0.4, 1.0 }, emissive = 0.75 })
                set_material("cam_room_door_led_out", { color = { 0.2, 0.95, 0.4, 1.0 }, emissive = 0.75 })
                net_broadcast(1, "DOOR|8.0")
                net_broadcast(1, "EVENT|PUERTA DE SALA ABIERTA POR " .. who .. " (8s)")
                set_alert("PUERTA DE SALA ABIERTA POR " .. who .. " (8s)", 3.5)
                play_tone(640, 960, "sine", 0.22, 0.5)
            else
                net_broadcast(1, "EVENT|PIN INCORRECTO EN PUERTA DE SALA")
                play_tone(170, 120, "sawtooth", 0.22, 0.45)
            end
        elseif tag == "DOOR_CLOSE" then
            cam_door_open_timer = 0.0
            cam_door_is_open = false
            tween_position("cam_room_door", 5.4, 6.6, -24.1, 0.35)
            set_material("cam_room_door_led_in",  { color = { 0.95, 0.2, 0.2, 1.0 }, emissive = 0.7 })
            set_material("cam_room_door_led_out", { color = { 0.95, 0.2, 0.2, 1.0 }, emissive = 0.7 })
            net_broadcast(1, "DOOR|0.0")
        end
    elseif net_is_client() then
        local my_cid = net_client_id()
        if tag == "ASSIGN" and #parts >= 4 then
            local cid = tonumber(parts[2]) or 0
            if cid == my_cid then
                my_role = parts[3]
                my_controlled_npc = tonumber(parts[4]) or 0
                set_alert("ROL ASIGNADO: " .. my_role, 4.0)
            end
        elseif tag == "SYNC" and #parts >= 11 then
            match_time = tonumber(parts[2]) or match_time
            phase = tonumber(parts[3]) or phase
            energy = tonumber(parts[4]) or energy
            lockdown_timer = tonumber(parts[5]) or 0.0
            server_hack_progress = tonumber(parts[6]) or 0.0
            antenna_hold_timer = tonumber(parts[7]) or 0.0
            cam_micro_installed = (parts[8] == "1")
            synced_jailed = tonumber(parts[9]) or 0
            synced_hacked = tonumber(parts[10]) or 0
            if parts[11] and parts[11] ~= "" then
                alert_banner = parts[11]
                alert_banner_timer = 1.5
            end
            if parts[12] == "SISTEMA" or parts[12] == "REBELDES" then
                game_over = true
                winner = parts[12]
                win_reason = parts[13] or ""
            end
        elseif tag == "EVENT" and #parts >= 2 then
            set_alert(parts[2], 4.0)
        elseif tag == "DOOR" and #parts >= 2 then
            cam_door_open_timer = tonumber(parts[2]) or 0.0
            cam_door_is_open = cam_door_open_timer > 0.0
            local lcol = cam_door_is_open and { 0.2, 0.95, 0.4, 1.0 } or { 0.95, 0.2, 0.2, 1.0 }
            set_material("cam_room_door_led_in",  { color = lcol, emissive = 0.7 })
            set_material("cam_room_door_led_out", { color = lcol, emissive = 0.7 })
        end
    end
end

-- ============================================================================
-- ACCIONES DEL OPERADOR Y DE RED
-- ============================================================================
local function switch_camera(idx)
    if idx == 5 and not cam_micro_installed then
        set_alert("MICRO-CAM aun no instalada por el Tecnico", 2.5)
        play_tone(180, 140, "sawtooth", 0.15, 0.4)
        return
    end
    active_cam = idx
    scan_progress = 0.0
    scanning = false
    play_tone(520, 740, "sine", 0.08, 0.35)
end

local function select_best_npc_in_view()
    local tx, _, tz = get_cam_target_point(active_cam)
    local best_idx = selected_npc_idx
    local best_d = 9999.0
    for i, n in ipairs(npcs) do
        if n.state ~= "PRESO" then
            local d = dist2d(n.x, n.z, tx, tz)
            if d < best_d then
                best_d = d
                best_idx = i
            end
        end
    end
    selected_npc_idx = best_idx
end

local function cycle_next_npc()
    local start = selected_npc_idx
    for step = 1, NUM_NPCS do
        local cand = ((start + step - 1) % NUM_NPCS) + 1
        if npcs[cand].state ~= "PRESO" then
            selected_npc_idx = cand
            scan_progress = 0.0
            play_tone(420, 520, "triangle", 0.06, 0.3)
            return
        end
    end
end

local function check_npc_active_infraction(n)
    if n.sprinting then return "LEY I: SPRINT EN VIA PUBLICA" end
    if phase >= 2 and n.crowding_timer >= 5.0 then return "LEY II: AGLOMERACION" end
    if n.has_contraband or n.has_keycard then return "LEY III: CONTRABANDO VISIBLE" end
    if n.state == "SABOTEAR" then return "LEY IV: SABOTAJE INFRAESTRUCTURA" end
    if phase == 3 and is_on_main_road(n.x, n.z) then return "LEY V: TOQUE DE QUEDA" end
    return nil
end

local function trigger_scan_action()
    if game_over or my_role ~= "OPERADOR" then return end
    if active_cam > 0 and not is_cam_operational(active_cam) then
        set_alert("CAMARA FUERA DE LINEA - IMPOSIBLE ESCANEAR", 2.5)
        play_tone(150, 120, "sawtooth", 0.2, 0.5)
        return
    end
    if not scanning then
        if energy < 12.0 then
            set_alert("ENERGIA INSUFICIENTE PARA ESCANER (REQ: 12)", 2.5)
            play_tone(180, 140, "sawtooth", 0.15, 0.4)
            return
        end
        select_best_npc_in_view()
        scanning = true
        scan_progress = 0.0
        play_tone(480, 680, "sine", 0.15, 0.4)
    end
end

local function trigger_police_ping()
    if game_over or my_role ~= "OPERADOR" then return end
    if comms_jam_timer > 0.0 then
        set_alert("COMUNICACIONES CORTADAS - POLICIA SIN ENLACE!", 3.0)
        play_tone(140, 100, "sawtooth", 0.25, 0.5)
        return
    end
    if energy < 5.0 then
        set_alert("ENERGIA INSUFICIENTE PARA PING", 2.0)
        return
    end
    energy = energy - 5.0
    local n = npcs[selected_npc_idx]
    if n and n.state ~= "PRESO" then
        police.ping_x = n.x
        police.ping_z = n.z
        if n.warrant or n.has_contraband then
            police.state = "PERSIGUE"
            police.target_npc = n.id
            set_alert("ORDEN TACTICA: ARRESTAR A CIUDADANO #" .. n.id, 3.0)
        else
            police.state = "PING"
            set_alert("PING TACTICO ENVIADO HACIA CIUDADANO #" .. n.id, 2.5)
        end
        if net_is_server() then
            net_broadcast(1, "EVENT|PING OPERADOR HACIA #" .. n.id)
        end
        play_tone(660, 880, "triangle", 0.18, 0.5)
    end
end

local function trigger_lockdown()
    if game_over or my_role ~= "OPERADOR" then return end
    if lockdown_timer > 0.0 then return end
    if energy < 30.0 then
        set_alert("ENERGIA INSUFICIENTE PARA BLOQUEO REMOTO (REQ: 30)", 2.5)
        play_tone(160, 120, "sawtooth", 0.2, 0.4)
        return
    end
    energy = energy - 30.0
    lockdown_timer = 10.0
    tween_position("sec_door_west", -6.0, 1.6, -16.0, 0.25)
    tween_position("sec_door_east",  6.0, 1.6, -16.0, 0.25)
    set_alert("BLOQUEO REMOTO ACTIVADO (10s) - COMPUERTAS CERRADAS", 4.0)
    play_tone(220, 440, "sawtooth", 0.35, 0.65)
    camera_shake(0.15, 0.25)
end

local function advance_phase_test()
    if game_over or net_is_client() then return end
    if phase == 1 then
        match_time = 479.0; phase = 2
        set_alert("FASE 2: ATARDECER - PROHIBIDA AGLOMERACION (LEY II)", 4.5)
    elseif phase == 2 then
        match_time = 239.0; phase = 3
        set_alert("FASE 3: TOQUE DE QUEDA NOCTURNO - CALLES PROHIBIDAS (LEY V)", 5.0)
    else
        match_time = 45.0
        set_alert("RECTA FINAL DE TURNO: 45s RESTANTES", 3.5)
    end
    play_tone(380, 560, "triangle", 0.22, 0.45)
end

-- Callbacks globales para ui_button
function on_cam_0() switch_camera(0) end
function on_cam_1() switch_camera(1) end
function on_cam_2() switch_camera(2) end
function on_cam_3() switch_camera(3) end
function on_cam_4() switch_camera(4) end
function on_cam_5() switch_camera(5) end
function on_btn_next_target() cycle_next_npc() end
function on_btn_scan() trigger_scan_action() end
function on_btn_ping() trigger_police_ping() end
function on_btn_lockdown() trigger_lockdown() end
function on_btn_phase() advance_phase_test() end
function on_btn_rtt() show_rtt_matrix = not show_rtt_matrix end
function on_btn_reset() reset_panopticon() end

function on_menu_play_solo()
    if net_is_server() or net_is_client() then net_disconnect() end
    my_role = "OPERADOR"
    my_controlled_npc = 0
    game_state = "PLAYING"
    menu_tab = "MAIN"
    reset_panopticon()
end

function on_menu_host()
    if not net_is_server() and not net_is_client() then
        net_host(5000)
    end
    my_role = "OPERADOR"
    my_controlled_npc = 0
    game_state = "PLAYING"
    menu_tab = "MAIN"
    reset_panopticon()
    set_alert("SERVIDOR UDP INICIADO EN PUERTO 5000 (ROL: OPERADOR)", 4.5)
end

function on_menu_join()
    if not net_is_server() and not net_is_client() then
        net_connect("127.0.0.1:5000")
    end
    game_state = "PLAYING"
    menu_tab = "MAIN"
    ui_clear()
    set_alert("CONECTANDO A 127.0.0.1:5000...", 4.0)
end

function on_menu_controls()
    menu_tab = (menu_tab == "MAIN") and "CONTROLS" or "MAIN"
end

function on_btn_to_menu()
    if net_is_server() or net_is_client() then net_disconnect() end
    if is_pointer_locked() then set_pointer_lock(false) end
    my_role = "OPERADOR"
    my_controlled_npc = 0
    keypad_open = false
    game_state = "MENU"
    menu_tab = "MAIN"
    ui_clear()
end

function on_btn_host()
    if not net_is_server() and not net_is_client() then
        net_host(5000)
        my_role = "OPERADOR"
        set_alert("SERVIDOR UDP INICIADO EN PUERTO 5000 (ROL: OPERADOR)", 4.5)
    elseif net_is_server() or net_is_client() then
        net_disconnect()
        my_role = "OPERADOR"
        my_controlled_npc = 0
        set_alert("RED DESCONECTADA - MODO SOLO", 3.0)
    end
end

function on_btn_join()
    if not net_is_server() and not net_is_client() then
        net_connect("127.0.0.1:5000")
        set_alert("CONECTANDO A 127.0.0.1:5000... (PULSA [Salir] SI NO HAY HOST)", 4.0)
    elseif net_is_client() then
        net_disconnect()
        my_role = "OPERADOR"
        my_controlled_npc = 0
        set_alert("DESCONECTADO DEL SERVIDOR - MODO SOLO", 3.0)
    end
end

function on_claim_police()
    if net_is_client() then net_send(1, string.format("CLAIM|%d|POLICIA", net_client_id())) end
end
function on_claim_rebel()
    if net_is_client() then net_send(1, string.format("CLAIM|%d|REBELDE", net_client_id())) end
end
function on_claim_tech()
    if net_is_client() then net_send(1, string.format("CLAIM|%d|TECNICO", net_client_id())) end
end

local function open_cam_room_door(who)
    cam_door_open_timer = 8.0
    cam_door_is_open = true
    tween_position("cam_room_door", 5.4, 6.6, -21.6, 0.35)
    set_material("cam_room_door_led_in",  { color = { 0.2, 0.95, 0.4, 1.0 }, emissive = 0.75 })
    set_material("cam_room_door_led_out", { color = { 0.2, 0.95, 0.4, 1.0 }, emissive = 0.75 })
    set_alert("PUERTA DE SALA ABIERTA (" .. (who or "SISTEMA") .. ") - 8s", 3.5)
    play_tone(640, 960, "sine", 0.22, 0.5)
    if net_is_server() then
        net_broadcast(1, "DOOR|8.0")
        net_broadcast(1, "EVENT|PUERTA DE SALA ABIERTA (" .. (who or "SISTEMA") .. ")")
    end
end

local function close_cam_room_door()
    if net_is_client() then
        net_send(1, "DOOR_CLOSE|1")
        return
    end
    cam_door_open_timer = 0.0
    cam_door_is_open = false
    tween_position("cam_room_door", 5.4, 6.6, -24.1, 0.35)
    set_material("cam_room_door_led_in",  { color = { 0.95, 0.2, 0.2, 1.0 }, emissive = 0.7 })
    set_material("cam_room_door_led_out", { color = { 0.95, 0.2, 0.2, 1.0 }, emissive = 0.7 })
    if net_is_server() then net_broadcast(1, "DOOR|0.0") end
    play_tone(320, 200, "sine", 0.15, 0.4)
end

local function submit_keypad_pin()
    if #keypad_input == 0 then return end
    if net_is_client() then
        net_send(1, string.format("PIN|%d|%s", net_client_id(), keypad_input))
        keypad_status = "ENVIADO..."
        keypad_input = ""
        keypad_open = false
        return
    end
    if keypad_input == DOOR_PIN then
        keypad_status = "ACCESO CONCEDIDO"
        keypad_input = ""
        keypad_open = false
        open_cam_room_door(my_role)
    else
        keypad_status = "PIN INCORRECTO!"
        keypad_input = ""
        play_tone(160, 110, "sawtooth", 0.25, 0.5)
    end
end

local function keypad_append(d)
    if #keypad_input < 4 then
        keypad_input = keypad_input .. d
        play_tone(720, 820, "sine", 0.05, 0.3)
        if #keypad_input == 4 then
            submit_keypad_pin()
        end
    end
end

function on_btn_keypad_toggle()
    keypad_open = not keypad_open
    keypad_input = ""
    keypad_status = "INGRESE PIN (4 DIGITOS)"
    set_pointer_lock(not keypad_open)
end
function on_kp_0() keypad_append("0") end
function on_kp_1() keypad_append("1") end
function on_kp_2() keypad_append("2") end
function on_kp_3() keypad_append("3") end
function on_kp_4() keypad_append("4") end
function on_kp_5() keypad_append("5") end
function on_kp_6() keypad_append("6") end
function on_kp_7() keypad_append("7") end
function on_kp_8() keypad_append("8") end
function on_kp_9() keypad_append("9") end
function on_kp_del()
    if #keypad_input > 0 then
        keypad_input = string.sub(keypad_input, 1, #keypad_input - 1)
    end
end
function on_kp_enter() submit_keypad_pin() end
function on_kp_auto()
    if my_role ~= "REBELDE" then
        keypad_input = DOOR_PIN
        submit_keypad_pin()
    end
end
function on_kp_close_door() close_cam_room_door(); keypad_open = false end

-- ============================================================================
-- FASES, ILUMINACION Y CAMARA
-- ============================================================================
local function update_day_night_cycle()
    local elapsed_match = MATCH_DURATION - match_time
    local new_phase = 1
    local tod = 12.0
    if elapsed_match < 240.0 then
        new_phase = 1
        tod = 12.0 + (elapsed_match / 240.0) * 4.5
    elseif elapsed_match < 480.0 then
        new_phase = 2
        tod = 16.5 + ((elapsed_match - 240.0) / 240.0) * 3.0
    else
        new_phase = 3
        tod = 21.5 + ((elapsed_match - 480.0) / 240.0) * 2.5
    end

    if new_phase ~= phase then
        phase = new_phase
        if phase == 2 then
            update_building_window_pbr(0.65)
            set_alert("FASE 2 (ATARDECER): ACTIVA LEY II - NO AGLOMERARSE", 5.0)
            set_music_intensity(0.50)
            play_tone(330, 495, "triangle", 0.3, 0.5)
        elseif phase == 3 then
            update_building_window_pbr(1.15)
            set_alert("FASE 3 (NOCHE): TOQUE DE QUEDA ACTIVO - LEY V", 5.0)
            set_music_intensity(0.75)
            play_tone(240, 180, "sawtooth", 0.4, 0.6)
        end
    end
    set_time_of_day(tod)

    local p_west = is_sector_powered(1)
    local p_east = is_sector_powered(2)
    local lamp_int = (phase == 3) and 4.2 or ((phase == 2) and 2.8 or 1.4)
    set_light("lamp_nw_bulb", 1.0, 0.86, 0.62, p_west and lamp_int or 0.0, 16.0)
    set_light("lamp_sw_bulb", 1.0, 0.86, 0.62, p_west and lamp_int or 0.0, 16.0)
    set_light("lamp_ne_bulb", 1.0, 0.86, 0.62, p_east and lamp_int or 0.0, 16.0)
    set_light("lamp_se_bulb", 1.0, 0.86, 0.62, p_east and lamp_int or 0.0, 16.0)
end

local function get_tower_floor_y(x, z)
    if x >= -8.6 and x <= 5.4 and z >= -27.6 and z <= -16.4 then
        return 5.2
    end
    if x > 5.4 and x <= 8.6 and z >= -27.6 and z <= -22.8 then
        return 5.2
    end
    if x > 5.4 and x <= 8.6 and z > -22.8 and z <= -16.8 then
        local t = clamp((-16.8 - z) / 6.0, 0.0, 1.0)
        return t * 5.2
    end
    return 0.0
end

local function is_cam_door_nearby()
    if my_role == "OPERADOR" then
        if active_cam ~= 0 then return false end
        local c = cameras[0]
        local floor_y = get_tower_floor_y(c.x, c.z)
        return math.abs(floor_y - 5.2) < 1.8 and dist2d(c.x, c.z, 5.4, -24.1) < 4.5
    else
        local tid = (my_role == "TECNICO") and "tech_root" or ((my_role == "REBELDE" and my_controlled_npc > 0) and ("npc_" .. my_controlled_npc .. "_root") or "police_root")
        local ent = get_entity(tid)
        if ent then
            return math.abs(ent.y - 5.2) < 2.2 and dist2d(ent.x, ent.z, 5.4, -24.1) < 4.5
        end
    end
    return false
end

local GROUND_AABBS = {
    -- Calles y mobiliario urbano
    { -17.04, -16.76,  -6.34,  -6.06 }, -- street_hydrant_1
    {  16.76,  17.04,   6.06,   6.34 }, -- street_hydrant_2
    { -17.04, -16.76,  10.46,  10.74 }, -- street_hydrant_3
    {   3.36,   3.64,  19.46,  19.74 }, -- street_hydrant_4
    { -17.13, -16.67,  -2.03,  -1.57 }, -- street_bin_1
    {  16.67,  17.13,  -2.03,  -1.57 }, -- street_bin_2
    {  -4.23,  -3.77,  19.37,  19.83 }, -- street_bin_3
    {  16.67,  17.13,  15.77,  16.23 }, -- street_bin_4
    {  -1.80,   1.80,  20.02,  20.18 }, -- street_bus_back
    {  -1.40,   1.40,  19.45,  19.95 }, -- street_bus_bench
    -- Cafeteria NW (ent_w1, puerta Este)
    { -26.00, -18.00, -13.00, -12.58 }, { -26.00, -18.00,  -7.42,  -7.00 },
    { -26.00, -25.58, -12.58,  -7.42 }, { -18.42, -18.00, -13.00, -11.20 },
    { -18.42, -18.00,  -8.80,  -7.00 }, { -24.50, -23.50, -11.80,  -8.20 },
    { -21.75, -20.65, -11.85, -10.95 }, { -21.75, -20.65,  -9.05,  -8.15 },
    -- Casas Oeste N y S (ent_w2n / ent_w2s, puertas Este)
    { -33.00, -27.00,  -9.00,  -8.58 }, { -33.00, -27.00,   1.38,   1.80 },
    { -33.00, -32.58,  -8.58,   1.38 }, { -27.42, -27.00,  -9.00,  -4.80 },
    { -27.42, -27.00,  -2.40,   1.80 }, { -32.40, -30.60,  -7.75,  -6.25 },
    { -30.35, -28.45,  -0.82,   0.02 }, { -32.25, -31.55,  -1.20,   0.20 },
    { -33.00, -27.00,   2.20,   2.62 }, { -33.00, -27.00,  12.58,  13.00 },
    { -33.00, -32.58,   2.62,  12.58 }, { -27.42, -27.00,   2.20,   6.40 },
    { -27.42, -27.00,   8.80,  13.00 }, { -32.05, -31.15,   3.40,   5.40 },
    { -30.45, -29.15,  10.10,  11.10 }, { -32.25, -31.55,  10.50,  11.90 },
    -- Casa Taller SW (ent_w3, puerta Norte)
    { -25.50, -22.20,  11.50,  11.92 }, { -19.80, -16.50,  11.50,  11.92 },
    { -25.50, -16.50,  16.08,  16.50 }, { -25.50, -25.08,  11.92,  16.08 },
    { -16.92, -16.50,  11.92,  16.08 }, { -24.70, -22.90,  14.55,  15.45 },
    { -19.10, -17.30,  14.35,  15.25 },
    -- Puesto Policial SE (ent_e1, puerta Oeste)
    {  18.50,  27.50,   8.50,   8.92 }, {  18.50,  27.50,  15.08,  15.50 },
    {  18.50,  18.92,   8.50,  10.80 }, {  18.50,  18.92,  13.20,  15.50 },
    {  27.08,  27.50,   8.92,  15.08 }, {  22.70,  24.50,  11.30,  12.30 },
    {  25.85,  26.55,   9.20,  10.80 }, {  25.85,  26.55,  13.00,  14.60 },
    -- Oficinas / Casa Este N y S (ent_e2n / ent_e2s, puertas Oeste)
    {  27.00,  33.00, -12.00, -11.58 }, {  27.00,  33.00,  -2.62,  -2.20 },
    {  27.00,  27.42, -12.00,  -8.20 }, {  27.00,  27.42,  -5.80,  -2.20 },
    {  32.58,  33.00, -11.58,  -2.62 }, {  30.20,  31.40,  -8.00,  -6.00 },
    {  31.45,  32.15, -11.20,  -9.80 },
    {  27.00,  33.00,  -1.80,  -1.38 }, {  27.00,  33.00,   7.58,   8.00 },
    {  27.00,  27.42,  -1.80,   1.80 }, {  27.00,  27.42,   4.20,   8.00 },
    {  32.58,  33.00,  -1.38,   7.58 }, {  30.55,  32.25,   5.20,   6.80 },
    {  30.95,  31.85,  -0.80,   1.20 },
    -- Residencia Sur-Oeste (ent_s1, puerta Norte)
    { -18.00, -12.20,  21.00,  21.42 }, {  -9.80,  -4.00,  21.00,  21.42 },
    { -18.00,  -4.00,  26.58,  27.00 }, { -18.00, -17.58,  21.42,  26.58 },
    {  -4.42,  -4.00,  21.42,  26.58 }, { -16.40, -14.00,  24.92,  25.88 },
    { -15.85, -14.55,  23.20,  24.00 }, {  -8.40,  -6.80,  22.90,  23.90 },
    {  -6.50,  -4.70,  24.70,  26.30 },
    -- Tienda / Casa Sur-Este (ent_s2, puerta Norte)
    {   4.00,   9.80,  21.00,  21.42 }, {  12.20,  18.00,  21.00,  21.42 },
    {   4.00,  18.00,  26.58,  27.00 }, {   4.00,   4.42,  21.42,  26.58 },
    {  17.58,  18.00,  21.42,  26.58 }, {   5.70,   7.90,  22.80,  23.60 },
    {  14.90,  16.70,  22.48,  23.12 }, {  14.90,  16.70,  24.88,  25.52 },
    {   5.70,   7.90,  25.15,  26.05 },
    -- Mercado y cajas (espaciados con pasillos amplios)
    { -22.8, -19.6,  -1.0,   1.0 }, -- market_stall_1
    { -23.8, -21.4,   5.2,   8.4 }, -- market_stall_2
    { -19.4, -18.2,  -0.4,   0.8 }, -- crate_1
    {  18.4,  19.6,   0.9,   2.1 }, -- crate_2
    -- Subestacion, antenas y fusibles
    {  22.2,  25.8, -21.8, -18.2 }, -- substation_core
    {  18.7,  20.3, -18.8, -17.2 }, -- substation_trans_1
    {  26.7,  28.3, -24.3, -22.7 }, -- substation_trans_2
    {  -8.8,  -7.2,  14.2,  15.8 }, -- antenna_1_base
    { -26.3, -24.7,  10.2,  11.8 }, -- antenna_2_base
    {  23.7,  25.3, -17.3, -15.7 }, -- antenna_3_base
    { -20.05,-18.95, -4.9,  -4.1 }, -- fuse_box_1
    {  18.95, 20.05,  5.1,   5.9 }, -- fuse_box_2
    -- Celda de detencion
    { -26.5, -25.9, -25.5, -18.5 }, -- prison_frame_l
    { -26.5, -19.5, -25.5, -24.9 }, -- prison_frame_b
    -- Bancas y maceteros de arboles
    { -5.45, -3.55,  1.67,  2.33 }, -- bench_1
    {  3.55,  5.45,  1.67,  2.33 }, -- bench_2
    { -0.95,  0.95, -3.53, -2.87 }, -- bench_3
    { -0.95,  0.95,  6.87,  7.53 }, -- bench_4
    { -9.4,  -7.6,  -7.4,  -5.6 },  -- tree_1_box
    {  7.6,   9.4,  -7.4,  -5.6 },  -- tree_2_box
    { -9.4,  -7.6,   9.6,  11.4 },  -- tree_3_box
    {  7.6,   9.4,   9.6,  11.4 },  -- tree_4_box
}

local GROUND_CIRCLES = {
    {  0.0,   2.0,  2.35 }, -- plaza_monument_base
    { -12.0,-10.0,  0.40 }, -- cctv_1_pole
    {  12.0, 13.0,  0.40 }, -- cctv_2_pole
    { -16.5, -4.0,  0.40 }, -- cctv_3_pole
    {  16.5,-11.0,  0.40 }, -- cctv_4_pole
    { -11.5,-10.5,  0.35 }, -- lamp_nw_pole
    {  11.5,-10.5,  0.35 }, -- lamp_ne_pole
    { -11.5, 14.5,  0.35 }, -- lamp_sw_pole
    {  11.5, 14.5,  0.35 }, -- lamp_se_pole
}

local function is_room_pos_blocked(x, z, cur_floor_y)
    -- Limites generales del distrito (incluye interiores de casas Oeste, Este y Sur)
    if x < -32.4 or x > 32.4 or z < -27.0 or z > 26.4 then return true end

    local in_tower_footprint = (x >= -9.4 and x <= 9.4 and z <= -15.6)
    if in_tower_footprint then
        if x < -8.0 or x > 8.1 or z < -27.0 then return true end
        -- Mantener separada la escalera Este de la planta baja y del muro Sur de la sala
        if cur_floor_y > 0.5 and cur_floor_y <= 2.4 and x < 5.65 and z < -16.8 then
            return true
        end
        if cur_floor_y > 2.4 then
            -- Mobiliario superior de la Sala de Control
            if x > -4.25 and x < 4.25 and z > -18.55 and z < -16.8 then return true end
            if x > -1.55 and x < 1.55 and z > -25.65 and z < -23.35 then return true end
            if x < -6.5 and z < -23.2 then return true end
            if x < -6.55 and z > -22.35 and z < -18.65 then return true end -- prop_side_desk
            if x > 2.1 and x < 4.3 and z < -26.0 then return true end
            -- Muro divisor y puerta con contraseña (x = 5.4)
            if x > 5.05 and x < 5.75 then
                local in_doorway = (z >= -25.1 and z <= -23.1)
                if not (in_doorway and cam_door_open_timer > 0.0) then
                    return true
                end
            end
        else
            -- En planta baja: muro frontal central y Servidor Central
            if z > -16.6 and x > -4.0 and x < 1.6 then return true end
            if z > -16.6 and lockdown_timer > 0.0 then return true end
            if x > -1.6 and x < 1.6 and z > -25.8 and z < -23.2 then return true end
        end
    else
        -- Entrando desde Plaza hacia la Torre por planta baja
        if z <= -15.6 and x >= -9.4 and x <= 9.4 then
            if x > -4.0 and x < 1.6 then return true end
            if lockdown_timer > 0.0 then return true end
        end
    end

    -- Colisiones fisicas del entorno exterior (edificios, subestacion, puestos, bancas, arboles)
    if cur_floor_y <= 2.4 then
        local r = 0.45
        for _, b in ipairs(GROUND_AABBS) do
            if x > (b[1] - r) and x < (b[2] + r) and z > (b[3] - r) and z < (b[4] + r) then
                return true
            end
        end
        for _, c in ipairs(GROUND_CIRCLES) do
            local dx = x - c[1]
            local dz = z - c[2]
            if (dx * dx + dz * dz) < ((c[3] + r) * (c[3] + r)) then
                return true
            end
        end
    end
    return false
end

local function handle_keypad_hotkeys()
    if is_key_just_pressed("G") then
        if is_cam_door_nearby() or (my_role == "OPERADOR" and active_cam == 0) then
            on_btn_keypad_toggle()
        end
    end
    if not keypad_open then return false end
    for d = 0, 9 do
        local s = tostring(d)
        if key_pressed_any(s, "DIGIT" .. s) then
            keypad_append(s)
        end
    end
    if is_key_just_pressed("BACKSPACE") then on_kp_del() end
    if is_key_just_pressed("RETURN") or is_key_just_pressed("ENTER") then submit_keypad_pin() end
    if is_key_just_pressed("ESCAPE") then keypad_open = false end
    return true
end

local function apply_mouse_look(yaw, pitch, min_p, max_p)
    if keypad_open or game_over then return yaw, pitch end
    local md = get_mouse_delta()
    if md and (math.abs(md.dx) > 0.001 or math.abs(md.dy) > 0.001) then
        local dx = clamp(md.dx, -60.0, 60.0)
        local dy = clamp(md.dy, -60.0, 60.0)
        local sens = is_pointer_locked() and 0.0026 or 0.0032
        yaw = yaw - dx * sens
        pitch = clamp(pitch - dy * sens, min_p, max_p)
    end
    return yaw, pitch
end

local op_vel_y = 0.0
local op_y_off = 0.0

local function update_operator_camera(dt)
    if is_key_just_pressed("F8") then on_btn_host() end
    if is_key_just_pressed("F9") then on_btn_join() end
    if is_key_just_pressed("M") then show_rtt_matrix = not show_rtt_matrix end
    if active_cam == 0 and is_key_just_pressed("L") then
        set_pointer_lock(not is_pointer_locked())
    end

    local typing_pin = handle_keypad_hotkeys()

    if not typing_pin then
        if key_pressed_any("DIGIT0", "0") then switch_camera(0) end
        if key_pressed_any("1", "DIGIT1") then switch_camera(1) end
        if key_pressed_any("2", "DIGIT2") then switch_camera(2) end
        if key_pressed_any("3", "DIGIT3") then switch_camera(3) end
        if key_pressed_any("4", "DIGIT4") then switch_camera(4) end
        if key_pressed_any("5", "DIGIT5") then switch_camera(5) end
    end

    if is_key_just_pressed("TAB") then cycle_next_npc() end
    if active_cam > 0 then
        if is_key_just_pressed("SPACE") or is_key_just_pressed("C") then trigger_scan_action() end
    else
        if is_key_just_pressed("C") then trigger_scan_action() end
    end
    if is_key_just_pressed("P") then trigger_police_ping() end
    if is_key_just_pressed("B") then trigger_lockdown() end
    if is_key_just_pressed("F") then advance_phase_test() end
    if is_key_just_pressed("R") then reset_panopticon() end

    local c = cameras[active_cam]
    if active_cam == 0 then
        -- Movimiento libre en 1ra persona: correr (SHIFT), saltar (ESPACIO) y explorar edificios
        c.yaw, c.pitch = apply_mouse_look(c.yaw, c.pitch, -1.25, 0.95)

        if not typing_pin and is_key_just_pressed("SPACE") and op_y_off <= 0.02 then
            op_vel_y = 6.4
            play_tone(240, 380, "sine", 0.08, 0.25)
        end
        op_vel_y = op_vel_y - 18.0 * dt
        op_y_off = math.max(0.0, op_y_off + op_vel_y * dt)
        if op_y_off == 0.0 then op_vel_y = 0.0 end

        local yaw_in = 0.0
        if is_key_down("LEFT") then yaw_in = yaw_in + 1.0 end
        if is_key_down("RIGHT") then yaw_in = yaw_in - 1.0 end
        local pitch_in = 0.0
        if is_key_down("UP") then pitch_in = pitch_in + 1.0 end
        if is_key_down("DOWN") then pitch_in = pitch_in - 1.0 end
        c.yaw = c.yaw + yaw_in * 1.8 * dt
        c.pitch = clamp(c.pitch + pitch_in * 1.2 * dt, -1.25, 0.95)

        local fwd_x = math.sin(c.yaw)
        local fwd_z = math.cos(c.yaw)
        local rgt_x = -math.cos(c.yaw)
        local rgt_z = math.sin(c.yaw)

        local mx, mz = 0.0, 0.0
        if is_key_down("W") then mx = mx + fwd_x; mz = mz + fwd_z end
        if is_key_down("S") then mx = mx - fwd_x; mz = mz - fwd_z end
        if is_key_down("D") then mx = mx + rgt_x; mz = mz + rgt_z end
        if is_key_down("A") then mx = mx - rgt_x; mz = mz - rgt_z end

        local cur_floor = get_tower_floor_y(c.x, c.z)
        local mag = math.sqrt(mx * mx + mz * mz)
        local sprinting = is_key_down("SHIFT")
        if mag > 0.01 then
            local spd = sprinting and 8.4 or 4.6
            local dx = (mx / mag) * spd * dt
            local dz = (mz / mag) * spd * dt
            if not is_room_pos_blocked(c.x + dx, c.z, cur_floor) then c.x = c.x + dx end
            cur_floor = get_tower_floor_y(c.x, c.z)
            if not is_room_pos_blocked(c.x, c.z + dz, cur_floor) then c.z = c.z + dz end
            local bob = (op_y_off <= 0.01) and (math.sin(op_walk_phase) * 0.04) or 0.0
            op_walk_phase = op_walk_phase + dt * (sprinting and 13.0 or 8.0)
            local target_y = get_tower_floor_y(c.x, c.z) + 1.65 + op_y_off + bob
            c.y = c.y + (target_y - c.y) * math.min(1.0, dt * 16.0)
        else
            local target_y = get_tower_floor_y(c.x, c.z) + 1.65 + op_y_off
            c.y = c.y + (target_y - c.y) * math.min(1.0, dt * 16.0)
        end
    else
        local yaw_in = 0.0
        if is_key_down("A") or is_key_down("LEFT") then yaw_in = yaw_in + 1.0 end
        if is_key_down("D") or is_key_down("RIGHT") then yaw_in = yaw_in - 1.0 end
        local pitch_in = 0.0
        if is_key_down("W") or is_key_down("UP") then pitch_in = pitch_in + 1.0 end
        if is_key_down("S") or is_key_down("DOWN") then pitch_in = pitch_in - 1.0 end
        local zoom_in = 0.0
        if is_key_down("E") then zoom_in = zoom_in + 1.0 end
        if is_key_down("Q") then zoom_in = zoom_in - 1.0 end

        if is_cam_operational(active_cam) then
            c.yaw = clamp(c.yaw + yaw_in * 1.15 * dt, c.yaw_min, c.yaw_max)
            c.pitch = clamp(c.pitch + pitch_in * 0.65 * dt, -0.95, -0.08)
            c.zoom = clamp(c.zoom + zoom_in * 0.9 * dt, 0.0, 1.0)
        end
    end

    local ex, ey, ez = get_cam_eye_pos(active_cam)
    local tx, ty, tz = get_cam_target_point(active_cam)

    if active_cam > 0 and not is_cam_operational(active_cam) then
        set_camera(ex, ey, ez, ex, ey - 6.0, ez + 0.5)
    else
        set_camera(ex, ey, ez, tx, ty, tz)
    end

    for i = 1, 5 do
        local cam = cameras[i]
        if i < 5 or cam_micro_installed then
            if cam.blind_timer > 0.0 then
                set_material(cam.head_id, { color = { 0.95, 0.18, 0.22, 1.0 }, emissive = 0.65 })
            elseif cam.sector > 0 and not is_sector_powered(cam.sector) then
                set_material(cam.head_id, { color = { 0.25, 0.25, 0.28, 1.0 }, emissive = 0.05 })
            elseif cam.loop_timer > 0.0 then
                set_material(cam.head_id, { color = { 0.95, 0.78, 0.15, 1.0 }, emissive = 0.6 })
            elseif i == active_cam then
                set_material(cam.head_id, { color = { 0.2, 0.95, 1.0, 1.0 }, emissive = 0.7 })
            else
                set_material(cam.head_id, { color = { 0.15, 0.9, 0.45, 1.0 }, emissive = 0.45 })
            end
            if is_cam_operational(i) then
                local ctx, _, ctz = get_cam_target_point(i)
                draw_line_3d(cam.x, cam.y, cam.z, ctx, 0.2, ctz, 0.2, 0.85, 0.55, 0.35)
            end
        end
    end
end

-- ============================================================================
-- MOVIMIENTO E IA (AUTORITATIVA EN SERVIDOR / SOLO)
-- ============================================================================
local rig_steer_mem = {}
local nav_baked_cells = nil
local NAV_X0, NAV_X1, NAV_Z0, NAV_Z1 = -26, 26, -26, 20

local function ensure_nav_grid()
    if nav_baked_cells then return end
    nav_baked_cells = {}
    for gx = NAV_X0, NAV_X1 do
        nav_baked_cells[gx] = {}
        for gz = NAV_Z0, NAV_Z1 do
            local blk = is_room_pos_blocked(gx, gz, 0.0)
            nav_baked_cells[gx][gz] = blk
        end
    end
end

local function is_line_clear_2d(x1, z1, x2, z2, fy)
    local d = dist2d(x1, z1, x2, z2)
    local steps = math.max(1, math.floor(d / 0.55))
    for i = 1, steps do
        local t = i / steps
        local px = x1 + (x2 - x1) * t
        local pz = z1 + (z2 - z1) * t
        if is_room_pos_blocked(px, pz, fy) then
            return false
        end
    end
    return true
end

local function astar_path_2d(sx, sz, gx, gz)
    ensure_nav_grid()
    local x0 = clamp(math.floor(sx + 0.5), NAV_X0, NAV_X1)
    local z0 = clamp(math.floor(sz + 0.5), NAV_Z0, NAV_Z1)
    local x1 = clamp(math.floor(gx + 0.5), NAV_X0, NAV_X1)
    local z1 = clamp(math.floor(gz + 0.5), NAV_Z0, NAV_Z1)

    local function find_free(cx, cz)
        if not nav_baked_cells[cx][cz] then return cx, cz end
        for rad = 1, 3 do
            for ox = -rad, rad do
                for oz = -rad, rad do
                    local nx, nz = clamp(cx + ox, NAV_X0, NAV_X1), clamp(cz + oz, NAV_Z0, NAV_Z1)
                    if not nav_baked_cells[nx][nz] then return nx, nz end
                end
            end
        end
        return cx, cz
    end
    x0, z0 = find_free(x0, z0)
    x1, z1 = find_free(x1, z1)

    local function key(x, z) return (x - NAV_X0) + (z - NAV_Z0) * 64 end
    local start_k, goal_k = key(x0, z0), key(x1, z1)
    if start_k == goal_k then return nil end

    local open = { { x = x0, z = z0, k = start_k, g = 0, f = dist2d(x0, z0, x1, z1) } }
    local gscore = { [start_k] = 0 }
    local parent = {}
    local closed = {}
    local dirs = { {1,0,1},{-1,0,1},{0,1,1},{0,-1,1},{1,1,1.41},{-1,1,1.41},{1,-1,1.41},{-1,-1,1.41} }

    local iters = 0
    while #open > 0 and iters < 360 do
        iters = iters + 1
        local bi = 1
        for i = 2, #open do if open[i].f < open[bi].f then bi = i end end
        local cur = table.remove(open, bi)
        if cur.k == goal_k then
            local rev = {}
            local ck = goal_k
            local kx, kz = x1, z1
            while ck and ck ~= start_k do
                table.insert(rev, { x = kx, z = kz })
                local p = parent[ck]
                if not p then break end
                ck, kx, kz = p.k, p.x, p.z
            end
            local path = {}
            for i = #rev, 1, -1 do table.insert(path, rev[i]) end
            return path
        end
        closed[cur.k] = true
        for _, dr in ipairs(dirs) do
            local nx, nz = cur.x + dr[1], cur.z + dr[2]
            if nx >= NAV_X0 and nx <= NAV_X1 and nz >= NAV_Z0 and nz <= NAV_Z1 and not nav_baked_cells[nx][nz] then
                if dr[1] == 0 or dr[2] == 0 or (not nav_baked_cells[cur.x + dr[1]][cur.z] and not nav_baked_cells[cur.x][cur.z + dr[2]]) then
                    local nk = key(nx, nz)
                    if not closed[nk] then
                        local ng = cur.g + dr[3]
                        if not gscore[nk] or ng < gscore[nk] then
                            gscore[nk] = ng
                            parent[nk] = { k = cur.k, x = cur.x, z = cur.z }
                            table.insert(open, { x = nx, z = nz, k = nk, g = ng, f = ng + dist2d(nx, nz, x1, z1) })
                        end
                    end
                end
            end
        end
    end
    return nil
end

local function steer_rig_2d(root_id, cur_x, cur_z, tx, tz, spd, dt)
    local d = dist2d(cur_x, cur_z, tx, tz)
    if d < 0.35 then
        rig_steer_mem[root_id] = nil
        return cur_x, cur_z, true
    end
    local ty = get_tower_floor_y(tx, tz)
    if d < 2.10 and is_room_pos_blocked(tx, tz, ty) then
        rig_steer_mem[root_id] = nil
        return cur_x, cur_z, true
    end

    local mem = rig_steer_mem[root_id]
    if not mem then
        mem = { path = nil, path_idx = 1, gx = tx, gz = tz, repath_t = 0.0, stuck_t = 0.0, last_x = cur_x, last_z = cur_z, prog_t = 0.0 }
        rig_steer_mem[root_id] = mem
    end
    mem.repath_t = math.max(0.0, mem.repath_t - dt)
    mem.prog_t = mem.prog_t + dt
    if mem.prog_t >= 0.70 then
        local moved = dist2d(cur_x, cur_z, mem.last_x, mem.last_z)
        mem.last_x, mem.last_z = cur_x, cur_z
        mem.prog_t = 0.0
        if moved < 0.28 then
            mem.stuck_t = mem.stuck_t + 0.70
            mem.path = nil
            mem.repath_t = 0.0
            if mem.stuck_t >= 2.1 then
                mem.stuck_t = 0.0
                return cur_x, cur_z, true
            end
        else
            mem.stuck_t = 0.0
        end
    end

    local cur_floor = get_tower_floor_y(cur_x, cur_z)
    if is_room_pos_blocked(cur_x, cur_z, cur_floor) then
        for a = 0, 7 do
            local ang = a * (math.pi * 0.25)
            local ux = cur_x + math.cos(ang) * 0.70
            local uz = cur_z + math.sin(ang) * 0.70
            if not is_room_pos_blocked(ux, uz, cur_floor) then
                tween_position(root_id, ux, get_tower_floor_y(ux, uz), uz, 0.0)
                return ux, uz, false
            end
        end
    end

    local sub_x, sub_y, sub_z = tx, ty, tz
    if cur_floor <= 1.0 and ty <= 1.0 then
        if not is_line_clear_2d(cur_x, cur_z, tx, tz, 0.0) then
            if not mem.path or mem.repath_t <= 0.0 or dist2d(mem.gx, mem.gz, tx, tz) > 1.5 then
                mem.path = astar_path_2d(cur_x, cur_z, tx, tz)
                mem.path_idx = 1
                mem.gx, mem.gz = tx, tz
                mem.repath_t = 1.6
            end
            if mem.path and mem.path_idx <= #mem.path then
                while mem.path_idx < #mem.path and dist2d(cur_x, cur_z, mem.path[mem.path_idx].x, mem.path[mem.path_idx].z) < 0.85 do
                    mem.path_idx = mem.path_idx + 1
                end
                local node = mem.path[mem.path_idx]
                sub_x, sub_y, sub_z = node.x, 0.0, node.z
            end
        else
            mem.path = nil
        end
    end

    local arrived = steer_towards(root_id, sub_x, sub_y, sub_z, spd, 9.0, dt)
    local ent = get_entity(root_id)
    if ent then
        if is_room_pos_blocked(ent.x, ent.z, get_tower_floor_y(ent.x, ent.z)) then
            tween_position(root_id, cur_x, cur_floor, cur_z, 0.0)
            mem.path = nil
            mem.repath_t = 0.0
            return cur_x, cur_z, false
        end
        if sub_x == tx and sub_z == tz then
            return ent.x, ent.z, (arrived and dist2d(ent.x, ent.z, tx, tz) < 0.45)
        end
        return ent.x, ent.z, false
    end
    return cur_x, cur_z, false
end

local human_jump = {}

local function apply_human_move(root_id, x, z, mx, mz, spd, dt, check_tower_doors, human_p)
    local st = human_jump[root_id]
    if not st then st = { y_off = 0.0, vy = 0.0 }; human_jump[root_id] = st end
    if human_p and human_p.jump_queued then
        human_p.jump_queued = false
        if st.y_off <= 0.02 then st.vy = 6.4 end
    end
    st.vy = st.vy - 18.0 * dt
    st.y_off = math.max(0.0, st.y_off + st.vy * dt)
    if st.y_off == 0.0 then st.vy = 0.0 end

    local mag = math.sqrt(mx * mx + mz * mz)
    if mag < 0.01 then
        if st.y_off > 0.001 then
            tween_position(root_id, x, get_tower_floor_y(x, z) + st.y_off, z, 0.0)
        end
        return x, z
    end
    local dx = mx / mag
    local dz = mz / mag
    local step_x = dx * spd * dt
    local step_z = dz * spd * dt
    local sx, sz = 0.0, 0.0
    local cur_floor = get_tower_floor_y(x, z)
    if not is_room_pos_blocked(x + step_x, z, cur_floor) then
        sx = dx
    end
    cur_floor = get_tower_floor_y(x + sx * spd * dt, z)
    if not is_room_pos_blocked(x + sx * spd * dt, z + step_z, cur_floor) then
        if not (check_tower_doors and lockdown_timer > 0.0 and z > -16.5 and (z + step_z) <= -16.5) then
            sz = dz
        end
    end
    local smag = math.sqrt(sx * sx + sz * sz)
    if smag < 0.01 then
        if st.y_off > 0.001 then
            tween_position(root_id, x, get_tower_floor_y(x, z) + st.y_off, z, 0.0)
        end
        return x, z
    end
    local lx = x + (sx / smag) * 1.4
    local lz = z + (sz / smag) * 1.4
    local ly = get_tower_floor_y(x + sx * spd * dt, z + sz * spd * dt) + st.y_off
    steer_towards(root_id, lx, ly, lz, spd, 12.0, dt)
    local ent = get_entity(root_id)
    if ent then return ent.x, ent.z end
    return x + sx * spd * dt, z + sz * spd * dt
end

local function handle_human_rebel_action(n)
    -- Rescatar aliado esposado
    for _, ally in ipairs(npcs) do
        if ally.id ~= n.id and ally.state == "ESPOSADO" and dist2d(n.x, n.z, ally.x, ally.z) < 2.5 then
            ally.state = "CAMINAR"; ally.warrant = false
            set_alert("RESCATE HUMANO! #" .. ally.id .. " LIBERADO!", 3.5)
            return
        end
    end
    -- Hackear Servidor Central
    if dist2d(n.x, n.z, 0.0, -23.5) < 3.2 then
        server_hack_progress = server_hack_progress + 3.0
        set_alert("REBELDE HUMANO HACKEANDO SERVIDOR CENTRAL!", 2.5)
        return
    end
    -- Hackear Antena cercana
    for i, ant in ipairs(antennas) do
        if not ant.hacked and dist2d(n.x, n.z, ant.x, ant.z) < 3.0 then
            ant.hacked = true
            set_material(ant.tip, { color = { 1.0, 0.2, 0.2, 1.0 }, emissive = 0.85 })
            set_light(ant.tip, 1.0, 0.2, 0.2, 3.2, 14.0)
            set_alert("ANTENA #" .. i .. " HACKEADA POR REBELDE!", 3.5)
            return
        end
    end
    -- Sabotear caja de fusibles
    for _, fb in ipairs(fuse_boxes) do
        if fb.down_timer <= 0.0 and dist2d(n.x, n.z, fb.x, fb.z) < 2.8 then
            fb.down_timer = 25.0
            comms_jam_timer = 12.0
            set_material(fb.id, { color = { 0.25, 0.25, 0.28, 1.0 }, emissive = 0.0 })
            set_alert("SABOTAJE ELECTRICO EN " .. fb.name .. "!", 4.0)
            return
        end
    end
    -- Cegar camara cercana
    for i = 1, 5 do
        local c = cameras[i]
        if (i < 5 or cam_micro_installed) and c.blind_timer <= 0.0 and dist2d(n.x, n.z, c.x, c.z) < 4.0 then
            c.blind_timer = 45.0
            set_alert(c.name .. " CEGADA POR REBELDE!", 3.5)
            return
        end
    end
end

local function update_npcs(dt)
    for i = 1, NUM_NPCS do
        local a = npcs[i]
        if a.state ~= "PRESO" and a.state ~= "ESPOSADO" then
            local near_count = 1
            for j = 1, NUM_NPCS do
                if i ~= j and npcs[j].state ~= "PRESO" then
                    if dist2d(a.x, a.z, npcs[j].x, npcs[j].z) < 4.0 then
                        near_count = near_count + 1
                    end
                end
            end
            if phase >= 2 and near_count >= 3 then
                a.crowding_timer = a.crowding_timer + dt
            else
                a.crowding_timer = math.max(0.0, a.crowding_timer - dt * 0.6)
            end
        end
    end

    for i = 1, NUM_NPCS do
        local n = npcs[i]
        n.anim = n.anim + dt

        -- Chequear si un jugador humano controla este NPC (Rol REBELDE)
        local human_p = nil
        for _, p in pairs(net_players) do
            if p.role == "REBELDE" and p.npc_id == n.id then
                human_p = p; break
            end
        end

        if n.state == "PRESO" then
            local px = -24.0 + ((i % 3) * 1.3)
            local pz = -22.5 + math.floor(i / 4) * 1.2
            n.x, n.z = px, pz
            tween_position(n.root, px, 0.0, pz, 0.0)
            tween_scale(n.item, 0.001, 0.001, 0.001, 0.0)
            set_world_label(n.head, "DETENIDO", 0.55, 0.5, 0.7, 1.0)
        elseif n.state == "ESPOSADO" then
            if n.false_arrest_timer > 0.0 then
                n.false_arrest_timer = n.false_arrest_timer - dt
                if n.false_arrest_timer <= 0.0 then
                    n.state = "CAMINAR"; n.warrant = false; n.scanned = false
                    set_alert("CIUDADANO #" .. n.id .. " LIBERADO (FALSA DENUNCIA)", 3.5)
                end
            else
                for _, ally in ipairs(npcs) do
                    if ally.id ~= n.id and ally.is_rebel and ally.state == "CAMINAR" then
                        if dist2d(ally.x, ally.z, n.x, n.z) < 2.2 then
                            n.state = "CAMINAR"; n.warrant = false
                            if police.target_npc == n.id then
                                police.state = "PATRULLA"; police.target_npc = nil
                            end
                            spawn_vfx("hit_spark", n.x, 1.2, n.z)
                            set_alert("RESCATE REBELDE! #" .. n.id .. " LIBERADO!", 3.5)
                            play_tone(300, 180, "sawtooth", 0.25, 0.5)
                            break
                        end
                    end
                end
            end
            tween_position(n.root, n.x, 0.0, n.z, 0.0)
            set_world_label(n.head, "ESPOSADO", 0.55, 1.0, 0.6, 0.2)
        elseif human_p then
            -- Control humano directo desde cliente Renet!
            n.sprinting = human_p.sprint
            local spd = n.sprinting and 7.8 or 3.6
            n.x, n.z = apply_human_move(n.root, n.x, n.z, human_p.mx, human_p.mz, spd, dt, true, human_p)
            if human_p.action_queued then
                human_p.action_queued = false
                handle_human_rebel_action(n)
            end
            set_world_label(n.head, n.warrant and "ORDEN DE ARRESTO" or "", 0.55, 1.0, 0.25, 0.25)
        else
            -- IA autonoma
            if n.is_rebel and n.state == "CAMINAR" then
                n.cooldown = n.cooldown - dt
                if n.cooldown <= 0.0 then
                    local watched = is_point_watched_by_any_cam(n.x, n.z)
                    local roll = math.random()
                    if phase >= 2 and not n.has_keycard and roll < 0.25 then
                        for k_i, kc in ipairs(keycards) do
                            if not kc.stolen then
                                n.state = "SABOTEAR"; n.task_kind = "keycard"; n.task_idx = k_i
                                n.has_contraband = true; break
                            end
                        end
                    elseif n.has_keycard and phase >= 2 then
                        n.state = "SABOTEAR"; n.task_kind = "server"; n.task_idx = 1
                        n.sprinting = not watched
                    elseif phase >= 2 and roll < 0.55 then
                        for a_i, ant in ipairs(antennas) do
                            if not ant.hacked then
                                n.state = "SABOTEAR"; n.task_kind = "antenna"; n.task_idx = a_i
                                n.has_contraband = true; break
                            end
                        end
                    elseif (not watched or roll < 0.75) then
                        if roll < 0.45 then
                            local cam_pick = math.random(1, 4)
                            if cameras[cam_pick].blind_timer <= 0.0 then
                                n.state = "SABOTEAR"; n.task_kind = "cam"; n.task_idx = cam_pick
                                n.has_contraband = true
                            end
                        else
                            local fb_pick = math.random(1, 2)
                            if fuse_boxes[fb_pick].down_timer <= 0.0 then
                                n.state = "SABOTEAR"; n.task_kind = "fuse"; n.task_idx = fb_pick
                                n.has_contraband = true
                            end
                        end
                    end
                    if n.state == "CAMINAR" then
                        n.sprinting = (not watched) and (math.random() < 0.45)
                        n.cooldown = 4.0 + math.random() * 5.0
                    end
                end
            end

            if n.state == "SABOTEAR" then
                local tx, tz = 0.0, 0.0
                if n.task_kind == "cam" then local c = cameras[n.task_idx]; tx, tz = c.x, c.z
                elseif n.task_kind == "fuse" then local fb = fuse_boxes[n.task_idx]; tx, tz = fb.x, fb.z
                elseif n.task_kind == "antenna" then local ant = antennas[n.task_idx]; tx, tz = ant.x, ant.z
                elseif n.task_kind == "keycard" then local kc = keycards[n.task_idx]; tx, tz = kc.x, kc.z
                elseif n.task_kind == "server" then tx, tz = 0.0, -23.5 end

                local spd = n.sprinting and 5.6 or 2.8
                if n.task_kind == "server" and lockdown_timer > 0.0 and n.z > -16.5 and tz < -16.5 then
                    spd = 0.0; n.task_timer = 0.0
                end

                local nx, nz, arrived = steer_rig_2d(n.root, n.x, n.z, tx, tz, spd, dt)
                n.x, n.z = nx, nz

                if arrived then
                    n.task_timer = n.task_timer + dt
                    if n.task_kind == "cam" and n.task_timer >= 2.2 then
                        local c = cameras[n.task_idx]
                        if math.random() < 0.65 then
                            c.blind_timer = 45.0
                            set_alert("ALERTA: " .. c.name .. " CEGADA CON SPRAY (45s)!", 4.0)
                        else
                            c.loop_timer = 20.0
                            set_alert("ALERTA: LOOP FALSO EN " .. c.name .. " (20s)!", 4.0)
                        end
                        spawn_vfx("hit_spark", c.x, 1.5, c.z)
                        play_tone(190, 120, "sawtooth", 0.3, 0.5)
                        n.state = "CAMINAR"; n.task_timer = 0.0; n.cooldown = 10.0
                        n.has_contraband = false; n.sprinting = false
                    elseif n.task_kind == "fuse" and n.task_timer >= 2.5 then
                        local fb = fuse_boxes[n.task_idx]
                        fb.down_timer = 25.0; comms_jam_timer = 12.0
                        set_material(fb.id, { color = { 0.25, 0.25, 0.28, 1.0 }, emissive = 0.0 })
                        spawn_vfx("explosion", fb.x, 1.2, fb.z)
                        set_alert("SABOTAJE ELECTRICO: " .. fb.name .. " SIN ENERGIA (25s)!", 4.5)
                        play_tone(160, 80, "sawtooth", 0.4, 0.6)
                        n.state = "CAMINAR"; n.task_timer = 0.0; n.cooldown = 12.0
                        n.has_contraband = false
                    elseif n.task_kind == "keycard" and n.task_timer >= 1.0 then
                        local kc = keycards[n.task_idx]; kc.stolen = true; n.has_keycard = true
                        tween_position(kc.id, kc.x, -10.0, kc.z, 0.0)
                        set_alert("TARJETA DE ACCESO ROBADA!", 3.5)
                        n.state = "CAMINAR"; n.task_timer = 0.0; n.cooldown = 5.0
                    elseif n.task_kind == "antenna" then
                        local ant = antennas[n.task_idx]
                        ant.hack_prog = ant.hack_prog + dt
                        if ant.hack_prog >= 4.0 then
                            ant.hacked = true
                            set_material(ant.tip, { color = { 1.0, 0.2, 0.2, 1.0 }, emissive = 0.85 })
                            set_light(ant.tip, 1.0, 0.2, 0.2, 3.2, 14.0)
                            spawn_vfx("magic_portal", ant.x, 6.2, ant.z)
                            set_alert("ANTENA #" .. n.task_idx .. " HACKEADA! ENVIA AL POLICIA!", 4.0)
                            play_tone(550, 880, "square", 0.3, 0.55)
                            n.state = "CAMINAR"; n.task_timer = 0.0; n.cooldown = 10.0
                            n.has_contraband = false
                        end
                    elseif n.task_kind == "server" then
                        server_hack_progress = server_hack_progress + dt
                        set_alert("INTRUSION EN TORRE! SERVIDOR SIENDO HACKEADO!", 1.5)
                        if server_hack_progress >= SERVER_HACK_GOAL then
                            game_over = true; winner = "REBELDES"
                            win_reason = "APAGON TOTAL: El Servidor Central fue hackeado."
                        end
                    end
                end
            else
                local wp = plaza_wps[n.wp_idx]
                local spd = n.sprinting and 5.5 or n.speed
                local nx, nz, arrived = steer_rig_2d(n.root, n.x, n.z, wp.x, wp.z, spd, dt)
                n.x, n.z = nx, nz
                if arrived then
                    local next_wp = math.random(1, #plaza_wps)
                    if next_wp == n.wp_idx then next_wp = (n.wp_idx % #plaza_wps) + 1 end
                    n.wp_idx = next_wp
                    n.sprinting = false
                    if phase == 3 and not n.is_rebel then
                        n.wp_idx = math.random(1, 6)
                    end
                end
            end

            if n.has_contraband or n.has_keycard then
                local col = n.has_keycard and { 1.0, 0.85, 0.15, 1.0 } or { 1.0, 0.25, 0.25, 1.0 }
                set_material(n.item, { color = col, emissive = 0.6 })
                tween_scale(n.item, 0.12, 0.22, 0.12, 0.0)
            else
                tween_scale(n.item, 0.001, 0.001, 0.001, 0.0)
            end

            local inf = check_npc_active_infraction(n)
            if n.warrant then
                draw_ring_3d(n.x, 0.15, n.z, 1.05, 1.0, 0.2, 0.2, 0.9)
                set_world_label(n.head, "ORDEN DE ARRESTO", 0.55, 1.0, 0.25, 0.25)
            elseif selected_npc_idx == n.id then
                draw_ring_3d(n.x, 0.15, n.z, 0.95, 0.2, 0.9, 1.0, 0.85)
                if scanning then
                    local pct = math.floor((scan_progress / 3.0) * 100)
                    set_world_label(n.head, "ESCANEANDO " .. pct .. "%", 0.60, 0.2, 0.95, 1.0)
                elseif inf then
                    set_world_label(n.head, "OBJETIVO: " .. inf, 0.55, 1.0, 0.78, 0.2)
                else
                    set_world_label(n.head, "OBJETIVO #" .. n.id .. " (SIN INFRACCION)", 0.55, 0.4, 0.9, 1.0)
                end
            elseif n.scanned and inf then
                set_world_label(n.head, inf, 0.55, 1.0, 0.6, 0.2)
            else
                set_world_label(n.head, "", 0.55, 1.0, 1.0, 1.0)
            end
        end
    end
end

local function arrest_npc_by_police(target)
    local inf = check_npc_active_infraction(target)
    local guilty = target.is_rebel or (inf ~= nil) or target.has_contraband
    target.state = "ESPOSADO"
    target.sprinting = false
    if not guilty then
        target.false_arrest_timer = 5.0
        energy = math.max(0.0, energy - 25.0)
        set_alert("FALSA DENUNCIA sobre #" .. target.id .. "! -25 Energia, libre en 5s", 4.5)
        play_tone(170, 110, "sawtooth", 0.35, 0.6)
        police.state = "PATRULLA"; police.target_npc = nil
    else
        target.false_arrest_timer = 0.0
        police.state = "ESCOLTA"
        police.target_npc = target.id
        set_alert("SOSPECHOSO #" .. target.id .. " ESPOSADO! Escoltando a celda...", 3.5)
        spawn_vfx("hit_spark", target.x, 1.4, target.z)
        play_tone(520, 780, "sine", 0.25, 0.5)
    end
end

local room_patrol_wps = {
    { x = 7.0, z = -15.5, need_door = false },
    { x = 7.0, z = -24.1, need_door = false },
    { x = 5.9, z = -24.1, need_door = true },
    { x = 2.8, z = -22.2, need_door = false },
    { x = 5.9, z = -24.1, need_door = true },
    { x = 7.0, z = -24.1, need_door = false },
    { x = 7.0, z = -15.5, need_door = false }
}
local police_room_step = 0
local police_room_cd = 18.0
local tech_room_step = 0
local tech_room_cd = 10.0

local function step_room_visit(root_id, x, z, step_idx, who, spd, dt)
    local wp = room_patrol_wps[step_idx]
    if not wp then return x, z, 0 end
    if wp.need_door and dist2d(x, z, wp.x, wp.z) < 2.2 and cam_door_open_timer <= 0.2 then
        open_cam_room_door(who)
    end
    local nx, nz, arrived = steer_rig_2d(root_id, x, z, wp.x, wp.z, spd, dt)
    if arrived then
        step_idx = step_idx + 1
        if step_idx > #room_patrol_wps then
            step_idx = 0
        end
    end
    return nx, nz, step_idx
end

local function update_police(dt)
    police.anim = police.anim + dt
    local _, human_pol = find_human_for_role("POLICIA")

    if human_pol then
        local spd = human_pol.sprint and police.speed_sprint or 4.5
        police.x, police.z = apply_human_move("police_root", police.x, police.z, human_pol.mx, human_pol.mz, spd, dt, false, human_pol)
        if police.state == "ESCOLTA" and police.target_npc then
            local t = npcs[police.target_npc]
            if t and t.state == "ESPOSADO" then
                t.x = police.x - 0.8; t.z = police.z + 0.4
                if dist2d(police.x, police.z, -23.0, -22.0) < 3.5 then
                    t.state = "PRESO"; t.warrant = false
                    police.state = "PATRULLA"; police.target_npc = nil
                    set_alert("REBELDE #" .. t.id .. " ENCARCELADO!", 4.0)
                end
            else
                police.state = "PATRULLA"; police.target_npc = nil
            end
        end
        if human_pol.action_queued then
            human_pol.action_queued = false
            -- Desconectar antena cercana o arrestar sospechoso cercano
            local acted = false
            for i, ant in ipairs(antennas) do
                if ant.hacked and dist2d(police.x, police.z, ant.x, ant.z) < 3.2 then
                    ant.hacked = false; ant.hack_prog = 0.0
                    set_material(ant.tip, { color = { 0.2, 0.85, 0.4, 1.0 }, emissive = 0.6 })
                    set_light(ant.tip, 0.2, 0.9, 0.4, 2.0, 10.0)
                    set_alert("POLICIA DESCONECTO ANTENA #" .. i, 3.5)
                    acted = true; break
                end
            end
            if not acted then
                for _, n in ipairs(npcs) do
                    if n.state == "CAMINAR" or n.state == "SABOTEAR" then
                        if dist2d(police.x, police.z, n.x, n.z) < 2.6 then
                            arrest_npc_by_police(n); break
                        end
                    end
                end
            end
        end
    else
        if police.state == "PATRULLA" and comms_jam_timer <= 0.0 then
            for _, n in ipairs(npcs) do
                if n.warrant and n.state ~= "ESPOSADO" and n.state ~= "PRESO" then
                    police.state = "PERSIGUE"; police.target_npc = n.id; break
                end
            end
        end

        if police.state == "PERSIGUE" and police.target_npc then
            local target = npcs[police.target_npc]
            if not target or target.state == "PRESO" then
                police.state = "PATRULLA"; police.target_npc = nil
            else
                draw_line_3d(police.x, 1.2, police.z, target.x, 1.2, target.z, 0.25, 0.65, 1.0, 0.75)
                local nx, nz, arrived = steer_rig_2d("police_root", police.x, police.z, target.x, target.z, police.speed_sprint, dt)
                police.x, police.z = nx, nz
                if arrived or dist2d(police.x, police.z, target.x, target.z) < 1.35 then
                    arrest_npc_by_police(target)
                end
            end
        elseif police.state == "ESCOLTA" and police.target_npc then
            local target = npcs[police.target_npc]
            if not target or target.state ~= "ESPOSADO" then
                police.state = "PATRULLA"; police.target_npc = nil
            else
                local nx, nz, arrived = steer_rig_2d("police_root", police.x, police.z, -23.0, -22.0, 4.6, dt)
                police.x, police.z = nx, nz
                target.x = police.x - 0.8; target.z = police.z + 0.4
                if arrived then
                    target.state = "PRESO"; target.warrant = false
                    police.state = "PATRULLA"; police.target_npc = nil
                    set_alert("REBELDE #" .. target.id .. " ENCARCELADO!", 4.0)
                    play_tone(440, 880, "triangle", 0.3, 0.55)
                end
            end
        elseif police.state == "PING" and police.ping_x then
            draw_ring_3d(police.ping_x, 0.2, police.ping_z, 1.4, 0.25, 0.75, 1.0, 0.85)
            local nx, nz, arrived = steer_rig_2d("police_root", police.x, police.z, police.ping_x, police.ping_z, police.speed_sprint, dt)
            police.x, police.z = nx, nz
            for i, ant in ipairs(antennas) do
                if ant.hacked and dist2d(police.x, police.z, ant.x, ant.z) < 2.8 then
                    ant.hacked = false; ant.hack_prog = 0.0
                    set_material(ant.tip, { color = { 0.2, 0.85, 0.4, 1.0 }, emissive = 0.6 })
                    set_light(ant.tip, 0.2, 0.9, 0.4, 2.0, 10.0)
                    set_alert("POLICIA DESCONECTO ANTENA #" .. i, 3.5)
                end
            end
            if arrived then
                police.state = "PATRULLA"; police.ping_x = nil; police.ping_z = nil
            end
        else
            if police_room_step > 0 then
                police.x, police.z, police_room_step = step_room_visit("police_root", police.x, police.z, police_room_step, "POLICIA", police.speed_patrol * 1.1, dt)
            else
                police_room_cd = police_room_cd - dt
                if police_room_cd <= 0.0 and lockdown_timer <= 0.0 then
                    police_room_step = 1
                    police_room_cd = 32.0
                else
                    local wp = plaza_wps[police.patrol_idx]
                    local nx, nz, arrived = steer_rig_2d("police_root", police.x, police.z, wp.x, wp.z, police.speed_patrol, dt)
                    police.x, police.z = nx, nz
                    if arrived then police.patrol_idx = (police.patrol_idx % #plaza_wps) + 1 end
                end
            end
        end
    end

    local flash = (police.state == "PERSIGUE") and (math.sin(elapsed_time * 18.0) > 0 and 1.0 or 0.2) or 0.5
    set_light("police_beacon", 0.2, 0.55 * flash, 1.0, (phase == 3 or police.state == "PERSIGUE") and 3.5 or 1.8, 12.0)
    set_world_label("police_head", "POLICIA [" .. (human_pol and "HUMANO" or police.state) .. "]", 0.60, 0.3, 0.75, 1.0)
end

local function update_technician(dt)
    tech.anim = tech.anim + dt
    local _, human_tec = find_human_for_role("TECNICO")

    if human_tec then
        local spd = human_tec.sprint and 7.8 or 4.6
        tech.x, tech.z = apply_human_move("tech_root", tech.x, tech.z, human_tec.mx, human_tec.mz, spd, dt, false, human_tec)
        if human_tec.action_queued then
            human_tec.action_queued = false
            if not cam_micro_installed then
                cam_micro_installed = true
                cameras[5].x = tech.x; cameras[5].z = tech.z
                tween_position("cctv_5_head", tech.x, cameras[5].y, tech.z, 0.0)
                set_alert("TECNICO INSTALO CAM-05 EN SU POSICION!", 4.0)
            else
                for _, fb in ipairs(fuse_boxes) do
                    if fb.down_timer > 0.0 and dist2d(tech.x, tech.z, fb.x, fb.z) < 3.0 then
                        fb.down_timer = 0.0
                        set_material(fb.id, { color = { 0.9, 0.6, 0.15, 1.0 }, emissive = 0.12 })
                        set_alert("TECNICO REPARO " .. fb.name, 3.0); break
                    end
                end
                for i = 1, 5 do
                    if (cameras[i].blind_timer > 0.0 or cameras[i].loop_timer > 0.0) and dist2d(tech.x, tech.z, cameras[i].x, cameras[i].z) < 4.0 then
                        cameras[i].blind_timer = 0.0; cameras[i].loop_timer = 0.0
                        set_alert("TECNICO REPARO " .. cameras[i].name, 3.0); break
                    end
                end
            end
        end
    else
        if tech.state == "INSTALAR_MICRO" then
            local nx, nz, arrived = steer_rig_2d("tech_root", tech.x, tech.z, -22.5, 9.5, tech.speed, dt)
            tech.x, tech.z = nx, nz
            if arrived then
                tech.repair_prog = tech.repair_prog + dt
                if tech.repair_prog >= 3.0 then
                    cam_micro_installed = true
                    tween_position("cctv_5_head", cameras[5].x, cameras[5].y, cameras[5].z, 0.0)
                    spawn_vfx("hit_spark", cameras[5].x, cameras[5].y, cameras[5].z)
                    set_alert("TECNICO INSTALO CAM-05 (MICRO-CAM EN CALLEJON)!", 4.5)
                    play_tone(580, 920, "sine", 0.25, 0.45)
                    tech.state = "PATRULLA"; tech.repair_prog = 0.0
                end
            end
        elseif tech.state == "REPARAR" then
            local tx, tz = tech.x, tech.z
            if tech.repair_kind == "cam" then local c = cameras[tech.repair_idx]; tx, tz = c.x, c.z
            elseif tech.repair_kind == "fuse" then local fb = fuse_boxes[tech.repair_idx]; tx, tz = fb.x, fb.z end
            local nx, nz, arrived = steer_rig_2d("tech_root", tech.x, tech.z, tx, tz, tech.speed * 1.2, dt)
            tech.x, tech.z = nx, nz
            if arrived then
                tech.repair_prog = tech.repair_prog + dt
                if tech.repair_prog >= 4.0 then
                    if tech.repair_kind == "cam" then
                        cameras[tech.repair_idx].blind_timer = 0.0; cameras[tech.repair_idx].loop_timer = 0.0
                        set_alert("TECNICO REPARO " .. cameras[tech.repair_idx].name, 3.0)
                    else
                        local fb = fuse_boxes[tech.repair_idx]; fb.down_timer = 0.0
                        set_material(fb.id, { color = { 0.9, 0.6, 0.15, 1.0 }, emissive = 0.12 })
                        set_alert("TECNICO RESTAURO ENERGIA EN " .. fb.name, 3.0)
                    end
                    spawn_vfx("hit_spark", tx, 1.4, tz)
                    tech.state = "PATRULLA"; tech.repair_prog = 0.0; tech.repair_kind = nil
                end
            end
        else
            for i, fb in ipairs(fuse_boxes) do
                if fb.down_timer > 0.0 then
                    tech.state = "REPARAR"; tech.repair_kind = "fuse"; tech.repair_idx = i; tech.repair_prog = 0.0; break
                end
            end
            if tech.state == "PATRULLA" then
                for i = 1, 5 do
                    if cameras[i].blind_timer > 0.0 or cameras[i].loop_timer > 0.0 then
                        tech.state = "REPARAR"; tech.repair_kind = "cam"; tech.repair_idx = i; tech.repair_prog = 0.0; break
                    end
                end
            end
            if tech.state == "PATRULLA" then
                if tech_room_step > 0 then
                    tech.x, tech.z, tech_room_step = step_room_visit("tech_root", tech.x, tech.z, tech_room_step, "TECNICO", tech.speed, dt)
                else
                    tech_room_cd = tech_room_cd - dt
                    if tech_room_cd <= 0.0 and lockdown_timer <= 0.0 then
                        tech_room_step = 1
                        tech_room_cd = 28.0
                    else
                        local wp = plaza_wps[tech.patrol_idx]
                        local nx, nz, arrived = steer_rig_2d("tech_root", tech.x, tech.z, wp.x, wp.z, tech.speed * 0.75, dt)
                        tech.x, tech.z = nx, nz
                        if arrived then tech.patrol_idx = (tech.patrol_idx % #plaza_wps) + 1 end
                    end
                end
            end
        end
    end

    set_world_label("tech_head", "TECNICO [" .. (human_tec and "HUMANO" or tech.state) .. "]", 0.55, 0.35, 0.9, 0.75)
end

-- ============================================================================
-- CLIENTE EN RED: INPUT Y CAMARA 3RA PERSONA
-- ============================================================================
local client_cam_yaw = 3.14159
local client_cam_pitch = -0.40
local client_jump_pending = false

local function update_client_mode(dt)
    if is_key_just_pressed("L") then
        set_pointer_lock(not is_pointer_locked())
    end
    client_cam_yaw, client_cam_pitch = apply_mouse_look(client_cam_yaw, client_cam_pitch, -0.95, -0.05)

    local fwd_in, rgt_in = 0.0, 0.0
    if is_key_down("W") or is_key_down("UP") then fwd_in = fwd_in + 1.0 end
    if is_key_down("S") or is_key_down("DOWN") then fwd_in = fwd_in - 1.0 end
    if is_key_down("A") or is_key_down("LEFT") then rgt_in = rgt_in - 1.0 end
    if is_key_down("D") or is_key_down("RIGHT") then rgt_in = rgt_in + 1.0 end
    local fwd_x = math.sin(client_cam_yaw)
    local fwd_z = math.cos(client_cam_yaw)
    local rgt_x = -math.cos(client_cam_yaw)
    local rgt_z = math.sin(client_cam_yaw)
    local mx = fwd_x * fwd_in + rgt_x * rgt_in
    local mz = fwd_z * fwd_in + rgt_z * rgt_in
    local sprint = is_key_down("SHIFT") and "1" or "0"

    if is_key_just_pressed("E") or is_key_just_pressed("C") then
        client_action_pending = true
    end
    if is_key_just_pressed("SPACE") then
        client_jump_pending = true
    end
    handle_keypad_hotkeys()

    client_input_accum = client_input_accum + dt
    if client_input_accum >= 0.05 or client_action_pending or client_jump_pending then
        client_input_accum = 0.0
        local act = client_action_pending and "1" or "0"
        local jmp = client_jump_pending and "1" or "0"
        client_action_pending = false
        client_jump_pending = false
        net_send(1, string.format("IN|%d|%.2f|%.2f|%s|%s|%s", net_client_id(), mx, mz, sprint, act, jmp))
    end

    -- Camara orbital en 3ra persona controlada con el mouse
    local target_id = "police_root"
    if my_role == "TECNICO" then
        target_id = "tech_root"
    elseif my_role == "REBELDE" and my_controlled_npc > 0 then
        target_id = "npc_" .. my_controlled_npc .. "_root"
    end
    local ent = get_entity(target_id)
    if ent then
        local cam_dist = 8.2
        local cp = math.cos(client_cam_pitch)
        local cx = ent.x - math.sin(client_cam_yaw) * cp * cam_dist
        local cy = ent.y + 1.4 - math.sin(client_cam_pitch) * cam_dist
        local cz = ent.z - math.cos(client_cam_yaw) * cp * cam_dist
        set_camera(cx, math.max(ent.y + 0.7, cy), cz, ent.x, ent.y + 1.0, ent.z)
        draw_ring_3d(ent.x, 0.15, ent.z, 0.95, 0.25, 0.95, 1.0, 0.85)
    end
end

-- ============================================================================
-- HUD DECLARATIVO PANOPTICON + MATRIZ CCTV RTT (lua_ui)
-- ============================================================================
local last_ui_sig = ""

local function draw_hud()
    local ui_sig = string.format("%s|%s|%s|%s|%s|%s|%s|%s|%s|%s|%s",
        game_state, menu_tab, my_role, tostring(keypad_open), tostring(show_rtt_matrix),
        tostring(alert_banner_timer > 0.0), tostring(game_over),
        tostring(is_cam_operational(1)), tostring(is_cam_operational(2)),
        tostring(is_cam_operational(3)), tostring(is_cam_operational(4)))
    if ui_sig ~= last_ui_sig then
        last_ui_sig = ui_sig
        ui_clear()
    end

    if game_state == "MENU" then
        if menu_tab == "MAIN" then
            ui_panel("pnl_menu", "center", 0, 0, 360, 286, 0.05, 0.08, 0.13, 0.94)
            ui_label("lbl_m_tag", "center", -142, -120, "SISTEMA DE VIGILANCIA URBANA", 10, 0.30, 0.92, 1.0)
            ui_label("lbl_m_tit", "center", -142, -96, "PROYECTO PANOPTICON", 20, 0.96, 0.98, 1.0)
            ui_label("lbl_m_sub", "center", -142, -66, "Vigilancia Asimetrica | RTT CCTV | Red UDP", 10, 0.72, 0.80, 0.90)
            ui_button("btn_m_solo", "center", 0, -24, 270, 34, "Jugar Modo Solo (Operador) [ENTER]", "on_menu_play_solo")
            ui_button("btn_m_host", "center", 0,  20, 270, 32, "Crear Partida Host (UDP :5000)", "on_menu_host")
            ui_button("btn_m_join", "center", 0,  62, 270, 32, "Unirse a Partida (127.0.0.1)", "on_menu_join")
            ui_button("btn_m_ctrl", "center", 0, 106, 270, 30, "Controles y Reglas", "on_menu_controls")
        else
            ui_panel("pnl_m_c", "center", 0, 0, 476, 306, 0.05, 0.08, 0.13, 0.95)
            ui_label("lbl_mc_t", "center", -212, -132, "CONTROLES Y REGLAS - PANOPTICON", 13, 0.30, 0.94, 1.0)
            ui_label("lbl_mc_1", "center", -212, -104, "Personaje [0]: WASD Mover | SHIFT Correr | ESPACIO Saltar | Mouse Mirar ([L])", 10, 0.90, 0.94, 0.98)
            ui_label("lbl_mc_2", "center", -212,  -84, "Camaras [1..5]: WASD/Flechas Rotar | Q/E Zoom | [M] Matriz RTT", 10, 0.90, 0.94, 0.98)
            ui_label("lbl_mc_3", "center", -212,  -64, "Sala de Camaras: [G] Abrir Puerta (PIN Sistema: 4826) | Casas Transitables", 10, 0.95, 0.86, 0.45)
            ui_label("lbl_mc_4", "center", -212,  -44, "Tactico: [TAB] Objetivo | [C/ESP] Escaner | [P] Ping | [B] Bloqueo | [F10] Menu", 10, 0.85, 0.90, 0.95)
            ui_label("lbl_mc_5", "center", -212,  -16, "LEYES: I No Correr | II No Aglomerarse (Fase 2+) | III Contrabando", 10, 0.95, 0.72, 0.35)
            ui_label("lbl_mc_6", "center", -212,    4, "       IV Sabotaje Infraestructura | V Toque de Queda Nocturno (Fase 3)", 10, 0.95, 0.72, 0.35)
            ui_label("lbl_mc_7", "center", -212,   34, "VICTORIA SISTEMA: Arrestar 3 Rebeldes o resistir 12 min.", 10, 0.40, 0.95, 0.70)
            ui_label("lbl_mc_8", "center", -212,   54, "VICTORIA REBELDES: Hackear 3 Antenas (35s) o el Servidor Central (15s).", 10, 1.0, 0.45, 0.45)
            ui_button("btn_mc_back", "center", 0, 112, 180, 32, "Volver al Menu", "on_menu_controls")
        end
        return
    end
    local mins = math.floor(match_time / 60.0)
    local secs = math.floor(match_time % 60.0)
    local phase_name = (phase == 1) and "FASE 1: DIA" or ((phase == 2) and "FASE 2: ATARDECER" or "FASE 3: TOQUE DE QUEDA")

    local jailed_rebels = synced_jailed
    local hacked_ants = synced_hacked
    if not net_is_client() then
        jailed_rebels = 0; hacked_ants = 0
        for _, n in ipairs(npcs) do
            if n.is_rebel and n.state == "PRESO" then jailed_rebels = jailed_rebels + 1 end
        end
        for _, a in ipairs(antennas) do
            if a.hacked then hacked_ants = hacked_ants + 1 end
        end
    end

    local net_tag = net_is_server() and "HOST:5000" or (net_is_client() and ("CLI#" .. net_client_id() .. " " .. my_role) or "SOLO")

    -- Panel Superior Izquierdo
    ui_panel("pnl_top_l", "top_left", 10, 10, 325, 114, 0.06, 0.09, 0.14, 0.90)
    ui_label("lbl_title", "top_left", 20, 18, string.format("PANOPTICON | %02d:%02d | %s [%s]", mins, secs, phase_name, net_tag), 11, 0.25, 0.90, 1.0)
    ui_bar("bar_energy", "top_left", 20, 38, 305, 16, energy / max_energy, string.format("Energia: %.0f / 100", energy), 0.2, 0.75, 1.0)

    local leyes_str = (phase == 1) and "Leyes: I (No Correr), III (Contrabando), IV (Infraest.)"
        or ((phase == 2) and "Leyes: I, II (No Aglomerarse), III, IV"
        or "Leyes: I, II, III, IV, V (TOQUE DE QUEDA)")
    ui_label("lbl_laws", "top_left", 20, 62, leyes_str, 10, 0.95, 0.85, 0.45)

    local obj_str = string.format("Capturados: %d/3 | Antenas: %d/3 | Transm: %.0f/%.0fs", jailed_rebels, hacked_ants, antenna_hold_timer, ANTENNA_HOLD_GOAL)
    ui_label("lbl_objs", "top_left", 20, 80, obj_str, 11, 0.9, 0.95, 1.0)
    local srv_str = string.format("Servidor Central: %.0f%% intacto", math.max(0.0, (1.0 - server_hack_progress / SERVER_HACK_GOAL) * 100.0))
    ui_label("lbl_srv", "top_left", 20, 98, srv_str, 10, (server_hack_progress > 0 or hacked_ants == 3) and 1.0 or 0.72, 0.80, 0.88)

    -- Panel Superior Derecho
    ui_panel("pnl_top_r", "top_right", 10, 10, 276, 114, 0.06, 0.09, 0.14, 0.90)
    if my_role == "OPERADOR" then
        local c = cameras[active_cam]
        local cam_ok = (active_cam == 0) or is_cam_operational(active_cam)
        local cam_status = (active_cam == 0) and "VENTANA BLINDADA"
            or (c.blind_timer > 0 and string.format("CIEGA (%.0fs)", c.blind_timer)
            or ((c.sector > 0 and not is_sector_powered(c.sector)) and "SIN ENERGIA"
            or (c.loop_timer > 0 and string.format("LOOP FALSO (%.0fs)", c.loop_timer) or "EN LINEA")))
        ui_label("lbl_cam", "top_right", 276, 18, "MON: " .. c.name, 12, 0.3, 1.0, 0.7)
        ui_label("lbl_cam_st", "top_right", 276, 36, "Estado: " .. cam_status, 11, cam_ok and 0.4 or 1.0, cam_ok and 0.95 or 0.3, 0.35)
        local ctrl_str = (active_cam == 0)
            and "Mouse: Mirar | SHIFT: Correr | ESP: Saltar"
            or "WASD/Flechas: Mover Cam | Q/E: Zoom | [M] RTT"
        ui_label("lbl_ctrl_1", "top_right", 276, 54, ctrl_str, 10, 0.8, 0.85, 0.92)
        local tgt = npcs[selected_npc_idx]
        local tgt_txt = tgt and string.format("Objetivo: #%d (%s)", tgt.id, tgt.state) or "Sin objetivo"
        ui_label("lbl_tgt", "top_right", 276, 72, tgt_txt, 11, 0.95, 0.85, 0.3)
        ui_bar("bar_scan", "top_right", 20, 92, 256, 18, scan_progress / 3.0,
            scanning and string.format("Escaneando... %.1fs / 3.0s", scan_progress) or ((active_cam == 0) and "[C] Escanear (3s)" or "ESPACIO: Escanear (3s)"), 0.25, 0.95, 0.85)
    else
        ui_label("lbl_cam", "top_right", 276, 18, "ROL DE CAMPO: " .. my_role, 12, 0.3, 1.0, 0.7)
        local pin_hint = (my_role ~= "REBELDE") and (" | PIN Sala: " .. DOOR_PIN) or ""
        ui_label("lbl_cam_st", "top_right", 276, 38, "WASD+Mouse: Mover | SHIFT: Correr" .. pin_hint, 10, 0.85, 0.9, 0.95)
        ui_label("lbl_ctrl_1", "top_right", 276, 58, "ESP: Saltar | E/C: Accion | [G] Puerta", 10, 0.95, 0.85, 0.35)
        ui_button("btn_r_pol", "top_right", 196, 86, 76, 22, "Policia", "on_claim_police")
        ui_button("btn_r_reb", "top_right", 110, 86, 76, 22, "Rebelde", "on_claim_rebel")
        ui_button("btn_r_tec", "top_right",  24, 86, 76, 22, "Tecnico", "on_claim_tech")
    end

    -- 4 Mini-Pantallas CCTV en Vivo (RTT) cuando el Operador tiene Matriz activa
    if my_role == "OPERADOR" and show_rtt_matrix then
        local rtt_pos = {
            { id = "rtt_cam_1", lbl = "CAM-01", x = 10,   y = 164, cam = 1 },
            { id = "rtt_cam_2", lbl = "CAM-02", x = 10,   y = 242, cam = 2 },
            { id = "rtt_cam_3", lbl = "CAM-03", x = -134, y = 164, cam = 3 },
            { id = "rtt_cam_4", lbl = "CAM-04", x = -134, y = 242, cam = 4 }
        }
        for _, rp in ipairs(rtt_pos) do
            if is_cam_operational(rp.cam) then
                ui_draw_rtt(rp.id, rp.x, rp.y, 124, 70)
            else
                local anc = (rp.x < 0) and "top_right" or "top_left"
                local ox = (rp.x < 0) and 10 or rp.x
                ui_panel("pnl_off_" .. rp.cam, anc, ox, rp.y, 124, 70, 0.14, 0.04, 0.04, 0.92)
            end
        end
    end

    -- Alerta Central Superior
    if alert_banner_timer > 0.0 then
        ui_panel("pnl_alert", "top_center", 0, 130, 440, 26, 0.16, 0.08, 0.08, 0.90)
        ui_label("lbl_alert", "top_center", -200, 136, alert_banner, 11, 1.0, 0.85, 0.3)
    end

    -- Barra Inferior Compacta
    if my_role == "OPERADOR" then
        ui_panel("pnl_bot", "bottom_center", 0, 8, 608, 66, 0.06, 0.09, 0.14, 0.90)
        ui_button("btn_c0", "bottom_center", -252, 44, 78, 22, "[0] Sala", "on_cam_0")
        ui_button("btn_c1", "bottom_center", -168, 44, 78, 22, "[1] Cam1", "on_cam_1")
        ui_button("btn_c2", "bottom_center",  -84, 44, 78, 22, "[2] Cam2", "on_cam_2")
        ui_button("btn_c3", "bottom_center",    0, 44, 78, 22, "[3] Cam3", "on_cam_3")
        ui_button("btn_c4", "bottom_center",   84, 44, 78, 22, "[4] Cam4", "on_cam_4")
        ui_button("btn_c5", "bottom_center",  168, 44, 78, 22, cam_micro_installed and "[5] Cam5" or "[5] Pend", "on_cam_5")
        ui_button("btn_mr", "bottom_center",  252, 44, 78, 22, show_rtt_matrix and "[M] RTT" or "[M] Off", "on_btn_rtt")

        ui_button("btn_tab",  "bottom_center", -252, 14, 78, 22, "[TAB] Obj", "on_btn_next_target")
        ui_button("btn_scan", "bottom_center", -168, 14, 78, 22, "[SPC] Scn", "on_btn_scan")
        ui_button("btn_ping", "bottom_center",  -84, 14, 78, 22, "[P] Ping", "on_btn_ping")
        ui_button("btn_lock", "bottom_center",    0, 14, 78, 22, lockdown_timer > 0 and string.format("Lock %.0fs", lockdown_timer) or "[B] Lock", "on_btn_lockdown")
        ui_button("btn_door", "bottom_center",   84, 14, 78, 22, cam_door_open_timer > 0 and string.format("Abierta %.0f", cam_door_open_timer) or "[G] Puerta", "on_btn_keypad_toggle")
        ui_button("btn_hst",  "bottom_center",  168, 14, 78, 22, net_is_server() and "StopHost" or "[F8] Host", "on_btn_host")
        ui_button("btn_jn",   "bottom_center",  252, 14, 78, 22, net_is_client() and "SalirNet" or "[F9] Join", "on_btn_join")
    else
        ui_panel("pnl_bot", "bottom_center", 0, 8, 390, 36, 0.06, 0.09, 0.14, 0.90)
        ui_button("btn_door", "bottom_center", -95, 15, 155, 22, cam_door_open_timer > 0 and string.format("Puerta (%.0fs)", cam_door_open_timer) or "[G] Puerta PIN", "on_btn_keypad_toggle")
        ui_button("btn_jn", "bottom_center",  95, 15, 175, 22, "Salir a SOLO", "on_btn_join")
    end

    -- Panel Teclado Numerico (PIN de Puerta de Sala de Camaras)
    if keypad_open then
        local mask = string.rep("*", #keypad_input) .. string.rep("_", 4 - #keypad_input)
        ui_panel("pnl_kp", "center", 0, 0, 236, 252, 0.05, 0.08, 0.13, 0.96)
        ui_label("lbl_kp_t", "center", -96, -108, "ACCESO SALA DE CAMARAS", 12, 0.3, 0.95, 1.0)
        ui_label("lbl_kp_s", "center", -96, -86, keypad_status, 10, 0.95, 0.85, 0.35)
        ui_label("lbl_kp_d", "center", -42, -62, "PIN: " .. mask, 16, 0.95, 0.98, 1.0)

        ui_button("kp_1", "center", -62, -22, 54, 28, "1", "on_kp_1")
        ui_button("kp_2", "center",   0, -22, 54, 28, "2", "on_kp_2")
        ui_button("kp_3", "center",  62, -22, 54, 28, "3", "on_kp_3")
        ui_button("kp_4", "center", -62,  12, 54, 28, "4", "on_kp_4")
        ui_button("kp_5", "center",   0,  12, 54, 28, "5", "on_kp_5")
        ui_button("kp_6", "center",  62,  12, 54, 28, "6", "on_kp_6")
        ui_button("kp_7", "center", -62,  46, 54, 28, "7", "on_kp_7")
        ui_button("kp_8", "center",   0,  46, 54, 28, "8", "on_kp_8")
        ui_button("kp_9", "center",  62,  46, 54, 28, "9", "on_kp_9")
        ui_button("kp_del", "center", -62, 80, 54, 28, "DEL", "on_kp_del")
        ui_button("kp_0",   "center",   0, 80, 54, 28, "0", "on_kp_0")
        ui_button("kp_ok",  "center",  62, 80, 54, 28, "OK", "on_kp_enter")

        if my_role ~= "REBELDE" then
            ui_button("kp_auto", "center", -52, 112, 98, 24, "PIN:" .. DOOR_PIN, "on_kp_auto")
        end
        ui_button("kp_cls", "center", 52, 112, 98, 24, cam_door_is_open and "Cerrar Puerta" or "Salir", cam_door_is_open and "on_kp_close_door" or "on_btn_keypad_toggle")
    end

    if game_over then
        local sys_win = (winner == "SISTEMA")
        ui_panel("pnl_end", "center", 0, 0, 440, 140, sys_win and 0.05 or 0.25, sys_win and 0.18 or 0.06, sys_win and 0.24 or 0.08, 0.95)
        ui_label("lbl_end_1", "center", -175, -40, sys_win and "VICTORIA DEL SISTEMA (PANOPTICON)" or "VICTORIA DE LOS REBELDES", 15, sys_win and 0.3 or 1.0, sys_win and 0.95 or 0.35, sys_win and 1.0 or 0.35)
        ui_label("lbl_end_2", "center", -185, -10, win_reason, 11, 0.92, 0.95, 1.0)
        ui_button("btn_rst", "center", -88, 32, 160, 30, "Nueva Partida [R]", "on_btn_reset")
        ui_button("btn_mnu", "center",  88, 32, 140, 30, "Menu Principal", "on_btn_to_menu")
    end
end

-- ============================================================================
-- BUCLE PRINCIPAL on_update
-- ============================================================================
function on_update(entity, dt)
    if dt <= 0.0 then return end
    if dt > 0.1 then dt = 0.1 end

    if not initialized then
        initialized = true
        reset_panopticon()
    end

    elapsed_time = elapsed_time + dt
    if alert_banner_timer > 0.0 then
        alert_banner_timer = math.max(0.0, alert_banner_timer - dt)
    end

    if game_state == "MENU" then
        if is_pointer_locked() then set_pointer_lock(false) end
        if is_key_just_pressed("RETURN") or is_key_just_pressed("ENTER") then
            on_menu_play_solo()
            return
        end
        set_time_of_day(13.5)
        local ang = elapsed_time * 0.14
        local cx = math.sin(ang) * 19.0
        local cz = 1.0 + math.cos(ang) * 19.0
        local cy = 9.2 + math.sin(elapsed_time * 0.35) * 1.1
        set_camera(cx, cy, cz, 0.0, 4.2, -8.5)
        update_npcs(dt)
        update_police(dt)
        update_technician(dt)
        draw_hud()
        return
    end

    if is_key_just_pressed("F10") then
        on_btn_to_menu()
        return
    end

    if net_is_client() and net_client_id() > 0 then
        -- Modo Cliente UDP conectado: sincroniza camara e input, el Servidor replica transforms
        update_day_night_cycle()
        if not game_over then
            update_client_mode(dt)
        end
        draw_hud()
        return
    elseif net_is_client() and net_client_id() == 0 then
        -- Esperando handshake del servidor (permite cancelar con F9 o boton SalirNet)
        if is_key_just_pressed("F9") then on_btn_join() end
        set_alert("CONECTANDO A 127.0.0.1:5000... (PULSA [F9] / SalirNet PARA MODO SOLO)", 0.5)
        draw_hud()
        return
    end

    -- Modo Servidor Autoritativo o Single-Player
    if not game_over then
        match_time = math.max(0.0, match_time - dt)
        energy = math.min(max_energy, energy + 4.5 * dt)

        if lockdown_timer > 0.0 then
            lockdown_timer = math.max(0.0, lockdown_timer - dt)
            if lockdown_timer == 0.0 then
                tween_position("sec_door_west", -6.0, 4.2, -16.0, 0.25)
                tween_position("sec_door_east",  6.0, 4.2, -16.0, 0.25)
                set_alert("BLOQUEO REMOTO FINALIZADO - COMPUERTAS ABIERTAS", 3.0)
            end
        end
        if cam_door_open_timer > 0.0 then
            cam_door_open_timer = math.max(0.0, cam_door_open_timer - dt)
            if cam_door_open_timer == 0.0 then
                close_cam_room_door()
            end
        end
        set_world_label("cam_room_door", cam_door_open_timer > 0.0 and string.format("ABIERTA %.0fs", cam_door_open_timer) or "PUERTA [G] PIN", 1.7, cam_door_open_timer > 0.0 and 0.3 or 1.0, cam_door_open_timer > 0.0 and 1.0 or 0.4, 0.35)
        if comms_jam_timer > 0.0 then
            comms_jam_timer = math.max(0.0, comms_jam_timer - dt)
        end

        for i = 1, 5 do
            if cameras[i].blind_timer > 0.0 then cameras[i].blind_timer = math.max(0.0, cameras[i].blind_timer - dt) end
            if cameras[i].loop_timer > 0.0 then cameras[i].loop_timer = math.max(0.0, cameras[i].loop_timer - dt) end
        end
        for _, fb in ipairs(fuse_boxes) do
            if fb.down_timer > 0.0 then
                fb.down_timer = math.max(0.0, fb.down_timer - dt)
                if fb.down_timer == 0.0 then
                    set_material(fb.id, { color = { 0.9, 0.6, 0.15, 1.0 }, emissive = 0.12 })
                end
            end
        end

        if scanning then
            local tgt = npcs[selected_npc_idx]
            if not tgt or tgt.state == "PRESO" then
                scanning = false; scan_progress = 0.0
            else
                local ex, ey, ez = get_cam_eye_pos(active_cam)
                draw_line_3d(ex, ey - 0.3, ez, tgt.x, 1.4, tgt.z, 0.2, 0.95, 1.0, 0.9)
                scan_progress = scan_progress + dt
                if scan_progress >= 3.0 then
                    scanning = false; scan_progress = 0.0
                    energy = math.max(0.0, energy - 12.0)
                    tgt.scanned = true
                    local inf = check_npc_active_infraction(tgt)
                    tgt.infraction = inf; tgt.warrant = true
                    if inf then
                        set_alert("ORDEN DE ARRESTO (#" .. tgt.id .. "): " .. inf, 4.0)
                        play_tone(620, 940, "sine", 0.25, 0.55)
                    else
                        set_alert("ORDEN FORZADA SIN INFRACCION (#" .. tgt.id .. ")", 4.0)
                        play_tone(320, 260, "triangle", 0.25, 0.45)
                    end
                    if comms_jam_timer <= 0.0 then
                        police.state = "PERSIGUE"; police.target_npc = tgt.id
                    end
                end
            end
        end

        update_day_night_cycle()
        update_operator_camera(dt)
        update_npcs(dt)
        update_police(dt)
        update_technician(dt)

        local jailed = 0
        for _, n in ipairs(npcs) do
            if n.is_rebel and n.state == "PRESO" then jailed = jailed + 1 end
        end
        if jailed >= 3 then
            game_over = true; winner = "SISTEMA"
            win_reason = "ORDEN RESTABLECIDO: Los 3 rebeldes fueron capturados."
            play_tone(440, 880, "triangle", 0.6, 0.7)
        elseif match_time <= 0.0 then
            game_over = true; winner = "SISTEMA"
            win_reason = "RESISTENCIA: Sobreviviste los 12 minutos sin caer."
            play_tone(440, 880, "triangle", 0.6, 0.7)
        end

        local hacked_count = 0
        for _, a in ipairs(antennas) do
            if a.hacked then hacked_count = hacked_count + 1 end
        end
        if hacked_count == 3 then
            antenna_hold_timer = antenna_hold_timer + dt
            set_alert(string.format("3 ANTENAS TRANSMITIENDO! CORTA LA SENAL (%.1fs)", ANTENNA_HOLD_GOAL - antenna_hold_timer), 0.5)
            if antenna_hold_timer >= ANTENNA_HOLD_GOAL then
                game_over = true; winner = "REBELDES"
                win_reason = "LEVANTAMIENTO POPULAR: 3 Antenas transmitieron 35s."
                play_tone(240, 120, "sawtooth", 0.6, 0.7)
            end
        else
            antenna_hold_timer = math.max(0.0, antenna_hold_timer - dt * 0.5)
        end

        -- Broadcast de estado HUD hacia clientes conectados (10 Hz)
        if net_is_server() then
            net_sync_accum = net_sync_accum + dt
            if net_sync_accum >= 0.1 then
                net_sync_accum = 0.0
                local b_msg = (alert_banner_timer > 0.0) and alert_banner or ""
                net_broadcast(1, string.format("SYNC|%.1f|%d|%.1f|%.1f|%.1f|%.1f|%s|%d|%d|%s|%s|%s",
                    match_time, phase, energy, lockdown_timer, server_hack_progress, antenna_hold_timer,
                    cam_micro_installed and "1" or "0", jailed, hacked_count, b_msg, winner, win_reason))
            end
        end
    else
        if is_key_just_pressed("R") then reset_panopticon() end
    end

    draw_hud()
end
