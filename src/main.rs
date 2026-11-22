pub mod assets;
pub mod camera;
pub mod ecs;
pub mod hot_reload;
pub mod mcp_server;
pub mod physics;
pub mod renderer;
pub mod scripting;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use egui::{CentralPanel, Frame, Panel};
use assets::SceneDef;
use camera::{Camera, CameraMode};
use ecs::{EntityMeta, MeshRender, Transform};
use glam::Vec3;
use hot_reload::{HotReloadEvent, HotReloadManager};
use mcp_server::{EngineLogsResponse, McpCommand, WorldInspection};
use physics::{PhysicsEngine, RigidBodyComponent};
use renderer::{create_instance_buffer, GpuContext, InstanceRaw};
use scripting::ScriptComponent;
use tokio::sync::{mpsc, oneshot};
use wgpu::CurrentSurfaceTexture;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    Edit,
    Play,
    Pause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Console,
    Assets,
    ScriptEditor,
}

#[derive(Clone)]
pub struct HistoryState {
    pub undo_stack: Vec<SceneDef>,
    pub redo_stack: Vec<SceneDef>,
    pub max_history: usize,
}

impl HistoryState {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 30,
        }
    }

    pub fn push(&mut self, state: SceneDef) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(state);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: SceneDef) -> Option<SceneDef> {
        if let Some(prev) = self.undo_stack.pop() {
            if self.redo_stack.len() >= self.max_history {
                self.redo_stack.remove(0);
            }
            self.redo_stack.push(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: SceneDef) -> Option<SceneDef> {
        if let Some(next) = self.redo_stack.pop() {
            if self.undo_stack.len() >= self.max_history {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(current);
            Some(next)
        } else {
            None
        }
    }
}

pub struct EngineState {
    pub window: Arc<Window>,
    pub gpu: GpuContext,
    pub world: hecs::World,
    pub camera: Camera,
    pub physics: PhysicsEngine,
    pub hot_reload: HotReloadManager,
    pub mcp_cmd_rx: mpsc::Receiver<McpCommand>,
    pub mcp_server_addr: String,
    pub _mcp_handle: mcp_server::McpServerHandle,
    pub pending_viewport_capture: Option<oneshot::Sender<Result<String, String>>>,
    pub last_frame_time: Instant,
    pub fps: f32,
    pub frame_counter: u32,
    pub fps_timer: Instant,
    pub logs: Vec<String>,
    pub pressed_keys: Arc<RwLock<HashSet<String>>>,
    pub last_lua_error: Option<String>,

    // Editor State
    pub play_mode: PlayMode,
    pub play_snapshot: Option<SceneDef>,
    pub gizmo_mode: GizmoMode,
    pub snap_enabled: bool,
    pub debug_colliders: bool,
    pub selected_entity: Option<hecs::Entity>,
    pub hierarchy_search: String,
    pub bottom_tab: BottomTab,
    pub script_editor_path: Option<String>,
    pub script_editor_content: String,
    pub history: HistoryState,
    pub entity_counter: u32,
    pub assets_cache: Vec<String>,
    pub last_assets_scan: Instant,
}

impl EngineState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let aspect = size.width as f32 / size.height.max(1) as f32;
        let mut camera = Camera::new_3d(Vec3::new(0.0, 3.5, 9.0), Vec3::ZERO, aspect);

        let mut gpu = GpuContext::new(window.clone()).await;
        gpu.update_camera(&camera);

        if !Path::new("assets/scripts").exists() {
            let _ = std::fs::create_dir_all("assets/scripts");
        }
        if !Path::new("assets/models").exists() {
            let _ = std::fs::create_dir_all("assets/models");
        }

        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));

        let mut world = hecs::World::new();
        let mut physics = PhysicsEngine::new();
        let mut logs = Vec::new();
        logs.push("Motor-dif engine initialized (Professional Editor Mode).".to_string());

        let scene_path = Path::new("assets/scene.json");
        if !scene_path.exists() {
            let default_scene = SceneDef::default();
            if let Ok(json) = serde_json::to_string_pretty(&default_scene) {
                let _ = std::fs::write(scene_path, json);
                logs.push("Created default assets/scene.json".to_string());
            }
        }

        match assets::load_scene_from_file(scene_path) {
            Ok(scene) => {
                assets::apply_scene_to_world(&scene, &mut world, &mut camera, &mut physics, Some(&pressed_keys));
                logs.push(format!("Scene loaded from assets/scene.json ({} entities)", world.len()));
            }
            Err(e) => {
                logs.push(format!("Warning loading scene: {}", e));
            }
        }

        let hot_reload = match HotReloadManager::new("assets") {
            Ok(hr) => {
                logs.push("Hot-Reload Watcher: active on 'assets/'".to_string());
                hr
            }
            Err(e) => {
                logs.push(format!("Hot-Reload Watcher failed: {}", e));
                HotReloadManager::fallback(e)
            }
        };

        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let mcp_handle = mcp_server::start_mcp_server(cmd_tx, 3030);
        let mcp_server_addr = mcp_handle.bound_addr.to_string();
        logs.push(format!("MCP Server: running at http://{}/mcp and stdio", mcp_server_addr));

        logs.push("Renderer: wgpu + WGSL directional lighting (Low-PBR).".to_string());
        logs.push("Depth Buffer: 32-bit Float enabled.".to_string());
        logs.push("Scripting: mlua (Lua 5.4 vendored) ready.".to_string());
        logs.push("Physics: Rapier3D (fixed 60Hz step) ready.".to_string());

        let mut initial_script_content = String::new();
        let initial_script_path = if Path::new("assets/scripts/orbit.lua").exists() {
            if let Ok(c) = std::fs::read_to_string("assets/scripts/orbit.lua") {
                initial_script_content = c;
            }
            Some("assets/scripts/orbit.lua".to_string())
        } else {
            None
        };

        Self {
            window,
            gpu,
            world,
            camera,
            physics,
            hot_reload,
            mcp_cmd_rx: cmd_rx,
            mcp_server_addr,
            _mcp_handle: mcp_handle,
            pending_viewport_capture: None,
            last_frame_time: Instant::now(),
            fps: 60.0,
            frame_counter: 0,
            fps_timer: Instant::now(),
            logs,
            pressed_keys,
            last_lua_error: None,

            play_mode: PlayMode::Edit,
            play_snapshot: None,
            gizmo_mode: GizmoMode::Translate,
            snap_enabled: false,
            debug_colliders: true,
            selected_entity: None,
            hierarchy_search: String::new(),
            bottom_tab: BottomTab::Console,
            script_editor_path: initial_script_path,
            script_editor_content: initial_script_content,
            history: HistoryState::new(),
            entity_counter: 10,
            assets_cache: Vec::new(),
            last_assets_scan: Instant::now() - std::time::Duration::from_secs(10),
        }
    }

    pub fn push_history(&mut self) {
        let snapshot = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        self.history.push(snapshot);
    }

    pub fn undo(&mut self) {
        let current = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        if let Some(prev) = self.history.undo(current) {
            assets::apply_scene_to_world(&prev, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.pressed_keys));
            self.selected_entity = None;
            self.logs.push("Undo performed".to_string());
        }
    }

    pub fn redo(&mut self) {
        let current = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        if let Some(next) = self.history.redo(current) {
            assets::apply_scene_to_world(&next, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.pressed_keys));
            self.selected_entity = None;
            self.logs.push("Redo performed".to_string());
        }
    }

    pub fn save_scene_to_file(&mut self, path: &str) {
        let scene = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        self.hot_reload.suppress_scene_events(std::time::Duration::from_millis(600));
        match assets::save_scene_to_file(path, &scene) {
            Ok(()) => {
                let msg = format!("Scene successfully saved to '{}'", path);
                self.logs.push(msg.clone());
                self.hot_reload.last_reload_status = Some(msg);
            }
            Err(e) => {
                self.logs.push(format!("Error saving scene: {}", e));
            }
        }
    }

    pub fn reload_scene(&mut self) {
        match assets::load_scene_from_file("assets/scene.json") {
            Ok(scene) => {
                assets::apply_scene_to_world(&scene, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.pressed_keys));
                self.selected_entity = None;
                self.hot_reload.last_reload_error = None;
                let msg = format!("Hot-Reload: Scene successfully reloaded ({} entities)", self.world.len());
                self.logs.push(msg.clone());
                self.hot_reload.last_reload_status = Some(msg);
            }
            Err(err) => {
                self.hot_reload.last_reload_error = Some(err.clone());
                self.logs.push(format!("Hot-Reload ERROR: {}", err));
            }
        }
    }

    pub fn spawn_primitive(&mut self, mesh_type: &str, color: [f32; 4]) {
        self.push_history();
        self.entity_counter += 1;
        let id = format!("{}_{}", mesh_type, self.entity_counter);
        let meta = EntityMeta {
            id: id.clone(),
            name: id.clone(),
        };
        let transform = Transform::from_xyz(0.0, 1.0, 0.0);
        let mesh = MeshRender {
            mesh_type: mesh_type.to_string(),
            color,
            emissive: 0.0,
        };

        let entity = self.world.spawn((meta, transform, mesh));
        self.selected_entity = Some(entity);
        self.logs.push(format!("Spawned primitive entity '{}'", id));
    }

    pub fn duplicate_selected(&mut self) {
        if let Some(entity) = self.selected_entity {
            let meta_val = if let Ok(m) = self.world.get::<&EntityMeta>(entity) {
                Some((*m).clone())
            } else {
                None
            };
            let trans_val = if let Ok(t) = self.world.get::<&Transform>(entity) {
                Some(*t)
            } else {
                None
            };
            let mesh_val = if let Ok(m) = self.world.get::<&MeshRender>(entity) {
                Some((*m).clone())
            } else {
                None
            };
            let script_val = if let Ok(s) = self.world.get::<&ScriptComponent>(entity) {
                Some(s.path.clone())
            } else {
                None
            };
            let phys_val = if let Ok(rb) = self.world.get::<&RigidBodyComponent>(entity) {
                self.physics.rigid_body_set.get(rb.handle).map(|b| (b.is_dynamic(), b.mass()))
            } else {
                None
            };

            let (meta, transform, mesh) = match (meta_val, trans_val, mesh_val) {
                (Some(m), Some(t), Some(mesh)) => (m, t, mesh),
                _ => return,
            };

            self.push_history();
            self.entity_counter += 1;

            let new_id = format!("{}_copy{}", meta.id, self.entity_counter);
            let new_meta = EntityMeta {
                id: new_id.clone(),
                name: new_id,
            };
            let mut new_trans = transform;
            new_trans.position.x += 1.0;
            let new_e = self.world.spawn((new_meta, new_trans, mesh));

            if let Some(script_path) = script_val {
                let mut s_comp = ScriptComponent::new(script_path);
                let _ = s_comp.init_or_reload(self.pressed_keys.clone());
                let _ = self.world.insert_one(new_e, s_comp);
            }

            if let Some((is_dyn, mass)) = phys_val {
                let phys_def = assets::PhysicsDef {
                    body: if is_dyn { "dynamic".to_string() } else { "fixed".to_string() },
                    collider: "cuboid".to_string(),
                    mass,
                    restitution: 0.5,
                };
                let handle = self.physics.add_body(&new_trans, &phys_def);
                let _ = self.world.insert_one(new_e, RigidBodyComponent { handle });
            }

            self.selected_entity = Some(new_e);
            self.logs.push(format!("Duplicated entity to '{}'", new_trans.position));
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(entity) = self.selected_entity.take() {
            self.push_history();
            if let Ok(rb) = self.world.get::<&RigidBodyComponent>(entity) {
                self.physics.remove_body(rb.handle);
            }
            let _ = self.world.despawn(entity);
            self.logs.push("Deleted selected entity".to_string());
        }
    }

    pub fn scan_assets(&mut self) {
        if self.last_assets_scan.elapsed().as_secs_f32() < 2.0 {
            return;
        }
        self.last_assets_scan = Instant::now();
        let mut list = Vec::new();
        fn visit_dirs(dir: &Path, list: &mut Vec<String>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        visit_dirs(&path, list);
                    } else if let Some(p) = path.to_str() {
                        let clean = p.replace('\\', "/");
                        list.push(clean);
                    }
                }
            }
        }
        visit_dirs(Path::new("assets"), &mut list);
        list.sort();
        self.assets_cache = list;
    }

    pub fn handle_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
        let response = self.gpu.egui_winit_state.on_window_event(&self.window, event);
        if response.consumed {
            return;
        }

        let egui_wants_kbd = self.gpu.egui_winit_state.egui_ctx().egui_wants_keyboard_input();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.gpu.resize(*physical_size);
                self.camera.aspect =
                    physical_size.width as f32 / physical_size.height.max(1) as f32;
                self.window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key,
                        state,
                        logical_key,
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == winit::event::ElementState::Pressed;
                let mut keys_to_update = Vec::new();

                match logical_key {
                    winit::keyboard::Key::Character(s) => {
                        keys_to_update.push(s.to_uppercase());
                    }
                    winit::keyboard::Key::Named(named) => {
                        keys_to_update.push(format!("{:?}", named).to_uppercase());
                    }
                    _ => {}
                }

                if let winit::keyboard::PhysicalKey::Code(code) = physical_key {
                    let code_str = format!("{:?}", code);
                    let clean = code_str.strip_prefix("Key").unwrap_or(&code_str).to_uppercase();
                    keys_to_update.push(clean);
                }

                if let Ok(mut set) = self.pressed_keys.write() {
                    for k in keys_to_update {
                        if is_pressed {
                            set.insert(k);
                        } else {
                            set.remove(&k);
                        }
                    }
                }

                // Keyboard Shortcuts when not focused on egui text edit
                if is_pressed && !egui_wants_kbd {
                    let (_ctrl, do_s, do_z, do_y, do_d, do_del, do_w, do_e, do_r, do_f) = {
                        let set = self.pressed_keys.read().unwrap();
                        let ctrl_down = set.contains("CONTROL") || set.contains("CONTROLRIGHT") || set.contains("CONTROLLEFT");
                        (
                            ctrl_down,
                            ctrl_down && set.contains("S"),
                            ctrl_down && set.contains("Z"),
                            ctrl_down && set.contains("Y"),
                            ctrl_down && set.contains("D"),
                            !ctrl_down && set.contains("DELETE"),
                            !ctrl_down && set.contains("W"),
                            !ctrl_down && set.contains("E"),
                            !ctrl_down && set.contains("R"),
                            !ctrl_down && set.contains("F"),
                        )
                    };

                    if do_s {
                        self.save_scene_to_file("assets/scene.json");
                    } else if do_z {
                        self.undo();
                    } else if do_y {
                        self.redo();
                    } else if do_d {
                        self.duplicate_selected();
                    } else if do_del {
                        self.delete_selected();
                    } else if do_w {
                        self.gizmo_mode = GizmoMode::Translate;
                    } else if do_e {
                        self.gizmo_mode = GizmoMode::Rotate;
                    } else if do_r {
                        self.gizmo_mode = GizmoMode::Scale;
                    } else if do_f {
                        if let Some(entity) = self.selected_entity {
                            if let Ok(trans) = self.world.get::<&Transform>(entity) {
                                let offset = self.camera.eye - self.camera.target;
                                self.camera.target = trans.position;
                                self.camera.eye = trans.position + offset;
                            }
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.update_and_render();
                self.window.request_redraw();
            }
            _ => {}
        }
    }

    pub fn process_mcp_commands(&mut self) {
        while let Ok(cmd) = self.mcp_cmd_rx.try_recv() {
            match cmd {
                McpCommand::InspectWorld { respond_to } => {
                    let scene = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
                    let _ = respond_to.send(WorldInspection {
                        camera: scene.camera,
                        entities: scene.entities,
                    });
                }
                McpCommand::UpsertEntity { entity, respond_to } => {
                    self.push_history();
                    let mut found_e = None;
                    for entity_ref in self.world.iter() {
                        if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                            if meta.id == entity.id {
                                found_e = Some(entity_ref.entity());
                                break;
                            }
                        }
                    }

                    if let Some(e) = found_e {
                        if let Ok(mut transform) = self.world.get::<&mut Transform>(e) {
                            transform.position = Vec3::from(entity.position);
                            transform.rotation = Vec3::from(entity.rotation);
                            transform.scale = Vec3::from(entity.scale);
                        }
                        if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(e) {
                            mesh.mesh_type = entity.mesh.clone();
                            mesh.color = entity.color;
                            mesh.emissive = entity.emissive;
                        }
                        if let Some(script_path) = &entity.script {
                            let mut script_comp = ScriptComponent::new(script_path.clone());
                            let _ = script_comp.init_or_reload(self.pressed_keys.clone());
                            let _ = self.world.insert_one(e, script_comp);
                        }
                        if let Some(phys) = &entity.physics {
                            let transform_opt = self.world.get::<&Transform>(e).map(|t| *t);
                            if let Ok(transform) = transform_opt {
                                let handle = self.physics.add_body(&transform, phys);
                                let _ = self.world.insert_one(e, RigidBodyComponent { handle });
                            }
                        }
                    } else {
                        let meta = EntityMeta {
                            id: entity.id.clone(),
                            name: entity.id.clone(),
                        };
                        let transform = Transform::new(
                            Vec3::from(entity.position),
                            Vec3::from(entity.rotation),
                            Vec3::from(entity.scale),
                        );
                        let mesh = MeshRender {
                            mesh_type: entity.mesh.clone(),
                            color: entity.color,
                            emissive: entity.emissive,
                        };

                        let e = self.world.spawn((meta, transform, mesh));

                        if let Some(phys) = &entity.physics {
                            let handle = self.physics.add_body(&transform, phys);
                            let _ = self.world.insert_one(e, RigidBodyComponent { handle });
                        }

                        if let Some(script_path) = &entity.script {
                            let mut script_comp = ScriptComponent::new(script_path.clone());
                            let _ = script_comp.init_or_reload(self.pressed_keys.clone());
                            let _ = self.world.insert_one(e, script_comp);
                        }
                    }

                    let msg = format!("Entity '{}' successfully upserted in world", entity.id);
                    self.logs.push(msg.clone());
                    let _ = respond_to.send(Ok(msg));
                }
                McpCommand::GetEngineLogs { respond_to } => {
                    let lua_err = self.last_lua_error.clone().or_else(|| self.hot_reload.last_reload_error.clone());
                    let _ = respond_to.send(EngineLogsResponse {
                        fps: self.fps,
                        logs: self.logs.clone(),
                        lua_errors: lua_err,
                    });
                }
                McpCommand::CaptureViewport { respond_to } => {
                    self.pending_viewport_capture = Some(respond_to);
                }
            }
        }
    }

    pub fn update_and_render(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;

        self.frame_counter += 1;
        if self.fps_timer.elapsed().as_secs_f32() >= 0.5 {
            self.fps = self.frame_counter as f32 / self.fps_timer.elapsed().as_secs_f32();
            self.frame_counter = 0;
            self.fps_timer = Instant::now();
        }

        self.process_mcp_commands();
        self.scan_assets();

        // Poll hot-reload filesystem events
        let events = self.hot_reload.poll_events();
        for event in events {
            match event {
                HotReloadEvent::SceneChanged => {
                    self.reload_scene();
                }
                HotReloadEvent::ScriptChanged(path) => {
                    let script_str = path.to_string_lossy().replace('\\', "/");
                    let mut reloaded = false;
                    for (meta, script) in self.world.query_mut::<(&EntityMeta, &mut ScriptComponent)>() {
                        let comp_str = script.path.replace('\\', "/");
                        if script_str.ends_with(&comp_str) || comp_str.ends_with(&script_str) {
                            match script.init_or_reload(self.pressed_keys.clone()) {
                                Ok(()) => {
                                    let msg = format!("Hot-Reload: Script '{}' reloaded for entity '{}'", script.path, meta.id);
                                    self.logs.push(msg.clone());
                                    self.hot_reload.last_reload_status = Some(msg);
                                    self.last_lua_error = None;
                                    reloaded = true;
                                }
                                Err(e) => {
                                    self.last_lua_error = Some(e.clone());
                                    self.logs.push(format!("Script compile error: {}", e));
                                }
                            }
                        }
                    }
                    if !reloaded {
                        self.logs.push(format!("Script modified: {}", path.display()));
                    }
                }
                HotReloadEvent::Other(_) => {}
            }
        }

        // Run simulation only if PlayMode::Play
        if self.play_mode == PlayMode::Play {
            // Run Lua scripts
            for (meta, transform, script) in self.world.query_mut::<(&EntityMeta, &mut Transform, &mut ScriptComponent)>() {
                if let Err(e) = script.update(transform, dt) {
                    let err_msg = format!("Lua runtime error on '{}': {}", meta.id, e);
                    if self.last_lua_error.as_ref() != Some(&err_msg) {
                        self.logs.push(err_msg.clone());
                        self.last_lua_error = Some(err_msg);
                    }
                }
            }

            // Apply forces & impulses
            for (rb, script) in self.world.query_mut::<(&RigidBodyComponent, &mut ScriptComponent)>() {
                if let Ok(mut forces) = script.pending_forces.lock() {
                    for f in forces.drain(..) {
                        self.physics.apply_force(rb.handle, f);
                    }
                }
                if let Ok(mut impulses) = script.pending_impulses.lock() {
                    for i in impulses.drain(..) {
                        self.physics.apply_impulse(rb.handle, i);
                    }
                }
            }

            // Step Physics
            self.physics.step(dt);
            self.physics.sync_to_ecs(&mut self.world);
        }

        // Update camera in GPU
        self.gpu.update_camera(&self.camera);

        // Collect instances by mesh type
        let mut instances_by_mesh: HashMap<String, Vec<InstanceRaw>> = HashMap::new();
        for (transform, mesh) in self.world.query::<(&Transform, &MeshRender)>().iter() {
            let raw = InstanceRaw {
                model: transform.matrix().to_cols_array_2d(),
                color: mesh.color,
                extra: [mesh.emissive, 0.0, 0.0, 0.0],
            };
            instances_by_mesh.entry(mesh.mesh_type.clone()).or_default().push(raw);
        }

        for mesh_id in instances_by_mesh.keys() {
            self.gpu.ensure_mesh(mesh_id);
        }

        let output = match self.gpu.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => frame,
            CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                self.gpu.resize(self.gpu.size);
                return;
            }
            _ => {
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            self.gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main Command Encoder"),
                });

        // Egui Pass Preparation
        let raw_input = self.gpu.egui_winit_state.take_egui_input(&self.window);

        // Extract UI state variables
        let fps = self.fps;
        let _mcp_addr = self.mcp_server_addr.clone();
        let hot_reload_error = self.hot_reload.last_reload_error.clone();
        let hot_reload_status = self.hot_reload.last_reload_status.clone();
        let lua_error = self.last_lua_error.clone();

        let mut trigger_save_scene = false;
        let mut trigger_new_scene = false;
        let mut trigger_reload_scene = false;
        let mut trigger_spawn_mesh = None;
        let mut trigger_duplicate = false;
        let mut trigger_delete = false;
        let mut trigger_save_script = false;
        let mut script_to_open = None;

        let mut trigger_refresh_assets = false;
        let egui_ctx = self.gpu.egui_winit_state.egui_ctx().clone();

        let full_output = egui_ctx.run_ui(raw_input, |ui| {
                ui.ctx().set_visuals(egui::Visuals::dark());

                // 1. TOP MENU & TOOLBAR
                Panel::top("top_toolbar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("📄 New Empty Scene").clicked() {
                                trigger_new_scene = true;
                                ui.close();
                            }
                            if ui.button("💾 Save Scene (Ctrl+S)").clicked() {
                                trigger_save_scene = true;
                                ui.close();
                            }
                            if ui.button("🔄 Reload Scene").clicked() {
                                trigger_reload_scene = true;
                                ui.close();
                            }
                        });

                        ui.separator();

                        // Simulation Controls Play / Pause / Stop
                        let is_playing = self.play_mode == PlayMode::Play;
                        let is_paused = self.play_mode == PlayMode::Pause;

                        if ui.selectable_label(is_playing, "▶ Play").clicked() {
                            if self.play_mode == PlayMode::Edit {
                                self.play_snapshot = Some(assets::extract_scene_from_world(&self.world, &self.camera, &self.physics));
                            }
                            self.play_mode = PlayMode::Play;
                            self.logs.push("Simulation started (Play Mode)".to_string());
                        }

                        if ui.selectable_label(is_paused, "⏸ Pause").clicked() {
                            self.play_mode = PlayMode::Pause;
                            self.logs.push("Simulation paused".to_string());
                        }

                        if ui.button("⏹ Stop").clicked() {
                            if let Some(snapshot) = self.play_snapshot.take() {
                                assets::apply_scene_to_world(&snapshot, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.pressed_keys));
                                self.logs.push("Simulation stopped. Scene restored to initial snapshot.".to_string());
                            }
                            self.play_mode = PlayMode::Edit;
                        }

                        ui.separator();

                        // Gizmo Mode Selectors
                        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Translate, "✛ [W] Move");
                        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Rotate, "⟲ [E] Rotate");
                        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Scale, "⤢ [R] Scale");

                        ui.separator();
                        ui.checkbox(&mut self.snap_enabled, "Snap (Ctrl)");
                        ui.checkbox(&mut self.debug_colliders, "Debug Colliders");

                        ui.separator();
                        let cam_mode_str = match self.camera.mode {
                            CameraMode::Perspective => "3D",
                            CameraMode::Orthographic => "2D",
                        };
                        if ui.button(format!("📷 Camera: {}", cam_mode_str)).clicked() {
                            self.camera.toggle_mode();
                        }

                        ui.separator();
                        ui.label(format!("{:.1} FPS", fps));

                        ui.separator();
                        ui.colored_label(egui::Color32::from_rgb(100, 255, 100), format!("● MCP: 3030"));
                    });
                });

                // 2. LEFT PANEL: SCENE HIERARCHY
                Panel::left("scene_hierarchy")
                    .resizable(true)
                    .default_size(240.0)
                    .show(ui, |ui| {
                        ui.heading("Scene Hierarchy");
                        ui.separator();

                        // Quick creation buttons
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("+ Cube").clicked() {
                                trigger_spawn_mesh = Some(("cube", [0.8, 0.4, 0.3, 1.0]));
                            }
                            if ui.button("+ Sphere").clicked() {
                                trigger_spawn_mesh = Some(("sphere", [0.3, 0.7, 0.9, 1.0]));
                            }
                            if ui.button("+ Cylinder").clicked() {
                                trigger_spawn_mesh = Some(("cylinder", [0.4, 0.8, 0.4, 1.0]));
                            }
                            if ui.button("+ Quad").clicked() {
                                trigger_spawn_mesh = Some(("quad", [0.9, 0.8, 0.2, 1.0]));
                            }
                        });

                        ui.horizontal(|ui| {
                            if ui.button("Duplicate (Ctrl+D)").clicked() {
                                trigger_duplicate = true;
                            }
                            if ui.button("Delete (Del)").clicked() {
                                trigger_delete = true;
                            }
                        });

                        ui.separator();
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.hierarchy_search);
                        ui.separator();

                        // Entity List
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut entity_items = Vec::new();
                            for entity_ref in self.world.iter() {
                                let entity = entity_ref.entity();
                                if let (Some(meta), Some(mesh)) = (entity_ref.get::<&EntityMeta>(), entity_ref.get::<&MeshRender>()) {
                                    if self.hierarchy_search.is_empty() || meta.id.to_lowercase().contains(&self.hierarchy_search.to_lowercase()) {
                                        entity_items.push((entity, meta.id.clone(), mesh.mesh_type.clone()));
                                    }
                                }
                            }

                            for (entity, id, mesh_type) in entity_items {
                                let is_selected = self.selected_entity == Some(entity);
                                let label_text = format!("📦 {} ({})", id, mesh_type);
                                if ui.selectable_label(is_selected, label_text).clicked() {
                                    self.selected_entity = Some(entity);
                                }
                            }
                        });
                    });

                // 3. RIGHT PANEL: INSPECTOR
                Panel::right("inspector_panel")
                    .resizable(true)
                    .default_size(290.0)
                    .show(ui, |ui| {
                        ui.heading("Inspector");
                        ui.separator();

                        if let Some(entity) = self.selected_entity {
                            if self.world.contains(entity) {
                                // 1. Identity
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Entity Identity").strong());
                                    if let Ok(mut meta) = self.world.get::<&mut EntityMeta>(entity) {
                                        ui.horizontal(|ui| {
                                            ui.label("ID:");
                                            ui.text_edit_singleline(&mut meta.id);
                                        });
                                    }
                                });

                                // 2. Transform
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Transform").strong());
                                    if let Ok(mut transform) = self.world.get::<&mut Transform>(entity) {
                                        ui.label("Position:");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut transform.position.x).speed(0.05).prefix("X: "));
                                            ui.add(egui::DragValue::new(&mut transform.position.y).speed(0.05).prefix("Y: "));
                                            ui.add(egui::DragValue::new(&mut transform.position.z).speed(0.05).prefix("Z: "));
                                        });

                                        ui.label("Rotation (Degrees):");
                                        let mut deg_x = transform.rotation.x.to_degrees();
                                        let mut deg_y = transform.rotation.y.to_degrees();
                                        let mut deg_z = transform.rotation.z.to_degrees();
                                        ui.horizontal(|ui| {
                                            if ui.add(egui::DragValue::new(&mut deg_x).speed(1.0).prefix("X: ").suffix("°")).changed() {
                                                transform.rotation.x = deg_x.to_radians();
                                            }
                                            if ui.add(egui::DragValue::new(&mut deg_y).speed(1.0).prefix("Y: ").suffix("°")).changed() {
                                                transform.rotation.y = deg_y.to_radians();
                                            }
                                            if ui.add(egui::DragValue::new(&mut deg_z).speed(1.0).prefix("Z: ").suffix("°")).changed() {
                                                transform.rotation.z = deg_z.to_radians();
                                            }
                                        });

                                        ui.label("Scale:");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut transform.scale.x).speed(0.05).prefix("X: ").range(0.01..=100.0));
                                            ui.add(egui::DragValue::new(&mut transform.scale.y).speed(0.05).prefix("Y: ").range(0.01..=100.0));
                                            ui.add(egui::DragValue::new(&mut transform.scale.z).speed(0.05).prefix("Z: ").range(0.01..=100.0));
                                        });
                                    }
                                });

                                // 3. Render & Material
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Render & Material").strong());
                                    if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(entity) {
                                        ui.horizontal(|ui| {
                                            ui.label("Color:");
                                            ui.color_edit_button_rgba_unmultiplied(&mut mesh.color);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Emissive:");
                                            ui.add(egui::Slider::new(&mut mesh.emissive, 0.0..=1.0));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Mesh:");
                                            egui::ComboBox::from_id_salt("mesh_select")
                                                .selected_text(&mesh.mesh_type)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut mesh.mesh_type, "cube".to_string(), "Cube");
                                                    ui.selectable_value(&mut mesh.mesh_type, "sphere".to_string(), "Sphere");
                                                    ui.selectable_value(&mut mesh.mesh_type, "cylinder".to_string(), "Cylinder");
                                                    ui.selectable_value(&mut mesh.mesh_type, "quad".to_string(), "Quad");
                                                    ui.selectable_value(&mut mesh.mesh_type, "plane".to_string(), "Plane");
                                                });
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Custom Path:");
                                            ui.text_edit_singleline(&mut mesh.mesh_type);
                                        });
                                    }
                                });

                                // 4. Scripting
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Scripting (Lua)").strong());
                                    let mut current_script = self.world.get::<&ScriptComponent>(entity).map(|s| s.path.clone()).unwrap_or_default();
                                    let mut script_changed = false;
                                    ui.horizontal(|ui| {
                                        ui.label("Path:");
                                        if ui.text_edit_singleline(&mut current_script).changed() {
                                            script_changed = true;
                                        }
                                    });

                                    if !current_script.is_empty() {
                                        if ui.button("✏ Edit Script").clicked() {
                                            script_to_open = Some(current_script.clone());
                                        }
                                    }

                                    if script_changed {
                                        if current_script.trim().is_empty() {
                                            let _ = self.world.remove_one::<ScriptComponent>(entity);
                                        } else {
                                            let mut comp = ScriptComponent::new(current_script);
                                            let _ = comp.init_or_reload(self.pressed_keys.clone());
                                            let _ = self.world.insert_one(entity, comp);
                                        }
                                    }
                                });

                                // 5. Physics
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Physics (Rapier3D)").strong());
                                    let has_physics = self.world.get::<&RigidBodyComponent>(entity).is_ok();
                                    let mut enable_physics = has_physics;

                                    if ui.checkbox(&mut enable_physics, "Enable Physics").changed() {
                                        if enable_physics {
                                            let trans_opt = if let Ok(trans) = self.world.get::<&Transform>(entity) {
                                                Some(*trans)
                                            } else {
                                                None
                                            };
                                            if let Some(trans) = trans_opt {
                                                let phys_def = assets::PhysicsDef {
                                                    body: "dynamic".to_string(),
                                                    collider: "cuboid".to_string(),
                                                    mass: 1.0,
                                                    restitution: 0.4,
                                                };
                                                let handle = self.physics.add_body(&trans, &phys_def);
                                                let _ = self.world.insert_one(entity, RigidBodyComponent { handle });
                                            }
                                        } else {
                                            let rb_handle = if let Ok(rb) = self.world.get::<&RigidBodyComponent>(entity) {
                                                Some(rb.handle)
                                            } else {
                                                None
                                            };
                                            if let Some(handle) = rb_handle {
                                                self.physics.remove_body(handle);
                                            }
                                            let _ = self.world.remove_one::<RigidBodyComponent>(entity);
                                        }
                                    }

                                    if let Ok(rb) = self.world.get::<&RigidBodyComponent>(entity) {
                                        if let Some(body) = self.physics.rigid_body_set.get_mut(rb.handle) {
                                            let mut is_dynamic = body.is_dynamic();
                                            if ui.checkbox(&mut is_dynamic, "Dynamic Body").changed() {
                                                if is_dynamic {
                                                    body.set_body_type(rapier3d::prelude::RigidBodyType::Dynamic, true);
                                                } else {
                                                    body.set_body_type(rapier3d::prelude::RigidBodyType::Fixed, true);
                                                }
                                            }

                                            let mut mass = body.mass();
                                            if ui.add(egui::DragValue::new(&mut mass).speed(0.1).prefix("Mass: ").range(0.01..=1000.0)).changed() {
                                                // mass updated
                                            }
                                        }
                                    }
                                });
                            } else {
                                self.selected_entity = None;
                                ui.label("Selected entity no longer exists.");
                            }
                        } else {
                            ui.label("No entity selected.");
                            ui.label("Select an entity from the hierarchy or viewport to inspect and modify it.");
                        }
                    });

                // 4. BOTTOM PANEL: CONSOLE, ASSETS BROWSER, SCRIPT EDITOR
                Panel::bottom("bottom_panel")
                    .resizable(true)
                    .default_size(200.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.bottom_tab, BottomTab::Console, "📋 Console & MCP");
                            ui.selectable_value(&mut self.bottom_tab, BottomTab::Assets, "📁 Assets Browser");
                            ui.selectable_value(&mut self.bottom_tab, BottomTab::ScriptEditor, "💻 Script Editor");
                        });
                        ui.separator();

                        match self.bottom_tab {
                            BottomTab::Console => {
                                ui.horizontal(|ui| {
                                    if ui.button("Clear Logs").clicked() {
                                        self.logs.clear();
                                    }
                                    if let Some(err) = &lua_error {
                                        ui.colored_label(egui::Color32::from_rgb(255, 90, 90), format!("Lua Error: {}", err));
                                    }
                                    if let Some(err) = &hot_reload_error {
                                        ui.colored_label(egui::Color32::from_rgb(255, 120, 80), format!("Hot-Reload: {}", err));
                                    }
                                    if let Some(status) = &hot_reload_status {
                                        ui.colored_label(egui::Color32::from_rgb(100, 220, 100), status);
                                    }
                                });
                                ui.separator();
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        for log in &self.logs {
                                            if log.contains("ERROR") || log.contains("error") {
                                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), log);
                                            } else if log.contains("MCP") {
                                                ui.colored_label(egui::Color32::from_rgb(100, 200, 255), log);
                                            } else {
                                                ui.monospace(log);
                                            }
                                        }
                                    });
                            }
                            BottomTab::Assets => {
                                ui.horizontal(|ui| {
                                    ui.label("Files in 'assets/':");
                                    if ui.button("🔄 Refresh").clicked() {
                                        trigger_refresh_assets = true;
                                    }
                                });
                                ui.separator();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for path in &self.assets_cache.clone() {
                                        ui.horizontal(|ui| {
                                            if path.ends_with(".json") {
                                                ui.label(format!("📄 {}", path));
                                                if ui.button("Load Scene").clicked() {
                                                    match assets::load_scene_from_file(path) {
                                                        Ok(scene) => {
                                                            assets::apply_scene_to_world(&scene, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.pressed_keys));
                                                            self.selected_entity = None;
                                                            self.logs.push(format!("Loaded scene from '{}'", path));
                                                        }
                                                        Err(e) => {
                                                            self.logs.push(format!("Failed to load scene '{}': {}", path, e));
                                                        }
                                                    }
                                                }
                                            } else if path.ends_with(".lua") {
                                                ui.label(format!("📜 {}", path));
                                                if ui.button("Open in Editor").clicked() {
                                                    script_to_open = Some(path.clone());
                                                }
                                                if let Some(entity) = self.selected_entity {
                                                    if ui.button("Assign to Selected").clicked() {
                                                        let mut comp = ScriptComponent::new(path.clone());
                                                        let _ = comp.init_or_reload(self.pressed_keys.clone());
                                                        let _ = self.world.insert_one(entity, comp);
                                                        self.logs.push(format!("Assigned script '{}' to selected entity", path));
                                                    }
                                                }
                                            } else if path.ends_with(".glb") || path.ends_with(".gltf") {
                                                ui.label(format!("🧊 {}", path));
                                                if let Some(entity) = self.selected_entity {
                                                    if ui.button("Assign Mesh to Selected").clicked() {
                                                        if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(entity) {
                                                            mesh.mesh_type = path.clone();
                                                            self.logs.push(format!("Assigned mesh '{}' to selected entity", path));
                                                        }
                                                    }
                                                }
                                            } else {
                                                ui.label(format!("📎 {}", path));
                                            }
                                        });
                                    }
                                });
                            }
                            BottomTab::ScriptEditor => {
                                ui.horizontal(|ui| {
                                    if let Some(path) = &self.script_editor_path {
                                        ui.label(egui::RichText::new(format!("Editing: {}", path)).strong());
                                        if ui.button("💾 Save & Hot-Reload").clicked() {
                                            trigger_save_script = true;
                                        }
                                    } else {
                                        ui.label("(No script loaded. Open a .lua script from the Assets Browser or Inspector)");
                                    }
                                });
                                ui.separator();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.script_editor_content)
                                            .font(egui::TextStyle::Monospace)
                                            .code_editor()
                                            .desired_rows(12)
                                            .desired_width(f32::INFINITY),
                                    );
                                });
                            }
                        }
                    });

                // Central Panel for remaining screen space
                CentralPanel::default()
                    .frame(Frame::new().fill(egui::Color32::TRANSPARENT))
                    .show(ui, |_ui| {
                        // Viewport area - 3D content rendered underneath
                    });
            });

        // Process triggered actions from UI
        if trigger_refresh_assets {
            self.last_assets_scan = Instant::now() - std::time::Duration::from_secs(10);
            self.scan_assets();
        }

        if trigger_new_scene {
            self.push_history();
            self.world.clear();
            self.physics.clear();
            self.selected_entity = None;
            self.logs.push("Created new empty scene".to_string());
        }

        if trigger_save_scene {
            self.save_scene_to_file("assets/scene.json");
        }

        if trigger_reload_scene {
            self.reload_scene();
        }

        if let Some((mesh_type, color)) = trigger_spawn_mesh {
            self.spawn_primitive(mesh_type, color);
        }

        if trigger_duplicate {
            self.duplicate_selected();
        }

        if trigger_delete {
            self.delete_selected();
        }

        if let Some(path) = script_to_open {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.script_editor_path = Some(path);
                self.script_editor_content = content;
                self.bottom_tab = BottomTab::ScriptEditor;
            } else {
                self.logs.push(format!("Could not read script file '{}'", path));
            }
        }

        if trigger_save_script {
            if let Some(path) = &self.script_editor_path {
                match std::fs::write(path, &self.script_editor_content) {
                    Ok(()) => {
                        let msg = format!("Script '{}' saved and hot-reloaded", path);
                        self.logs.push(msg.clone());
                        self.hot_reload.last_reload_status = Some(msg);
                        // Trigger reload on all entities using this script
                        for (meta, script) in self.world.query_mut::<(&EntityMeta, &mut ScriptComponent)>() {
                            if script.path == *path {
                                let _ = script.init_or_reload(self.pressed_keys.clone());
                                self.logs.push(format!("Reloaded script for entity '{}'", meta.id));
                            }
                        }
                    }
                    Err(e) => {
                        self.logs.push(format!("Error saving script '{}': {}", path, e));
                    }
                }
            }
        }

        self.gpu
            .egui_winit_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let pixels_per_point = self.window.scale_factor() as f32;
        let clipped_primitives = self
            .gpu
            .egui_winit_state
            .egui_ctx()
            .tessellate(full_output.shapes, pixels_per_point);

        let mut textures_delta = full_output.textures_delta;
        for (id, deltas) in &textures_delta.set {
            for delta in deltas {
                self.gpu
                    .egui_renderer
                    .update_texture(&self.gpu.device, &self.gpu.queue, *id, delta);
            }
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.gpu.config.width, self.gpu.config.height],
            pixels_per_point,
        };

        self.gpu.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        // 1. 3D Scene Render Pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3D Scene Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.09,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.gpu.render_pipeline);
            render_pass.set_bind_group(0, &self.gpu.camera_bind_group, &[]);

            for (mesh_id, instances) in &instances_by_mesh {
                if instances.is_empty() {
                    continue;
                }
                if let Some(mesh_gpu) = self.gpu.meshes.get(mesh_id) {
                    let instance_buffer = create_instance_buffer(&self.gpu.device, instances);
                    render_pass.set_vertex_buffer(0, mesh_gpu.vertex_buffer.slice(..));
                    render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                    render_pass.set_index_buffer(
                        mesh_gpu.index_buffer.slice(..),
                        mesh_gpu.index_format,
                    );
                    render_pass.draw_indexed(
                        0..mesh_gpu.num_indices,
                        0,
                        0..instances.len() as u32,
                    );
                }
            }
        }

        // 2. Egui Overlay Render Pass
        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            let mut static_rpass = rpass.forget_lifetime();
            self.gpu.egui_renderer.render(
                &mut static_rpass,
                &clipped_primitives,
                &screen_descriptor,
            );
        }

        for id in &textures_delta.free {
            self.gpu.egui_renderer.free_texture(id);
        }
        textures_delta.clear();

        self.gpu.queue.submit(std::iter::once(encoder.finish()));

        // 3. Handle pending viewport capture if requested by MCP
        if let Some(respond_to) = self.pending_viewport_capture.take() {
            let capture_result = renderer::capture_frame(
                &self.gpu.device,
                &self.gpu.queue,
                &output.texture,
                self.gpu.config.format,
                self.gpu.config.width,
                self.gpu.config.height,
            );
            let _ = respond_to.send(capture_result);
        }

        self.gpu.queue.present(output);
    }
}

pub struct App {
    state: Option<EngineState>,
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if self.state.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Motor-dif | AI-Native Lightweight 2D/3D Engine (Editor)")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
                .with_visible(true);
            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create winit window"),
            );
            window.set_visible(true);
            window.focus_window();
            let state = pollster::block_on(EngineState::new(window.clone()));
            window.request_redraw();
            self.state = Some(state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(state) = &mut self.state {
            if state.window.id() == window_id {
                state.handle_event(event_loop, &event);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Some(state) = &mut self.state {
            state.window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Error running event loop");
}
