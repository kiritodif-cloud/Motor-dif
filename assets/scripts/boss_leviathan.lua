-- Cosmos.io 3D: Apophis, Cosmic Void Leviathan Boss Script
local boss_hp = 1000.0
local boss_max_hp = 1000.0
local patrol_angle = 0.0
local patrol_speed = 0.35
local hit_flash = 0.0

function on_update(entity, dt)
    -- Elliptical orbit around the cosmic perimeter
    patrol_angle = patrol_angle + patrol_speed * dt
    entity.x = math.cos(patrol_angle) * 48.0
    entity.z = math.sin(patrol_angle) * 36.0
    entity.y = math.sin(patrol_angle * 2.0) * 1.5

    -- Menacing self-rotation
    entity.ry = entity.ry + 0.9 * dt
    entity.rx = entity.rx + 0.4 * dt

    if hit_flash > 0.0 then
        hit_flash = hit_flash - dt
    end

    -- Check collision with plasma shots and mines
    local all = get_all_entities()
    if all then
        for id, ent in pairs(all) do
            if string.sub(id, 1, 12) == "plasma_shot_" or string.sub(id, 1, 15) == "planet_missile_" then
                local dx = ent.x - entity.x
                local dz = ent.z - entity.z
                local dist = math.sqrt(dx * dx + dz * dz)
                if dist < (entity.sx * 0.5) + (ent.sx * 0.5) then
                    local damage = string.sub(id, 1, 14) == "planet_missile" and 120.0 or 45.0
                    boss_hp = math.max(0.0, boss_hp - damage)
                    hit_flash = 0.25
                    destroy_entity(id)
                    spawn_particles(ent.x, 0.0, ent.z, 1.0, 0.3, 0.1, 16, 5.0, 0.5)
                    camera_shake(0.6, 0.2)
                end
            elseif string.sub(id, 1, 5) == "mine_" then
                local dx = ent.x - entity.x
                local dz = ent.z - entity.z
                local dist = math.sqrt(dx * dx + dz * dz)
                if dist < (entity.sx * 0.5) + (ent.sx * 0.5) then
                    boss_hp = math.max(0.0, boss_hp - 180.0)
                    hit_flash = 0.4
                    destroy_entity(id)
                    spawn_particles(ent.x, 0.0, ent.z, 1.0, 0.1, 0.2, 35, 9.0, 0.8)
                    camera_shake(1.4, 0.35)
                end
            end
        end
    end

    -- Check if defeated
    if boss_hp <= 0.0 then
        -- Supernova explosion!
        spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.8, 0.1, 60, 14.0, 1.2)
        spawn_particles(entity.x, 0.0, entity.z, 1.0, 0.2, 0.4, 40, 10.0, 1.0)
        camera_shake(2.0, 0.6)

        -- Spawn Hypernova Core
        spawn_entity({
            id = "hypernova_core",
            mesh = "sphere",
            x = entity.x,
            y = 0.0,
            z = entity.z,
            sx = 2.4,
            sy = 2.4,
            sz = 2.4,
            color = { 1.0, 0.9, 0.1, 1.0 },
            emissive = 1.0
        })

        set_hud_text("08_JEFE", "☠️ ¡LEVIATÁN APOPHIS DERROTADO! NÚCLEO DE HIPERNOVA LIBERADO (+500 TJ)")
        destroy_entity("boss_apophis")
        return
    end

    -- Update Boss HUD Bar
    local pct = math.floor((boss_hp / boss_max_hp) * 100)
    set_hud_text("08_JEFE", string.format("Apophis, Leviatán del Vacío — [%d / %d HP] (%d%%)", math.floor(boss_hp), math.floor(boss_max_hp), pct))
end
