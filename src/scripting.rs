use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use glam::Vec3;
use crate::assets::{EntityDef, PhysicsDef};
use crate::ecs::Transform;

#[derive(Clone)]
pub struct LuaEngineContext {
    pub pressed_keys: Arc<RwLock<HashSet<String>>>,
    pub pending_spawns: Arc<Mutex<Vec<EntityDef>>>,
    pub pending_destroys: Arc<Mutex<Vec<String>>>,
    pub pending_parents: Arc<Mutex<Vec<(String, Option<String>)>>>,
    pub pending_particles: Arc<Mutex<Vec<crate::particles::ParticleSpawnRequest>>>,
    pub hud_texts: Arc<RwLock<HashMap<String, String>>>,
    pub world_entities: Arc<RwLock<HashMap<String, [f32; 9]>>>,
    pub audio: Arc<Mutex<crate::audio::AudioManager>>,
}

impl LuaEngineContext {
    pub fn new(
        pressed_keys: Arc<RwLock<HashSet<String>>>,
        audio: Arc<Mutex<crate::audio::AudioManager>>,
    ) -> Self {
        Self {
            pressed_keys,
            pending_spawns: Arc::new(Mutex::new(Vec::new())),
            pending_destroys: Arc::new(Mutex::new(Vec::new())),
            pending_parents: Arc::new(Mutex::new(Vec::new())),
            pending_particles: Arc::new(Mutex::new(Vec::new())),
            hud_texts: Arc::new(RwLock::new(HashMap::new())),
            world_entities: Arc::new(RwLock::new(HashMap::new())),
            audio,
        }
    }
}

pub struct ScriptComponent {
    pub path: String,
    pub lua: Option<Arc<Mutex<mlua::Lua>>>,
    pub last_error: Option<String>,
    pub has_error: bool,
    pub pending_forces: Arc<Mutex<Vec<Vec3>>>,
    pub pending_impulses: Arc<Mutex<Vec<Vec3>>>,
}

impl ScriptComponent {
    pub fn new(path: String) -> Self {
        Self {
            path,
            lua: None,
            last_error: None,
            has_error: false,
            pending_forces: Arc::new(Mutex::new(Vec::new())),
            pending_impulses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn init_or_reload(&mut self, ctx: LuaEngineContext) -> Result<(), String> {
        let path = Path::new(&self.path);
        if !path.exists() {
            let err = format!("Script file '{}' not found on disk", self.path);
            self.last_error = Some(err.clone());
            self.has_error = true;
            return Err(err);
        }

        let code = fs::read_to_string(path)
            .map_err(|e| format!("Error reading script '{}': {}", self.path, e))?;

        let lua = mlua::Lua::new();

        // 1. is_key_down(key)
        let keys_ref = ctx.pressed_keys.clone();
        let is_key_down = lua
            .create_function(move |_, key: String| {
                let keys = keys_ref.read().unwrap();
                let clean = key.trim().to_uppercase();
                Ok(keys.contains(&clean))
            })
            .map_err(|e| format!("Failed to register is_key_down: {}", e))?;
        lua.globals().set("is_key_down", is_key_down).map_err(|e| e.to_string())?;

        // 2. apply_force(fx, fy, fz)
        let forces_ref = self.pending_forces.clone();
        let apply_force = lua
            .create_function(move |_, (fx, fy, fz): (f32, f32, f32)| {
                if let Ok(mut forces) = forces_ref.lock() {
                    forces.push(Vec3::new(fx, fy, fz));
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register apply_force: {}", e))?;
        lua.globals().set("apply_force", apply_force).map_err(|e| e.to_string())?;

        // 3. apply_impulse(ix, iy, iz)
        let impulses_ref = self.pending_impulses.clone();
        let apply_impulse = lua
            .create_function(move |_, (ix, iy, iz): (f32, f32, f32)| {
                if let Ok(mut impulses) = impulses_ref.lock() {
                    impulses.push(Vec3::new(ix, iy, iz));
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register apply_impulse: {}", e))?;
        lua.globals().set("apply_impulse", apply_impulse).map_err(|e| e.to_string())?;

        // 4. spawn_entity(def_table)
        let spawns_ref = ctx.pending_spawns.clone();
        let spawn_entity = lua
            .create_function(move |_, table: mlua::Table| {
                let id: String = table.get("id").unwrap_or_else(|_| "spawned_entity".to_string());
                let mesh: String = table.get("mesh").unwrap_or_else(|_| "cube".to_string());
                let x: f32 = table.get("x").unwrap_or(0.0);
                let y: f32 = table.get("y").unwrap_or(0.0);
                let z: f32 = table.get("z").unwrap_or(0.0);
                let rx: f32 = table.get("rx").unwrap_or(0.0);
                let ry: f32 = table.get("ry").unwrap_or(0.0);
                let rz: f32 = table.get("rz").unwrap_or(0.0);
                let sx: f32 = table.get("sx").unwrap_or(1.0);
                let sy: f32 = table.get("sy").unwrap_or(1.0);
                let sz: f32 = table.get("sz").unwrap_or(1.0);
                let color: [f32; 4] = if let Ok(col_tbl) = table.get::<mlua::Table>("color") {
                    [
                        col_tbl.get(1).unwrap_or(1.0),
                        col_tbl.get(2).unwrap_or(1.0),
                        col_tbl.get(3).unwrap_or(1.0),
                        col_tbl.get(4).unwrap_or(1.0),
                    ]
                } else {
                    [0.8, 0.8, 0.8, 1.0]
                };
                let emissive: f32 = table.get("emissive").unwrap_or(0.0);
                let texture: Option<String> = table.get("texture").ok();
                let script: Option<String> = table.get("script").ok();

                let phys_def = if let Ok(pt) = table.get::<mlua::Table>("physics") {
                    Some(PhysicsDef {
                        body: pt.get("body").unwrap_or_else(|_| "dynamic".to_string()),
                        collider: pt.get("collider").unwrap_or_else(|_| "cuboid".to_string()),
                        mass: pt.get("mass").unwrap_or(1.0),
                        restitution: pt.get("restitution").unwrap_or(0.4),
                    })
                } else {
                    None
                };

                let parent: Option<String> = table.get("parent").ok();
                let entity_def = EntityDef {
                    id,
                    mesh,
                    position: [x, y, z],
                    rotation: [rx, ry, rz],
                    scale: [sx, sy, sz],
                    texture,
                    parent,
                    color,
                    emissive,
                    script,
                    physics: phys_def,
                };

                if let Ok(mut spawns) = spawns_ref.lock() {
                    spawns.push(entity_def);
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register spawn_entity: {}", e))?;
        lua.globals().set("spawn_entity", spawn_entity).map_err(|e| e.to_string())?;

        // 5. destroy_entity(id)
        let destroys_ref = ctx.pending_destroys.clone();
        let destroy_entity = lua
            .create_function(move |_, id: String| {
                if let Ok(mut list) = destroys_ref.lock() {
                    list.push(id);
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register destroy_entity: {}", e))?;
        lua.globals().set("destroy_entity", destroy_entity).map_err(|e| e.to_string())?;

        // 6. get_entity(id)
        let entities_ref = ctx.world_entities.clone();
        let get_entity = lua
            .create_function(move |lua, id: Option<String>| {
                if let Some(id) = id {
                    let map = entities_ref.read().unwrap();
                    if let Some(t) = map.get(&id) {
                        let tbl = lua.create_table()?;
                        tbl.set("x", t[0])?;
                        tbl.set("y", t[1])?;
                        tbl.set("z", t[2])?;
                        tbl.set("rx", t[3])?;
                        tbl.set("ry", t[4])?;
                        tbl.set("rz", t[5])?;
                        tbl.set("sx", t[6])?;
                        tbl.set("sy", t[7])?;
                        tbl.set("sz", t[8])?;
                        Ok(Some(tbl))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            })
            .map_err(|e| format!("Failed to register get_entity: {}", e))?;
        lua.globals().set("get_entity", get_entity).map_err(|e| e.to_string())?;

        // 7. set_hud_text(key, text)
        let hud_ref = ctx.hud_texts.clone();
        let set_hud_text = lua
            .create_function(move |_, (key, text): (String, String)| {
                if let Ok(mut map) = hud_ref.write() {
                    map.insert(key, text);
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register set_hud_text: {}", e))?;
        lua.globals().set("set_hud_text", set_hud_text).map_err(|e| e.to_string())?;

        // 8. play_sound(path, [volume])
        let audio_sound = ctx.audio.clone();
        let play_sound = lua
            .create_function(move |_, (path, volume): (String, Option<f32>)| {
                let vol = volume.unwrap_or(1.0);
                if let Ok(audio) = audio_sound.lock() {
                    let _ = audio.play_sound(&path, vol);
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register play_sound: {}", e))?;
        lua.globals().set("play_sound", play_sound).map_err(|e| e.to_string())?;

        // 9. play_music(path, [volume])
        let audio_music = ctx.audio.clone();
        let play_music = lua
            .create_function(move |_, (path, volume): (String, Option<f32>)| {
                let vol = volume.unwrap_or(1.0);
                if let Ok(mut audio) = audio_music.lock() {
                    let _ = audio.play_music(&path, vol);
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register play_music: {}", e))?;
        lua.globals().set("play_music", play_music).map_err(|e| e.to_string())?;

        // 10. stop_music()
        let audio_stop = ctx.audio.clone();
        let stop_music = lua
            .create_function(move |_, ()| {
                if let Ok(mut audio) = audio_stop.lock() {
                    audio.stop_music();
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register stop_music: {}", e))?;
        lua.globals().set("stop_music", stop_music).map_err(|e| e.to_string())?;

        // 11. spawn_particles(x, y, z, r, g, b, [count], [speed], [lifetime])
        let particles_ref = ctx.pending_particles.clone();
        let spawn_particles = lua
            .create_function(
                move |_,
                      (x, y, z, r, g, b, count, speed, lifetime): (
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    Option<usize>,
                    Option<f32>,
                    Option<f32>,
                )| {
                    let req = crate::particles::ParticleSpawnRequest {
                        position: [x, y, z],
                        color: [r, g, b, 1.0],
                        count: count.unwrap_or(30),
                        speed: speed.unwrap_or(4.0),
                        lifetime: lifetime.unwrap_or(1.0),
                    };
                    if let Ok(mut list) = particles_ref.lock() {
                        list.push(req);
                    }
                    Ok(())
                },
            )
            .map_err(|e| format!("Failed to register spawn_particles: {}", e))?;
        lua.globals()
            .set("spawn_particles", spawn_particles)
            .map_err(|e| e.to_string())?;

        // 12. spawn_prefab(name, [x], [y], [z])
        let spawns_ref_prefab = ctx.pending_spawns.clone();
        let spawn_prefab = lua
            .create_function(
                move |_, (name, x, y, z): (String, Option<f32>, Option<f32>, Option<f32>)| {
                    let file_path = if name.ends_with(".json") {
                        if name.starts_with("assets/prefabs/") {
                            name.clone()
                        } else {
                            format!("assets/prefabs/{}", name)
                        }
                    } else {
                        format!("assets/prefabs/{}.json", name)
                    };

                    match crate::assets::load_prefab_from_file(&file_path) {
                        Ok(mut def) => {
                            let ts = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.subsec_nanos())
                                .unwrap_or(1);
                            def.id = format!("{}_{}", def.id, ts % 10000);
                            def.position = [x.unwrap_or(0.0), y.unwrap_or(0.0), z.unwrap_or(0.0)];
                            if let Ok(mut list) = spawns_ref_prefab.lock() {
                                list.push(def);
                            }
                            Ok(true)
                        }
                        Err(e) => {
                            eprintln!("[Lua] Failed to spawn prefab '{}': {}", name, e);
                            Ok(false)
                        }
                    }
                },
            )
            .map_err(|e| format!("Failed to register spawn_prefab: {}", e))?;
        lua.globals()
            .set("spawn_prefab", spawn_prefab)
            .map_err(|e| e.to_string())?;

        // 13. set_parent(child_id, parent_id)
        let parents_ref = ctx.pending_parents.clone();
        let set_parent = lua
            .create_function(
                move |_, (child_id, parent_id): (String, Option<String>)| {
                    if let Ok(mut list) = parents_ref.lock() {
                        let clean_parent = parent_id.and_then(|p| {
                            let trimmed = p.trim().to_string();
                            if trimmed.is_empty() {
                                None
                            } else {
                                Some(trimmed)
                            }
                        });
                        list.push((child_id, clean_parent));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_parent: {}", e))?;
        lua.globals()
            .set("set_parent", set_parent)
            .map_err(|e| e.to_string())?;

        // Execute the script chunk to define on_update, on_collision, and globals
        lua.load(&code)
            .exec()
            .map_err(|e| format!("Lua syntax/runtime error in '{}': {}", self.path, e))?;

        self.lua = Some(Arc::new(Mutex::new(lua)));
        self.last_error = None;
        self.has_error = false;
        Ok(())
    }

    pub fn on_collision(&self, self_id: &str, other_id: &str) -> Result<(), String> {
        if self.has_error {
            return Ok(());
        }

        if let Some(lua_arc) = &self.lua {
            let lua = lua_arc.lock().map_err(|e| e.to_string())?;
            let on_col_fn: mlua::Result<mlua::Function> = lua.globals().get("on_collision");
            if let Ok(func) = on_col_fn {
                let entity_tbl = lua.create_table().map_err(|e| e.to_string())?;
                entity_tbl.set("id", self_id).map_err(|e| e.to_string())?;
                func.call::<()>((entity_tbl, other_id))
                    .map_err(|e| format!("Lua on_collision error on '{}': {}", self_id, e))?;
            }
        }
        Ok(())
    }

    pub fn update(&mut self, transform: &mut Transform, dt: f32) -> Result<(), String> {
        if self.has_error {
            return Ok(());
        }

        if let Some(lua_arc) = &self.lua {
            let lua = lua_arc.lock().map_err(|e| e.to_string())?;
            let on_update_fn: mlua::Result<mlua::Function> = lua.globals().get("on_update");
            match on_update_fn {
                Ok(func) => {
                    let entity_table = lua
                        .create_table()
                        .map_err(|e| format!("Error creating Lua entity table: {}", e))?;

                    entity_table
                        .set("x", transform.position.x)
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("y", transform.position.y)
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("z", transform.position.z)
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("rx", transform.rotation.x)
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("ry", transform.rotation.y)
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("rz", transform.rotation.z)
                        .map_err(|e| e.to_string())?;

                    let forces_ref = self.pending_forces.clone();
                    let entity_apply_force = lua
                        .create_function(move |_, (fx, fy, fz): (f32, f32, f32)| {
                            if let Ok(mut forces) = forces_ref.lock() {
                                forces.push(Vec3::new(fx, fy, fz));
                            }
                            Ok(())
                        })
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("apply_force", entity_apply_force)
                        .map_err(|e| e.to_string())?;

                    let impulses_ref = self.pending_impulses.clone();
                    let entity_apply_impulse = lua
                        .create_function(move |_, (ix, iy, iz): (f32, f32, f32)| {
                            if let Ok(mut impulses) = impulses_ref.lock() {
                                impulses.push(Vec3::new(ix, iy, iz));
                            }
                            Ok(())
                        })
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("apply_impulse", entity_apply_impulse)
                        .map_err(|e| e.to_string())?;

                    let call_res: mlua::Result<()> = func.call((entity_table.clone(), dt));
                    match call_res {
                        Ok(()) => {
                            if let Ok(x) = entity_table.get("x") {
                                transform.position.x = x;
                            }
                            if let Ok(y) = entity_table.get("y") {
                                transform.position.y = y;
                            }
                            if let Ok(z) = entity_table.get("z") {
                                transform.position.z = z;
                            }
                            if let Ok(rx) = entity_table.get("rx") {
                                transform.rotation.x = rx;
                            }
                            if let Ok(ry) = entity_table.get("ry") {
                                transform.rotation.y = ry;
                            }
                            if let Ok(rz) = entity_table.get("rz") {
                                transform.rotation.z = rz;
                            }
                            Ok(())
                        }
                        Err(e) => {
                            let err_msg = format!("Lua runtime error in '{}': {}", self.path, e);
                            self.last_error = Some(err_msg.clone());
                            self.has_error = true;
                            Err(err_msg)
                        }
                    }
                }
                Err(_) => Ok(()),
            }
        } else {
            Ok(())
        }
    }
}
