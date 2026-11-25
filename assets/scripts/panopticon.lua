-- ============================================================================
-- PROYECTO "PANOPTICON" - Prototipo Jugable en Motor-dif (Hito 1 + Hito 2)
-- Escena: assets/panopticon.json
-- Script: assets/scripts/panopticon.lua
-- Rol del Jugador: OPERADOR (Torre de Control + Matriz CCTV)
-- ============================================================================

local MATCH_DURATION = 720.0 -- 12 minutos (4 min por fase)
local ANTENNA_HOLD_GOAL = 35.0
local SERVER_HACK_GOAL = 15.0

-- Estado global de partida
local initialized = false
local match_time = MATCH_DURATION
local elapsed_time = 0.0
local phase = 1 -- 1: Dia (0-4m), 2: Atardecer (4-8m), 3: Toque de Queda / Noche (8-12m)
local game_over = false
local winner = "" -- "SISTEMA" o "REBELDES"
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
local active_cam = 0 -- 0=Torre, 1..4=CCTV Fijas, 5=Micro-CAM Tecnico
local selected_npc_idx = 1
local scan_progress = 0.0
local scanning = false
local cam_micro_installed = false

-- Definicion de camaras
local cameras = {
    [0] = {
        name = "TORRE (VENTANA)",
        x = 0.0, y = 6.8, z = -19.5,
        yaw = 0.0, pitch = -0.25, zoom = 0.0,
        yaw_min = -0.85, yaw_max = 0.85,
        blind_timer = 0.0, loop_timer = 0.0, sector = 0, fixed = true
    },
    [1] = {
        name = "CAM-01 PLAZA NORTE",
        x = -12.0, y = 8.2, z = -10.0,
        yaw = 0.78, pitch = -0.52, zoom = 0.0,
        yaw_min = -0.2, yaw_max = 1.75,
        blind_timer = 0.0, loop_timer = 0.0, sector = 1, fixed = true,
        head_id = "cctv_1_head"
    },
    [2] = {
        name = "CAM-02 PLAZA SUR",
        x = 12.0, y = 8.2, z = 13.0,
        yaw = -2.35, pitch = -0.52, zoom = 0.0,
        yaw_min = -3.3, yaw_max = -1.4,
        blind_timer = 0.0, loop_timer = 0.0, sector = 2, fixed = true,
        head_id = "cctv_2_head"
    },
    [3] = {
        name = "CAM-03 MERCADO OESTE",
        x = -16.5, y = 7.2, z = -4.0,
        yaw = 2.35, pitch = -0.48, zoom = 0.0,
        yaw_min = 1.1, yaw_max = 3.5,
        blind_timer = 0.0, loop_timer = 0.0, sector = 1, fixed = true,
        head_id = "cctv_3_head"
    },
    [4] = {
        name = "CAM-04 SUBESTACION",
        x = 16.5, y = 7.2, z = -11.0,
        yaw = -0.85, pitch = -0.48, zoom = 0.0,
        yaw_min = -2.0, yaw_max = 0.25,
        blind_timer = 0.0, loop_timer = 0.0, sector = 2, fixed = true,
        head_id = "cctv_4_head"
    },
    [5] = {
        name = "CAM-05 MICRO-CAM",
        x = -22.5, y = 5.4, z = 9.5,
        yaw = -1.2, pitch = -0.45, zoom = 0.0,
        yaw_min = -3.14, yaw_max = 3.14,
        blind_timer = 0.0, loop_timer = 0.0, sector = 1, fixed = false,
        head_id = "cctv_5_head"
    }
}

-- Infraestructura: Cajas de fusibles y Antenas
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

-- Waypoints urbanos para patrulla y movimiento ciudadano
local plaza_wps = {
    { x = -9.0, z = -8.0 }, { x = 9.0, z = -8.0 },
    { x = 9.0,  z = 11.0 }, { x = -9.0, z = 11.0 },
    { x = -3.0, z = 2.0 },  { x = 4.0,  z = 5.0 },
    { x = -19.0, z = 1.0 }, { x = -23.0, z = 9.0 },
    { x = 19.0,  z = 2.0 }, { x = 21.0,  z = -13.0 }
}

-- NPCs (12 ciudadanos, 3 de ellos Rebeldes IA ocultos)
local npcs = {}
local NUM_NPCS = 12

-- Policia Bot
local police = {
    x = 5.0, z = -8.0,
    speed_patrol = 3.4, speed_sprint = 8.4,
    state = "PATRULLA", -- "PATRULLA", "PING", "PERSIGUE", "ESCOLTA"
    target_npc = nil,
    ping_x = nil, ping_z = nil,
    patrol_idx = 1,
    anim = 0.0
}

-- Tecnico Bot
local tech = {
    x = -5.0, z = -7.0,
    speed = 3.6,
    state = "INSTALAR_MICRO", -- "INSTALAR_MICRO", "PATRULLA", "REPARAR"
    repair_kind = nil, -- "cam" o "fuse"
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
    -- Calles principales (Ley V Toque de Queda)
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

local function get_cam_target_point(idx)
    local c = cameras[idx]
    local look_dist = 14.0 - c.zoom * 5.0
    local tx = c.x + math.sin(c.yaw) * look_dist
    local tz = c.z + math.cos(c.yaw) * look_dist
    local ty = clamp(c.y + math.sin(c.pitch) * look_dist, 0.5, 4.5)
    return tx, ty, tz
end

local function get_cam_eye_pos(idx)
    local c = cameras[idx]
    local forward_zoom = c.zoom * 4.5
    local ex = c.x + math.sin(c.yaw) * forward_zoom
    local ez = c.z + math.cos(c.yaw) * forward_zoom
    local ey = c.y + math.sin(c.pitch) * forward_zoom
    return ex, ey, ez
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

-- ============================================================================
-- INICIALIZACION Y RESET
-- ============================================================================
function reset_panopticon()
    ui_clear()
    clear_timers()
    clear_events()

    match_time = MATCH_DURATION
    elapsed_time = 0.0
    phase = 1
    game_over = false
    winner = ""
    win_reason = ""
    energy = 100.0
    lockdown_timer = 0.0
    comms_jam_timer = 0.0
    server_hack_progress = 0.0
    antenna_hold_timer = 0.0
    active_cam = 0
    selected_npc_idx = 1
    scan_progress = 0.0
    scanning = false
    cam_micro_installed = false

    set_time_of_day(12.0)
    set_alert("TURNO INICIADO: VIGILA LA PLAZA Y DETECTA INFRACTORES", 5.0)

    -- Reset camaras
    cameras[1].yaw = 0.78
    cameras[2].yaw = -2.35
    cameras[3].yaw = 2.35
    cameras[4].yaw = -0.85
    cameras[5].yaw = -1.2
    for i = 0, 5 do
        cameras[i].pitch = (i == 0) and -0.25 or -0.50
        cameras[i].zoom = 0.0
        cameras[i].blind_timer = 0.0
        cameras[i].loop_timer = 0.0
    end
    tween_position("cctv_5_head", 0.0, -15.0, 0.0, 0.0)

    -- Reset fusibles, antenas, tarjetas, puertas
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

    -- Elegir 3 rebeldes aleatorios entre los 12 NPCs
    math.randomseed(math.floor(elapsed_time * 1000) + 7919)
    local indices = { 1,2,3,4,5,6,7,8,9,10,11,12 }
    for i = 12, 2, -1 do
        local j = math.random(1, i)
        indices[i], indices[j] = indices[j], indices[i]
    end
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
            body = "npc_" .. i .. "_body",
            head = "npc_" .. i .. "_head",
            item = "npc_" .. i .. "_item",
            x = sx, z = sz,
            wp_idx = wp,
            is_rebel = rebel_set[i] == true,
            state = "CAMINAR", -- "CAMINAR", "ARRESTADO", "ESPOSADO", "PRESO", "SABOTEAR"
            task_kind = nil,   -- "cam", "fuse", "antenna", "keycard", "server", "rescue"
            task_idx = nil,
            task_timer = 0.0,
            cooldown = 5.0 + math.random() * 8.0,
            speed = 2.1 + math.random() * 0.4,
            sprinting = false,
            has_contraband = false,
            has_keycard = false,
            warrant = false,
            scanned = false,
            infraction = nil, -- texto de infraccion detectada
            crowding_timer = 0.0,
            false_arrest_timer = 0.0,
            anim = i * 0.6
        }
        tween_position(npcs[i].body, sx, 0.8, sz, 0.0)
        tween_position(npcs[i].head, sx, 1.72, sz, 0.0)
        tween_position(npcs[i].item, sx, -6.0, sz, 0.0)
        set_world_label(npcs[i].head, "", 0.8, 1.0, 1.0, 1.0)
    end

    -- Reset Policia y Tecnico
    police.x = 5.0; police.z = -8.0
    police.state = "PATRULLA"; police.target_npc = nil
    police.ping_x = nil; police.ping_z = nil
    tween_position("police_body", police.x, 0.85, police.z, 0.0)
    tween_position("police_head", police.x, 1.85, police.z, 0.0)
    tween_position("police_beacon", police.x, 2.25, police.z, 0.0)

    tech.x = -5.0; tech.z = -7.0
    tech.state = "INSTALAR_MICRO"; tech.repair_kind = nil; tech.repair_prog = 0.0
    tween_position("tech_body", tech.x, 0.85, tech.z, 0.0)
    tween_position("tech_head", tech.x, 1.8, tech.z, 0.0)
    tween_position("tech_pack", tech.x, 1.1, tech.z - 0.35, 0.0)

    -- Musica adaptativa cyberpunk/vigilancia
    play_music_preset("cyber_action")
    set_music_intensity(0.25)
    play_tone(320, 640, "sine", 0.25, 0.5)
end

-- ============================================================================
-- ACCIONES DEL OPERADOR (Teclado y Botones HUD)
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
    if game_over then return end
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
    if game_over then return end
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
            set_alert("POLICIA EN CAMINO A ARRESTAR A CIUDADANO #" .. n.id, 3.0)
        else
            police.state = "PING"
            set_alert("PING TACTICO ENVIADO HACIA CIUDADANO #" .. n.id, 2.5)
        end
        play_tone(660, 880, "triangle", 0.18, 0.5)
    end
end

local function trigger_lockdown()
    if game_over then return end
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
    if game_over then return end
    if phase == 1 then
        match_time = 479.0
        phase = 2
        set_alert("FASE 2: ATARDECER - PROHIBIDA AGLOMERACION (LEY II)", 4.5)
    elseif phase == 2 then
        match_time = 239.0
        phase = 3
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
function on_btn_reset() reset_panopticon() end

-- ============================================================================
-- ACTUALIZACION DE FASES, ILUMINACION Y CAMARA
-- ============================================================================
local function update_day_night_cycle()
    local elapsed_match = MATCH_DURATION - match_time
    local new_phase = 1
    local tod = 12.0
    if elapsed_match < 240.0 then
        new_phase = 1
        tod = 12.0 + (elapsed_match / 240.0) * 4.5 -- 12:00 -> 16:30
    elseif elapsed_match < 480.0 then
        new_phase = 2
        tod = 16.5 + ((elapsed_match - 240.0) / 240.0) * 3.0 -- 16:30 -> 19:30 (Atardecer)
    else
        new_phase = 3
        tod = 21.5 + ((elapsed_match - 480.0) / 240.0) * 2.5 -- 21:30 -> 24:00 (Noche)
    end

    if new_phase ~= phase then
        phase = new_phase
        if phase == 2 then
            set_alert("FASE 2 (ATARDECER): ACTIVA LEY II - NO AGLOMERARSE", 5.0)
            set_music_intensity(0.50)
            play_tone(330, 495, "triangle", 0.3, 0.5)
        elseif phase == 3 then
            set_alert("FASE 3 (NOCHE): TOQUE DE QUEDA ACTIVO - LEY V", 5.0)
            set_music_intensity(0.75)
            play_tone(240, 180, "sawtooth", 0.4, 0.6)
        end
    end
    set_time_of_day(tod)

    -- Iluminacion dinamica de farolas segun fase y corte de luz
    local p_west = is_sector_powered(1)
    local p_east = is_sector_powered(2)
    local lamp_int = (phase == 3) and 4.2 or ((phase == 2) and 2.8 or 1.4)
    set_light("lamp_nw_bulb", 1.0, 0.86, 0.62, p_west and lamp_int or 0.0, 16.0)
    set_light("lamp_sw_bulb", 1.0, 0.86, 0.62, p_west and lamp_int or 0.0, 16.0)
    set_light("lamp_ne_bulb", 1.0, 0.86, 0.62, p_east and lamp_int or 0.0, 16.0)
    set_light("lamp_se_bulb", 1.0, 0.86, 0.62, p_east and lamp_int or 0.0, 16.0)
end

local function update_operator_camera(dt)
    if key_pressed_any("DIGIT0", "0") then switch_camera(0) end
    if key_pressed_any("1", "DIGIT1") then switch_camera(1) end
    if key_pressed_any("2", "DIGIT2") then switch_camera(2) end
    if key_pressed_any("3", "DIGIT3") then switch_camera(3) end
    if key_pressed_any("4", "DIGIT4") then switch_camera(4) end
    if key_pressed_any("5", "DIGIT5") then switch_camera(5) end

    if is_key_just_pressed("TAB") then cycle_next_npc() end
    if is_key_just_pressed("SPACE") then trigger_scan_action() end
    if is_key_just_pressed("P") then trigger_police_ping() end
    if is_key_just_pressed("B") then trigger_lockdown() end
    if is_key_just_pressed("F") then advance_phase_test() end
    if is_key_just_pressed("R") then reset_panopticon() end

    local c = cameras[active_cam]
    local yaw_in = 0.0
    if is_key_down("A") or is_key_down("LEFT") then yaw_in = yaw_in + 1.0 end
    if is_key_down("D") or is_key_down("RIGHT") then yaw_in = yaw_in - 1.0 end
    local pitch_in = 0.0
    if is_key_down("W") or is_key_down("UP") then pitch_in = pitch_in + 1.0 end
    if is_key_down("S") or is_key_down("DOWN") then pitch_in = pitch_in - 1.0 end
    local zoom_in = 0.0
    if is_key_down("E") then zoom_in = zoom_in + 1.0 end
    if is_key_down("Q") then zoom_in = zoom_in - 1.0 end

    if active_cam == 0 or is_cam_operational(active_cam) then
        c.yaw = clamp(c.yaw + yaw_in * 1.15 * dt, c.yaw_min, c.yaw_max)
        c.pitch = clamp(c.pitch + pitch_in * 0.65 * dt, -0.95, -0.08)
        c.zoom = clamp(c.zoom + zoom_in * 0.9 * dt, 0.0, 1.0)
    end

    local ex, ey, ez = get_cam_eye_pos(active_cam)
    local tx, ty, tz = get_cam_target_point(active_cam)

    if active_cam > 0 and not is_cam_operational(active_cam) then
        -- Camara ciega o sin energia apunta al cielo/suelo oscuro
        set_camera(c.x, c.y, c.z, c.x, c.y - 6.0, c.z + 0.5)
    else
        set_camera(ex, ey, ez, tx, ty, tz)
    end

    -- Actualizar colores de cabezas de camara y rayos de vision
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
-- IA DE CIUDADANOS Y REBELDES
-- ============================================================================
local function move_towards_2d(cur_x, cur_z, tx, tz, spd, dt)
    local dx = tx - cur_x
    local dz = tz - cur_z
    local d = math.sqrt(dx * dx + dz * dz)
    if d < 0.25 then return cur_x, cur_z, true end
    local step = math.min(d, spd * dt)
    return cur_x + (dx / d) * step, cur_z + (dz / d) * step, false
end

local function update_npcs(dt)
    -- Calculo de aglomeracion (Ley II)
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

        if n.state == "PRESO" then
            -- Dentro de la celda
            local px = -24.0 + ((i % 3) * 1.3)
            local pz = -22.5 + math.floor(i / 4) * 1.2
            tween_position(n.body, px, 0.8, pz, 0.0)
            tween_position(n.head, px, 1.72, pz, 0.0)
            tween_position(n.item, px, -6.0, pz, 0.0)
            set_world_label(n.head, "DETENIDO", 0.7, 0.5, 0.7, 1.0)
        elseif n.state == "ESPOSADO" then
            -- Si fue arrestado falsamente sin infraccion, queda libre tras 5s
            if n.false_arrest_timer > 0.0 then
                n.false_arrest_timer = n.false_arrest_timer - dt
                if n.false_arrest_timer <= 0.0 then
                    n.state = "CAMINAR"
                    n.warrant = false
                    n.scanned = false
                    set_alert("CIUDADANO #" .. n.id .. " LIBERADO (FALSA DENUNCIA)", 3.5)
                end
            else
                -- Compañeros rebeldes cercanos pueden intentar rescatarlo!
                for _, ally in ipairs(npcs) do
                    if ally.id ~= n.id and ally.is_rebel and ally.state == "CAMINAR" then
                        if dist2d(ally.x, ally.z, n.x, n.z) < 2.2 then
                            n.state = "CAMINAR"
                            n.warrant = false
                            if police.target_npc == n.id then
                                police.state = "PATRULLA"
                                police.target_npc = nil
                            end
                            spawn_vfx("hit_spark", n.x, 1.2, n.z)
                            set_alert("RESCATE REBELDE! #" .. n.id .. " FUE LIBERADO!", 3.5)
                            play_tone(300, 180, "sawtooth", 0.25, 0.5)
                            break
                        end
                    end
                end
            end
            tween_position(n.body, n.x, 0.72, n.z, 0.0)
            tween_position(n.head, n.x, 1.62, n.z, 0.0)
            set_world_label(n.head, "ESPOSADO", 0.8, 1.0, 0.6, 0.2)
        else
            -- Comportamiento libre (CAMINAR o SABOTEAR)
            if n.is_rebel and n.state == "CAMINAR" then
                n.cooldown = n.cooldown - dt
                if n.cooldown <= 0.0 then
                    local watched = is_point_watched_by_any_cam(n.x, n.z)
                    -- Decide mision clandestina segun fase y vigilancia
                    local roll = math.random()
                    if phase >= 2 and not n.has_keycard and roll < 0.25 then
                        for k_i, kc in ipairs(keycards) do
                            if not kc.stolen then
                                n.state = "SABOTEAR"; n.task_kind = "keycard"; n.task_idx = k_i
                                n.has_contraband = true
                                break
                            end
                        end
                    elseif n.has_keycard and phase >= 2 then
                        n.state = "SABOTEAR"; n.task_kind = "server"; n.task_idx = 1
                        n.sprinting = not watched
                    elseif phase >= 2 and roll < 0.55 then
                        for a_i, ant in ipairs(antennas) do
                            if not ant.hacked then
                                n.state = "SABOTEAR"; n.task_kind = "antenna"; n.task_idx = a_i
                                n.has_contraband = true
                                break
                            end
                        end
                    elseif (not watched or roll < 0.75) then
                        if roll < 0.45 then
                            -- Sabotear camara cercana
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
                        -- A veces corre impaciente violando Ley I
                        n.sprinting = (not watched) and (math.random() < 0.45)
                        n.cooldown = 4.0 + math.random() * 5.0
                    end
                end
            end

            -- Ejecutar movimiento segun estado
            if n.state == "SABOTEAR" then
                local tx, tz = 0.0, 0.0
                if n.task_kind == "cam" then
                    local c = cameras[n.task_idx]; tx, tz = c.x, c.z
                elseif n.task_kind == "fuse" then
                    local fb = fuse_boxes[n.task_idx]; tx, tz = fb.x, fb.z
                elseif n.task_kind == "antenna" then
                    local ant = antennas[n.task_idx]; tx, tz = ant.x, ant.z
                elseif n.task_kind == "keycard" then
                    local kc = keycards[n.task_idx]; tx, tz = kc.x, kc.z
                elseif n.task_kind == "server" then
                    tx, tz = 0.0, -23.5
                end

                local spd = n.sprinting and 5.6 or 2.8
                -- Puertas de bloqueo detienen entrada a la Torre en z = -15.5
                if n.task_kind == "server" and lockdown_timer > 0.0 and n.z > -16.5 and tz < -16.5 then
                    spd = 0.0
                    n.task_timer = 0.0
                end

                local nx, nz, arrived = move_towards_2d(n.x, n.z, tx, tz, spd, dt)
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
                        fb.down_timer = 25.0
                        comms_jam_timer = 12.0
                        set_material(fb.id, { color = { 0.25, 0.25, 0.28, 1.0 }, emissive = 0.0 })
                        spawn_vfx("explosion", fb.x, 1.2, fb.z)
                        set_alert("SABOTAJE ELECTRICO: " .. fb.name .. " SIN ENERGIA (25s)!", 4.5)
                        play_tone(160, 80, "sawtooth", 0.4, 0.6)
                        n.state = "CAMINAR"; n.task_timer = 0.0; n.cooldown = 12.0
                        n.has_contraband = false
                    elseif n.task_kind == "keycard" and n.task_timer >= 1.0 then
                        local kc = keycards[n.task_idx]
                        kc.stolen = true
                        n.has_keycard = true
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
                            game_over = true
                            winner = "REBELDES"
                            win_reason = "APAGON TOTAL: El Servidor Central fue hackeado."
                        end
                    end
                end
            else
                -- Patrulla civil normal (en Fase 3 civiles evitan calles principales, rebeldes a veces arriesgan)
                local wp = plaza_wps[n.wp_idx]
                local spd = n.sprinting and 5.5 or n.speed
                local nx, nz, arrived = move_towards_2d(n.x, n.z, wp.x, wp.z, spd, dt)
                n.x, n.z = nx, nz
                if arrived then
                    n.wp_idx = (n.wp_idx % #plaza_wps) + 1
                    n.sprinting = false
                    -- Los civiles normales nunca pisan calles principales de noche si pueden ir por plaza
                    if phase == 3 and not n.is_rebel then
                        n.wp_idx = ((n.wp_idx - 1) % 6) + 1
                    end
                end
            end

            -- Actualizar posicion visual 3D
            local bob = math.abs(math.sin(n.anim * (n.sprinting and 12.0 or 6.0))) * 0.08
            tween_position(n.body, n.x, 0.8 + bob, n.z, 0.0)
            tween_position(n.head, n.x, 1.72 + bob, n.z, 0.0)
            if n.has_contraband or n.has_keycard then
                local col = n.has_keycard and { 1.0, 0.85, 0.15, 1.0 } or { 1.0, 0.25, 0.25, 1.0 }
                set_material(n.item, { color = col, emissive = 0.6 })
                tween_position(n.item, n.x + 0.45, 1.05 + bob, n.z, 0.0)
            else
                tween_position(n.item, n.x, -6.0, n.z, 0.0)
            end

            -- Etiquetas y anillos 3D
            local inf = check_npc_active_infraction(n)
            if n.warrant then
                draw_ring_3d(n.x, 0.15, n.z, 1.05, 1.0, 0.2, 0.2, 0.9)
                set_world_label(n.head, "ORDEN DE ARRESTO", 0.8, 1.0, 0.25, 0.25)
            elseif selected_npc_idx == n.id then
                draw_ring_3d(n.x, 0.15, n.z, 0.95, 0.2, 0.9, 1.0, 0.85)
                if scanning then
                    local pct = math.floor((scan_progress / 3.0) * 100)
                    set_world_label(n.head, "ESCANEANDO " .. pct .. "%", 0.85, 0.2, 0.95, 1.0)
                elseif inf then
                    set_world_label(n.head, "OBJETIVO: " .. inf, 0.8, 1.0, 0.78, 0.2)
                else
                    set_world_label(n.head, "OBJETIVO #" .. n.id .. " (SIN INFRACCION)", 0.8, 0.4, 0.9, 1.0)
                end
            elseif n.scanned and inf then
                set_world_label(n.head, inf, 0.75, 1.0, 0.6, 0.2)
            else
                set_world_label(n.head, "", 0.75, 1.0, 1.0, 1.0)
            end
        end
    end
end

-- ============================================================================
-- IA DEL POLICIA Y DEL TECNICO
-- ============================================================================
local function update_police(dt)
    police.anim = police.anim + dt

    -- Si esta en patrulla y hay una orden de arresto activa, ir por ella (si hay comms)
    if police.state == "PATRULLA" and comms_jam_timer <= 0.0 then
        for _, n in ipairs(npcs) do
            if n.warrant and n.state ~= "ESPOSADO" and n.state ~= "PRESO" then
                police.state = "PERSIGUE"
                police.target_npc = n.id
                break
            end
        end
    end

    local tx, tz = police.x, police.z
    local spd = police.speed_patrol

    if police.state == "PERSIGUE" and police.target_npc then
        local target = npcs[police.target_npc]
        if not target or target.state == "PRESO" then
            police.state = "PATRULLA"; police.target_npc = nil
        else
            tx, tz = target.x, target.z
            spd = police.speed_sprint
            draw_line_3d(police.x, 1.2, police.z, tx, 1.2, tz, 0.25, 0.65, 1.0, 0.75)
            local nx, nz, arrived = move_towards_2d(police.x, police.z, tx, tz, spd, dt)
            police.x, police.z = nx, nz
            if arrived or dist2d(police.x, police.z, tx, tz) < 1.35 then
                -- Verificar si es culpable o Falsa Denuncia!
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
                    set_alert("SOSPECHOSO #" .. target.id .. " ESPOSADO! Escoltando a celda...", 3.5)
                    spawn_vfx("hit_spark", target.x, 1.4, target.z)
                    play_tone(520, 780, "sine", 0.25, 0.5)
                end
            end
        end
    elseif police.state == "ESCOLTA" and police.target_npc then
        local target = npcs[police.target_npc]
        if not target or target.state ~= "ESPOSADO" then
            police.state = "PATRULLA"; police.target_npc = nil
        else
            -- Escoltar a celda (-23, -22)
            local nx, nz, arrived = move_towards_2d(police.x, police.z, -23.0, -22.0, 4.6, dt)
            police.x, police.z = nx, nz
            target.x = police.x - 0.8
            target.z = police.z + 0.4
            if arrived then
                target.state = "PRESO"
                target.warrant = false
                police.state = "PATRULLA"; police.target_npc = nil
                set_alert("REBELDE #" .. target.id .. " ENCARCELADO!", 4.0)
                play_tone(440, 880, "triangle", 0.3, 0.55)
            end
        end
    elseif police.state == "PING" and police.ping_x then
        spd = police.speed_sprint
        draw_ring_3d(police.ping_x, 0.2, police.ping_z, 1.4, 0.25, 0.75, 1.0, 0.85)
        local nx, nz, arrived = move_towards_2d(police.x, police.z, police.ping_x, police.ping_z, spd, dt)
        police.x, police.z = nx, nz
        -- Si ve alguna antena hackeada cerca, la restaura!
        for i, ant in ipairs(antennas) do
            if ant.hacked and dist2d(police.x, police.z, ant.x, ant.z) < 2.8 then
                ant.hacked = false; ant.hack_prog = 0.0
                set_material(ant.tip, { color = { 0.2, 0.85, 0.4, 1.0 }, emissive = 0.6 })
                set_light(ant.tip, 0.2, 0.9, 0.4, 2.0, 10.0)
                set_alert("POLICIA DESCONECTO TRANSMISION PIRATA EN ANTENA #" .. i, 3.5)
            end
        end
        if arrived then
            police.state = "PATRULLA"
            police.ping_x = nil; police.ping_z = nil
        end
    else
        local wp = plaza_wps[police.patrol_idx]
        local nx, nz, arrived = move_towards_2d(police.x, police.z, wp.x, wp.z, police.speed_patrol, dt)
        police.x, police.z = nx, nz
        if arrived then
            police.patrol_idx = (police.patrol_idx % #plaza_wps) + 1
        end
    end

    local bob = math.abs(math.sin(police.anim * 7.0)) * 0.07
    tween_position("police_body", police.x, 0.85 + bob, police.z, 0.0)
    tween_position("police_head", police.x, 1.85 + bob, police.z, 0.0)
    tween_position("police_beacon", police.x, 2.25 + bob, police.z, 0.0)
    local flash = (police.state == "PERSIGUE") and (math.sin(elapsed_time * 18.0) > 0 and 1.0 or 0.2) or 0.5
    set_light("police_beacon", 0.2, 0.55 * flash, 1.0, (phase == 3 or police.state == "PERSIGUE") and 3.5 or 1.8, 12.0)
    set_world_label("police_head", "POLICIA [" .. police.state .. "]", 0.75, 0.3, 0.75, 1.0)
end

local function update_technician(dt)
    tech.anim = tech.anim + dt

    if tech.state == "INSTALAR_MICRO" then
        local nx, nz, arrived = move_towards_2d(tech.x, tech.z, -22.5, 9.5, tech.speed, dt)
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
        if tech.repair_kind == "cam" then
            local c = cameras[tech.repair_idx]; tx, tz = c.x, c.z
        elseif tech.repair_kind == "fuse" then
            local fb = fuse_boxes[tech.repair_idx]; tx, tz = fb.x, fb.z
        end
        local nx, nz, arrived = move_towards_2d(tech.x, tech.z, tx, tz, tech.speed * 1.2, dt)
        tech.x, tech.z = nx, nz
        if arrived then
            tech.repair_prog = tech.repair_prog + dt
            if tech.repair_prog >= 4.0 then
                if tech.repair_kind == "cam" then
                    cameras[tech.repair_idx].blind_timer = 0.0
                    cameras[tech.repair_idx].loop_timer = 0.0
                    set_alert("TECNICO REPARO " .. cameras[tech.repair_idx].name, 3.0)
                else
                    local fb = fuse_boxes[tech.repair_idx]
                    fb.down_timer = 0.0
                    set_material(fb.id, { color = { 0.9, 0.6, 0.15, 1.0 }, emissive = 0.12 })
                    set_alert("TECNICO RESTAURO ENERGIA EN " .. fb.name, 3.0)
                end
                spawn_vfx("hit_spark", tx, 1.4, tz)
                tech.state = "PATRULLA"; tech.repair_prog = 0.0; tech.repair_kind = nil
            end
        end
    else
        -- Buscar averias para reparar
        for i, fb in ipairs(fuse_boxes) do
            if fb.down_timer > 0.0 then
                tech.state = "REPARAR"; tech.repair_kind = "fuse"; tech.repair_idx = i; tech.repair_prog = 0.0
                break
            end
        end
        if tech.state == "PATRULLA" then
            for i = 1, 5 do
                if cameras[i].blind_timer > 0.0 or cameras[i].loop_timer > 0.0 then
                    tech.state = "REPARAR"; tech.repair_kind = "cam"; tech.repair_idx = i; tech.repair_prog = 0.0
                    break
                end
            end
        end
        if tech.state == "PATRULLA" then
            local wp = plaza_wps[tech.patrol_idx]
            local nx, nz, arrived = move_towards_2d(tech.x, tech.z, wp.x, wp.z, tech.speed * 0.75, dt)
            tech.x, tech.z = nx, nz
            if arrived then
                tech.patrol_idx = (tech.patrol_idx % #plaza_wps) + 1
            end
        end
    end

    local bob = math.abs(math.sin(tech.anim * 6.5)) * 0.06
    tween_position("tech_body", tech.x, 0.85 + bob, tech.z, 0.0)
    tween_position("tech_head", tech.x, 1.80 + bob, tech.z, 0.0)
    tween_position("tech_pack", tech.x, 1.10 + bob, tech.z - 0.35, 0.0)
    set_world_label("tech_head", "TECNICO [" .. tech.state .. "]", 0.7, 0.35, 0.9, 0.75)
end

-- ============================================================================
-- HUD DECLARATIVO PANOPTICON (lua_ui)
-- ============================================================================
local function draw_hud()
    local mins = math.floor(match_time / 60.0)
    local secs = math.floor(match_time % 60.0)
    local phase_name = (phase == 1) and "FASE 1: DIA" or ((phase == 2) and "FASE 2: ATARDECER" or "FASE 3: TOQUE DE QUEDA")

    local jailed_rebels = 0
    local hacked_ants = 0
    for _, n in ipairs(npcs) do
        if n.is_rebel and n.state == "PRESO" then jailed_rebels = jailed_rebels + 1 end
    end
    for _, a in ipairs(antennas) do
        if a.hacked then hacked_ants = hacked_ants + 1 end
    end

    -- Panel Superior Izquierdo: Estado de Partida, Leyes y Energia
    ui_panel("pnl_top_l", "top_left", 10, 10, 325, 114, 0.06, 0.09, 0.14, 0.90)
    ui_label("lbl_title", "top_left", 20, 18, string.format("PANOPTICON | %02d:%02d | %s", mins, secs, phase_name), 12, 0.25, 0.90, 1.0)
    ui_bar("bar_energy", "top_left", 20, 38, 305, 16, energy / max_energy, string.format("Energia: %.0f / 100", energy), 0.2, 0.75, 1.0)

    local leyes_str = (phase == 1) and "Leyes: I (No Correr), III (Contrabando), IV (Infraest.)"
        or ((phase == 2) and "Leyes: I, II (No Aglomerarse), III, IV"
        or "Leyes: I, II, III, IV, V (TOQUE DE QUEDA)")
    ui_label("lbl_laws", "top_left", 20, 62, leyes_str, 10, 0.95, 0.85, 0.45)

    local obj_str = string.format("Capturados: %d/3 | Antenas: %d/3 | Transm: %.0f/%.0fs", jailed_rebels, hacked_ants, antenna_hold_timer, ANTENNA_HOLD_GOAL)
    ui_label("lbl_objs", "top_left", 20, 80, obj_str, 11, 0.9, 0.95, 1.0)

    local srv_str = string.format("Servidor Central: %.0f%% intacto", math.max(0.0, (1.0 - server_hack_progress / SERVER_HACK_GOAL) * 100.0))
    ui_label("lbl_srv", "top_left", 20, 98, srv_str, 10, (server_hack_progress > 0 or hacked_ants == 3) and 1.0 or 0.72, 0.80, 0.88)

    -- Panel Superior Derecho: Camara Activa & Escaner
    ui_panel("pnl_top_r", "top_right", 10, 10, 276, 114, 0.06, 0.09, 0.14, 0.90)
    local c = cameras[active_cam]
    local cam_ok = (active_cam == 0) or is_cam_operational(active_cam)
    local cam_status = (active_cam == 0) and "VENTANA BLINDADA"
        or (c.blind_timer > 0 and string.format("CIEGA (%.0fs)", c.blind_timer)
        or ((c.sector > 0 and not is_sector_powered(c.sector)) and "SIN ENERGIA"
        or (c.loop_timer > 0 and string.format("LOOP FALSO (%.0fs)", c.loop_timer) or "EN LINEA")))
    ui_label("lbl_cam", "top_right", 276, 18, "MON: " .. c.name, 12, 0.3, 1.0, 0.7)
    ui_label("lbl_cam_st", "top_right", 276, 36, "Estado: " .. cam_status, 11, cam_ok and 0.4 or 1.0, cam_ok and 0.95 or 0.3, 0.35)
    ui_label("lbl_ctrl_1", "top_right", 276, 54, "WASD: Rotar | Q/E: Zoom | TAB: Objetivo", 10, 0.8, 0.85, 0.92)

    local tgt = npcs[selected_npc_idx]
    local tgt_txt = tgt and string.format("Objetivo: #%d (%s)", tgt.id, tgt.state) or "Sin objetivo"
    ui_label("lbl_tgt", "top_right", 276, 72, tgt_txt, 11, 0.95, 0.85, 0.3)
    ui_bar("bar_scan", "top_right", 20, 92, 256, 18, scan_progress / 3.0,
        scanning and string.format("Escaneando... %.1fs / 3.0s", scan_progress) or "ESPACIO: Escanear (3s)", 0.25, 0.95, 0.85)

    -- Alerta Central Superior (debajo de las tarjetas)
    if alert_banner_timer > 0.0 then
        ui_panel("pnl_alert", "top_center", 0, 130, 460, 28, 0.16, 0.08, 0.08, 0.90)
        ui_label("lbl_alert", "top_center", -210, 137, alert_banner, 11, 1.0, 0.85, 0.3)
    end

    -- Barra Inferior Compacta: Selector de Camaras + Botones de Accion
    ui_panel("pnl_bot", "bottom_center", 0, 10, 560, 68, 0.06, 0.09, 0.14, 0.90)
    ui_button("btn_c0", "bottom_center", -225, 48, 84, 22, "[0] Torre", "on_cam_0")
    ui_button("btn_c1", "bottom_center", -135, 48, 84, 22, "[1] Plaza N", "on_cam_1")
    ui_button("btn_c2", "bottom_center",  -45, 48, 84, 22, "[2] Plaza S", "on_cam_2")
    ui_button("btn_c3", "bottom_center",   45, 48, 84, 22, "[3] Merc.", "on_cam_3")
    ui_button("btn_c4", "bottom_center",  135, 48, 84, 22, "[4] Subest.", "on_cam_4")
    ui_button("btn_c5", "bottom_center",  225, 48, 88, 22, cam_micro_installed and "[5] Micro" or "[5] Pend.", "on_cam_5")

    ui_button("btn_tab",  "bottom_center", -216, 18, 102, 22, "[TAB] Obj.", "on_btn_next_target")
    ui_button("btn_scan", "bottom_center", -108, 18, 102, 22, "[ESP] Scan", "on_btn_scan")
    ui_button("btn_ping", "bottom_center",    0, 18, 102, 22, "[P] Ping", "on_btn_ping")
    ui_button("btn_lock", "bottom_center",  108, 18, 102, 22, lockdown_timer > 0 and string.format("Lock %.0fs", lockdown_timer) or "[B] Bloqueo", "on_btn_lockdown")
    ui_button("btn_ph",   "bottom_center",  216, 18, 102, 22, "[F] +Fase", "on_btn_phase")

    -- Modal de Victoria / Derrota
    if game_over then
        local sys_win = (winner == "SISTEMA")
        ui_panel("pnl_end", "center", 0, 0, 440, 140, sys_win and 0.05 or 0.25, sys_win and 0.18 or 0.06, sys_win and 0.24 or 0.08, 0.95)
        ui_label("lbl_end_1", "center", -175, -40, sys_win and "VICTORIA DEL SISTEMA (PANOPTICON)" or "VICTORIA DE LOS REBELDES", 15, sys_win and 0.3 or 1.0, sys_win and 0.95 or 0.35, sys_win and 1.0 or 0.35)
        ui_label("lbl_end_2", "center", -185, -10, win_reason, 11, 0.92, 0.95, 1.0)
        ui_button("btn_rst", "center", 0, 32, 160, 30, "Nueva Partida [R]", "on_btn_reset")
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
        if comms_jam_timer > 0.0 then
            comms_jam_timer = math.max(0.0, comms_jam_timer - dt)
        end

        -- Timers de camaras y fusibles
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

        -- Escaner Biometrico (3s)
        if scanning then
            local tgt = npcs[selected_npc_idx]
            if not tgt or tgt.state == "PRESO" then
                scanning = false; scan_progress = 0.0
            else
                local ex, ey, ez = get_cam_eye_pos(active_cam)
                draw_line_3d(ex, ey - 0.3, ez, tgt.x, 1.4, tgt.z, 0.2, 0.95, 1.0, 0.9)
                scan_progress = scan_progress + dt
                if scan_progress >= 3.0 then
                    scanning = false
                    scan_progress = 0.0
                    energy = math.max(0.0, energy - 12.0)
                    tgt.scanned = true
                    local inf = check_npc_active_infraction(tgt)
                    tgt.infraction = inf
                    tgt.warrant = true
                    if inf then
                        set_alert("ORDEN DE ARRESTO EMITIDA (#" .. tgt.id .. "): " .. inf, 4.0)
                        play_tone(620, 940, "sine", 0.25, 0.55)
                    else
                        set_alert("ORDEN FORZADA SIN INFRACCION (#" .. tgt.id .. ") - RIESGO DE FALSA DENUNCIA", 4.0)
                        play_tone(320, 260, "triangle", 0.25, 0.45)
                    end
                    if comms_jam_timer <= 0.0 then
                        police.state = "PERSIGUE"
                        police.target_npc = tgt.id
                    end
                end
            end
        end

        update_day_night_cycle()
        update_operator_camera(dt)
        update_npcs(dt)
        update_police(dt)
        update_technician(dt)

        -- Condiciones de Victoria
        local jailed = 0
        for _, n in ipairs(npcs) do
            if n.is_rebel and n.state == "PRESO" then jailed = jailed + 1 end
        end
        if jailed >= 3 then
            game_over = true
            winner = "SISTEMA"
            win_reason = "ORDEN RESTABLECIDO: Los 3 rebeldes fueron capturados."
            play_tone(440, 880, "triangle", 0.6, 0.7)
        elseif match_time <= 0.0 then
            game_over = true
            winner = "SISTEMA"
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
                game_over = true
                winner = "REBELDES"
                win_reason = "LEVANTAMIENTO POPULAR: 3 Antenas transmitieron 35s."
                play_tone(240, 120, "sawtooth", 0.6, 0.7)
            end
        else
            antenna_hold_timer = math.max(0.0, antenna_hold_timer - dt * 0.5)
        end
    else
        if is_key_just_pressed("R") then reset_panopticon() end
    end

    draw_hud()
end
