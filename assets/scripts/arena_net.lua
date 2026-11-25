-- arena_net.lua: Demo de multijugador con replicacion UDP y LERP/SLERP
local time = 0.0
local speed = 6.0
local is_hosting = false

set_hud_text("net_title", "[Motor-dif Net Arena]")
set_hud_text("net_controls", "WASD: Mover | [H]: Host Server | [C]: Conectar Cliente | [ESPACIO]: Ping")
set_hud_text("net_status", "Estado: Desconectado")

function on_update(entity, dt)
    time = time + dt

    -- Host / Connect controls
    if is_key_just_pressed("H") then
        local ok = net_host(5000)
        if ok then
            is_hosting = true
            set_hud_text("net_status", "Estado: SERVIDOR Activo (Puerto 5000)")
            log("Servidor iniciado en puerto 5000")
        end
    end

    if is_key_just_pressed("C") then
        local ok = net_connect("127.0.0.1:5000")
        if ok then
            set_hud_text("net_status", "Estado: Conectando a 127.0.0.1:5000...")
            log("Conectando cliente a 127.0.0.1:5000")
        end
    end

    -- Movement for the local controlled player
    local dx = 0.0
    local dz = 0.0

    if is_key_down("W") or is_key_down("UP") then
        dz = dz - speed * dt
    end
    if is_key_down("S") or is_key_down("DOWN") then
        dz = dz + speed * dt
    end
    if is_key_down("A") or is_key_down("LEFT") then
        dx = dx - speed * dt
    end
    if is_key_down("D") or is_key_down("RIGHT") then
        dx = dx + speed * dt
    end

    entity.x = entity.x + dx
    entity.z = entity.z + dz
    entity.ry = entity.ry + (dx * 2.0) * dt

    -- Send ping message over reliable channel 1
    if is_key_just_pressed("SPACE") then
        if net_is_server() then
            net_broadcast(1, { event = "server_pulse", time = time })
            log("Servidor envió pulso a los clientes")
        elseif net_is_client() then
            net_send(1, { event = "client_ping", time = time })
            log("Cliente envió ping al servidor")
        end
    end

    -- Update status info
    if net_is_server() then
        set_hud_text("net_status", "SERVIDOR | Peers activos: " .. tostring(net_client_id()))
    elseif net_is_client() then
        set_hud_text("net_status", "CLIENTE #" .. tostring(net_client_id()) .. " | Conectado")
    end
end

function on_net_connect(client_id)
    log("[NET EVENT] Nuevo cliente conectado: #" .. tostring(client_id))
    set_hud_text("net_last_event", "Cliente #" .. tostring(client_id) .. " conectado")
end

function on_net_disconnect(client_id)
    log("[NET EVENT] Cliente desconectado: #" .. tostring(client_id))
    set_hud_text("net_last_event", "Cliente #" .. tostring(client_id) .. " desconectado")
end

function on_net_message(sender, channel, message)
    log("[NET MSG] De #" .. tostring(sender) .. " en canal " .. tostring(channel) .. ": " .. tostring(message))
end
