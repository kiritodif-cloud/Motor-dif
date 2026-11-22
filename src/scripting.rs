use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use glam::Vec3;
use crate::ecs::Transform;

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

    pub fn init_or_reload(
        &mut self,
        pressed_keys: Arc<RwLock<HashSet<String>>>,
    ) -> Result<(), String> {
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

        // Expose is_key_down(key) to Lua
        let keys_ref = pressed_keys.clone();
        let is_key_down = lua
            .create_function(move |_, key: String| {
                let keys = keys_ref.read().unwrap();
                let clean = key.trim().to_uppercase();
                Ok(keys.contains(&clean))
            })
            .map_err(|e| format!("Failed to register is_key_down in Lua: {}", e))?;

        lua.globals()
            .set("is_key_down", is_key_down)
            .map_err(|e| format!("Failed to set is_key_down global: {}", e))?;

        // Expose apply_force(fx, fy, fz) to Lua
        let forces_ref = self.pending_forces.clone();
        let apply_force = lua
            .create_function(move |_, (fx, fy, fz): (f32, f32, f32)| {
                if let Ok(mut forces) = forces_ref.lock() {
                    forces.push(Vec3::new(fx, fy, fz));
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register apply_force: {}", e))?;

        lua.globals()
            .set("apply_force", apply_force)
            .map_err(|e| format!("Failed to set apply_force global: {}", e))?;

        // Expose apply_impulse(ix, iy, iz) to Lua
        let impulses_ref = self.pending_impulses.clone();
        let apply_impulse = lua
            .create_function(move |_, (ix, iy, iz): (f32, f32, f32)| {
                if let Ok(mut impulses) = impulses_ref.lock() {
                    impulses.push(Vec3::new(ix, iy, iz));
                }
                Ok(())
            })
            .map_err(|e| format!("Failed to register apply_impulse: {}", e))?;

        lua.globals()
            .set("apply_impulse", apply_impulse)
            .map_err(|e| format!("Failed to set apply_impulse global: {}", e))?;

        // Execute the script chunk to define on_update and globals
        lua.load(&code)
            .exec()
            .map_err(|e| format!("Lua syntax/runtime error in '{}': {}", self.path, e))?;

        self.lua = Some(Arc::new(Mutex::new(lua)));
        self.last_error = None;
        self.has_error = false;
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

                    // Also bind entity.apply_force and entity.apply_impulse methods
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
