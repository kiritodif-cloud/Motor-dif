use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use glam::Vec3;
use crate::assets::{EntityDef, PhysicsDef};
use crate::ecs::Transform;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HudButtonDef {
    pub id: String,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub callback: String,
}

pub fn lua_to_json(val: &mlua::Value) -> serde_json::Value {
    match val {
        mlua::Value::Nil => serde_json::Value::Null,
        mlua::Value::Boolean(b) => serde_json::Value::Bool(*b),
        mlua::Value::Integer(i) => serde_json::json!(*i),
        mlua::Value::Number(n) => serde_json::json!(*n),
        mlua::Value::String(s) => {
            if let Ok(str_val) = s.to_str() {
                serde_json::Value::String(str_val.to_string())
            } else {
                serde_json::Value::Null
            }
        }
        mlua::Value::Table(t) => {
            let mut is_array = true;
            let mut max_idx = 0;
            for pair in t.clone().pairs::<mlua::Value, mlua::Value>() {
                if let Ok((mlua::Value::Integer(idx), _)) = pair {
                    if idx > 0 {
                        max_idx = max_idx.max(idx as usize);
                        continue;
                    }
                }
                is_array = false;
                break;
            }
            if is_array && max_idx > 0 {
                let mut arr = Vec::with_capacity(max_idx);
                for i in 1..=max_idx {
                    let v: mlua::Value = t.get(i).unwrap_or(mlua::Value::Nil);
                    arr.push(lua_to_json(&v));
                }
                serde_json::Value::Array(arr)
            } else {
                let mut map = serde_json::Map::new();
                for pair in t.clone().pairs::<String, mlua::Value>() {
                    if let Ok((k, v)) = pair {
                        map.insert(k, lua_to_json(&v));
                    }
                }
                serde_json::Value::Object(map)
            }
        }
        _ => serde_json::Value::Null,
    }
}

pub fn json_to_lua(lua: &mlua::Lua, val: &serde_json::Value) -> mlua::Result<mlua::Value> {
    match val {
        serde_json::Value::Null => Ok(mlua::Value::Nil),
        serde_json::Value::Bool(b) => Ok(mlua::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(mlua::Value::Integer(i))
            } else {
                Ok(mlua::Value::Number(n.as_f64().unwrap_or(0.0)))
            }
        }
        serde_json::Value::String(s) => Ok(mlua::Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let tbl = lua.create_table()?;
            for (idx, item) in arr.iter().enumerate() {
                tbl.set(idx + 1, json_to_lua(lua, item)?)?;
            }
            Ok(mlua::Value::Table(tbl))
        }
        serde_json::Value::Object(map) => {
            let tbl = lua.create_table()?;
            for (k, v) in map {
                tbl.set(k.as_str(), json_to_lua(lua, v)?)?;
            }
            Ok(mlua::Value::Table(tbl))
        }
    }
}

#[derive(Clone)]
pub struct LuaEngineContext {
    pub pressed_keys: Arc<RwLock<HashSet<String>>>,
    pub just_pressed_keys: Arc<RwLock<HashSet<String>>>,
    pub pending_spawns: Arc<Mutex<Vec<EntityDef>>>,
    pub pending_destroys: Arc<Mutex<Vec<String>>>,
    pub pending_parents: Arc<Mutex<Vec<(String, Option<String>)>>>,
    pub pending_lights: Arc<Mutex<Vec<(String, [f32; 3], f32, f32)>>>,
    pub pending_particles: Arc<Mutex<Vec<crate::particles::ParticleSpawnRequest>>>,
    pub pending_camera_sets: Arc<Mutex<Vec<([f32; 3], [f32; 3])>>>,
    pub pending_camera_follows: Arc<Mutex<Vec<(Option<String>, [f32; 3], f32)>>>,
    pub pending_camera_shakes: Arc<Mutex<Vec<(f32, f32)>>>,
    pub pending_camera_lerps: Arc<Mutex<Vec<([f32; 3], [f32; 3], f32)>>>,
    pub pending_camera_orbits: Arc<Mutex<Vec<(Option<String>, f32, f32, f32)>>>,
    pub pending_tweens: Arc<Mutex<Vec<crate::tweens::TweenRequest>>>,
    pub pending_scene_load: Arc<Mutex<Option<String>>>,
    pub pending_sprite_frames: Arc<Mutex<Vec<(String, u32, u32, u32)>>>,
    pub save_data_store: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub hud_buttons: Arc<RwLock<HashMap<String, HudButtonDef>>>,
    pub pending_button_clicks: Arc<Mutex<Vec<String>>>,
    pub hud_texts: Arc<RwLock<HashMap<String, String>>>,
    pub world_entities: Arc<RwLock<HashMap<String, [f32; 9]>>>,
    pub audio: Arc<Mutex<crate::audio::AudioManager>>,
    pub pending_post_process: Arc<Mutex<Option<crate::assets::PostProcessDef>>>,
    pub post_process_settings: Arc<RwLock<crate::assets::PostProcessDef>>,
    pub pending_materials: Arc<Mutex<Vec<MaterialUpdate>>>,
    pub pending_trails: Arc<Mutex<Vec<TrailCommand>>>,
    pub debug_lines: Arc<Mutex<Vec<DebugLine3D>>>,
    pub debug_rings: Arc<Mutex<Vec<DebugRing3D>>>,
    pub world_labels: Arc<RwLock<HashMap<String, WorldLabel>>>,
    pub floating_texts: Arc<Mutex<Vec<FloatingText>>>,
    pub pending_environment: Arc<Mutex<Option<crate::assets::EnvironmentDef>>>,
    pub environment_settings: Arc<RwLock<crate::assets::EnvironmentDef>>,
    pub physics_raycaster: Arc<RwLock<Option<PhysicsRaycaster>>>,
    pub game_ui_elements: Arc<RwLock<Vec<crate::game_ui::UiElement>>>,
    pub pending_anims: Arc<Mutex<Vec<AnimationCommand>>>,
    pub anim_playing_states: Arc<RwLock<HashMap<String, bool>>>,
    pub event_listeners: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub pending_events: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    pub timers: Arc<Mutex<Vec<LuaTimer>>>,
    pub next_timer_id: Arc<std::sync::atomic::AtomicU64>,
    pub input_map: Arc<RwLock<crate::input_map::InputMap>>,
    pub mouse_info: Arc<RwLock<crate::input_map::MouseInfo>>,
    pub active_camera: Arc<RwLock<crate::camera::Camera>>,
    pub nav_grid: Arc<RwLock<crate::nav::NavGrid>>,
    pub pending_nav_bake: Arc<Mutex<Option<(f32, f32)>>>,
}

#[derive(Debug, Clone)]
pub struct LuaTimer {
    pub id: u64,
    pub callback: String,
    pub remaining: f32,
    pub interval: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum AnimationCommand {
    Play {
        entity_id: String,
        clip_name: String,
        looping: bool,
    },
    Stop {
        entity_id: String,
    },
    PlayPreset {
        entity_id: String,
        preset_name: String,
    },
}

#[derive(Clone)]
pub struct PhysicsRaycaster {
    pub broad_phase: rapier3d::geometry::DefaultBroadPhase,
    pub narrow_phase: rapier3d::geometry::NarrowPhase,
    pub rigid_body_set: rapier3d::dynamics::RigidBodySet,
    pub collider_set: rapier3d::geometry::ColliderSet,
    pub collider_to_entity: HashMap<rapier3d::geometry::ColliderHandle, String>,
}

#[derive(Debug, Clone)]
pub struct TrailCommand {
    pub entity_id: String,
    pub color: [f32; 4],
    pub lifetime: f32,
    pub width: f32,
}

#[derive(Debug, Clone)]
pub struct DebugLine3D {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct DebugRing3D {
    pub center: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct WorldLabel {
    pub entity_id: String,
    pub text: String,
    pub offset_y: f32,
    pub color: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct FloatingText {
    pub position: [f32; 3],
    pub text: String,
    pub color: [f32; 3],
    pub lifetime: f32,
    pub elapsed: f32,
}

#[derive(Debug, Clone)]
pub struct MaterialUpdate {
    pub entity_id: String,
    pub color: Option<[f32; 4]>,
    pub emissive: Option<f32>,
    pub fresnel_color: Option<[f32; 3]>,
    pub fresnel_power: Option<f32>,
    pub blend_mode: Option<String>,
    pub wireframe: Option<bool>,
    pub uv_scroll: Option<[f32; 2]>,
}

pub fn check_key_match(keys: &HashSet<String>, clean: &str) -> bool {
    if keys.contains(clean) {
        return true;
    }
    match clean {
        " " | "SPACE" => keys.contains("SPACE"),
        "SHIFT" => keys.contains("SHIFT") || keys.contains("SHIFTLEFT") || keys.contains("SHIFTRIGHT"),
        "CTRL" | "CONTROL" => keys.contains("CONTROL") || keys.contains("CONTROLLEFT") || keys.contains("CONTROLRIGHT"),
        "1" => keys.contains("DIGIT1") || keys.contains("NUMPAD1"),
        "2" => keys.contains("DIGIT2") || keys.contains("NUMPAD2"),
        "3" => keys.contains("DIGIT3") || keys.contains("NUMPAD3"),
        _ => false,
    }
}

impl LuaEngineContext {
    pub fn new(
        pressed_keys: Arc<RwLock<HashSet<String>>>,
        audio: Arc<Mutex<crate::audio::AudioManager>>,
    ) -> Self {
        let save_data_store = {
            let p = Path::new("assets/save_data.json");
            let map = if p.exists() {
                if let Ok(c) = fs::read_to_string(p) {
                    serde_json::from_str::<HashMap<String, serde_json::Value>>(&c).unwrap_or_default()
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            };
            Arc::new(RwLock::new(map))
        };

        Self {
            pressed_keys,
            just_pressed_keys: Arc::new(RwLock::new(HashSet::new())),
            pending_spawns: Arc::new(Mutex::new(Vec::new())),
            pending_destroys: Arc::new(Mutex::new(Vec::new())),
            pending_parents: Arc::new(Mutex::new(Vec::new())),
            pending_lights: Arc::new(Mutex::new(Vec::new())),
            pending_particles: Arc::new(Mutex::new(Vec::new())),
            pending_camera_sets: Arc::new(Mutex::new(Vec::new())),
            pending_camera_follows: Arc::new(Mutex::new(Vec::new())),
            pending_camera_shakes: Arc::new(Mutex::new(Vec::new())),
            pending_camera_lerps: Arc::new(Mutex::new(Vec::new())),
            pending_camera_orbits: Arc::new(Mutex::new(Vec::new())),
            pending_tweens: Arc::new(Mutex::new(Vec::new())),
            pending_scene_load: Arc::new(Mutex::new(None)),
            pending_sprite_frames: Arc::new(Mutex::new(Vec::new())),
            save_data_store,
            hud_buttons: Arc::new(RwLock::new(HashMap::new())),
            pending_button_clicks: Arc::new(Mutex::new(Vec::new())),
            hud_texts: Arc::new(RwLock::new(HashMap::new())),
            world_entities: Arc::new(RwLock::new(HashMap::new())),
            audio,
            pending_post_process: Arc::new(Mutex::new(None)),
            post_process_settings: Arc::new(RwLock::new(crate::assets::PostProcessDef::default())),
            pending_materials: Arc::new(Mutex::new(Vec::new())),
            pending_trails: Arc::new(Mutex::new(Vec::new())),
            debug_lines: Arc::new(Mutex::new(Vec::new())),
            debug_rings: Arc::new(Mutex::new(Vec::new())),
            world_labels: Arc::new(RwLock::new(HashMap::new())),
            floating_texts: Arc::new(Mutex::new(Vec::new())),
            pending_environment: Arc::new(Mutex::new(None)),
            environment_settings: Arc::new(RwLock::new(crate::assets::EnvironmentDef::default())),
            physics_raycaster: Arc::new(RwLock::new(None)),
            game_ui_elements: Arc::new(RwLock::new(Vec::new())),
            pending_anims: Arc::new(Mutex::new(Vec::new())),
            anim_playing_states: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(RwLock::new(HashMap::new())),
            pending_events: Arc::new(Mutex::new(Vec::new())),
            timers: Arc::new(Mutex::new(Vec::new())),
            next_timer_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            input_map: Arc::new(RwLock::new(crate::input_map::InputMap::load_or_default(std::path::Path::new("assets/input_map.json")))),
            mouse_info: Arc::new(RwLock::new(crate::input_map::MouseInfo::default())),
            active_camera: Arc::new(RwLock::new(crate::camera::Camera::new_3d(glam::Vec3::new(0.0, 3.5, 9.0), glam::Vec3::ZERO, 16.0 / 9.0))),
            nav_grid: Arc::new(RwLock::new(crate::nav::NavGrid::new(1.0, 20.0))),
            pending_nav_bake: Arc::new(Mutex::new(None)),
        }
    }

    pub fn clear_timers_and_events(&self) {
        if let Ok(mut t) = self.timers.lock() {
            t.clear();
        }
        if let Ok(mut e) = self.pending_events.lock() {
            e.clear();
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
                Ok(check_key_match(&keys, &clean))
            })
            .map_err(|e| format!("Failed to register is_key_down: {}", e))?;
        lua.globals().set("is_key_down", is_key_down).map_err(|e| e.to_string())?;

        // 1b. is_key_just_pressed(key)
        let just_pressed_ref = ctx.just_pressed_keys.clone();
        let is_key_just_pressed = lua
            .create_function(move |_, key: String| {
                let keys = just_pressed_ref.read().unwrap();
                let clean = key.trim().to_uppercase();
                Ok(check_key_match(&keys, &clean))
            })
            .map_err(|e| format!("Failed to register is_key_just_pressed: {}", e))?;
        lua.globals().set("is_key_just_pressed", is_key_just_pressed).map_err(|e| e.to_string())?;

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
                        is_sensor: pt.get("is_sensor").ok(),
                    })
                } else {
                    None
                };

                let parent: Option<String> = table.get("parent").ok();
                let fresnel_color: Option<[f32; 3]> = table.get("fresnel_color").ok();
                let fresnel_power: Option<f32> = table.get("fresnel_power").ok();
                let blend_mode: Option<String> = table.get("blend_mode").ok();
                let wireframe: Option<bool> = table.get("wireframe").ok();
                let uv_scroll: Option<[f32; 2]> = table.get("uv_scroll").ok();

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
                    fresnel_color,
                    fresnel_power,
                    blend_mode,
                    wireframe,
                    uv_scroll,
                    script,
                    physics: phys_def,
                    point_light: None,
                    animation_player: None,
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

        // 6b. get_all_entities() -> table with [id] = { id, x, y, z, rx, ry, rz, sx, sy, sz }
        let entities_ref_all = ctx.world_entities.clone();
        let get_all_entities = lua
            .create_function(move |lua, ()| {
                let tbl = lua.create_table()?;
                if let Ok(map) = entities_ref_all.read() {
                    for (id, t) in map.iter() {
                        let etbl = lua.create_table()?;
                        etbl.set("id", id.as_str())?;
                        etbl.set("x", t[0])?;
                        etbl.set("y", t[1])?;
                        etbl.set("z", t[2])?;
                        etbl.set("rx", t[3])?;
                        etbl.set("ry", t[4])?;
                        etbl.set("rz", t[5])?;
                        etbl.set("sx", t[6])?;
                        etbl.set("sy", t[7])?;
                        etbl.set("sz", t[8])?;
                        tbl.set(id.as_str(), etbl)?;
                    }
                }
                Ok(tbl)
            })
            .map_err(|e| format!("Failed to register get_all_entities: {}", e))?;
        lua.globals().set("get_all_entities", get_all_entities).map_err(|e| e.to_string())?;

        // 7. set_hud_text(key, text)
        let hud_ref = ctx.hud_texts.clone();
        let set_hud_text = lua
            .create_function(move |_, (key, text): (String, String)| {
                if let Ok(mut map) = hud_ref.write() {
                    if text.trim().is_empty() {
                        map.remove(&key);
                    } else {
                        map.insert(key, text);
                    }
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register set_hud_text: {}", e))?;
        lua.globals().set("set_hud_text", set_hud_text).map_err(|e| e.to_string())?;

        // 8. play_sound(path, [volume], [bus])
        let audio_sound = ctx.audio.clone();
        let play_sound = lua
            .create_function(move |_, (path, volume, bus): (String, Option<f32>, Option<String>)| {
                let vol = volume.unwrap_or(1.0);
                let bus_str = bus.unwrap_or_else(|| "sfx".to_string());
                if let Ok(audio) = audio_sound.lock() {
                    let _ = audio.play_sound_bus(&path, vol, &bus_str);
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

        // 10b. play_sound_3d(path, x, y, z, max_dist, [volume], [bus])
        let audio_3d = ctx.audio.clone();
        let cam_3d = ctx.active_camera.clone();
        let play_sound_3d = lua
            .create_function(
                move |_,
                      (path, x, y, z, max_dist, volume, bus): (
                    String,
                    f32,
                    f32,
                    f32,
                    f32,
                    Option<f32>,
                    Option<String>,
                )| {
                    let vol = volume.unwrap_or(1.0);
                    let listener_eye = {
                        let cam = cam_3d.read().unwrap();
                        [cam.eye.x, cam.eye.y, cam.eye.z]
                    };
                    if let Ok(audio) = audio_3d.lock() {
                        let _ = audio.play_sound_3d(
                            &path,
                            [x, y, z],
                            listener_eye,
                            max_dist,
                            vol,
                            bus.as_deref(),
                        );
                    }
                    Ok(())
                },
            )
            .map_err(|e| format!("Failed to register play_sound_3d: {}", e))?;
        lua.globals().set("play_sound_3d", play_sound_3d).map_err(|e| e.to_string())?;

        // 10c. set_bus_volume(bus, volume)
        let audio_set_bus = ctx.audio.clone();
        let set_bus_volume = lua
            .create_function(move |_, (bus, vol): (String, f32)| {
                if let Ok(mut audio) = audio_set_bus.lock() {
                    audio.set_bus_volume(&bus, vol);
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register set_bus_volume: {}", e))?;
        lua.globals().set("set_bus_volume", set_bus_volume).map_err(|e| e.to_string())?;

        // 10d. get_bus_volume(bus) -> number
        let audio_get_bus = ctx.audio.clone();
        let get_bus_volume = lua
            .create_function(move |_, bus: String| {
                if let Ok(audio) = audio_get_bus.lock() {
                    Ok(audio.get_bus_volume(&bus))
                } else {
                    Ok(1.0)
                }
            })
            .map_err(|e| format!("Failed to register get_bus_volume: {}", e))?;
        lua.globals().set("get_bus_volume", get_bus_volume).map_err(|e| e.to_string())?;

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

        // 14. set_light(entity_id, r, g, b, intensity, radius)
        let lights_ref = ctx.pending_lights.clone();
        let set_light = lua
            .create_function(
                move |_, (id, r, g, b, intensity, radius): (String, f32, f32, f32, f32, f32)| {
                    if let Ok(mut list) = lights_ref.lock() {
                        list.push((id, [r, g, b], intensity, radius));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_light: {}", e))?;
        lua.globals()
            .set("set_light", set_light)
            .map_err(|e| e.to_string())?;

        // 15. set_camera(x, y, z, tx, ty, tz)
        let cam_set_ref = ctx.pending_camera_sets.clone();
        let set_camera = lua
            .create_function(
                move |_, (x, y, z, tx, ty, tz): (f32, f32, f32, f32, f32, f32)| {
                    if let Ok(mut list) = cam_set_ref.lock() {
                        list.push(([x, y, z], [tx, ty, tz]));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_camera: {}", e))?;
        lua.globals()
            .set("set_camera", set_camera)
            .map_err(|e| e.to_string())?;

        // 16. follow_entity(entity_id, offset_x, offset_y, offset_z, smoothness)
        let cam_follow_ref = ctx.pending_camera_follows.clone();
        let follow_entity = lua
            .create_function(
                move |_, (entity_id, ox, oy, oz, smoothness): (Option<String>, Option<f32>, Option<f32>, Option<f32>, Option<f32>)| {
                    if let Ok(mut list) = cam_follow_ref.lock() {
                        let offset = [ox.unwrap_or(0.0), oy.unwrap_or(3.0), oz.unwrap_or(8.0)];
                        let sm = smoothness.unwrap_or(0.2);
                        list.push((entity_id, offset, sm));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register follow_entity: {}", e))?;
        lua.globals()
            .set("follow_entity", follow_entity)
            .map_err(|e| e.to_string())?;

        // 17. camera_shake(intensity, duration)
        let cam_shake_ref = ctx.pending_camera_shakes.clone();
        let camera_shake = lua
            .create_function(
                move |_, (intensity, duration): (f32, f32)| {
                    if let Ok(mut list) = cam_shake_ref.lock() {
                        list.push((intensity, duration));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register camera_shake: {}", e))?;
        lua.globals()
            .set("camera_shake", camera_shake)
            .map_err(|e| e.to_string())?;

        // 17b. camera_lerp_to(eye_x, eye_y, eye_z, target_x, target_y, target_z, duration)
        let cam_lerp_ref = ctx.pending_camera_lerps.clone();
        let camera_lerp_to = lua
            .create_function(
                move |_, (ex, ey, ez, tx, ty, tz, duration): (f32, f32, f32, f32, f32, f32, f32)| {
                    if let Ok(mut list) = cam_lerp_ref.lock() {
                        list.push(([ex, ey, ez], [tx, ty, tz], duration));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register camera_lerp_to: {}", e))?;
        lua.globals()
            .set("camera_lerp_to", camera_lerp_to)
            .map_err(|e| e.to_string())?;

        // 17c. camera_orbit_target(entity_id, distance, pitch_deg, yaw_speed)
        let cam_orbit_ref = ctx.pending_camera_orbits.clone();
        let camera_orbit_target = lua
            .create_function(
                move |_, (target_id, distance, pitch_deg, yaw_speed): (Option<String>, f32, f32, f32)| {
                    if let Ok(mut list) = cam_orbit_ref.lock() {
                        list.push((target_id, distance, pitch_deg, yaw_speed));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register camera_orbit_target: {}", e))?;
        lua.globals()
            .set("camera_orbit_target", camera_orbit_target)
            .map_err(|e| e.to_string())?;

        // 18. tween_position(id, x, y, z, duration)
        let tweens_pos_ref = ctx.pending_tweens.clone();
        let tween_position = lua
            .create_function(
                move |_, (id, x, y, z, duration): (String, f32, f32, f32, f32)| {
                    if let Ok(mut list) = tweens_pos_ref.lock() {
                        list.push(crate::tweens::TweenRequest {
                            entity_id: id,
                            target: glam::Vec3::new(x, y, z),
                            duration,
                            is_scale: false,
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register tween_position: {}", e))?;
        lua.globals()
            .set("tween_position", tween_position)
            .map_err(|e| e.to_string())?;

        // 19. tween_scale(id, sx, sy, sz, duration)
        let tweens_scale_ref = ctx.pending_tweens.clone();
        let tween_scale = lua
            .create_function(
                move |_, (id, sx, sy, sz, duration): (String, f32, f32, f32, f32)| {
                    if let Ok(mut list) = tweens_scale_ref.lock() {
                        list.push(crate::tweens::TweenRequest {
                            entity_id: id,
                            target: glam::Vec3::new(sx, sy, sz),
                            duration,
                            is_scale: true,
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register tween_scale: {}", e))?;
        lua.globals()
            .set("tween_scale", tween_scale)
            .map_err(|e| e.to_string())?;

        // 20. load_scene(path)
        let scene_load_ref = ctx.pending_scene_load.clone();
        let load_scene = lua
            .create_function(move |_, path: String| {
                if let Ok(mut pending) = scene_load_ref.lock() {
                    *pending = Some(path);
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register load_scene: {}", e))?;
        lua.globals()
            .set("load_scene", load_scene)
            .map_err(|e| e.to_string())?;

        // 21. save_data(key, value)
        let save_store_ref = ctx.save_data_store.clone();
        let save_data = lua
            .create_function(move |_, (key, val): (String, mlua::Value)| {
                let json_val = lua_to_json(&val);
                if let Ok(mut store) = save_store_ref.write() {
                    store.insert(key, json_val);
                    if let Ok(serialized) = serde_json::to_string_pretty(&*store) {
                        let _ = std::fs::write("assets/save_data.json", serialized);
                    }
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register save_data: {}", e))?;
        lua.globals()
            .set("save_data", save_data)
            .map_err(|e| e.to_string())?;

        // 22. load_data(key)
        let load_store_ref = ctx.save_data_store.clone();
        let load_data = lua
            .create_function(move |lua, key: String| {
                if let Ok(store) = load_store_ref.read() {
                    if let Some(val) = store.get(&key) {
                        return json_to_lua(lua, val);
                    }
                }
                Ok(mlua::Value::Nil)
            })
            .map_err(|e| format!("Failed to register load_data: {}", e))?;
        lua.globals()
            .set("load_data", load_data)
            .map_err(|e| e.to_string())?;

        // 23. set_hud_button(id, text, x, y, callback_name)
        let buttons_ref = ctx.hud_buttons.clone();
        let set_hud_button = lua
            .create_function(
                move |_, (id, text, x, y, callback): (String, String, f32, f32, String)| {
                    if let Ok(mut btns) = buttons_ref.write() {
                        btns.insert(
                            id.clone(),
                            HudButtonDef {
                                id,
                                text,
                                x,
                                y,
                                callback,
                            },
                        );
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_hud_button: {}", e))?;
        lua.globals()
            .set("set_hud_button", set_hud_button)
            .map_err(|e| e.to_string())?;

        // 24. set_sprite_frame(entity_id, cols, rows, frame_index)
        let sprite_frames_ref = ctx.pending_sprite_frames.clone();
        let set_sprite_frame = lua
            .create_function(
                move |_, (id, cols, rows, frame): (String, u32, u32, u32)| {
                    if let Ok(mut list) = sprite_frames_ref.lock() {
                        list.push((id, cols.max(1), rows.max(1), frame));
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_sprite_frame: {}", e))?;
        lua.globals()
            .set("set_sprite_frame", set_sprite_frame)
            .map_err(|e| e.to_string())?;

        // 25. set_post_process(bloom_intensity, vignette_intensity, exposure)
        let pp_settings_ref = ctx.post_process_settings.clone();
        let pending_pp_ref = ctx.pending_post_process.clone();
        let set_post_process = lua
            .create_function(
                move |_, (bloom_int, vig_int, exp): (f32, Option<f32>, Option<f32>)| {
                    let mut pp = pp_settings_ref.read().map(|s| s.clone()).unwrap_or_default();
                    pp.bloom_intensity = bloom_int;
                    if let Some(v) = vig_int {
                        pp.vignette_intensity = v;
                    }
                    if let Some(e) = exp {
                        pp.exposure = e;
                    }
                    if let Ok(mut lock) = pp_settings_ref.write() {
                        *lock = pp.clone();
                    }
                    if let Ok(mut pending) = pending_pp_ref.lock() {
                        *pending = Some(pp);
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_post_process: {}", e))?;
        lua.globals()
            .set("set_post_process", set_post_process)
            .map_err(|e| e.to_string())?;

        // 26. get_post_process()
        let pp_settings_get_ref = ctx.post_process_settings.clone();
        let get_post_process = lua
            .create_function(move |lua, ()| {
                let pp = pp_settings_get_ref.read().map(|s| s.clone()).unwrap_or_default();
                let table = lua.create_table()?;
                table.set("bloom_enabled", pp.bloom_enabled)?;
                table.set("bloom_intensity", pp.bloom_intensity)?;
                table.set("bloom_threshold", pp.bloom_threshold)?;
                table.set("vignette_intensity", pp.vignette_intensity)?;
                table.set("exposure", pp.exposure)?;
                Ok(table)
            })
            .map_err(|e| format!("Failed to register get_post_process: {}", e))?;
        lua.globals()
            .set("get_post_process", get_post_process)
            .map_err(|e| e.to_string())?;

        // 27. set_material(entity_id, material_table)
        let materials_ref = ctx.pending_materials.clone();
        let set_material = lua
            .create_function(move |_, (id, mat_table): (String, mlua::Table)| {
                let color: Option<[f32; 4]> = if let Ok(col) = mat_table.get::<mlua::Table>("color") {
                    let r: f32 = col.get(1).unwrap_or(1.0);
                    let g: f32 = col.get(2).unwrap_or(1.0);
                    let b: f32 = col.get(3).unwrap_or(1.0);
                    let a: f32 = col.get(4).unwrap_or(1.0);
                    Some([r, g, b, a])
                } else {
                    None
                };
                let emissive: Option<f32> = mat_table.get("emissive").ok();
                let fresnel_color: Option<[f32; 3]> = if let Ok(fc) = mat_table.get::<mlua::Table>("fresnel_color") {
                    let r: f32 = fc.get(1).unwrap_or(1.0);
                    let g: f32 = fc.get(2).unwrap_or(1.0);
                    let b: f32 = fc.get(3).unwrap_or(1.0);
                    Some([r, g, b])
                } else {
                    None
                };
                let fresnel_power: Option<f32> = mat_table.get("fresnel_power").ok();
                let blend_mode: Option<String> = mat_table.get("blend_mode").ok();
                let wireframe: Option<bool> = mat_table.get("wireframe").ok();
                let uv_scroll: Option<[f32; 2]> = if let Ok(uv) = mat_table.get::<mlua::Table>("uv_scroll") {
                    let x: f32 = uv.get(1).unwrap_or(0.0);
                    let y: f32 = uv.get(2).unwrap_or(0.0);
                    Some([x, y])
                } else {
                    None
                };

                if let Ok(mut list) = materials_ref.lock() {
                    list.push(MaterialUpdate {
                        entity_id: id,
                        color,
                        emissive,
                        fresnel_color,
                        fresnel_power,
                        blend_mode,
                        wireframe,
                        uv_scroll,
                    });
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register set_material: {}", e))?;
        lua.globals()
            .set("set_material", set_material)
            .map_err(|e| e.to_string())?;

        // 28. set_trail(id, r, g, b, [lifetime], [width])
        let trails_ref = ctx.pending_trails.clone();
        let set_trail = lua
            .create_function(
                move |_,
                      (id, r, g, b, lifetime, width): (
                    String,
                    f32,
                    f32,
                    f32,
                    Option<f32>,
                    Option<f32>,
                )| {
                    if let Ok(mut list) = trails_ref.lock() {
                        list.push(TrailCommand {
                            entity_id: id,
                            color: [r, g, b, 1.0],
                            lifetime: lifetime.unwrap_or(0.8),
                            width: width.unwrap_or(0.25),
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_trail: {}", e))?;
        lua.globals().set("set_trail", set_trail).map_err(|e| e.to_string())?;

        // 29. draw_line_3d(x1, y1, z1, x2, y2, z2, r, g, b, [a])
        let debug_lines_ref = ctx.debug_lines.clone();
        let draw_line_3d = lua
            .create_function(
                move |_,
                      (x1, y1, z1, x2, y2, z2, r, g, b, a): (
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    Option<f32>,
                )| {
                    if let Ok(mut list) = debug_lines_ref.lock() {
                        list.push(DebugLine3D {
                            start: [x1, y1, z1],
                            end: [x2, y2, z2],
                            color: [r, g, b, a.unwrap_or(1.0)],
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register draw_line_3d: {}", e))?;
        lua.globals().set("draw_line_3d", draw_line_3d).map_err(|e| e.to_string())?;

        // 30. draw_ring_3d(x, y, z, radius, r, g, b, [a])
        let debug_rings_ref = ctx.debug_rings.clone();
        let draw_ring_3d = lua
            .create_function(
                move |_,
                      (x, y, z, radius, r, g, b, a): (
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    Option<f32>,
                )| {
                    if let Ok(mut list) = debug_rings_ref.lock() {
                        list.push(DebugRing3D {
                            center: [x, y, z],
                            radius,
                            color: [r, g, b, a.unwrap_or(1.0)],
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register draw_ring_3d: {}", e))?;
        lua.globals().set("draw_ring_3d", draw_ring_3d).map_err(|e| e.to_string())?;

        // 31. set_world_label(id, text, offset_y, [r], [g], [b])
        let labels_ref = ctx.world_labels.clone();
        let set_world_label = lua
            .create_function(
                move |_,
                      (id, text, offset_y, r, g, b): (
                    String,
                    String,
                    f32,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                )| {
                    if let Ok(mut labels) = labels_ref.write() {
                        labels.insert(
                            id.clone(),
                            WorldLabel {
                                entity_id: id,
                                text,
                                offset_y,
                                color: [r.unwrap_or(1.0), g.unwrap_or(1.0), b.unwrap_or(1.0)],
                            },
                        );
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_world_label: {}", e))?;
        lua.globals().set("set_world_label", set_world_label).map_err(|e| e.to_string())?;

        // 32. spawn_floating_text(x, y, z, text, [r], [g], [b], [lifetime])
        let float_texts_ref = ctx.floating_texts.clone();
        let spawn_floating_text = lua
            .create_function(
                move |_,
                      (x, y, z, text, r, g, b, lifetime): (
                    f32,
                    f32,
                    f32,
                    String,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                    Option<f32>,
                )| {
                    if let Ok(mut list) = float_texts_ref.lock() {
                        list.push(FloatingText {
                            position: [x, y, z],
                            text,
                            color: [r.unwrap_or(1.0), g.unwrap_or(0.8), b.unwrap_or(0.2)],
                            lifetime: lifetime.unwrap_or(1.5),
                            elapsed: 0.0,
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register spawn_floating_text: {}", e))?;
        lua.globals().set("spawn_floating_text", spawn_floating_text).map_err(|e| e.to_string())?;

        // 33. set_environment(sky_mode, fog_r, fog_g, fog_b, fog_density, [zenith_table], [horizon_table])
        let env_ref = ctx.pending_environment.clone();
        let env_settings_ref = ctx.environment_settings.clone();
        let set_environment = lua
            .create_function(
                move |_,
                      (sky_mode, fog_r, fog_g, fog_b, fog_density, zenith_table, horizon_table): (
                    String,
                    f32,
                    f32,
                    f32,
                    f32,
                    Option<mlua::Table>,
                    Option<mlua::Table>,
                )| {
                    let mut current = env_settings_ref.read().map(|e| e.clone()).unwrap_or_default();
                    current.sky_mode = sky_mode;
                    current.fog_color = [fog_r, fog_g, fog_b];
                    current.fog_density = fog_density;
                    if let Some(zt) = zenith_table {
                        let r: f32 = zt.get(1).unwrap_or(current.zenith_color[0]);
                        let g: f32 = zt.get(2).unwrap_or(current.zenith_color[1]);
                        let b: f32 = zt.get(3).unwrap_or(current.zenith_color[2]);
                        current.zenith_color = [r, g, b];
                    }
                    if let Some(ht) = horizon_table {
                        let r: f32 = ht.get(1).unwrap_or(current.horizon_color[0]);
                        let g: f32 = ht.get(2).unwrap_or(current.horizon_color[1]);
                        let b: f32 = ht.get(3).unwrap_or(current.horizon_color[2]);
                        current.horizon_color = [r, g, b];
                    }
                    if let Ok(mut pending) = env_ref.lock() {
                        *pending = Some(current.clone());
                    }
                    if let Ok(mut s) = env_settings_ref.write() {
                        *s = current;
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register set_environment: {}", e))?;
        lua.globals().set("set_environment", set_environment).map_err(|e| e.to_string())?;

        // 34. get_environment()
        let env_get_ref = ctx.environment_settings.clone();
        let get_environment = lua
            .create_function(move |lua, ()| {
                let env = env_get_ref.read().map(|e| e.clone()).unwrap_or_default();
                let tbl = lua.create_table()?;
                tbl.set("sky_mode", env.sky_mode)?;
                tbl.set("fog_density", env.fog_density)?;
                let fc = lua.create_table()?;
                fc.set(1, env.fog_color[0])?;
                fc.set(2, env.fog_color[1])?;
                fc.set(3, env.fog_color[2])?;
                tbl.set("fog_color", fc)?;
                Ok(tbl)
            })
            .map_err(|e| format!("Failed to register get_environment: {}", e))?;
        lua.globals().set("get_environment", get_environment).map_err(|e| e.to_string())?;

        // 35. raycast(ox, oy, oz, dx, dy, dz, max_dist)
        let raycaster_arc = ctx.physics_raycaster.clone();
        let raycast = lua
            .create_function(
                move |lua,
                      (ox, oy, oz, dx, dy, dz, max_dist): (
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                )| {
                    let hit_table = lua.create_table()?;
                    if let Ok(guard) = raycaster_arc.read() {
                        if let Some(raycaster) = guard.as_ref() {
                            let dir = glam::Vec3::new(dx, dy, dz);
                            let dir_norm = dir.normalize_or_zero();
                            if dir_norm.length_squared() >= 0.0001 {
                                let query_pipeline = raycaster.broad_phase.as_query_pipeline(
                                    raycaster.narrow_phase.query_dispatcher(),
                                    &raycaster.rigid_body_set,
                                    &raycaster.collider_set,
                                    rapier3d::pipeline::QueryFilter::default().exclude_sensors(),
                                );
                                let ray = rapier3d::geometry::Ray::new(
                                    glam::Vec3::new(ox, oy, oz),
                                    dir_norm,
                                );
                                if let Some((col_handle, hit)) =
                                    query_pipeline.cast_ray_and_get_normal(&ray, max_dist, true)
                                {
                                    let entity_id = raycaster
                                        .collider_to_entity
                                        .get(&col_handle)
                                        .cloned()
                                        .unwrap_or_default();
                                    hit_table.set("hit", true)?;
                                    hit_table.set("entity", entity_id)?;
                                    hit_table.set("dist", hit.time_of_impact)?;
                                    hit_table.set("nx", hit.normal.x)?;
                                    hit_table.set("ny", hit.normal.y)?;
                                    hit_table.set("nz", hit.normal.z)?;
                                    return Ok(hit_table);
                                }
                            }
                        }
                    }
                    hit_table.set("hit", false)?;
                    hit_table.set("entity", "")?;
                    hit_table.set("dist", 0.0)?;
                    hit_table.set("nx", 0.0)?;
                    hit_table.set("ny", 0.0)?;
                    hit_table.set("nz", 0.0)?;
                    Ok(hit_table)
                },
            )
            .map_err(|e| format!("Failed to register raycast: {}", e))?;
        lua.globals().set("raycast", raycast).map_err(|e| e.to_string())?;

        // 36. play_tone(f_start, f_end, wave_type, duration, vol)
        let audio_tone_ref = ctx.audio.clone();
        let play_tone = lua
            .create_function(
                move |_,
                      (f_start, f_end, wave_type, duration, vol): (
                    f32,
                    f32,
                    String,
                    f32,
                    f32,
                )| {
                    if let Ok(audio) = audio_tone_ref.lock() {
                        let _ = audio.play_tone(f_start, f_end, &wave_type, duration, vol);
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register play_tone: {}", e))?;
        lua.globals().set("play_tone", play_tone).map_err(|e| e.to_string())?;

        // 37. ui_clear()
        let ui_clear_ref = ctx.game_ui_elements.clone();
        let ui_clear = lua
            .create_function(move |_, ()| {
                if let Ok(mut list) = ui_clear_ref.write() {
                    list.clear();
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register ui_clear: {}", e))?;
        lua.globals().set("ui_clear", ui_clear).map_err(|e| e.to_string())?;

        // 38. ui_panel(id, anchor, offset_x, offset_y, width, height, r, g, b, a)
        let ui_panel_ref = ctx.game_ui_elements.clone();
        let ui_panel = lua
            .create_function(
                move |_,
                      (id, anchor, ox, oy, w, h, r, g, b, a): (
                    String,
                    String,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                )| {
                    if let Ok(mut list) = ui_panel_ref.write() {
                        let elem = crate::game_ui::UiElement::Panel {
                            id: id.clone(),
                            anchor,
                            offset_x: ox,
                            offset_y: oy,
                            width: w,
                            height: h,
                            color: [r, g, b, a],
                        };
                        if let Some(pos) = list.iter().position(|e| e.id() == id) {
                            list[pos] = elem;
                        } else {
                            list.push(elem);
                        }
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register ui_panel: {}", e))?;
        lua.globals().set("ui_panel", ui_panel).map_err(|e| e.to_string())?;

        // 39. ui_bar(id, anchor, offset_x, offset_y, width, height, progress, label, r, g, b)
        let ui_bar_ref = ctx.game_ui_elements.clone();
        let ui_bar = lua
            .create_function(
                move |_,
                      (id, anchor, ox, oy, w, h, progress, label, r, g, b): (
                    String,
                    String,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    String,
                    f32,
                    f32,
                    f32,
                )| {
                    if let Ok(mut list) = ui_bar_ref.write() {
                        let elem = crate::game_ui::UiElement::Bar {
                            id: id.clone(),
                            anchor,
                            offset_x: ox,
                            offset_y: oy,
                            width: w,
                            height: h,
                            progress,
                            label,
                            color: [r, g, b],
                        };
                        if let Some(pos) = list.iter().position(|e| e.id() == id) {
                            list[pos] = elem;
                        } else {
                            list.push(elem);
                        }
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register ui_bar: {}", e))?;
        lua.globals().set("ui_bar", ui_bar).map_err(|e| e.to_string())?;

        // 40. ui_label(id, anchor, offset_x, offset_y, text, font_size, r, g, b)
        let ui_label_ref = ctx.game_ui_elements.clone();
        let ui_label = lua
            .create_function(
                move |_,
                      (id, anchor, ox, oy, text, font_size, r, g, b): (
                    String,
                    String,
                    f32,
                    f32,
                    String,
                    f32,
                    f32,
                    f32,
                    f32,
                )| {
                    if let Ok(mut list) = ui_label_ref.write() {
                        let elem = crate::game_ui::UiElement::Label {
                            id: id.clone(),
                            anchor,
                            offset_x: ox,
                            offset_y: oy,
                            text,
                            font_size,
                            color: [r, g, b],
                        };
                        if let Some(pos) = list.iter().position(|e| e.id() == id) {
                            list[pos] = elem;
                        } else {
                            list.push(elem);
                        }
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register ui_label: {}", e))?;
        lua.globals().set("ui_label", ui_label).map_err(|e| e.to_string())?;

        // 41. ui_button(id, anchor, offset_x, offset_y, width, height, text, callback_name)
        let ui_btn_ref = ctx.game_ui_elements.clone();
        let ui_button = lua
            .create_function(
                move |_,
                      (id, anchor, ox, oy, w, h, text, callback): (
                    String,
                    String,
                    f32,
                    f32,
                    f32,
                    f32,
                    String,
                    String,
                )| {
                    if let Ok(mut list) = ui_btn_ref.write() {
                        let elem = crate::game_ui::UiElement::Button {
                            id: id.clone(),
                            anchor,
                            offset_x: ox,
                            offset_y: oy,
                            width: w,
                            height: h,
                            text,
                            callback,
                        };
                        if let Some(pos) = list.iter().position(|e| e.id() == id) {
                            list[pos] = elem;
                        } else {
                            list.push(elem);
                        }
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register ui_button: {}", e))?;
        lua.globals().set("ui_button", ui_button).map_err(|e| e.to_string())?;

        // 42. play_anim(entity_id, clip_name, [looping])
        let anim_play_ref = ctx.pending_anims.clone();
        let play_anim = lua
            .create_function(
                move |_, (entity_id, clip_name, looping): (String, String, Option<bool>)| {
                    if let Ok(mut list) = anim_play_ref.lock() {
                        list.push(AnimationCommand::Play {
                            entity_id,
                            clip_name,
                            looping: looping.unwrap_or(true),
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register play_anim: {}", e))?;
        lua.globals().set("play_anim", play_anim).map_err(|e| e.to_string())?;

        // 42b. play_preset_anim(entity_id, preset_name)
        let anim_preset_ref = ctx.pending_anims.clone();
        let play_preset_anim = lua
            .create_function(
                move |_, (entity_id, preset_name): (String, String)| {
                    if let Ok(mut list) = anim_preset_ref.lock() {
                        list.push(AnimationCommand::PlayPreset {
                            entity_id,
                            preset_name,
                        });
                    }
                    Ok(true)
                },
            )
            .map_err(|e| format!("Failed to register play_preset_anim: {}", e))?;
        lua.globals().set("play_preset_anim", play_preset_anim).map_err(|e| e.to_string())?;

        // 43. stop_anim(entity_id)
        let anim_stop_ref = ctx.pending_anims.clone();
        let stop_anim = lua
            .create_function(move |_, entity_id: String| {
                if let Ok(mut list) = anim_stop_ref.lock() {
                    list.push(AnimationCommand::Stop { entity_id });
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register stop_anim: {}", e))?;
        lua.globals().set("stop_anim", stop_anim).map_err(|e| e.to_string())?;

        // 44. is_anim_playing(entity_id) -> bool
        let anim_states_ref = ctx.anim_playing_states.clone();
        let is_anim_playing = lua
            .create_function(move |_, entity_id: String| {
                if let Ok(states) = anim_states_ref.read() {
                    Ok(states.get(&entity_id).copied().unwrap_or(false))
                } else {
                    Ok(false)
                }
            })
            .map_err(|e| format!("Failed to register is_anim_playing: {}", e))?;
        lua.globals().set("is_anim_playing", is_anim_playing).map_err(|e| e.to_string())?;

        // 45. on_event(event_name, callback_name)
        let listeners_ref = ctx.event_listeners.clone();
        let on_event = lua
            .create_function(move |_, (event_name, callback): (String, String)| {
                if let Ok(mut map) = listeners_ref.write() {
                    let list = map.entry(event_name).or_insert_with(Vec::new);
                    if !list.contains(&callback) {
                        list.push(callback);
                    }
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register on_event: {}", e))?;
        lua.globals().set("on_event", on_event).map_err(|e| e.to_string())?;

        // 46. emit_event(event_name, [payload])
        let events_ref = ctx.pending_events.clone();
        let emit_event = lua
            .create_function(move |_, (event_name, payload): (String, Option<mlua::Value>)| {
                let json_val = match payload {
                    Some(v) => lua_to_json(&v),
                    None => serde_json::Value::Null,
                };
                if let Ok(mut list) = events_ref.lock() {
                    list.push((event_name, json_val));
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register emit_event: {}", e))?;
        lua.globals().set("emit_event", emit_event).map_err(|e| e.to_string())?;

        // 47. clear_events()
        let listeners_clear_ref = ctx.event_listeners.clone();
        let events_clear_ref = ctx.pending_events.clone();
        let clear_events = lua
            .create_function(move |_, ()| {
                if let Ok(mut map) = listeners_clear_ref.write() {
                    map.clear();
                }
                if let Ok(mut list) = events_clear_ref.lock() {
                    list.clear();
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register clear_events: {}", e))?;
        lua.globals().set("clear_events", clear_events).map_err(|e| e.to_string())?;

        // 48. after(delay_seconds, callback_name) -> timer_id
        let timers_ref = ctx.timers.clone();
        let next_timer_id = ctx.next_timer_id.clone();
        let after = lua
            .create_function(move |_, (delay, callback): (f32, String)| {
                let id = next_timer_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if let Ok(mut list) = timers_ref.lock() {
                    list.push(LuaTimer {
                        id,
                        callback,
                        remaining: delay.max(0.0),
                        interval: None,
                    });
                }
                Ok(id)
            })
            .map_err(|e| format!("Failed to register after: {}", e))?;
        lua.globals().set("after", after).map_err(|e| e.to_string())?;

        // 49. every(interval_seconds, callback_name) -> timer_id
        let timers_every_ref = ctx.timers.clone();
        let next_timer_id_every = ctx.next_timer_id.clone();
        let every = lua
            .create_function(move |_, (interval, callback): (f32, String)| {
                let id = next_timer_id_every.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let inter = interval.max(0.001);
                if let Ok(mut list) = timers_every_ref.lock() {
                    list.push(LuaTimer {
                        id,
                        callback,
                        remaining: inter,
                        interval: Some(inter),
                    });
                }
                Ok(id)
            })
            .map_err(|e| format!("Failed to register every: {}", e))?;
        lua.globals().set("every", every).map_err(|e| e.to_string())?;

        // 50. cancel_timer(timer_id) -> bool
        let timers_cancel_ref = ctx.timers.clone();
        let cancel_timer = lua
            .create_function(move |_, timer_id: u64| {
                if let Ok(mut list) = timers_cancel_ref.lock() {
                    if let Some(pos) = list.iter().position(|t| t.id == timer_id) {
                        list.remove(pos);
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .map_err(|e| format!("Failed to register cancel_timer: {}", e))?;
        lua.globals().set("cancel_timer", cancel_timer).map_err(|e| e.to_string())?;

        // 51. clear_timers()
        let timers_clear_ref = ctx.timers.clone();
        let clear_timers = lua
            .create_function(move |_, ()| {
                if let Ok(mut list) = timers_clear_ref.lock() {
                    list.clear();
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register clear_timers: {}", e))?;
        lua.globals().set("clear_timers", clear_timers).map_err(|e| e.to_string())?;

        // 52. is_action_down(action) -> bool
        let im_down_ref = ctx.input_map.clone();
        let keys_down_ref = ctx.pressed_keys.clone();
        let mouse_down_ref = ctx.mouse_info.clone();
        let is_action_down = lua
            .create_function(move |_, action: String| {
                let map = im_down_ref.read().unwrap();
                let keys = keys_down_ref.read().unwrap();
                let mouse = mouse_down_ref.read().unwrap();
                Ok(map.is_action_down(&action, &keys, &mouse.buttons_down))
            })
            .map_err(|e| format!("Failed to register is_action_down: {}", e))?;
        lua.globals().set("is_action_down", is_action_down).map_err(|e| e.to_string())?;

        // 53. is_action_just_pressed(action) -> bool
        let im_jp_ref = ctx.input_map.clone();
        let jp_keys_ref = ctx.just_pressed_keys.clone();
        let mouse_jp_ref = ctx.mouse_info.clone();
        let is_action_just_pressed = lua
            .create_function(move |_, action: String| {
                let map = im_jp_ref.read().unwrap();
                let keys = jp_keys_ref.read().unwrap();
                let mouse = mouse_jp_ref.read().unwrap();
                Ok(map.is_action_just_pressed(&action, &keys, &mouse.buttons_just_pressed))
            })
            .map_err(|e| format!("Failed to register is_action_just_pressed: {}", e))?;
        lua.globals().set("is_action_just_pressed", is_action_just_pressed).map_err(|e| e.to_string())?;

        // 54. bind_action(action, input) -> bool
        let im_bind_ref = ctx.input_map.clone();
        let bind_action = lua
            .create_function(move |_, (action, input): (String, String)| {
                if let Ok(mut map) = im_bind_ref.write() {
                    map.bind(&action, &input);
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register bind_action: {}", e))?;
        lua.globals().set("bind_action", bind_action).map_err(|e| e.to_string())?;

        // 55. get_mouse_world_pos([plane_y]) -> table { hit = bool, x = f32, y = f32, z = f32 }
        let mouse_pos_ref = ctx.mouse_info.clone();
        let cam_pos_ref = ctx.active_camera.clone();
        let get_mouse_world_pos = lua
            .create_function(move |lua, plane_y: Option<f32>| {
                let py = plane_y.unwrap_or(0.0);
                let mouse = mouse_pos_ref.read().unwrap();
                let cam = cam_pos_ref.read().unwrap();
                let hit_pos = cam.intersect_horizontal_plane(mouse.screen_pos, mouse.screen_size, py);
                let tbl = lua.create_table()?;
                if let Some(pos) = hit_pos {
                    tbl.set("hit", true)?;
                    tbl.set("x", pos.x)?;
                    tbl.set("y", pos.y)?;
                    tbl.set("z", pos.z)?;
                } else {
                    tbl.set("hit", false)?;
                    tbl.set("x", 0.0)?;
                    tbl.set("y", py)?;
                    tbl.set("z", 0.0)?;
                }
                Ok(tbl)
            })
            .map_err(|e| format!("Failed to register get_mouse_world_pos: {}", e))?;
        lua.globals().set("get_mouse_world_pos", get_mouse_world_pos).map_err(|e| e.to_string())?;

        // 55b. get_mouse_delta() -> table { dx, dy }
        let mouse_delta_ref = ctx.mouse_info.clone();
        let get_mouse_delta = lua
            .create_function(move |lua, ()| {
                let delta = if let Ok(m) = mouse_delta_ref.read() {
                    m.delta
                } else {
                    [0.0, 0.0]
                };
                let tbl = lua.create_table()?;
                tbl.set("dx", delta[0])?;
                tbl.set("dy", delta[1])?;
                Ok(tbl)
            })
            .map_err(|e| format!("Failed to register get_mouse_delta: {}", e))?;
        lua.globals().set("get_mouse_delta", get_mouse_delta).map_err(|e| e.to_string())?;

        // 56. nav_bake([cell_size], [half_extent]) -> bool
        let nav_grid_bake_ref = ctx.nav_grid.clone();
        let we_bake_ref = ctx.world_entities.clone();
        let pending_bake_ref = ctx.pending_nav_bake.clone();
        let nav_bake = lua
            .create_function(move |_, (cs_opt, extent_opt): (Option<f32>, Option<f32>)| {
                let cs = cs_opt.unwrap_or(1.0);
                let extent = extent_opt.unwrap_or(20.0);
                if let Ok(mut pending) = pending_bake_ref.lock() {
                    *pending = Some((cs, extent));
                }
                if let Ok(mut grid) = nav_grid_bake_ref.write() {
                    *grid = crate::nav::NavGrid::new(cs, extent);
                    if let Ok(entities) = we_bake_ref.read() {
                        grid.bake_from_entities_map(&entities);
                    }
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register nav_bake: {}", e))?;
        lua.globals().set("nav_bake", nav_bake).map_err(|e| e.to_string())?;

        // 57. find_path(start_x, start_z, goal_x, goal_z) -> table or nil
        let nav_grid_find_ref = ctx.nav_grid.clone();
        let find_path = lua
            .create_function(move |lua, (sx, sz, gx, gz): (f32, f32, f32, f32)| {
                let grid = nav_grid_find_ref.read().unwrap();
                if let Some(waypoints) = grid.find_path(sx, sz, gx, gz) {
                    let tbl = lua.create_table()?;
                    for (i, pt) in waypoints.iter().enumerate() {
                        let pt_tbl = lua.create_table()?;
                        pt_tbl.set("x", pt[0])?;
                        pt_tbl.set("z", pt[1])?;
                        tbl.set(i + 1, pt_tbl)?;
                    }
                    Ok(Some(tbl))
                } else {
                    Ok(None)
                }
            })
            .map_err(|e| format!("Failed to register find_path: {}", e))?;
        lua.globals().set("find_path", find_path).map_err(|e| e.to_string())?;

        // 58. nav_set_blocked(world_x, world_z, is_blocked) -> bool
        let nav_grid_set_ref = ctx.nav_grid.clone();
        let nav_set_blocked = lua
            .create_function(move |_, (wx, wz, is_blocked): (f32, f32, bool)| {
                if let Ok(mut grid) = nav_grid_set_ref.write() {
                    grid.set_world_blocked(wx, wz, is_blocked);
                }
                Ok(true)
            })
            .map_err(|e| format!("Failed to register nav_set_blocked: {}", e))?;
        lua.globals().set("nav_set_blocked", nav_set_blocked).map_err(|e| e.to_string())?;

        // 59. nav_is_blocked(world_x, world_z) -> bool
        let nav_grid_is_ref = ctx.nav_grid.clone();
        let nav_is_blocked = lua
            .create_function(move |_, (wx, wz): (f32, f32)| {
                let grid = nav_grid_is_ref.read().unwrap();
                Ok(grid.is_world_blocked(wx, wz))
            })
            .map_err(|e| format!("Failed to register nav_is_blocked: {}", e))?;
        lua.globals().set("nav_is_blocked", nav_is_blocked).map_err(|e| e.to_string())?;

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

    pub fn on_trigger_enter(&self, self_id: &str, other_id: &str) -> Result<(), String> {
        if self.has_error {
            return Ok(());
        }

        if let Some(lua_arc) = &self.lua {
            let lua = lua_arc.lock().map_err(|e| e.to_string())?;
            let on_trigger_fn: mlua::Result<mlua::Function> = lua.globals().get("on_trigger_enter");
            if let Ok(func) = on_trigger_fn {
                let entity_tbl = lua.create_table().map_err(|e| e.to_string())?;
                entity_tbl.set("id", self_id).map_err(|e| e.to_string())?;
                func.call::<()>((other_id, entity_tbl))
                    .map_err(|e| format!("Lua on_trigger_enter error on '{}': {}", self_id, e))?;
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
                    entity_table
                        .set("sx", transform.scale.x)
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("sy", transform.scale.y)
                        .map_err(|e| e.to_string())?;
                    entity_table
                        .set("sz", transform.scale.z)
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
                            if let Ok(sx) = entity_table.get("sx") {
                                transform.scale.x = sx;
                            }
                            if let Ok(sy) = entity_table.get("sy") {
                                transform.scale.y = sy;
                            }
                            if let Ok(sz) = entity_table.get("sz") {
                                transform.scale.z = sz;
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

pub fn process_timers_and_events(
    world: &mut hecs::World,
    ctx: &LuaEngineContext,
    dt: f32,
    logs: &mut Vec<String>,
) {
    // 1. Process Timers
    let triggered_callbacks = {
        let mut triggered = Vec::new();
        if let Ok(mut list) = ctx.timers.lock() {
            let mut kept = Vec::new();
            for mut timer in list.drain(..) {
                timer.remaining -= dt;
                if timer.remaining <= 0.0 {
                    triggered.push(timer.callback.clone());
                    if let Some(interval) = timer.interval {
                        timer.remaining = interval;
                        kept.push(timer);
                    }
                } else {
                    kept.push(timer);
                }
            }
            *list = kept;
        }
        triggered
    };

    for cb in triggered_callbacks {
        for (_meta, script) in world.query_mut::<(&crate::ecs::EntityMeta, &mut ScriptComponent)>() {
            if let Some(lua_arc) = &script.lua {
                if let Ok(lua) = lua_arc.lock() {
                    if let Ok(func) = lua.globals().get::<mlua::Function>(cb.as_str()) {
                        if let Err(e) = func.call::<()>(()) {
                            logs.push(format!("Timer callback error in '{}': {}", cb, e));
                        }
                    }
                }
            }
        }
    }

    // 2. Process Events
    let events = {
        if let Ok(mut list) = ctx.pending_events.lock() {
            std::mem::take(&mut *list)
        } else {
            Vec::new()
        }
    };

    if !events.is_empty() {
        let listeners_snapshot = if let Ok(l) = ctx.event_listeners.read() {
            l.clone()
        } else {
            HashMap::new()
        };

        for (event_name, payload) in events {
            if let Some(callbacks) = listeners_snapshot.get(&event_name) {
                for cb in callbacks {
                    for (_meta, script) in world.query_mut::<(&crate::ecs::EntityMeta, &mut ScriptComponent)>() {
                        if let Some(lua_arc) = &script.lua {
                            if let Ok(lua) = lua_arc.lock() {
                                if let Ok(func) = lua.globals().get::<mlua::Function>(cb.as_str()) {
                                    let lua_payload = json_to_lua(&lua, &payload).unwrap_or(mlua::Value::Nil);
                                    if let Err(e) = func.call::<()>(lua_payload) {
                                        logs.push(format!("Event callback error on '{}' (listener '{}'): {}", event_name, cb, e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_json_roundtrip() {
        let lua = mlua::Lua::new();
        let val: mlua::Value = lua.load("return { score = 100, player = 'Hero', active = true }").eval().unwrap();
        let json = lua_to_json(&val);
        assert_eq!(json["score"], 100);
        assert_eq!(json["player"], "Hero");
        assert_eq!(json["active"], true);

        let lua_back = json_to_lua(&lua, &json).unwrap();
        if let mlua::Value::Table(tbl) = lua_back {
            let score: i64 = tbl.get("score").unwrap();
            let player: String = tbl.get("player").unwrap();
            let active: bool = tbl.get("active").unwrap();
            assert_eq!(score, 100);
            assert_eq!(player, "Hero");
            assert!(active);
        } else {
            panic!("Expected Lua table");
        }
    }

    #[test]
    fn test_cosmos_script_compilation() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);
        let mut comp = ScriptComponent::new("assets/scripts/cosmos_game.lua".to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Failed to compile cosmos_game.lua: {:?}", res.err());

        // Test one update frame: verify entity transforms, declarative UI elements, and animation presets
        let mut transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let update_res = comp.update(&mut transform, 0.016);
        assert!(update_res.is_ok(), "Failed to run on_update in cosmos_game.lua: {:?}", update_res.err());

        let ui_len = ctx.game_ui_elements.read().unwrap().len();
        assert!(ui_len >= 8, "Expected at least 8 declarative UI elements in cosmos_game, got {}", ui_len);

        let anims_len = ctx.pending_anims.lock().unwrap().len();
        assert!(anims_len >= 3, "Expected at least 3 animation presets triggered in cosmos_game, got {}", anims_len);
    }

    #[test]
    fn test_showcase_v11_script_compilation() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);
        let mut comp = ScriptComponent::new("assets/scripts/showcase_v11.lua".to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Failed to compile showcase_v11.lua: {:?}", res.err());
        assert_eq!(ctx.game_ui_elements.read().unwrap().len(), 10);
    }

    #[test]
    fn test_showcase_v12_script_compilation() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);
        let mut comp = ScriptComponent::new("assets/scripts/showcase_v12.lua".to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Failed to compile showcase_v12.lua: {:?}", res.err());
        assert_eq!(ctx.game_ui_elements.read().unwrap().len(), 14);
        assert_eq!(ctx.timers.lock().unwrap().len(), 1);
        assert_eq!(ctx.event_listeners.read().unwrap().len(), 2);
    }

    #[test]
    fn test_raycast_and_tone_and_trigger() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);

        let test_script = r#"
            trigger_called = false
            other_entity_id = ""

            function on_trigger_enter(other_id)
                trigger_called = true
                other_entity_id = other_id
                play_tone(440, 880, "sine", 0.05, 0.5)
            end

            function on_update(entity, dt)
                local hit = raycast(0, 5, 0, 0, -1, 0, 10)
                assert(hit.hit == false)
            end
        "#;

        let script_path = "assets/scripts/test_trigger.lua";
        std::fs::write(script_path, test_script).unwrap();

        let mut comp = ScriptComponent::new(script_path.to_string());
        let res = comp.init_or_reload(ctx);
        assert!(res.is_ok(), "Init failed: {:?}", res.err());

        let mut transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let res_update = comp.update(&mut transform, 0.016);
        assert!(res_update.is_ok(), "Update failed: {:?}", res_update.err());

        let res_trigger = comp.on_trigger_enter("sensor_zone", "player");
        assert!(res_trigger.is_ok(), "Trigger failed: {:?}", res_trigger.err());

        let _ = std::fs::remove_file(script_path);
    }

    #[test]
    fn test_declarative_ui_bindings() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);

        let test_script = r#"
            ui_clear()
            ui_panel("hud_panel", "top_left", 20, 20, 240, 100, 0.1, 0.1, 0.15, 0.9)
            ui_bar("hp_bar", "top_left", 30, 30, 220, 24, 0.85, "HP: 85/100", 0.2, 0.8, 0.3)
            ui_label("score_lbl", "top_right", 30, 30, "Score: 9999", 18, 1.0, 0.9, 0.2)
            ui_button("btn_dash", "bottom_center", 0, 50, 140, 36, "DASH (SPACE)", "on_dash_click")

            function on_update(entity, dt)
            end
        "#;

        let script_path = "assets/scripts/test_ui_decl.lua";
        std::fs::write(script_path, test_script).unwrap();

        let mut comp = ScriptComponent::new(script_path.to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Init failed: {:?}", res.err());

        let elements = ctx.game_ui_elements.read().unwrap();
        assert_eq!(elements.len(), 4);
        assert_eq!(elements[0].id(), "hud_panel");
        assert_eq!(elements[1].id(), "hp_bar");
        assert_eq!(elements[2].id(), "score_lbl");
        assert_eq!(elements[3].id(), "btn_dash");

        let _ = std::fs::remove_file(script_path);
    }

    #[test]
    fn test_animation_lua_bindings() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);

        let test_script = r#"
            play_anim("boss_leviathan", "rage_mode", false)
            stop_anim("boss_minion")
            local is_boss_playing = is_anim_playing("boss_leviathan")
            assert(is_boss_playing == false)

            function on_update(entity, dt)
            end
        "#;

        let script_path = "assets/scripts/test_anim_lua.lua";
        std::fs::write(script_path, test_script).unwrap();

        let mut comp = ScriptComponent::new(script_path.to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Init failed: {:?}", res.err());

        let anims = ctx.pending_anims.lock().unwrap();
        assert_eq!(anims.len(), 2);
        match &anims[0] {
            AnimationCommand::Play { entity_id, clip_name, looping } => {
                assert_eq!(entity_id, "boss_leviathan");
                assert_eq!(clip_name, "rage_mode");
                assert_eq!(*looping, false);
            }
            _ => panic!("Expected Play command"),
        }
        match &anims[1] {
            AnimationCommand::Stop { entity_id } => {
                assert_eq!(entity_id, "boss_minion");
            }
            _ => panic!("Expected Stop command"),
        }

        let _ = std::fs::remove_file(script_path);
    }

    #[test]
    fn test_cinematics_and_preset_anim_lua_bindings() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);

        if let Ok(mut minfo) = ctx.mouse_info.write() {
            minfo.delta = [12.5, -7.0];
        }

        let test_script = r#"
            camera_lerp_to(0, 10, 20, 0, 0, 0, 1.5)
            camera_orbit_target("boss_entity", 15.0, 30.0, 0.8)
            play_preset_anim("player_hero", "hover_idle")

            local delta = get_mouse_delta()
            assert(delta.dx == 12.5)
            assert(delta.dy == -7.0)

            function on_update(entity, dt)
            end
        "#;

        let script_path = "assets/scripts/test_cinematics_lua.lua";
        std::fs::write(script_path, test_script).unwrap();

        let mut comp = ScriptComponent::new(script_path.to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Init failed: {:?}", res.err());

        let lerps = ctx.pending_camera_lerps.lock().unwrap();
        assert_eq!(lerps.len(), 1);
        assert_eq!(lerps[0].0, [0.0, 10.0, 20.0]);
        assert_eq!(lerps[0].1, [0.0, 0.0, 0.0]);
        assert_eq!(lerps[0].2, 1.5);

        let orbits = ctx.pending_camera_orbits.lock().unwrap();
        assert_eq!(orbits.len(), 1);
        assert_eq!(orbits[0].0.as_deref(), Some("boss_entity"));
        assert_eq!(orbits[0].1, 15.0);

        let anims = ctx.pending_anims.lock().unwrap();
        assert_eq!(anims.len(), 1);
        match &anims[0] {
            AnimationCommand::PlayPreset { entity_id, preset_name } => {
                assert_eq!(entity_id, "player_hero");
                assert_eq!(preset_name, "hover_idle");
            }
            _ => panic!("Expected PlayPreset command"),
        }

        let _ = std::fs::remove_file(script_path);
    }

    #[test]
    fn test_signals_and_timers() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);

        let test_script_path = "assets/scripts/temp_signal_test.lua";
        let script_code = r#"
            received_health = 0
            timer_ticks = 0
            one_shot_fired = false

            function on_damage_taken(payload)
                if payload and payload.amount then
                    received_health = payload.amount
                end
            end

            function on_tick()
                timer_ticks = timer_ticks + 1
            end

            function on_boom()
                one_shot_fired = true
            end

            on_event("player_damaged", "on_damage_taken")
            t_every = every(0.5, "on_tick")
            t_once = after(1.0, "on_boom")
        "#;
        std::fs::write(test_script_path, script_code).unwrap();

        let mut comp = ScriptComponent::new(test_script_path.to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Failed to init test script: {:?}", res.err());

        let mut world = hecs::World::new();
        let e = world.spawn((
            crate::ecs::EntityMeta {
                id: "test_entity".to_string(),
                name: "Test Entity".to_string(),
                parent: None,
            },
            Transform::default(),
            comp,
        ));

        let mut logs = Vec::new();

        // 1. Test Event emission
        {
            let script = world.get::<&ScriptComponent>(e).unwrap();
            let lua_arc = script.lua.as_ref().unwrap();
            let lua = lua_arc.lock().unwrap();
            lua.load(r#"emit_event("player_damaged", { amount = 42 })"#).exec().unwrap();
        }

        process_timers_and_events(&mut world, &ctx, 0.1, &mut logs);

        {
            let script = world.get::<&ScriptComponent>(e).unwrap();
            let lua_arc = script.lua.as_ref().unwrap();
            let lua = lua_arc.lock().unwrap();
            let received: i32 = lua.globals().get("received_health").unwrap();
            assert_eq!(received, 42);
        }

        // 2. Test Timers update
        // Advance 0.5s -> every should trigger once, after not yet
        process_timers_and_events(&mut world, &ctx, 0.5, &mut logs);
        {
            let script = world.get::<&ScriptComponent>(e).unwrap();
            let lua_arc = script.lua.as_ref().unwrap();
            let lua = lua_arc.lock().unwrap();
            let ticks: i32 = lua.globals().get("timer_ticks").unwrap();
            let fired: bool = lua.globals().get("one_shot_fired").unwrap();
            assert_eq!(ticks, 1);
            assert!(!fired);
        }

        // Advance another 0.5s (total 1.0s) -> every should be 2, after should fire
        process_timers_and_events(&mut world, &ctx, 0.5, &mut logs);
        {
            let script = world.get::<&ScriptComponent>(e).unwrap();
            let lua_arc = script.lua.as_ref().unwrap();
            let lua = lua_arc.lock().unwrap();
            let ticks: i32 = lua.globals().get("timer_ticks").unwrap();
            let fired: bool = lua.globals().get("one_shot_fired").unwrap();
            assert_eq!(ticks, 2);
            assert!(fired);
        }

        // Clean up test file
        let _ = std::fs::remove_file(test_script_path);
    }

    #[test]
    fn test_action_input_and_mouse_world_pos() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys.clone(), audio);

        // Position camera looking down from (0, 10, 0) to (0, 0, 0)
        {
            let mut cam = ctx.active_camera.write().unwrap();
            cam.eye = glam::Vec3::new(0.0, 10.0, 0.0001);
            cam.target = glam::Vec3::new(0.0, 0.0, 0.0);
            cam.up = glam::Vec3::Z;
        }

        // Mouse centered on screen 1280x720
        {
            let mut mouse = ctx.mouse_info.write().unwrap();
            mouse.screen_pos = [640.0, 360.0];
            mouse.screen_size = [1280.0, 720.0];
            mouse.buttons_down[0] = true; // Left click
        }

        let test_script_path = "assets/scripts/temp_action_test.lua";
        let script_code = r#"
            is_firing = is_action_down("fire")
            mouse_hit = get_mouse_world_pos(0.0)
            bind_action("boost", "Space")
            has_boost = is_action_down("boost")
        "#;
        std::fs::write(test_script_path, script_code).unwrap();

        let mut comp = ScriptComponent::new(test_script_path.to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Init failed: {:?}", res.err());

        if let Some(lua_arc) = &comp.lua {
            let lua = lua_arc.lock().unwrap();
            let is_firing: bool = lua.globals().get("is_firing").unwrap();
            assert!(is_firing, "Expected is_action_down('fire') to be true with Left Click");

            let mouse_hit: mlua::Table = lua.globals().get("mouse_hit").unwrap();
            let hit: bool = mouse_hit.get("hit").unwrap();
            let y: f32 = mouse_hit.get("y").unwrap();
            assert!(hit, "Expected mouse ray to hit horizontal plane");
            assert!((y - 0.0).abs() < 1e-4);

            let has_boost: bool = lua.globals().get("has_boost").unwrap();
            assert!(!has_boost, "Boost should not be active yet");
        }

        let _ = std::fs::remove_file(test_script_path);
    }

    #[test]
    fn test_nav_lua_bindings() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio);

        let test_script_path = "assets/scripts/temp_nav_test.lua";
        let script_code = r#"
            nav_bake(1.0, 10.0)
            nav_set_blocked(0.0, 0.0, true)
            is_zero_blocked = nav_is_blocked(0.0, 0.0)

            path = find_path(-2.0, 0.0, 2.0, 0.0)
            path_len = 0
            if path then
                path_len = #path
            end
        "#;
        std::fs::write(test_script_path, script_code).unwrap();

        let mut comp = ScriptComponent::new(test_script_path.to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Init failed: {:?}", res.err());

        if let Some(lua_arc) = &comp.lua {
            let lua = lua_arc.lock().unwrap();
            let is_blocked: bool = lua.globals().get("is_zero_blocked").unwrap();
            assert!(is_blocked, "Cell at (0, 0) should be blocked");

            let path_len: i32 = lua.globals().get("path_len").unwrap();
            assert!(path_len > 2, "Expected a valid path around the blocked cell");
        }

        let _ = std::fs::remove_file(test_script_path);
    }

    #[test]
    fn test_audio_buses_lua_bindings() {
        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (_, audio_inner) = crate::audio::AudioManager::init();
        let audio = Arc::new(std::sync::Mutex::new(audio_inner));
        let ctx = LuaEngineContext::new(pressed_keys, audio.clone());

        let test_script_path = "assets/scripts/temp_audio_test.lua";
        let script_code = r#"
            initial_master = get_bus_volume("master")
            set_bus_volume("sfx", 0.6)
            new_sfx = get_bus_volume("sfx")
            play_sound_3d("assets/sfx/bounce.wav", 0.0, 0.0, 0.0, 25.0, 1.0, "sfx")
        "#;
        std::fs::write(test_script_path, script_code).unwrap();

        let mut comp = ScriptComponent::new(test_script_path.to_string());
        let res = comp.init_or_reload(ctx.clone());
        assert!(res.is_ok(), "Init failed: {:?}", res.err());

        if let Some(lua_arc) = &comp.lua {
            let lua = lua_arc.lock().unwrap();
            let initial_master: f32 = lua.globals().get("initial_master").unwrap();
            let new_sfx: f32 = lua.globals().get("new_sfx").unwrap();
            assert_eq!(initial_master, 1.0);
            assert!((new_sfx - 0.6).abs() < 1e-4);
        }

        assert_eq!(audio.lock().unwrap().get_bus_volume("sfx"), 0.6);

        let _ = std::fs::remove_file(test_script_path);
    }
}
