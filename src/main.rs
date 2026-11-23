pub mod animation;
pub mod assets;
pub mod audio;
pub mod camera;
pub mod ecs;
pub mod exporter;
pub mod game_ui;
pub mod glb_exporter;
pub mod hot_reload;
pub mod input_map;
pub mod mcp_server;
pub mod modeler;
pub mod music_forge;
pub mod nav;
pub mod particles;
pub mod physics;
pub mod renderer;
pub mod scripting;
pub mod sdf;
pub mod templates;
pub mod texture_forge;
pub mod tweens;
pub mod ui_theme;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use egui::{CentralPanel, Frame, Panel};
use assets::SceneDef;
use camera::{Camera, CameraMode};
use ecs::{EntityMeta, MeshRender, Transform};
use glam::{EulerRot, Quat, Vec3};
use hot_reload::{HotReloadEvent, HotReloadManager};
use mcp_server::{EngineLogsResponse, McpCommand, WorldInspection};
use wgpu::util::DeviceExt;
use physics::{PhysicsEngine, RigidBodyComponent};
use renderer::{create_instance_buffer, GpuContext, InstanceRaw, LightsUniform, PointLightRaw};
use scripting::ScriptComponent;
use tweens::TweenManager;
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
pub enum GizmoAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Console,
    Assets,
    ScriptEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleFilter {
    All,
    Info,
    Error,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorWorkspace {
    Scene,
    Modeler,
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
    pub _audio_stream: Option<rodio::OutputStream>,
    pub audio: Arc<std::sync::Mutex<audio::AudioManager>>,
    pub lua_ctx: scripting::LuaEngineContext,
    pub last_lua_error: Option<String>,
    pub particles: particles::ParticleSystem,
    pub tweens: TweenManager,
    pub is_standalone_play: bool,
    pub post_process_settings: assets::PostProcessDef,
    pub environment_settings: assets::EnvironmentDef,
    pub total_time: f32,

    // Editor State
    pub workspace: EditorWorkspace,
    pub forge_recipe: crate::modeler::ModelRecipe,
    pub forge_selected_part: Option<usize>,
    pub play_mode: PlayMode,
    pub play_snapshot: Option<SceneDef>,
    pub gizmo_mode: GizmoMode,
    pub snap_enabled: bool,
    pub debug_colliders: bool,
    pub selected_entity: Option<hecs::Entity>,
    pub hierarchy_search: String,
    pub bottom_tab: BottomTab,
    pub console_filter: ConsoleFilter,
    pub script_editor_path: Option<String>,
    pub script_editor_content: String,
    pub script_editor_original_content: String,
    pub history: HistoryState,
    pub entity_counter: u32,
    pub assets_cache: Vec<String>,
    pub last_assets_scan: Instant,
    pub toasts: Vec<(String, Instant, egui::Color32)>,
    pub command_palette_open: bool,
    pub command_palette_query: String,
    pub show_export_modal: bool,
    pub export_game_name: String,
    pub export_entry_scene: String,
    pub show_profiler: bool,
    pub frametime_history: std::collections::VecDeque<f32>,
    pub view_mode: renderer::ViewMode,
    pub camera_bookmarks: [Option<(Vec3, Vec3)>; 4],
    pub last_draw_calls: usize,

    // Mouse & Gizmo Interaction
    pub mouse_pos: [f32; 2],
    pub is_right_mouse_down: bool,
    pub is_middle_mouse_down: bool,
    pub is_left_mouse_down: bool,
    pub just_pressed_mouse: [bool; 3],
    pub viewport_rect: egui::Rect,
    pub active_gizmo_axis: Option<GizmoAxis>,
    pub hovered_gizmo_axis: Option<GizmoAxis>,
    pub gizmo_drag_start_pos: Option<Vec3>,
    pub gizmo_drag_start_rot: Option<Vec3>,
    pub gizmo_drag_start_scale: Option<Vec3>,
    pub gizmo_drag_mouse_origin: Option<[f32; 2]>,
}

pub fn dist_to_segment_2d(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-4 {
        let px = p[0] - a[0];
        let py = p[1] - a[1];
        return (px * px + py * py).sqrt();
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    let proj_x = a[0] + t * dx;
    let proj_y = a[1] + t * dy;
    let rx = p[0] - proj_x;
    let ry = p[1] - proj_y;
    (rx * rx + ry * ry).sqrt()
}

pub fn compute_world_transforms(
    world: &hecs::World,
) -> HashMap<hecs::Entity, (glam::Mat4, Vec3)> {
    let mut id_to_entity = HashMap::new();
    let mut parent_map = HashMap::new();
    let mut local_mats = HashMap::new();

    for entity_ref in world.iter() {
        let entity = entity_ref.entity();
        if let (Some(meta), Some(transform)) = (entity_ref.get::<&EntityMeta>(), entity_ref.get::<&Transform>()) {
            id_to_entity.insert(meta.id.clone(), entity);
            local_mats.insert(meta.id.clone(), transform.matrix());
            if let Some(pid) = &meta.parent {
                let clean = pid.trim();
                if !clean.is_empty() {
                    parent_map.insert(meta.id.clone(), clean.to_string());
                }
            }
        }
    }

    let mut resolved_matrices = HashMap::new();
    let mut visiting = HashSet::new();

    for id in local_mats.keys() {
        resolve_world_matrix(id, &parent_map, &local_mats, &mut resolved_matrices, &mut visiting);
    }

    let mut result = HashMap::new();
    for (id, world_mat) in resolved_matrices {
        if let Some(&entity) = id_to_entity.get(&id) {
            let world_pos = world_mat.transform_point3(Vec3::ZERO);
            result.insert(entity, (world_mat, world_pos));
        }
    }
    result
}

fn resolve_world_matrix(
    id: &str,
    parent_map: &HashMap<String, String>,
    local_mats: &HashMap<String, glam::Mat4>,
    resolved: &mut HashMap<String, glam::Mat4>,
    visiting: &mut HashSet<String>,
) -> glam::Mat4 {
    if let Some(&mat) = resolved.get(id) {
        return mat;
    }
    if visiting.contains(id) {
        // Cycle detected! Fallback to local transform to break cycle safely
        return local_mats.get(id).copied().unwrap_or(glam::Mat4::IDENTITY);
    }
    visiting.insert(id.to_string());

    let local = local_mats.get(id).copied().unwrap_or(glam::Mat4::IDENTITY);
    let world = if let Some(parent_id) = parent_map.get(id) {
        if parent_id != id && local_mats.contains_key(parent_id) {
            let parent_world = resolve_world_matrix(parent_id, parent_map, local_mats, resolved, visiting);
            parent_world * local
        } else {
            local
        }
    } else {
        local
    };

    visiting.remove(id);
    resolved.insert(id.to_string(), world);
    world
}

impl EngineState {
    pub async fn new(window: Arc<Window>, is_standalone_play: bool, initial_scene_path: String) -> Self {
        let size = window.inner_size();
        let aspect = size.width as f32 / size.height.max(1) as f32;
        let mut camera = Camera::new_3d(Vec3::new(0.0, 3.5, 9.0), Vec3::ZERO, aspect);

        let mut gpu = GpuContext::new(window.clone()).await;
        gpu.update_camera(&camera);
        ui_theme::setup_custom_fonts(gpu.egui_winit_state.egui_ctx());
        ui_theme::apply_theme(gpu.egui_winit_state.egui_ctx());

        if !Path::new("assets/scripts").exists() {
            let _ = std::fs::create_dir_all("assets/scripts");
        }
        if !Path::new("assets/models").exists() {
            let _ = std::fs::create_dir_all("assets/models");
        }

        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));
        let (audio_stream, audio_inner) = audio::AudioManager::init();
        let audio_mgr = Arc::new(std::sync::Mutex::new(audio_inner));
        let lua_ctx = scripting::LuaEngineContext::new(pressed_keys.clone(), audio_mgr.clone());

        // Generate procedural bounce.wav if not present
        let bounce_path = Path::new("assets/sfx/bounce.wav");
        if !bounce_path.exists() {
            let _ = audio::generate_bounce_wav(bounce_path);
        }

        // Generate procedural textures if not present
        let checker_path = Path::new("assets/textures/checker.png");
        if !checker_path.exists() {
            let _ = renderer::generate_checker_texture(checker_path);
        }
        let orb_path = Path::new("assets/textures/sprite_orb.png");
        if !orb_path.exists() {
            let _ = renderer::generate_sprite_orb_texture(orb_path);
        }

        let mut world = hecs::World::new();
        let mut physics = PhysicsEngine::new();
        let mut logs = Vec::new();
        logs.push("Motor-dif engine initialized (Professional Editor Mode).".to_string());
        if audio_mgr.lock().unwrap().available {
            logs.push("Audio: Rodio initialized (WAV/OGG).".to_string());
        } else {
            logs.push("Audio: No audio device detected (No-op fallback).".to_string());
        }

        let scene_path = Path::new(&initial_scene_path);
        if !scene_path.exists() && initial_scene_path == "assets/scene.json" {
            let default_scene = SceneDef::default();
            if let Ok(json) = serde_json::to_string_pretty(&default_scene) {
                let _ = std::fs::write(scene_path, json);
                logs.push("Created default assets/scene.json".to_string());
            }
        }

        let mut loaded_pp = assets::PostProcessDef::default();
        let mut loaded_env = assets::EnvironmentDef::default();
        match assets::load_scene_from_file(scene_path) {
            Ok(scene) => {
                if let Some(pp) = &scene.post_process {
                    loaded_pp = pp.clone();
                }
                if let Some(env) = &scene.environment {
                    loaded_env = env.clone();
                }
                assets::apply_scene_to_world(&scene, &mut world, &mut camera, &mut physics, Some(&lua_ctx));
                logs.push(format!("Scene loaded from '{}' ({} entities)", initial_scene_path, world.len()));
            }
            Err(e) => {
                logs.push(format!("Warning loading scene '{}': {}", initial_scene_path, e));
            }
        }
        if let Ok(mut lock) = lua_ctx.post_process_settings.write() {
            *lock = loaded_pp.clone();
        }
        if let Ok(mut lock) = lua_ctx.environment_settings.write() {
            *lock = loaded_env.clone();
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

        let mut initial_selected = None;
        for er in world.iter() {
            if let Some(m) = er.get::<&EntityMeta>() {
                if m.id == "pedestal" {
                    initial_selected = Some(er.entity());
                    break;
                } else if initial_selected.is_none() {
                    initial_selected = Some(er.entity());
                }
            }
        }

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
            _audio_stream: audio_stream,
            audio: audio_mgr,
            lua_ctx,
            last_lua_error: None,
            particles: particles::ParticleSystem::new(),
            tweens: TweenManager::new(),
            is_standalone_play,
            post_process_settings: loaded_pp,
            environment_settings: loaded_env,
            total_time: 0.0,

            workspace: EditorWorkspace::Scene,
            forge_recipe: crate::modeler::preset_spaceship(),
            forge_selected_part: None,

            play_mode: if is_standalone_play { PlayMode::Play } else { PlayMode::Edit },
            play_snapshot: None,
            gizmo_mode: GizmoMode::Translate,
            snap_enabled: false,
            debug_colliders: true,
            selected_entity: initial_selected,
            hierarchy_search: String::new(),
            bottom_tab: BottomTab::Console,
            console_filter: ConsoleFilter::All,
            script_editor_path: initial_script_path,
            script_editor_content: initial_script_content.clone(),
            script_editor_original_content: initial_script_content,
            history: HistoryState::new(),
            entity_counter: 10,
            assets_cache: Vec::new(),
            last_assets_scan: Instant::now() - std::time::Duration::from_secs(10),
            toasts: Vec::new(),
            command_palette_open: false,
            command_palette_query: String::new(),
            show_export_modal: false,
            export_game_name: "MyGame".to_string(),
            export_entry_scene: initial_scene_path.clone(),
            show_profiler: false,
            frametime_history: std::collections::VecDeque::with_capacity(100),
            view_mode: renderer::ViewMode::Lit,
            camera_bookmarks: [None, None, None, None],
            last_draw_calls: 0,

            mouse_pos: [0.0, 0.0],
            is_right_mouse_down: false,
            is_middle_mouse_down: false,
            is_left_mouse_down: false,
            just_pressed_mouse: [false; 3],
            viewport_rect: egui::Rect::NOTHING,
            active_gizmo_axis: None,
            hovered_gizmo_axis: None,
            gizmo_drag_start_pos: None,
            gizmo_drag_start_rot: None,
            gizmo_drag_start_scale: None,
            gizmo_drag_mouse_origin: None,
        }
    }

    pub fn push_history(&mut self) {
        let snapshot = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        self.history.push(snapshot);
    }

    pub fn add_toast(&mut self, text: impl Into<String>, color: egui::Color32) {
        self.toasts.push((text.into(), Instant::now(), color));
    }

    pub fn sync_entity_to_physics(&mut self, entity: hecs::Entity) {
        let tr_opt = self.world.get::<&Transform>(entity).map(|t| (t.position, t.rotation)).ok();
        let rb_opt = self.world.get::<&RigidBodyComponent>(entity).map(|r| r.handle).ok();
        if let (Some((pos, rot)), Some(handle)) = (tr_opt, rb_opt) {
            if let Some(body) = self.physics.rigid_body_set.get_mut(handle) {
                body.set_translation(pos, true);
                let q = Quat::from_euler(
                    EulerRot::XYZ,
                    rot.x,
                    rot.y,
                    rot.z,
                );
                body.set_rotation(q, true);
                body.set_linvel(Vec3::ZERO, true);
                body.set_angvel(Vec3::ZERO, true);
            }
        }
    }

    pub fn undo(&mut self) {
        let current = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        if let Some(prev) = self.history.undo(current) {
            assets::apply_scene_to_world(&prev, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
            self.selected_entity = None;
            self.logs.push("Undo performed".to_string());
        }
    }

    pub fn redo(&mut self) {
        let current = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        if let Some(next) = self.history.redo(current) {
            assets::apply_scene_to_world(&next, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
            self.selected_entity = None;
            self.logs.push("Redo performed".to_string());
        }
    }

    pub fn save_scene_to_file(&mut self, path: &str) {
        let mut scene = assets::extract_scene_from_world(&self.world, &self.camera, &self.physics);
        scene.post_process = Some(self.post_process_settings.clone());
        scene.environment = Some(self.environment_settings.clone());
        self.hot_reload.suppress_scene_events(std::time::Duration::from_millis(600));
        match assets::save_scene_to_file(path, &scene) {
            Ok(()) => {
                let msg = format!("Scene successfully saved to '{}'", path);
                self.logs.push(msg.clone());
                self.hot_reload.last_reload_status = Some(msg);
                self.add_toast("Scene saved to assets/scene.json", egui::Color32::from_rgb(56, 189, 248));
            }
            Err(e) => {
                self.logs.push(format!("Error saving scene: {}", e));
                self.add_toast(format!("Error saving scene: {}", e), egui::Color32::from_rgb(239, 68, 68));
            }
        }
    }

    pub fn reload_scene(&mut self) {
        match assets::load_scene_from_file("assets/scene.json") {
            Ok(scene) => {
                assets::apply_scene_to_world(&scene, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
                if let Some(pp) = &scene.post_process {
                    self.post_process_settings = pp.clone();
                    if let Ok(mut lock) = self.lua_ctx.post_process_settings.write() {
                        *lock = pp.clone();
                    }
                }
                if let Some(env) = &scene.environment {
                    self.environment_settings = env.clone();
                    if let Ok(mut lock) = self.lua_ctx.environment_settings.write() {
                        *lock = env.clone();
                    }
                }
                self.selected_entity = None;
                self.hot_reload.last_reload_error = None;
                let msg = format!("Hot-Reload: Scene successfully reloaded ({} entities)", self.world.len());
                self.logs.push(msg.clone());
                self.hot_reload.last_reload_status = Some(msg);
                self.add_toast("Scene reloaded", egui::Color32::from_rgb(56, 189, 248));
            }
            Err(err) => {
                self.hot_reload.last_reload_error = Some(err.clone());
                self.logs.push(format!("Hot-Reload ERROR: {}", err));
                self.add_toast(format!("Hot-reload error: {}", err), egui::Color32::from_rgb(239, 68, 68));
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
            parent: None,
        };
        let transform = Transform::from_xyz(0.0, 1.0, 0.0);
        let mesh = MeshRender {
            mesh_type: mesh_type.to_string(),
            color,
            texture: None,
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
        };

        let entity = self.world.spawn((meta, transform, mesh));
        self.selected_entity = Some(entity);
        self.logs.push(format!("Spawned primitive entity '{}'", id));
        self.add_toast(format!("Spawned '{}'", id), egui::Color32::from_rgb(56, 189, 248));
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
                parent: meta.parent.clone(),
            };
            let mut new_trans = transform;
            new_trans.position.x += 1.0;
            let new_e = self.world.spawn((new_meta, new_trans, mesh));

            if let Some(script_path) = script_val {
                let mut s_comp = ScriptComponent::new(script_path);
                let _ = s_comp.init_or_reload(self.lua_ctx.clone());
                let _ = self.world.insert_one(new_e, s_comp);
            }

            if let Some((is_dyn, mass)) = phys_val {
                let phys_def = assets::PhysicsDef {
                    body: if is_dyn { "dynamic".to_string() } else { "fixed".to_string() },
                    collider: "cuboid".to_string(),
                    mass,
                    restitution: 0.5,
                    is_sensor: None,
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
        if response.repaint {
            self.window.request_redraw();
        }
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
                    let mut jp_opt = self.lua_ctx.just_pressed_keys.write().ok();
                    for k in keys_to_update {
                        if is_pressed {
                            if !set.contains(&k) {
                                if let Some(jp) = &mut jp_opt {
                                    jp.insert(k.clone());
                                }
                            }
                            set.insert(k);
                        } else {
                            set.remove(&k);
                        }
                    }
                }

                if is_pressed {
                    let do_f3 = match logical_key {
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::F3) => true,
                        _ => false,
                    };
                    if do_f3 {
                        self.show_profiler = !self.show_profiler;
                    }

                    if self.is_standalone_play {
                        let is_escape = match logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => true,
                            _ => false,
                        };
                        if is_escape {
                            event_loop.exit();
                            return;
                        }
                    }
                }

                // Keyboard Shortcuts when not focused on egui text edit (Editor mode only)
                if !self.is_standalone_play && is_pressed && !egui_wants_kbd {
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

                    let (ctrl_down, alt_down, key_1, key_2, key_3, key_4) = {
                        let set = self.pressed_keys.read().unwrap();
                        let ctrl = set.contains("CONTROL") || set.contains("CONTROLRIGHT") || set.contains("CONTROLLEFT");
                        let alt = set.contains("ALT") || set.contains("ALTRIGHT") || set.contains("ALTLEFT");
                        let k1 = set.contains("1") || set.contains("DIGIT1") || set.contains("NUMPAD1");
                        let k2 = set.contains("2") || set.contains("DIGIT2") || set.contains("NUMPAD2");
                        let k3 = set.contains("3") || set.contains("DIGIT3") || set.contains("NUMPAD3");
                        let k4 = set.contains("4") || set.contains("DIGIT4") || set.contains("NUMPAD4");
                        (ctrl, alt, k1, k2, k3, k4)
                    };

                    let bookmark_slot = if key_1 { Some(0) } else if key_2 { Some(1) } else if key_3 { Some(2) } else if key_4 { Some(3) } else { None };

                    if let Some(slot) = bookmark_slot {
                        if ctrl_down {
                            self.camera_bookmarks[slot] = Some((self.camera.eye, self.camera.target));
                            self.toasts.push((
                                format!("Camera Bookmark {} saved", slot + 1),
                                Instant::now(),
                                egui::Color32::from_rgb(56, 189, 248),
                            ));
                        } else if alt_down {
                            if let Some((eye, target)) = self.camera_bookmarks[slot] {
                                self.camera.eye = eye;
                                self.camera.target = target;
                                self.toasts.push((
                                    format!("Camera Bookmark {} restored", slot + 1),
                                    Instant::now(),
                                    egui::Color32::from_rgb(56, 189, 248),
                                ));
                            } else {
                                self.toasts.push((
                                    format!("Camera Bookmark {} is empty", slot + 1),
                                    Instant::now(),
                                    egui::Color32::from_rgb(250, 204, 21),
                                ));
                            }
                        }
                    }

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
            WindowEvent::CursorMoved { position, .. } => {
                let new_x = position.x as f32;
                let new_y = position.y as f32;
                let dx = new_x - self.mouse_pos[0];
                let dy = new_y - self.mouse_pos[1];
                self.mouse_pos = [new_x, new_y];
                if let Ok(mut minfo) = self.lua_ctx.mouse_info.write() {
                    minfo.screen_pos = [new_x, new_y];
                    minfo.delta[0] += dx;
                    minfo.delta[1] += dy;
                }
                self.window.request_redraw();
            }
            WindowEvent::MouseWheel { .. } => {
                self.window.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let is_pressed = *state == winit::event::ElementState::Pressed;
                match button {
                    winit::event::MouseButton::Left => {
                        if is_pressed && !self.is_left_mouse_down {
                            self.just_pressed_mouse[0] = true;
                        }
                        self.is_left_mouse_down = is_pressed;
                    }
                    winit::event::MouseButton::Right => {
                        if is_pressed && !self.is_right_mouse_down {
                            self.just_pressed_mouse[1] = true;
                        }
                        self.is_right_mouse_down = is_pressed;
                    }
                    winit::event::MouseButton::Middle => {
                        if is_pressed && !self.is_middle_mouse_down {
                            self.just_pressed_mouse[2] = true;
                        }
                        self.is_middle_mouse_down = is_pressed;
                    }
                    _ => {}
                }
                self.window.request_redraw();
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
                        if let Ok(mut meta) = self.world.get::<&mut EntityMeta>(e) {
                            meta.parent = entity.parent.clone();
                        }
                        if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(e) {
                            mesh.mesh_type = entity.mesh.clone();
                            mesh.color = entity.color;
                            mesh.texture = entity.texture.clone();
                            mesh.emissive = entity.emissive;
                            mesh.fresnel_color = entity.fresnel_color;
                            mesh.fresnel_power = entity.fresnel_power;
                            mesh.blend_mode = entity.blend_mode.clone();
                            mesh.wireframe = entity.wireframe;
                            mesh.uv_scroll = entity.uv_scroll;
                        }
                        if let Some(script_path) = &entity.script {
                            let mut script_comp = ScriptComponent::new(script_path.clone());
                            let _ = script_comp.init_or_reload(self.lua_ctx.clone());
                            let _ = self.world.insert_one(e, script_comp);
                        }
                        if let Some(phys) = &entity.physics {
                            let transform_opt = self.world.get::<&Transform>(e).map(|t| *t);
                            if let Ok(transform) = transform_opt {
                                let handle = self.physics.add_body(&transform, phys);
                                let _ = self.world.insert_one(e, RigidBodyComponent { handle });
                            }
                        }
                        if let Some(pl) = &entity.point_light {
                            let _ = self.world.insert_one(e, ecs::PointLight {
                                color: pl.color,
                                intensity: pl.intensity,
                                radius: pl.radius,
                            });
                        }
                    } else {
                        let meta = EntityMeta {
                            id: entity.id.clone(),
                            name: entity.id.clone(),
                            parent: entity.parent.clone(),
                        };
                        let transform = Transform::new(
                            Vec3::from(entity.position),
                            Vec3::from(entity.rotation),
                            Vec3::from(entity.scale),
                        );
                        let mesh = MeshRender {
                            mesh_type: entity.mesh.clone(),
                            color: entity.color,
                            texture: entity.texture.clone(),
                            emissive: entity.emissive,
                            fresnel_color: entity.fresnel_color,
                            fresnel_power: entity.fresnel_power,
                            blend_mode: entity.blend_mode.clone(),
                            wireframe: entity.wireframe,
                            uv_scroll: entity.uv_scroll,
                        };

                        let e = self.world.spawn((meta, transform, mesh));

                        if let Some(phys) = &entity.physics {
                            let handle = self.physics.add_body(&transform, phys);
                            let _ = self.world.insert_one(e, RigidBodyComponent { handle });
                        }

                        if let Some(script_path) = &entity.script {
                            let mut script_comp = ScriptComponent::new(script_path.clone());
                            let _ = script_comp.init_or_reload(self.lua_ctx.clone());
                            let _ = self.world.insert_one(e, script_comp);
                        }

                        if let Some(pl) = &entity.point_light {
                            let _ = self.world.insert_one(e, ecs::PointLight {
                                color: pl.color,
                                intensity: pl.intensity,
                                radius: pl.radius,
                            });
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
                McpCommand::GetConsoleLogs { filter, limit, respond_to } => {
                    let f = filter.as_deref().unwrap_or("all").to_lowercase();
                    let lim = limit.unwrap_or(50);
                    let filtered: Vec<String> = self.logs.iter().filter(|l| {
                        match f.as_str() {
                            "error" => l.to_lowercase().contains("error") || l.to_lowercase().contains("fail"),
                            "lua" => l.to_lowercase().contains("lua") || l.to_lowercase().contains("script"),
                            "mcp" => l.to_lowercase().contains("mcp"),
                            _ => true,
                        }
                    }).cloned().collect();
                    let len = filtered.len();
                    let start = len.saturating_sub(lim);
                    let result = filtered[start..].to_vec();
                    let _ = respond_to.send(result);
                }
                McpCommand::GetSceneTree { respond_to } => {
                    let selected_id = self.selected_entity.and_then(|e| {
                        self.world.get::<&EntityMeta>(e).ok().map(|m| m.id.clone())
                    });

                    let mut nodes: HashMap<String, mcp_server::SceneTreeNode> = HashMap::new();
                    for entity_ref in self.world.iter() {
                        if let (Some(meta), Some(mesh)) = (entity_ref.get::<&EntityMeta>(), entity_ref.get::<&MeshRender>()) {
                            let mut comps = vec!["EntityMeta".to_string(), "Transform".to_string(), "MeshRender".to_string()];
                            if entity_ref.get::<&ScriptComponent>().is_some() { comps.push("ScriptComponent".to_string()); }
                            if entity_ref.get::<&RigidBodyComponent>().is_some() { comps.push("RigidBody".to_string()); }
                            if entity_ref.get::<&crate::ecs::PointLight>().is_some() { comps.push("PointLight".to_string()); }
                            if entity_ref.get::<&crate::animation::AnimationPlayer>().is_some() { comps.push("AnimationPlayer".to_string()); }
                            if entity_ref.get::<&crate::ecs::TrailComponent>().is_some() { comps.push("Trail".to_string()); }

                            nodes.insert(meta.id.clone(), mcp_server::SceneTreeNode {
                                id: meta.id.clone(),
                                mesh: mesh.mesh_type.clone(),
                                parent: meta.parent.clone(),
                                components: comps,
                                children: Vec::new(),
                            });
                        }
                    }

                    let total_entities = nodes.len();
                    let mut parent_to_children: HashMap<String, Vec<String>> = HashMap::new();
                    for node in nodes.values() {
                        if let Some(p) = &node.parent {
                            parent_to_children.entry(p.clone()).or_default().push(node.id.clone());
                        }
                    }

                    fn build_node(id: &str, nodes: &HashMap<String, mcp_server::SceneTreeNode>, parent_to_children: &HashMap<String, Vec<String>>) -> Option<mcp_server::SceneTreeNode> {
                        let mut node = nodes.get(id)?.clone();
                        if let Some(child_ids) = parent_to_children.get(id) {
                            for cid in child_ids {
                                if let Some(child_node) = build_node(cid, nodes, parent_to_children) {
                                    node.children.push(child_node);
                                }
                            }
                        }
                        Some(node)
                    }

                    let mut roots = Vec::new();
                    for node in nodes.values() {
                        if node.parent.is_none() || !nodes.contains_key(node.parent.as_ref().unwrap()) {
                            if let Some(root_tree) = build_node(&node.id, &nodes, &parent_to_children) {
                                roots.push(root_tree);
                            }
                        }
                    }

                    let _ = respond_to.send(mcp_server::SceneTreeResponse {
                        selected_entity: selected_id,
                        root_nodes: roots,
                        total_entities,
                    });
                }
                McpCommand::GetPerformanceStats { respond_to } => {
                    let bodies_count = self.physics.dynamic_bodies_count() + self.physics.fixed_bodies_count();
                    let particles_count = self.particles.active_count();
                    let frame_time_ms = if self.fps > 0.0 { 1000.0 / self.fps } else { 0.0 };

                    let _ = respond_to.send(mcp_server::PerformanceStatsResponse {
                        fps: self.fps,
                        frame_time_ms,
                        entities: self.world.len() as usize,
                        draw_calls: self.last_draw_calls,
                        active_particles: particles_count,
                        physics_bodies: bodies_count,
                    });
                }
                McpCommand::CaptureViewport { respond_to } => {
                    self.pending_viewport_capture = Some(respond_to);
                }
                McpCommand::DeleteEntity { id, respond_to } => {
                    self.push_history();
                    let to_remove: Option<(hecs::Entity, Option<rapier3d::prelude::RigidBodyHandle>)> = {
                        let mut found = None;
                        for entity_ref in self.world.iter() {
                            let entity = entity_ref.entity();
                            if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                                if meta.id == id {
                                    let rb_handle = entity_ref.get::<&RigidBodyComponent>().map(|r| r.handle);
                                    found = Some((entity, rb_handle));
                                    break;
                                }
                            }
                        }
                        found
                    };
                    if let Some((e, rb_opt)) = to_remove {
                        if let Some(handle) = rb_opt {
                            self.physics.remove_body(handle);
                        }
                        let _ = self.world.despawn(e);
                        if self.selected_entity == Some(e) {
                            self.selected_entity = None;
                        }
                        let msg = format!("Entity '{}' successfully deleted", id);
                        self.logs.push(msg.clone());
                        let _ = respond_to.send(Ok(msg));
                    } else {
                        let err = format!("Entity '{}' not found", id);
                        let _ = respond_to.send(Err(err));
                    }
                }
                McpCommand::SetCamera { mode, position, target, ortho_size, respond_to } => {
                    if let Some(m) = mode {
                        match m.to_lowercase().as_str() {
                            "2d" | "orthographic" => self.camera.mode = CameraMode::Orthographic,
                            "3d" | "perspective" => self.camera.mode = CameraMode::Perspective,
                            _ => {}
                        }
                    }
                    if let Some(pos) = position {
                        self.camera.eye = Vec3::from(pos);
                    }
                    if let Some(tgt) = target {
                        self.camera.target = Vec3::from(tgt);
                    }
                    if let Some(size) = ortho_size {
                        self.camera.ortho_size = size;
                    }
                    self.gpu.update_camera(&self.camera);
                    let msg = format!("Camera updated: mode={:?}, eye={}, target={}", self.camera.mode, self.camera.eye, self.camera.target);
                    self.logs.push(msg.clone());
                    let _ = respond_to.send(Ok(msg));
                }
                McpCommand::SetPlayMode { mode, respond_to } => {
                    match mode.to_lowercase().as_str() {
                        "play" => {
                            if self.play_mode == PlayMode::Edit {
                                self.play_snapshot = Some(assets::extract_scene_from_world(&self.world, &self.camera, &self.physics));
                            }
                            self.play_mode = PlayMode::Play;
                            let msg = "Play mode set to Play".to_string();
                            self.logs.push(msg.clone());
                            let _ = respond_to.send(Ok(msg));
                        }
                        "pause" => {
                            self.play_mode = PlayMode::Pause;
                            let msg = "Play mode set to Pause".to_string();
                            self.logs.push(msg.clone());
                            let _ = respond_to.send(Ok(msg));
                        }
                        "stop" => {
                            if let Some(snapshot) = self.play_snapshot.take() {
                                assets::apply_scene_to_world(&snapshot, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
                            }
                            self.particles.clear();
                            self.lua_ctx.clear_timers_and_events();
                            self.play_mode = PlayMode::Edit;
                            let msg = "Play mode set to Stop (Scene restored)".to_string();
                            self.logs.push(msg.clone());
                            let _ = respond_to.send(Ok(msg));
                        }
                        _ => {
                            let err = format!("Unknown play mode '{}'. Valid modes: 'play', 'pause', 'stop'", mode);
                            let _ = respond_to.send(Err(err));
                        }
                    }
                }
                McpCommand::StepSimulation { dt, respond_to } => {
                    let step_dt = if dt <= 0.0 { 1.0 / 60.0 } else { dt };
                    if let Ok(mut we) = self.lua_ctx.world_entities.write() {
                        we.clear();
                        for (meta, transform) in self.world.query::<(&EntityMeta, &Transform)>().iter() {
                            we.insert(
                                meta.id.clone(),
                                [
                                    transform.position.x, transform.position.y, transform.position.z,
                                    transform.rotation.x, transform.rotation.y, transform.rotation.z,
                                    transform.scale.x, transform.scale.y, transform.scale.z,
                                ],
                            );
                        }
                    }

                    for (_meta, transform, script) in self.world.query_mut::<(&EntityMeta, &mut Transform, &mut ScriptComponent)>() {
                        let _ = script.update(transform, step_dt);
                    }
                    scripting::process_timers_and_events(&mut self.world, &self.lua_ctx, step_dt, &mut self.logs);

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

                    self.physics.step(step_dt);
                    self.physics.sync_to_ecs(&mut self.world);

                    let msg = format!("Simulation stepped by {:.4}s", step_dt);
                    self.logs.push(msg.clone());
                    let _ = respond_to.send(Ok(msg));
                }
                McpCommand::RunLuaSnippet { code, respond_to } => {
                    let lua = mlua::Lua::new();
                    let keys_ref = self.pressed_keys.clone();
                    let is_key_down = lua.create_function(move |_, key: String| {
                        let keys = keys_ref.read().unwrap();
                        let clean = key.trim().to_uppercase();
                        Ok(scripting::check_key_match(&keys, &clean))
                    });
                    if let Ok(f) = is_key_down { let _ = lua.globals().set("is_key_down", f); }

                    let just_pressed_ref = self.lua_ctx.just_pressed_keys.clone();
                    let is_key_just_pressed = lua.create_function(move |_, key: String| {
                        let keys = just_pressed_ref.read().unwrap();
                        let clean = key.trim().to_uppercase();
                        Ok(scripting::check_key_match(&keys, &clean))
                    });
                    if let Ok(f) = is_key_just_pressed { let _ = lua.globals().set("is_key_just_pressed", f); }

                    let events_ref = self.lua_ctx.pending_events.clone();
                    let emit_event = lua.create_function(move |_, (event_name, payload): (String, Option<mlua::Value>)| {
                        let json_val = match payload {
                            Some(v) => scripting::lua_to_json(&v),
                            None => serde_json::Value::Null,
                        };
                        if let Ok(mut list) = events_ref.lock() {
                            list.push((event_name, json_val));
                        }
                        Ok(true)
                    });
                    if let Ok(f) = emit_event { let _ = lua.globals().set("emit_event", f); }

                    let spawns_ref = self.lua_ctx.pending_spawns.clone();
                    let spawn_entity = lua.create_function(move |_, table: mlua::Table| {
                        let id: String = table.get("id").unwrap_or_else(|_| "spawned".to_string());
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
                        let color = [1.0, 1.0, 1.0, 1.0];
                        let texture: Option<String> = table.get("texture").ok();
                        let parent: Option<String> = table.get("parent").ok();
                        let def = assets::EntityDef {
                            id, mesh, position: [x,y,z], rotation: [rx,ry,rz], scale: [sx,sy,sz],
                            color, texture, parent, emissive: 0.0,
                            fresnel_color: None, fresnel_power: None, blend_mode: None, wireframe: None, uv_scroll: None,
                            script: None, physics: None, point_light: None, animation_player: None
                        };
                        if let Ok(mut list) = spawns_ref.lock() { list.push(def); }
                        Ok(())
                    });
                    if let Ok(f) = spawn_entity { let _ = lua.globals().set("spawn_entity", f); }

                    let destroys_ref = self.lua_ctx.pending_destroys.clone();
                    let destroy_entity = lua.create_function(move |_, id: String| {
                        if let Ok(mut list) = destroys_ref.lock() { list.push(id); }
                        Ok(())
                    });
                    if let Ok(f) = destroy_entity { let _ = lua.globals().set("destroy_entity", f); }

                    let entities_ref = self.lua_ctx.world_entities.clone();
                    let get_entity = lua.create_function(move |lua, id: Option<String>| {
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
                    });
                    if let Ok(f) = get_entity { let _ = lua.globals().set("get_entity", f); }

                    let hud_ref = self.lua_ctx.hud_texts.clone();
                    let set_hud_text = lua.create_function(move |_, (k, v): (String, String)| {
                        if let Ok(mut h) = hud_ref.write() { h.insert(k, v); }
                        Ok(())
                    });
                    if let Ok(f) = set_hud_text { let _ = lua.globals().set("set_hud_text", f); }

                    let particles_ref = self.lua_ctx.pending_particles.clone();
                    let spawn_particles = lua.create_function(
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
                            let req = particles::ParticleSpawnRequest {
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
                    );
                    if let Ok(f) = spawn_particles {
                        let _ = lua.globals().set("spawn_particles", f);
                    }

                    let spawns_ref_p = self.lua_ctx.pending_spawns.clone();
                    let spawn_prefab = lua.create_function(
                        move |_, (name, x, y, z): (String, Option<f32>, Option<f32>, Option<f32>)| {
                            let file_path = if name.ends_with(".json") {
                                if name.starts_with("assets/prefabs/") { name.clone() } else { format!("assets/prefabs/{}", name) }
                            } else {
                                format!("assets/prefabs/{}.json", name)
                            };
                            match assets::load_prefab_from_file(&file_path) {
                                Ok(mut def) => {
                                    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1);
                                    def.id = format!("{}_{}", def.id, ts % 10000);
                                    def.position = [x.unwrap_or(0.0), y.unwrap_or(0.0), z.unwrap_or(0.0)];
                                    if let Ok(mut list) = spawns_ref_p.lock() { list.push(def); }
                                    Ok(true)
                                }
                                Err(_) => Ok(false),
                            }
                        },
                    );
                    if let Ok(f) = spawn_prefab { let _ = lua.globals().set("spawn_prefab", f); }

                    let parents_ref_snippet = self.lua_ctx.pending_parents.clone();
                    let set_parent = lua.create_function(
                        move |_, (child_id, parent_id): (String, Option<String>)| {
                            if let Ok(mut list) = parents_ref_snippet.lock() {
                                let clean = parent_id.and_then(|p| {
                                    let t = p.trim().to_string();
                                    if t.is_empty() { None } else { Some(t) }
                                });
                                list.push((child_id, clean));
                            }
                            Ok(true)
                        },
                    );
                    if let Ok(f) = set_parent { let _ = lua.globals().set("set_parent", f); }

                    let lights_ref_snippet = self.lua_ctx.pending_lights.clone();
                    let set_light = lua.create_function(
                        move |_, (id, r, g, b, intensity, radius): (String, f32, f32, f32, f32, f32)| {
                            if let Ok(mut list) = lights_ref_snippet.lock() {
                                list.push((id, [r, g, b], intensity, radius));
                            }
                            Ok(true)
                        },
                    );
                    if let Ok(f) = set_light { let _ = lua.globals().set("set_light", f); }

                    let cam_set_s = self.lua_ctx.pending_camera_sets.clone();
                    let set_camera = lua.create_function(move |_, (x, y, z, tx, ty, tz): (f32, f32, f32, f32, f32, f32)| {
                        if let Ok(mut list) = cam_set_s.lock() { list.push(([x, y, z], [tx, ty, tz])); }
                        Ok(true)
                    });
                    if let Ok(f) = set_camera { let _ = lua.globals().set("set_camera", f); }

                    let cam_follow_s = self.lua_ctx.pending_camera_follows.clone();
                    let follow_entity = lua.create_function(move |_, (entity_id, ox, oy, oz, sm): (Option<String>, Option<f32>, Option<f32>, Option<f32>, Option<f32>)| {
                        if let Ok(mut list) = cam_follow_s.lock() {
                            let offset = [ox.unwrap_or(0.0), oy.unwrap_or(3.0), oz.unwrap_or(8.0)];
                            list.push((entity_id, offset, sm.unwrap_or(0.2)));
                        }
                        Ok(true)
                    });
                    if let Ok(f) = follow_entity { let _ = lua.globals().set("follow_entity", f); }

                    let cam_shake_s = self.lua_ctx.pending_camera_shakes.clone();
                    let camera_shake = lua.create_function(move |_, (intensity, duration): (f32, f32)| {
                        if let Ok(mut list) = cam_shake_s.lock() { list.push((intensity, duration)); }
                        Ok(true)
                    });
                    if let Ok(f) = camera_shake { let _ = lua.globals().set("camera_shake", f); }

                    let tw_pos_s = self.lua_ctx.pending_tweens.clone();
                    let tween_position = lua.create_function(move |_, (id, x, y, z, duration): (String, f32, f32, f32, f32)| {
                        if let Ok(mut list) = tw_pos_s.lock() {
                            list.push(tweens::TweenRequest {
                                entity_id: id,
                                target: glam::Vec3::new(x, y, z),
                                duration,
                                is_scale: false,
                            });
                        }
                        Ok(true)
                    });
                    if let Ok(f) = tween_position { let _ = lua.globals().set("tween_position", f); }

                    let tw_scale_s = self.lua_ctx.pending_tweens.clone();
                    let tween_scale = lua.create_function(move |_, (id, sx, sy, sz, duration): (String, f32, f32, f32, f32)| {
                        if let Ok(mut list) = tw_scale_s.lock() {
                            list.push(tweens::TweenRequest {
                                entity_id: id,
                                target: glam::Vec3::new(sx, sy, sz),
                                duration,
                                is_scale: true,
                            });
                        }
                        Ok(true)
                    });
                    if let Ok(f) = tween_scale { let _ = lua.globals().set("tween_scale", f); }

                    let scene_load_s = self.lua_ctx.pending_scene_load.clone();
                    let load_scene = lua.create_function(move |_, path: String| {
                        if let Ok(mut pending) = scene_load_s.lock() {
                            *pending = Some(path);
                        }
                        Ok(true)
                    });
                    if let Ok(f) = load_scene { let _ = lua.globals().set("load_scene", f); }

                    let save_store_s = self.lua_ctx.save_data_store.clone();
                    let save_data = lua.create_function(move |_, (key, val): (String, mlua::Value)| {
                        let json_val = scripting::lua_to_json(&val);
                        if let Ok(mut store) = save_store_s.write() {
                            store.insert(key, json_val);
                            if let Ok(serialized) = serde_json::to_string_pretty(&*store) {
                                let _ = std::fs::write("assets/save_data.json", serialized);
                            }
                        }
                        Ok(true)
                    });
                    if let Ok(f) = save_data { let _ = lua.globals().set("save_data", f); }

                    let load_store_s = self.lua_ctx.save_data_store.clone();
                    let load_data = lua.create_function(move |lua, key: String| {
                        if let Ok(store) = load_store_s.read() {
                            if let Some(val) = store.get(&key) {
                                return scripting::json_to_lua(lua, val);
                            }
                        }
                        Ok(mlua::Value::Nil)
                    });
                    if let Ok(f) = load_data { let _ = lua.globals().set("load_data", f); }

                    let btns_s = self.lua_ctx.hud_buttons.clone();
                    let set_hud_button = lua.create_function(move |_, (id, text, x, y, callback): (String, String, f32, f32, String)| {
                        if let Ok(mut btns) = btns_s.write() {
                            btns.insert(id.clone(), scripting::HudButtonDef { id, text, x, y, callback });
                        }
                        Ok(true)
                    });
                    if let Ok(f) = set_hud_button { let _ = lua.globals().set("set_hud_button", f); }

                    let sprites_s = self.lua_ctx.pending_sprite_frames.clone();
                    let set_sprite_frame = lua.create_function(move |_, (id, cols, rows, frame): (String, u32, u32, u32)| {
                        if let Ok(mut list) = sprites_s.lock() {
                            list.push((id, cols.max(1), rows.max(1), frame));
                        }
                        Ok(true)
                    });
                    if let Ok(f) = set_sprite_frame { let _ = lua.globals().set("set_sprite_frame", f); }

                    let chunk = lua.load(&code);
                    let result_str = match chunk.eval::<mlua::Value>() {
                        Ok(val) => format!("{:?}", val),
                        Err(_) => match lua.load(&code).exec() {
                            Ok(()) => "Code executed successfully (no return value)".to_string(),
                            Err(e) => {
                                let err_msg = format!("Lua snippet error: {}", e);
                                self.logs.push(err_msg.clone());
                                let _ = respond_to.send(Err(err_msg));
                                return;
                            }
                        },
                    };
                    self.logs.push(format!("Executed Lua snippet: {}", code));
                    let _ = respond_to.send(Ok(result_str));
                }
                McpCommand::SaveScene { path, respond_to } => {
                    let target_path = path.unwrap_or_else(|| "assets/scene.json".to_string());
                    self.save_scene_to_file(&target_path);
                    let msg = format!("Scene saved to '{}'", target_path);
                    let _ = respond_to.send(Ok(msg));
                }
                McpCommand::SpawnPrefab { name, position, respond_to } => {
                    let file_path = if name.ends_with(".json") {
                        if name.starts_with("assets/prefabs/") {
                            name.clone()
                        } else {
                            format!("assets/prefabs/{}", name)
                        }
                    } else {
                        format!("assets/prefabs/{}.json", name)
                    };

                    match assets::load_prefab_from_file(&file_path) {
                        Ok(mut def) => {
                            self.entity_counter += 1;
                            def.id = format!("{}_{}", def.id, self.entity_counter);
                            def.position = position;

                            let meta = EntityMeta {
                                id: def.id.clone(),
                                name: def.id.clone(),
                                parent: def.parent.clone(),
                            };
                            let transform = Transform::new(
                                Vec3::from(def.position),
                                Vec3::from(def.rotation),
                                Vec3::from(def.scale),
                            );
                            let mesh = MeshRender {
                                mesh_type: def.mesh.clone(),
                                color: def.color,
                                texture: def.texture.clone(),
                                emissive: def.emissive,
                                fresnel_color: def.fresnel_color,
                                fresnel_power: def.fresnel_power,
                                blend_mode: def.blend_mode.clone(),
                                wireframe: def.wireframe,
                                uv_scroll: def.uv_scroll,
                            };
                            let e = self.world.spawn((meta, transform, mesh));

                            if let Some(phys) = &def.physics {
                                let handle = self.physics.add_body(&transform, phys);
                                let _ = self.world.insert_one(e, RigidBodyComponent { handle });
                            }

                            if let Some(script_path) = &def.script {
                                let mut s = ScriptComponent::new(script_path.clone());
                                let _ = s.init_or_reload(self.lua_ctx.clone());
                                let _ = self.world.insert_one(e, s);
                            }

                            if let Some(pl) = &def.point_light {
                                let _ = self.world.insert_one(e, ecs::PointLight {
                                    color: pl.color,
                                    intensity: pl.intensity,
                                    radius: pl.radius,
                                });
                            }

                            self.selected_entity = Some(e);
                            let msg = format!("Spawned prefab '{}' as entity '{}' at {:?}", name, def.id, position);
                            self.logs.push(msg.clone());
                            let _ = respond_to.send(Ok(msg));
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to spawn prefab '{}': {}", name, e);
                            self.logs.push(err_msg.clone());
                            let _ = respond_to.send(Err(err_msg));
                        }
                    }
                }
                McpCommand::SelectEntity { id, respond_to } => {
                    let mut found = None;
                    for er in self.world.iter() {
                        if let Some(meta) = er.get::<&EntityMeta>() {
                            if meta.id == id {
                                found = Some(er.entity());
                                break;
                            }
                        }
                    }
                    if let Some(e) = found {
                        self.selected_entity = Some(e);
                        self.add_toast(format!("Selected '{}'", id), egui::Color32::from_rgb(56, 189, 248));
                        let _ = respond_to.send(Ok(format!("Selected entity '{}'", id)));
                    } else {
                        let _ = respond_to.send(Err(format!("Entity '{}' not found", id)));
                    }
                }
                McpCommand::ForgeCreateModel { recipe, respond_to } => {
                    self.workspace = EditorWorkspace::Modeler;
                    let mesh = recipe.build_mesh();
                    self.forge_recipe = recipe;
                    let v_count = mesh.positions.len();
                    let t_count = mesh.indices.len() / 3;
                    self.add_toast(format!("Forge: Loaded '{}' ({} verts)", self.forge_recipe.name, v_count), egui::Color32::from_rgb(56, 189, 248));
                    let _ = respond_to.send(Ok(serde_json::json!({
                        "success": true,
                        "recipe_name": self.forge_recipe.name,
                        "vertex_count": v_count,
                        "triangle_count": t_count
                    })));
                }
                McpCommand::ForgeBakeGlb { recipe, output_name, insert_in_scene, respond_to } => {
                    let mesh = recipe.build_mesh();
                    let out_path = format!("assets/models/{}.glb", output_name);
                    match crate::glb_exporter::export_glb_to_file(&mesh, &output_name, &out_path) {
                        Ok(()) => {
                            self.gpu.ensure_mesh(&out_path);
                            if insert_in_scene {
                                let id = format!("{}_spawned", output_name);
                                let pos = [0.0, 0.0, 0.0];
                                let mut ent_builder = hecs::EntityBuilder::new();
                                ent_builder.add(EntityMeta { id: id.clone(), name: output_name.clone(), parent: None });
                                ent_builder.add(Transform {
                                    position: Vec3::from(pos),
                                    rotation: Vec3::ZERO,
                                    scale: Vec3::ONE,
                                });
                                ent_builder.add(MeshRender {
                                    mesh_type: out_path.clone(),
                                    color: [1.0, 1.0, 1.0, 1.0],
                                    emissive: 0.0,
                                    wireframe: Some(false),
                                    texture: None,
                                    blend_mode: Some("opaque".to_string()),
                                    fresnel_color: None,
                                    fresnel_power: None,
                                    uv_scroll: None,
                                });
                                let spawned = self.world.spawn(ent_builder.build());
                                self.selected_entity = Some(spawned);
                            }
                            let v_count = mesh.positions.len();
                            let t_count = mesh.indices.len() / 3;
                            self.logs.push(format!("Baked Forge GLB model to '{}' ({} verts, {} tris)", out_path, v_count, t_count));
                            self.add_toast(format!("Baked '{}'", out_path), egui::Color32::from_rgb(34, 197, 94));
                            let _ = respond_to.send(Ok(serde_json::json!({
                                "success": true,
                                "path": out_path,
                                "vertices": v_count,
                                "triangles": t_count
                            })));
                        }
                        Err(e) => {
                            let _ = respond_to.send(Err(format!("Error baking GLB: {}", e)));
                        }
                    }
                }
                McpCommand::ForgeCaptureTurnaround { angle, respond_to } => {
                    let (eye, target) = match angle.to_lowercase().as_str() {
                        "front" => (Vec3::new(0.0, 1.5, 4.0), Vec3::new(0.0, 1.0, 0.0)),
                        "side" => (Vec3::new(4.0, 1.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),
                        "top" => (Vec3::new(0.0, 6.0, 0.001), Vec3::new(0.0, 0.0, 0.0)),
                        _ => (Vec3::new(3.0, 3.0, 3.0), Vec3::new(0.0, 0.5, 0.0)),
                    };
                    self.camera.eye = eye;
                    self.camera.target = target;
                    self.gpu.update_camera(&self.camera);
                    self.pending_viewport_capture = Some(respond_to);
                }
                McpCommand::ForgeCreateTexture { style, filename, size, seed, color_tint, respond_to } => {
                    match crate::texture_forge::save_procedural_texture(&style, &filename, size, seed, color_tint) {
                        Ok(saved_path) => {
                            self.gpu.get_or_load_texture(&saved_path);
                            self.logs.push(format!("Generated procedural texture '{}' (style: {}, {}x{})", saved_path, style, size, size));
                            self.add_toast(format!("Texture created: {}", saved_path), egui::Color32::from_rgb(34, 197, 94));
                            let _ = respond_to.send(Ok(serde_json::json!({
                                "success": true,
                                "path": saved_path,
                                "style": style,
                                "size": size,
                            })));
                        }
                        Err(e) => {
                            let _ = respond_to.send(Err(format!("Error generating procedural texture: {}", e)));
                        }
                    }
                }
            }
        }
    }

    pub fn update_and_render(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;
        self.total_time += dt;

        // Drain pending post process updates from Lua
        if let Ok(mut pending_pp) = self.lua_ctx.pending_post_process.lock() {
            if let Some(pp) = pending_pp.take() {
                self.post_process_settings = pp;
            }
        }

        // Drain pending environment updates from Lua
        if let Ok(mut pending_env) = self.lua_ctx.pending_environment.lock() {
            if let Some(env) = pending_env.take() {
                self.environment_settings = env;
            }
        }

        self.frametime_history.push_back(dt * 1000.0);
        if self.frametime_history.len() > 90 {
            self.frametime_history.pop_front();
        }

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
                            match script.init_or_reload(self.lua_ctx.clone()) {
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
            // Sync active camera to Lua context
            if let Ok(mut cam) = self.lua_ctx.active_camera.write() {
                *cam = self.camera.clone();
            }

            // Sync mouse info to Lua context
            let (screen_pos, screen_size) = if self.is_standalone_play || !self.viewport_rect.is_positive() {
                (self.mouse_pos, [self.gpu.config.width as f32, self.gpu.config.height as f32])
            } else {
                let v_min = self.viewport_rect.min;
                (
                    [self.mouse_pos[0] - v_min.x, self.mouse_pos[1] - v_min.y],
                    [self.viewport_rect.width().max(1.0), self.viewport_rect.height().max(1.0)],
                )
            };

            if let Ok(mut m) = self.lua_ctx.mouse_info.write() {
                m.screen_pos = screen_pos;
                m.screen_size = screen_size;
                m.buttons_down = [self.is_left_mouse_down, self.is_right_mouse_down, self.is_middle_mouse_down];
                m.buttons_just_pressed = self.just_pressed_mouse;
            }

            // 1. Sync entity positions to LuaEngineContext for get_entity(id)
            if let Ok(mut we) = self.lua_ctx.world_entities.write() {
                we.clear();
                for (meta, transform) in self.world.query::<(&EntityMeta, &Transform)>().iter() {
                    we.insert(
                        meta.id.clone(),
                        [
                            transform.position.x,
                            transform.position.y,
                            transform.position.z,
                            transform.rotation.x,
                            transform.rotation.y,
                            transform.rotation.z,
                            transform.scale.x,
                            transform.scale.y,
                            transform.scale.z,
                        ],
                    );
                }
            }

            // Update physics raycaster snapshot for Lua raycast queries
            let mut collider_to_entity = HashMap::new();
            for entity_ref in self.world.iter() {
                if let (Some(meta), Some(rb)) = (entity_ref.get::<&EntityMeta>(), entity_ref.get::<&RigidBodyComponent>()) {
                    if let Some(rb_obj) = self.physics.rigid_body_set.get(rb.handle) {
                        for col_h in rb_obj.colliders() {
                            collider_to_entity.insert(*col_h, meta.id.clone());
                        }
                    }
                }
            }
            if let Ok(mut rc) = self.lua_ctx.physics_raycaster.write() {
                *rc = Some(scripting::PhysicsRaycaster {
                    broad_phase: self.physics.broad_phase.clone(),
                    narrow_phase: self.physics.narrow_phase.clone(),
                    rigid_body_set: self.physics.rigid_body_set.clone(),
                    collider_set: self.physics.collider_set.clone(),
                    collider_to_entity,
                });
            }

            // 2. Run Lua scripts update
            for (meta, transform, script) in self.world.query_mut::<(&EntityMeta, &mut Transform, &mut ScriptComponent)>() {
                if let Err(e) = script.update(transform, dt) {
                    let err_msg = format!("Lua runtime error on '{}': {}", meta.id, e);
                    if self.last_lua_error.as_ref() != Some(&err_msg) {
                        self.logs.push(err_msg.clone());
                        self.last_lua_error = Some(err_msg);
                    }
                }
            }
            scripting::process_timers_and_events(&mut self.world, &self.lua_ctx, dt, &mut self.logs);

            // 3. Apply forces & impulses
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

            // 4. Step Physics
            self.physics.step(dt);
            self.physics.sync_to_ecs(&mut self.world);

            // 5. Handle Collision Events & dispatch on_collision to Lua
            let collisions = if let Ok(mut lock) = self.physics.collision_events.lock() {
                std::mem::take(&mut *lock)
            } else {
                Vec::new()
            };

            if !collisions.is_empty() {
                let mut handle_to_info = HashMap::new();
                for entity_ref in self.world.iter() {
                    let entity = entity_ref.entity();
                    if let (Some(meta), Some(rb)) = (entity_ref.get::<&EntityMeta>(), entity_ref.get::<&RigidBodyComponent>()) {
                        handle_to_info.insert(rb.handle, (entity, meta.id.clone()));
                    }
                }

                for col in collisions {
                    if !col.started {
                        continue;
                    }
                    let body_a = self.physics.collider_set.get(col.collider_1).and_then(|c| c.parent());
                    let body_b = self.physics.collider_set.get(col.collider_2).and_then(|c| c.parent());

                    if let (Some(h_a), Some(h_b)) = (body_a, body_b) {
                        let a_info = handle_to_info.get(&h_a).cloned();
                        let b_info = handle_to_info.get(&h_b).cloned();

                        if let (Some((e_a, id_a)), Some((e_b, id_b))) = (a_info, b_info) {
                            let is_sensor_a = self.physics.collider_set.get(col.collider_1).map(|c| c.is_sensor()).unwrap_or(false);
                            let is_sensor_b = self.physics.collider_set.get(col.collider_2).map(|c| c.is_sensor()).unwrap_or(false);

                            if is_sensor_a || is_sensor_b || col.is_sensor {
                                if is_sensor_a {
                                    if let Ok(script) = self.world.get::<&ScriptComponent>(e_a) {
                                        if let Err(e) = script.on_trigger_enter(&id_a, &id_b) {
                                            self.logs.push(format!("Trigger error on '{}': {}", id_a, e));
                                        }
                                    }
                                }
                                if is_sensor_b {
                                    if let Ok(script) = self.world.get::<&ScriptComponent>(e_b) {
                                        if let Err(e) = script.on_trigger_enter(&id_b, &id_a) {
                                            self.logs.push(format!("Trigger error on '{}': {}", id_b, e));
                                        }
                                    }
                                }
                            } else {
                                if let Ok(script) = self.world.get::<&ScriptComponent>(e_a) {
                                    if let Err(e) = script.on_collision(&id_a, &id_b) {
                                        self.logs.push(format!("Collision error on '{}': {}", id_a, e));
                                    }
                                }
                                if let Ok(script) = self.world.get::<&ScriptComponent>(e_b) {
                                    if let Err(e) = script.on_collision(&id_b, &id_a) {
                                        self.logs.push(format!("Collision error on '{}': {}", id_b, e));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 6. Handle pending spawns from Lua
            let pending_spawns = if let Ok(mut spawns) = self.lua_ctx.pending_spawns.lock() {
                std::mem::take(&mut *spawns)
            } else {
                Vec::new()
            };
            for def in pending_spawns {
                let meta = EntityMeta {
                    id: def.id.clone(),
                    name: def.id.clone(),
                    parent: def.parent.clone(),
                };
                let transform = Transform::new(
                    Vec3::from(def.position),
                    Vec3::from(def.rotation),
                    Vec3::from(def.scale),
                );
                let mesh = MeshRender {
                    mesh_type: def.mesh.clone(),
                    color: def.color,
                    texture: def.texture.clone(),
                    emissive: def.emissive,
                    fresnel_color: def.fresnel_color,
                    fresnel_power: def.fresnel_power,
                    blend_mode: def.blend_mode.clone(),
                    wireframe: def.wireframe,
                    uv_scroll: def.uv_scroll,
                };
                let e = self.world.spawn((meta, transform, mesh));

                if let Some(phys) = &def.physics {
                    let handle = self.physics.add_body(&transform, phys);
                    let _ = self.world.insert_one(e, RigidBodyComponent { handle });
                }

                if let Some(script_path) = &def.script {
                    let mut s = ScriptComponent::new(script_path.clone());
                    let _ = s.init_or_reload(self.lua_ctx.clone());
                    let _ = self.world.insert_one(e, s);
                }
            }

            // 7. Handle pending destroys from Lua
            let pending_destroys = if let Ok(mut destroys) = self.lua_ctx.pending_destroys.lock() {
                std::mem::take(&mut *destroys)
            } else {
                Vec::new()
            };
            for target_id in pending_destroys {
                let to_remove: Option<(hecs::Entity, Option<rapier3d::prelude::RigidBodyHandle>)> = {
                    let mut found = None;
                    for entity_ref in self.world.iter() {
                        let entity = entity_ref.entity();
                        if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                            if meta.id == target_id {
                                let rb_handle = entity_ref.get::<&RigidBodyComponent>().map(|r| r.handle);
                                found = Some((entity, rb_handle));
                                break;
                            }
                        }
                    }
                    found
                };
                if let Some((e, rb_opt)) = to_remove {
                    if let Some(handle) = rb_opt {
                        self.physics.remove_body(handle);
                    }
                    let _ = self.world.despawn(e);
                    if self.selected_entity == Some(e) {
                        self.selected_entity = None;
                    }
                }
            }

            // 8. Handle pending animations from Lua
            let pending_anims = if let Ok(mut a) = self.lua_ctx.pending_anims.lock() {
                std::mem::take(&mut *a)
            } else {
                Vec::new()
            };
            for cmd in pending_anims {
                match cmd {
                    scripting::AnimationCommand::Play { entity_id, clip_name, looping } => {
                        for entity_ref in self.world.iter() {
                            if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                                if meta.id == entity_id {
                                    let e = entity_ref.entity();
                                    if let Ok(mut player) = self.world.get::<&mut crate::animation::AnimationPlayer>(e) {
                                        player.play(&clip_name, looping);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    scripting::AnimationCommand::Stop { entity_id } => {
                        for entity_ref in self.world.iter() {
                            if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                                if meta.id == entity_id {
                                    let e = entity_ref.entity();
                                    if let Ok(mut player) = self.world.get::<&mut crate::animation::AnimationPlayer>(e) {
                                        player.stop();
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    scripting::AnimationCommand::PlayPreset { entity_id, preset_name } => {
                        let mut target = None;
                        for entity_ref in self.world.iter() {
                            if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                                if meta.id == entity_id {
                                    target = Some(entity_ref.entity());
                                    break;
                                }
                            }
                        }
                        if let Some(e) = target {
                            if let Ok(mut player) = self.world.get::<&mut crate::animation::AnimationPlayer>(e) {
                                player.play_preset(&preset_name);
                            } else {
                                let mut player = crate::animation::AnimationPlayer::new();
                                player.play_preset(&preset_name);
                                let _ = self.world.insert_one(e, player);
                            }
                        }
                    }
                }
            }

            // 9. Update Animation Players (Play mode)
            let mut anim_states = HashMap::new();
            for (meta, player, transform, mesh_render) in self.world.query_mut::<(
                &EntityMeta,
                &mut crate::animation::AnimationPlayer,
                &mut Transform,
                Option<&mut MeshRender>,
            )>() {
                if let Some(state) = player.update(dt) {
                    crate::animation::AnimationPlayer::apply_to(&state, transform, mesh_render);
                }
                anim_states.insert(meta.id.clone(), player.is_playing());
            }
            if let Ok(mut states_lock) = self.lua_ctx.anim_playing_states.write() {
                *states_lock = anim_states;
            }

            // Handle pending nav bake from Lua
            let pending_bake = if let Ok(mut lock) = self.lua_ctx.pending_nav_bake.lock() {
                lock.take()
            } else {
                None
            };
            if let Some((cs, extent)) = pending_bake {
                if let Ok(mut grid) = self.lua_ctx.nav_grid.write() {
                    *grid = nav::NavGrid::new(cs, extent);
                    grid.bake_from_world(&self.world);
                }
            }
        }

        // Preview Animation Players in Edit mode if playing
        if self.play_mode == PlayMode::Edit {
            for (_meta, player, transform, mesh_render) in self.world.query_mut::<(
                &EntityMeta,
                &mut crate::animation::AnimationPlayer,
                &mut Transform,
                Option<&mut MeshRender>,
            )>() {
                if player.playing {
                    if let Some(state) = player.update(dt) {
                        crate::animation::AnimationPlayer::apply_to(&state, transform, mesh_render);
                    }
                }
            }
        }

        // Check for pending scene transition from Lua
        let pending_scene = if let Ok(mut s) = self.lua_ctx.pending_scene_load.lock() {
            s.take()
        } else {
            None
        };
        if let Some(scene_file) = pending_scene {
            let p = Path::new(&scene_file);
            match assets::load_scene_from_file(p) {
                Ok(scene) => {
                    self.world.clear();
                    self.physics.clear();
                    self.particles.clear();
                    self.selected_entity = None;
                    assets::apply_scene_to_world(&scene, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
                    let msg = format!("Scene transitioned to '{}'", scene_file);
                    self.logs.push(msg);
                }
                Err(e) => {
                    self.logs.push(format!("Failed to transition to scene '{}': {}", scene_file, e));
                }
            }
        }

        // Dispatch pending HUD button clicks to Lua scripts
        let pending_clicks = if let Ok(mut c) = self.lua_ctx.pending_button_clicks.lock() {
            std::mem::take(&mut *c)
        } else {
            Vec::new()
        };
        for callback_name in pending_clicks {
            for (_, script) in self.world.query_mut::<(&EntityMeta, &mut ScriptComponent)>() {
                if let Some(lua_arc) = &script.lua {
                    if let Ok(lua) = lua_arc.lock() {
                        if let Ok(func) = lua.globals().get::<mlua::Function>(callback_name.as_str()) {
                            let _ = func.call::<()>(());
                        }
                    }
                }
            }
        }

        // Drain and apply pending parents from Lua
        let pending_parents = if let Ok(mut parents) = self.lua_ctx.pending_parents.lock() {
            std::mem::take(&mut *parents)
        } else {
            Vec::new()
        };
        for (child_id, new_parent) in pending_parents {
            for meta in self.world.query_mut::<&mut EntityMeta>() {
                if meta.id == child_id {
                    meta.parent = new_parent;
                    break;
                }
            }
        }

        // Drain and apply pending light updates
        let pending_lights = if let Ok(mut lights) = self.lua_ctx.pending_lights.lock() {
            std::mem::take(&mut *lights)
        } else {
            Vec::new()
        };
        for (id, col, intensity, radius) in pending_lights {
            let target_entity = {
                let mut found = None;
                for entity_ref in self.world.iter() {
                    if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                        if meta.id == id {
                            found = Some(entity_ref.entity());
                            break;
                        }
                    }
                }
                found
            };
            if let Some(entity) = target_entity {
                if let Ok(mut pl) = self.world.get::<&mut ecs::PointLight>(entity) {
                    pl.color = col;
                    pl.intensity = intensity;
                    pl.radius = radius;
                } else {
                    let _ = self.world.insert_one(entity, ecs::PointLight {
                        color: col,
                        intensity,
                        radius,
                    });
                }
            }
        }

        // Drain and apply material updates from Lua
        let pending_mat_updates = if let Ok(mut m) = self.lua_ctx.pending_materials.lock() {
            std::mem::take(&mut *m)
        } else {
            Vec::new()
        };
        for update in pending_mat_updates {
            for entity_ref in self.world.iter() {
                if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                    if meta.id == update.entity_id {
                        let e = entity_ref.entity();
                        if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(e) {
                            if let Some(c) = update.color { mesh.color = c; }
                            if let Some(em) = update.emissive { mesh.emissive = em; }
                            if let Some(fc) = update.fresnel_color { mesh.fresnel_color = Some(fc); }
                            if let Some(fp) = update.fresnel_power { mesh.fresnel_power = Some(fp); }
                            if let Some(bm) = update.blend_mode { mesh.blend_mode = Some(bm); }
                            if let Some(wf) = update.wireframe { mesh.wireframe = Some(wf); }
                            if let Some(uv) = update.uv_scroll { mesh.uv_scroll = Some(uv); }
                        }
                        break;
                    }
                }
            }
        }

        // Drain and apply sprite frame updates from Lua
        let pending_sprites = if let Ok(mut s) = self.lua_ctx.pending_sprite_frames.lock() {
            std::mem::take(&mut *s)
        } else {
            Vec::new()
        };
        for (id, cols, rows, frame) in pending_sprites {
            let target_entity = {
                let mut found = None;
                for entity_ref in self.world.iter() {
                    if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                        if meta.id == id {
                            found = Some(entity_ref.entity());
                            break;
                        }
                    }
                }
                found
            };
            if let Some(entity) = target_entity {
                let _ = self.world.insert_one(entity, ecs::SpriteAnimation::new(cols, rows, frame));
            }
        }

        // Drain and apply camera commands from Lua
        let pending_cam_sets = if let Ok(mut c) = self.lua_ctx.pending_camera_sets.lock() {
            std::mem::take(&mut *c)
        } else {
            Vec::new()
        };
        for (eye, target) in pending_cam_sets {
            self.camera.eye = Vec3::from(eye);
            self.camera.target = Vec3::from(target);
        }

        let pending_cam_follows = if let Ok(mut c) = self.lua_ctx.pending_camera_follows.lock() {
            std::mem::take(&mut *c)
        } else {
            Vec::new()
        };
        for (target_id, offset, smoothness) in pending_cam_follows {
            self.camera.follow_entity(target_id, Vec3::from(offset), smoothness);
        }

        let pending_cam_shakes = if let Ok(mut s) = self.lua_ctx.pending_camera_shakes.lock() {
            std::mem::take(&mut *s)
        } else {
            Vec::new()
        };
        for (intensity, duration) in pending_cam_shakes {
            self.camera.shake(intensity, duration);
        }

        let pending_cam_lerps = if let Ok(mut c) = self.lua_ctx.pending_camera_lerps.lock() {
            std::mem::take(&mut *c)
        } else {
            Vec::new()
        };
        for (eye, target, duration) in pending_cam_lerps {
            self.camera.start_lerp_to(Vec3::from(eye), Vec3::from(target), duration);
        }

        let pending_cam_orbits = if let Ok(mut c) = self.lua_ctx.pending_camera_orbits.lock() {
            std::mem::take(&mut *c)
        } else {
            Vec::new()
        };
        for (target_id, distance, pitch, yaw_speed) in pending_cam_orbits {
            self.camera.start_orbit_target(target_id, distance, pitch, yaw_speed);
        }

        // Drain pending tweens from Lua
        let pending_tweens = if let Ok(mut t) = self.lua_ctx.pending_tweens.lock() {
            std::mem::take(&mut *t)
        } else {
            Vec::new()
        };
        for req in pending_tweens {
            self.tweens.pending_requests.push_back(req);
        }

        // Update active tweens
        self.tweens.update(dt, &mut self.world);

        // Drain and spawn pending particles
        let pending_particles = if let Ok(mut particles) = self.lua_ctx.pending_particles.lock() {
            std::mem::take(&mut *particles)
        } else {
            Vec::new()
        };
        for req in pending_particles {
            self.particles.spawn(
                Vec3::from(req.position),
                req.color,
                req.count,
                req.speed,
                req.lifetime,
            );
        }

        // Update active particles
        self.particles.update(dt);

        // Clear just_pressed_keys and mouse delta at end of frame
        if let Ok(mut jp) = self.lua_ctx.just_pressed_keys.write() {
            jp.clear();
        }
        self.just_pressed_mouse = [false; 3];
        if let Ok(mut minfo) = self.lua_ctx.mouse_info.write() {
            minfo.delta = [0.0, 0.0];
            minfo.buttons_just_pressed = [false; 3];
        }

        // Compute world transforms (ParentWorld * Local) with cycle detection
        let world_transforms = compute_world_transforms(&self.world);

        // Update camera cinematics (lerp & orbit)
        let mut entity_pos_map = HashMap::new();
        for entity_ref in self.world.iter() {
            if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                if let Some((_, pos)) = world_transforms.get(&entity_ref.entity()) {
                    entity_pos_map.insert(meta.id.clone(), *pos);
                } else if let Some(t) = entity_ref.get::<&Transform>() {
                    entity_pos_map.insert(meta.id.clone(), t.position);
                }
            }
        }
        self.camera.update_cinematics(dt, &entity_pos_map);

        // Update camera follow (smooth lerp towards target entity world position)
        if let Some((ref target_id, offset, smoothness)) = self.camera.follow_target.clone() {
            let target_pos = {
                let mut found = None;
                for entity_ref in self.world.iter() {
                    if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                        if meta.id == *target_id {
                            if let Some((_, pos)) = world_transforms.get(&entity_ref.entity()) {
                                found = Some(*pos);
                            } else if let Some(t) = entity_ref.get::<&Transform>() {
                                found = Some(t.position);
                            }
                            break;
                        }
                    }
                }
                found
            };
            if let Some(pos) = target_pos {
                let target_eye = pos + offset;
                let lerp_factor = (smoothness * dt * 10.0).clamp(0.0, 1.0);
                self.camera.eye = self.camera.eye.lerp(target_eye, lerp_factor);
                self.camera.target = self.camera.target.lerp(pos, lerp_factor);
            }
        }

        // Update camera shake
        self.camera.update_shake(dt);

        // Update camera in GPU with simulation time and environment settings
        self.gpu.update_camera_with_environment(
            &self.camera,
            self.total_time,
            &self.environment_settings.sky_mode,
            self.environment_settings.zenith_color,
            self.environment_settings.horizon_color,
            self.environment_settings.fog_color,
            self.environment_settings.fog_density,
        );

        // Collect active point lights into GPU uniform
        let mut lights_uniform = LightsUniform::default();
        let mut light_count = 0u32;
        for entity_ref in self.world.iter() {
            if light_count >= 16 {
                break;
            }
            let entity = entity_ref.entity();
            if let Some(light) = entity_ref.get::<&ecs::PointLight>() {
                let world_pos = if let Some((_, pos)) = world_transforms.get(&entity) {
                    [pos.x, pos.y, pos.z]
                } else if let Some(t) = entity_ref.get::<&Transform>() {
                    [t.position.x, t.position.y, t.position.z]
                } else {
                    [0.0, 0.0, 0.0]
                };
                lights_uniform.lights[light_count as usize] = PointLightRaw {
                    position: world_pos,
                    radius: light.radius,
                    color: light.color,
                    intensity: light.intensity,
                };
                light_count += 1;
            }
        }
        lights_uniform.num_lights = light_count;
        self.gpu.update_lights(&lights_uniform);

        // Drain pending trails from Lua
        let pending_trails = if let Ok(mut t) = self.lua_ctx.pending_trails.lock() {
            std::mem::take(&mut *t)
        } else {
            Vec::new()
        };
        for cmd in pending_trails {
            let target_e = {
                let mut found = None;
                for er in self.world.iter() {
                    if let Some(meta) = er.get::<&EntityMeta>() {
                        if meta.id == cmd.entity_id {
                            found = Some(er.entity());
                            break;
                        }
                    }
                }
                found
            };
            if let Some(e) = target_e {
                let _ = self.world.insert_one(e, ecs::TrailComponent::new(cmd.color, cmd.lifetime, cmd.width));
            }
        }

        // Update trail points
        let mut trail_updates = Vec::new();
        for er in self.world.iter() {
            if er.get::<&ecs::TrailComponent>().is_some() {
                let entity = er.entity();
                let world_pos = if let Some((_, pos)) = world_transforms.get(&entity) {
                    *pos
                } else if let Some(t) = er.get::<&Transform>() {
                    t.position
                } else {
                    Vec3::ZERO
                };
                trail_updates.push((entity, world_pos));
            }
        }
        for (entity, world_pos) in trail_updates {
            if let Ok(mut trail) = self.world.get::<&mut ecs::TrailComponent>(entity) {
                trail.push_point(world_pos, self.total_time);
                trail.prune(self.total_time);
            }
        }

        // Build dynamic trail ribbon mesh
        let mut trail_vertices: Vec<renderer::Vertex> = Vec::new();
        let mut trail_indices: Vec<u16> = Vec::new();
        for trail in self.world.query_mut::<&ecs::TrailComponent>() {
            if trail.points.len() < 2 {
                continue;
            }
            let pts: Vec<_> = trail.points.iter().cloned().collect();
            for i in 0..pts.len() - 1 {
                let p_a = &pts[i];
                let p_b = &pts[i + 1];
                let dir = (p_b.position - p_a.position).normalize_or_zero();
                let cam_dir = (self.camera.eye - p_a.position).normalize_or_zero();
                let mut side = dir.cross(cam_dir).normalize_or_zero();
                if side.length_squared() < 0.01 {
                    side = dir.cross(Vec3::Y).normalize_or_zero();
                }

                let t_a = ((trail.lifetime - (self.total_time - p_a.time)) / trail.lifetime).clamp(0.0, 1.0);
                let t_b = ((trail.lifetime - (self.total_time - p_b.time)) / trail.lifetime).clamp(0.0, 1.0);

                let w_a = side * (trail.width * 0.5 * t_a);
                let w_b = side * (trail.width * 0.5 * t_b);

                let col_a = [trail.color[0], trail.color[1], trail.color[2], trail.color[3] * t_a];
                let col_b = [trail.color[0], trail.color[1], trail.color[2], trail.color[3] * t_b];

                let base_idx = trail_vertices.len() as u16;
                trail_vertices.push(renderer::Vertex {
                    position: (p_a.position - w_a).to_array(),
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 0.0],
                    color: col_a,
                });
                trail_vertices.push(renderer::Vertex {
                    position: (p_a.position + w_a).to_array(),
                    normal: [0.0, 1.0, 0.0],
                    uv: [1.0, 0.0],
                    color: col_a,
                });
                trail_vertices.push(renderer::Vertex {
                    position: (p_b.position - w_b).to_array(),
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 1.0],
                    color: col_b,
                });
                trail_vertices.push(renderer::Vertex {
                    position: (p_b.position + w_b).to_array(),
                    normal: [0.0, 1.0, 0.0],
                    uv: [1.0, 1.0],
                    color: col_b,
                });

                trail_indices.push(base_idx);
                trail_indices.push(base_idx + 2);
                trail_indices.push(base_idx + 1);

                trail_indices.push(base_idx + 1);
                trail_indices.push(base_idx + 2);
                trail_indices.push(base_idx + 3);
            }
        }

        // Build dynamic debug lines (from draw_line_3d and draw_ring_3d)
        let mut debug_line_vertices: Vec<renderer::Vertex> = Vec::new();
        let debug_lines_list = if let Ok(mut l) = self.lua_ctx.debug_lines.lock() {
            std::mem::take(&mut *l)
        } else {
            Vec::new()
        };
        for line in debug_lines_list {
            debug_line_vertices.push(renderer::Vertex {
                position: line.start,
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                color: line.color,
            });
            debug_line_vertices.push(renderer::Vertex {
                position: line.end,
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
                color: line.color,
            });
        }

        let debug_rings_list = if let Ok(mut r) = self.lua_ctx.debug_rings.lock() {
            std::mem::take(&mut *r)
        } else {
            Vec::new()
        };
        for ring in debug_rings_list {
            let segs = 32;
            for i in 0..segs {
                let theta1 = 2.0 * std::f32::consts::PI * (i as f32 / segs as f32);
                let theta2 = 2.0 * std::f32::consts::PI * ((i + 1) as f32 / segs as f32);
                let p1 = [
                    ring.center[0] + ring.radius * theta1.cos(),
                    ring.center[1],
                    ring.center[2] + ring.radius * theta1.sin(),
                ];
                let p2 = [
                    ring.center[0] + ring.radius * theta2.cos(),
                    ring.center[1],
                    ring.center[2] + ring.radius * theta2.sin(),
                ];
                debug_line_vertices.push(renderer::Vertex {
                    position: p1,
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0, 0.0],
                    color: ring.color,
                });
                debug_line_vertices.push(renderer::Vertex {
                    position: p2,
                    normal: [0.0, 1.0, 0.0],
                    uv: [1.0, 1.0],
                    color: ring.color,
                });
            }
        }

        // Collect instances by mesh type, texture, and blend mode
        let mut instances_by_batch: HashMap<(String, Option<String>, String), Vec<InstanceRaw>> = HashMap::new();
        for entity_ref in self.world.iter() {
            let entity = entity_ref.entity();
            if let (Some(transform), Some(mesh)) = (entity_ref.get::<&Transform>(), entity_ref.get::<&MeshRender>()) {
                let model_mat = if let Some((mat, _)) = world_transforms.get(&entity) {
                    mat.to_cols_array_2d()
                } else {
                    transform.matrix().to_cols_array_2d()
                };
                let (uv_offset, uv_scale) = if let Some(anim) = entity_ref.get::<&ecs::SpriteAnimation>() {
                    anim.uv_offset_and_scale()
                } else {
                    ([0.0, 0.0], [1.0, 1.0])
                };
                let fresnel = if let Some(fc) = mesh.fresnel_color {
                    [fc[0], fc[1], fc[2], mesh.fresnel_power.unwrap_or(2.0)]
                } else {
                    [0.0, 0.0, 0.0, 0.0]
                };
                let scroll = mesh.uv_scroll.unwrap_or([0.0, 0.0]);
                let wire_flag = match self.view_mode {
                    renderer::ViewMode::Lit => {
                        if mesh.wireframe.unwrap_or(false) { 2.0 } else { 0.0 }
                    }
                    renderer::ViewMode::Unlit => 1.0,
                    renderer::ViewMode::Wireframe => 2.0,
                };
                let raw = InstanceRaw {
                    model: model_mat,
                    color: mesh.color,
                    extra: [mesh.emissive, scroll[0], scroll[1], wire_flag],
                    fresnel,
                    uv_offset,
                    uv_scale,
                };
                let blend_mode = mesh.blend_mode.clone().unwrap_or_else(|| "opaque".to_string());
                instances_by_batch
                    .entry((mesh.mesh_type.clone(), mesh.texture.clone(), blend_mode))
                    .or_default()
                    .push(raw);
            }
        }

        if self.workspace == EditorWorkspace::Modeler {
            let forge_mesh = self.forge_recipe.build_mesh();
            self.gpu.insert_forge_mesh("forge_preview", &forge_mesh);
            let raw = InstanceRaw {
                model: glam::Mat4::IDENTITY.to_cols_array_2d(),
                color: [1.0, 1.0, 1.0, 1.0],
                extra: [0.0, 0.0, 0.0, 0.0],
                fresnel: [0.0, 0.0, 0.0, 0.0],
                uv_offset: [0.0, 0.0],
                uv_scale: [1.0, 1.0],
            };
            instances_by_batch
                .entry(("forge_preview".to_string(), None, "opaque".to_string()))
                .or_default()
                .push(raw);
        }

        self.last_draw_calls = instances_by_batch.len();

        for (mesh_id, tex_opt, _) in instances_by_batch.keys() {
            self.gpu.ensure_mesh(mesh_id);
            if let Some(tex) = tex_opt {
                self.gpu.get_or_load_texture(tex);
            }
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
        let mut trigger_template: Option<&'static str> = None;
        let mut trigger_reload_scene = false;
        let mut trigger_spawn_mesh = None;
        let mut trigger_duplicate = false;
        let mut trigger_delete = false;
        let mut trigger_save_script = false;
        let mut script_to_open = None;

        let mut trigger_refresh_assets = false;
        let egui_ctx = self.gpu.egui_winit_state.egui_ctx().clone();

        let full_output = egui_ctx.run_ui(raw_input, |ui| {
                ui_theme::apply_theme(ui.ctx());

                if self.is_standalone_play {
                    // In Standalone Game Mode: Render HUD overlay and buttons inside full-viewport CentralPanel
                    CentralPanel::default()
                        .frame(Frame::new().fill(egui::Color32::TRANSPARENT))
                        .show(ui, |ui| {
                            if let Ok(hud) = self.lua_ctx.hud_texts.read() {
                                if !hud.is_empty() {
                                    let hud_area = egui::Area::new(ui.make_persistent_id("lua_hud_overlay_standalone"))
                                        .fixed_pos(egui::pos2(25.0, 25.0))
                                        .interactable(false);
                                    hud_area.show(ui.ctx(), |ui| {
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_black_alpha(180))
                                            .corner_radius(8.0)
                                            .inner_margin(12.0)
                                            .show(ui, |ui| {
                                                for (k, v) in hud.iter() {
                                                    ui.colored_label(
                                                        egui::Color32::from_rgb(100, 220, 255),
                                                        format!("{}: {}", k, v),
                                                    );
                                                }
                                            });
                                    });
                                }
                            }

                            if let Ok(btns) = self.lua_ctx.hud_buttons.read() {
                                for btn in btns.values() {
                                    let btn_area = egui::Area::new(ui.make_persistent_id(format!("hud_btn_std_{}", btn.id)))
                                        .fixed_pos(egui::pos2(btn.x, btn.y));
                                    btn_area.show(ui.ctx(), |ui| {
                                        if ui.button(egui::RichText::new(&btn.text).size(15.0).strong()).clicked() {
                                            if let Ok(mut clicks) = self.lua_ctx.pending_button_clicks.lock() {
                                                clicks.push(btn.callback.clone());
                                            }
                                        }
                                    });
                                }
                            }

                            let painter = ui.painter();
                            let screen_size = [self.gpu.config.width as f32, self.gpu.config.height as f32];
                            let viewport_rect = ui.max_rect();

                            // Declarative In-Game UI from Lua (game_ui)
                            if let Ok(elements) = self.lua_ctx.game_ui_elements.read() {
                                game_ui::render_declarative_ui(ui.ctx(), &painter, viewport_rect, &elements, &self.lua_ctx.pending_button_clicks);
                            }

                            if let Ok(labels) = self.lua_ctx.world_labels.read() {
                                for label in labels.values() {
                                    let mut found_pos = None;
                                    for er in self.world.iter() {
                                        if let Some(meta) = er.get::<&EntityMeta>() {
                                            if meta.id == label.entity_id {
                                                if let Some((_, pos)) = world_transforms.get(&er.entity()) {
                                                    found_pos = Some(*pos);
                                                } else if let Some(t) = er.get::<&Transform>() {
                                                    found_pos = Some(t.position);
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    if let Some(pos) = found_pos {
                                        let target = pos + glam::Vec3::new(0.0, label.offset_y, 0.0);
                                        if let Some([sx, sy]) = self.camera.world_to_screen(target, screen_size) {
                                            let p2 = egui::pos2(sx, sy);
                                            if viewport_rect.contains(p2) {
                                                let col = egui::Color32::from_rgb(
                                                    (label.color[0] * 255.0) as u8,
                                                    (label.color[1] * 255.0) as u8,
                                                    (label.color[2] * 255.0) as u8,
                                                );
                                                painter.text(
                                                    p2,
                                                    egui::Align2::CENTER_CENTER,
                                                    &label.text,
                                                    egui::FontId::proportional(13.0),
                                                    col,
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            if let Ok(mut texts) = self.lua_ctx.floating_texts.lock() {
                                texts.retain_mut(|ft| {
                                    ft.elapsed += dt;
                                    ft.position[1] += dt * 0.8;
                                    let alpha = (1.0 - ft.elapsed / ft.lifetime).clamp(0.0, 1.0);
                                    if let Some([sx, sy]) = self.camera.world_to_screen(glam::Vec3::from(ft.position), screen_size) {
                                        let p2 = egui::pos2(sx, sy);
                                        if viewport_rect.contains(p2) {
                                            let col = egui::Color32::from_rgba_unmultiplied(
                                                (ft.color[0] * 255.0) as u8,
                                                (ft.color[1] * 255.0) as u8,
                                                (ft.color[2] * 255.0) as u8,
                                                (alpha * 255.0) as u8,
                                            );
                                            painter.text(
                                                p2,
                                                egui::Align2::CENTER_CENTER,
                                                &ft.text,
                                                egui::FontId::proportional(15.0),
                                                col,
                                            );
                                        }
                                    }
                                    ft.elapsed < ft.lifetime
                                });
                            }
                        });
                    return;
                }

                // 1. TOP MENU & TOOLBAR
                Panel::top("top_toolbar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("[+] New Empty Scene").clicked() {
                                trigger_new_scene = true;
                                ui.close();
                            }
                            ui.menu_button("[T] New from Template", |ui| {
                                if ui.button("Empty 3D").clicked() {
                                    trigger_template = Some("empty_3d");
                                    ui.close();
                                }
                                if ui.button("Top-Down 2D").clicked() {
                                    trigger_template = Some("top_down_2d");
                                    ui.close();
                                }
                                if ui.button("Platformer 3D").clicked() {
                                    trigger_template = Some("platformer_3d");
                                    ui.close();
                                }
                                if ui.button("Space Arena").clicked() {
                                    trigger_template = Some("space_arena");
                                    ui.close();
                                }
                            });
                            if ui.button("[^] Save Scene (Ctrl+S)").clicked() {
                                trigger_save_scene = true;
                                ui.close();
                            }
                            if ui.button("[*] Reload Scene").clicked() {
                                trigger_reload_scene = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("[Export] Export Game...").clicked() {
                                self.show_export_modal = true;
                                ui.close();
                            }
                        });

                        ui.separator();

                        // Simulation Controls Play / Pause / Stop
                        let is_playing = self.play_mode == PlayMode::Play;
                        let is_paused = self.play_mode == PlayMode::Pause;

                        if ui.selectable_label(is_playing, "> Play").clicked() {
                            if self.play_mode == PlayMode::Edit {
                                self.play_snapshot = Some(assets::extract_scene_from_world(&self.world, &self.camera, &self.physics));
                            }
                            self.play_mode = PlayMode::Play;
                            self.logs.push("Simulation started (Play Mode)".to_string());
                            self.add_toast("Play mode active", egui::Color32::from_rgb(34, 197, 94));
                        }

                        if ui.selectable_label(is_paused, "|| Pause").clicked() {
                            self.play_mode = PlayMode::Pause;
                            self.logs.push("Simulation paused".to_string());
                            self.add_toast("Simulation paused", egui::Color32::from_rgb(250, 204, 21));
                        }

                        if ui.button("[] Stop").clicked() {
                            if let Some(snapshot) = self.play_snapshot.take() {
                                assets::apply_scene_to_world(&snapshot, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
                                self.logs.push("Simulation stopped. Scene restored to initial snapshot.".to_string());
                                self.add_toast("Simulation stopped & restored", egui::Color32::from_rgb(239, 68, 68));
                            }
                            self.particles.clear();
                            self.lua_ctx.clear_timers_and_events();
                            self.play_mode = PlayMode::Edit;
                        }

                        ui.separator();

                        // Gizmo Mode Selectors
                        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Translate, "[W] Move");
                        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Rotate, "[E] Rotate");
                        ui.selectable_value(&mut self.gizmo_mode, GizmoMode::Scale, "[R] Scale");

                        ui.separator();
                        ui.checkbox(&mut self.snap_enabled, "Snap (Ctrl)");

                        ui.separator();
                        ui.label("Workspace:");
                        ui.selectable_value(&mut self.workspace, EditorWorkspace::Scene, "🌐 Scene");
                        ui.selectable_value(&mut self.workspace, EditorWorkspace::Modeler, "⚒ Modeler");
                    });
                });

                // Status Bar at the bottom of the window
                Panel::bottom("status_bar")
                    .resizable(false)
                    .default_size(24.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 14.0;

                            // 1. MCP Server Status
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "●");
                                ui.label(egui::RichText::new("MCP: 3030").size(11.5).strong());
                            });

                            // 2. Hot-Reload Status
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(56, 189, 248), "●");
                                ui.label(egui::RichText::new("Hot-Reload: Active").size(11.5).strong());
                            });

                            // 3. Selected Entity
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Selected:").color(ui_theme::COLOR_TEXT_MUTED).size(11.5));
                                let selected_str = if let Some(e) = self.selected_entity {
                                    self.world.get::<&EntityMeta>(e).map(|m| m.id.clone()).unwrap_or_else(|_| format!("{:?}", e))
                                } else {
                                    "None".to_string()
                                };
                                ui.label(egui::RichText::new(selected_str).strong().color(ui_theme::COLOR_ACCENT_PRIMARY).size(11.5));
                            });

                            // 4. Command Palette hint aligned right
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("[Ctrl+K] Command Palette").clicked() {
                                    self.command_palette_open = true;
                                    self.command_palette_query.clear();
                                }
                            });
                        });
                    });

                // 2. LEFT PANEL: SCENE HIERARCHY
                Panel::left("scene_hierarchy")
                    .resizable(true)
                    .default_size(240.0)
                    .show(ui, |ui| {
                        if self.workspace == EditorWorkspace::Modeler {
                            ui.heading("⚒ Asset Forge");
                            ui.label(egui::RichText::new("Native Parametric 3D Modeler").size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.text_edit_singleline(&mut self.forge_recipe.name);
                            });

                            ui.add_space(4.0);
                            ui_theme::collapsible_card(ui, "forge_presets", "Presets", true, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if ui.button("🚀 Spaceship").clicked() {
                                        self.forge_recipe = crate::modeler::preset_spaceship();
                                        self.forge_selected_part = None;
                                    }
                                    if ui.button("🪨 Asteroid").clicked() {
                                        self.forge_recipe = crate::modeler::preset_asteroid();
                                        self.forge_selected_part = None;
                                    }
                                    if ui.button("🗼 Turret").clicked() {
                                        self.forge_recipe = crate::modeler::preset_turret();
                                        self.forge_selected_part = None;
                                    }
                                    if ui.button("👾 Boss").clicked() {
                                        self.forge_recipe = crate::modeler::preset_boss();
                                        self.forge_selected_part = None;
                                    }
                                });
                            });

                            ui.add_space(4.0);
                            ui_theme::collapsible_card(ui, "forge_add_parts", "Add Primitives", true, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    let mut add_prim = |name_prefix: &str, ptype: &str, col: [f32; 4]| {
                                        let idx = self.forge_recipe.parts.len() + 1;
                                        self.forge_recipe.parts.push(crate::modeler::PartDef {
                                            name: format!("{}_{}", name_prefix, idx),
                                            primitive: ptype.to_string(),
                                            points: None,
                                            position: [0.0, 0.0, 0.0],
                                            rotation_deg: [0.0, 0.0, 0.0],
                                            scale: [1.0, 1.0, 1.0],
                                            color: col,
                                            mirror_x: false,
                                            mirror_z: false,
                                            modifiers: Vec::new(),
                                        });
                                        self.forge_selected_part = Some(self.forge_recipe.parts.len() - 1);
                                    };

                                    if ui.button("+ Box").clicked() {
                                        add_prim("box", "box", [0.7, 0.7, 0.8, 1.0]);
                                    }
                                    if ui.button("+ Sphere").clicked() {
                                        add_prim("sphere", "sphere", [0.3, 0.7, 0.9, 1.0]);
                                    }
                                    if ui.button("+ Cylinder").clicked() {
                                        add_prim("cylinder", "cylinder", [0.4, 0.8, 0.4, 1.0]);
                                    }
                                    if ui.button("+ Cone").clicked() {
                                        add_prim("cone", "cone", [0.9, 0.6, 0.2, 1.0]);
                                    }
                                    if ui.button("+ Torus").clicked() {
                                        add_prim("torus", "torus", [0.9, 0.4, 0.8, 1.0]);
                                    }
                                    if ui.button("+ Capsule").clicked() {
                                        add_prim("capsule", "capsule", [0.2, 0.8, 0.7, 1.0]);
                                    }
                                    if ui.button("+ Prism").clicked() {
                                        add_prim("prism", "prism", [0.8, 0.7, 0.3, 1.0]);
                                    }
                                    if ui.button("+ Lathe").clicked() {
                                        add_prim("lathe", "lathe", [0.8, 0.3, 0.5, 1.0]);
                                    }
                                });
                            });

                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(format!("Parts ({})", self.forge_recipe.parts.len())).strong());
                            let mut to_delete = None;
                            egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                                for (i, part) in self.forge_recipe.parts.iter().enumerate() {
                                    let is_sel = self.forge_selected_part == Some(i);
                                    ui.horizontal(|ui| {
                                        if ui.selectable_label(is_sel, format!("{} ({})", part.name, part.primitive)).clicked() {
                                            self.forge_selected_part = Some(i);
                                        }
                                        if ui.small_button("✖").clicked() {
                                            to_delete = Some(i);
                                        }
                                    });
                                }
                            });
                            if let Some(del_idx) = to_delete {
                                self.forge_recipe.parts.remove(del_idx);
                                self.forge_selected_part = None;
                            }
                            return;
                        }

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
                            if ui.button("+ Torus").clicked() {
                                trigger_spawn_mesh = Some(("torus", [0.9, 0.4, 0.9, 1.0]));
                            }
                            if ui.button("+ Ring").clicked() {
                                trigger_spawn_mesh = Some(("ring", [0.2, 0.9, 0.8, 1.0]));
                            }
                            if ui.button("+ Cone").clicked() {
                                trigger_spawn_mesh = Some(("cone", [0.9, 0.6, 0.2, 1.0]));
                            }
                            if ui.button("+ Icosphere").clicked() {
                                trigger_spawn_mesh = Some(("icosphere", [0.4, 0.8, 0.9, 1.0]));
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

                        // Entity List with Tree Hierarchy
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut all_entities = Vec::new();
                            let mut children_map: HashMap<String, Vec<(hecs::Entity, String, String, bool, bool, bool)>> = HashMap::new();
                            let mut known_ids = HashSet::new();

                            for entity_ref in self.world.iter() {
                                let entity = entity_ref.entity();
                                if let (Some(meta), Some(mesh)) = (entity_ref.get::<&EntityMeta>(), entity_ref.get::<&MeshRender>()) {
                                    if self.hierarchy_search.is_empty() || meta.id.to_lowercase().contains(&self.hierarchy_search.to_lowercase()) {
                                        let has_physics = entity_ref.get::<&RigidBodyComponent>().is_some();
                                        let has_light = entity_ref.get::<&ecs::PointLight>().is_some();
                                        let has_script = entity_ref.get::<&ScriptComponent>().is_some();
                                        all_entities.push((entity, meta.id.clone(), meta.parent.clone(), mesh.mesh_type.clone(), has_physics, has_light, has_script));
                                        known_ids.insert(meta.id.clone());
                                    }
                                }
                            }

                            for (entity, id, parent, mesh_type, has_physics, has_light, has_script) in &all_entities {
                                if let Some(pid) = parent {
                                    if known_ids.contains(pid) && pid != id {
                                        children_map.entry(pid.clone()).or_default().push((*entity, id.clone(), mesh_type.clone(), *has_physics, *has_light, *has_script));
                                    }
                                }
                            }

                            fn render_node(
                                ui: &mut egui::Ui,
                                entity: hecs::Entity,
                                id: &str,
                                mesh_type: &str,
                                has_physics: bool,
                                has_light: bool,
                                has_script: bool,
                                selected: &mut Option<hecs::Entity>,
                                children_map: &HashMap<String, Vec<(hecs::Entity, String, String, bool, bool, bool)>>,
                                depth: usize,
                            ) {
                                let is_selected = *selected == Some(entity);
                                let indent_str = if depth == 0 { "* " } else { "|- " };
                                let label = format!("{}{}{} ({})", "  ".repeat(depth), indent_str, id, mesh_type);
                                
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(is_selected, label).clicked() {
                                        *selected = Some(entity);
                                    }
                                    if has_physics {
                                        ui.label(egui::RichText::new("[P]").color(egui::Color32::from_rgb(251, 146, 60)).small())
                                            .on_hover_text("RigidBody Physics Active");
                                    }
                                    if has_light {
                                        ui.label(egui::RichText::new("[L]").color(egui::Color32::from_rgb(250, 204, 21)).small())
                                            .on_hover_text("Point Light Active");
                                    }
                                    if has_script {
                                        ui.label(egui::RichText::new("[S]").color(egui::Color32::from_rgb(56, 189, 248)).small())
                                            .on_hover_text("Lua Script Attached");
                                    }
                                });

                                if let Some(children) = children_map.get(id) {
                                    for (c_e, c_id, c_mesh, c_phys, c_light, c_script) in children {
                                        render_node(ui, *c_e, c_id, c_mesh, *c_phys, *c_light, *c_script, selected, children_map, depth + 1);
                                    }
                                }
                            }

                            for (entity, id, parent, mesh_type, has_physics, has_light, has_script) in &all_entities {
                                let is_root = match parent {
                                    None => true,
                                    Some(pid) => !known_ids.contains(pid) || pid == id,
                                };
                                if is_root {
                                    render_node(ui, *entity, id, mesh_type, *has_physics, *has_light, *has_script, &mut self.selected_entity, &children_map, 0);
                                }
                            }
                        });
                    });

                // 3. RIGHT PANEL: INSPECTOR
                Panel::right("inspector_panel")
                    .resizable(true)
                    .default_size(290.0)
                    .show(ui, |ui| {
                        if self.workspace == EditorWorkspace::Modeler {
                            ui.heading("⚒ Part Inspector");
                            ui.separator();

                            if let Some(idx) = self.forge_selected_part {
                                if idx < self.forge_recipe.parts.len() {
                                    let part = &mut self.forge_recipe.parts[idx];
                                    ui_theme::collapsible_card(ui, "forge_part_basic", "Basic Properties", true, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Name:");
                                            ui.text_edit_singleline(&mut part.name);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Type:");
                                            ui.label(egui::RichText::new(&part.primitive).strong());
                                        });
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut part.mirror_x, "Mirror X");
                                            ui.checkbox(&mut part.mirror_z, "Mirror Z");
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Color:");
                                            let mut c = egui::Color32::from_rgba_unmultiplied(
                                                (part.color[0] * 255.0) as u8,
                                                (part.color[1] * 255.0) as u8,
                                                (part.color[2] * 255.0) as u8,
                                                (part.color[3] * 255.0) as u8,
                                            );
                                            if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                                                part.color = [
                                                    c.r() as f32 / 255.0,
                                                    c.g() as f32 / 255.0,
                                                    c.b() as f32 / 255.0,
                                                    c.a() as f32 / 255.0,
                                                ];
                                            }
                                        });
                                    });

                                    ui.add_space(4.0);
                                    ui_theme::collapsible_card(ui, "forge_part_transform", "Transform", true, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Pos:");
                                            ui.add(egui::DragValue::new(&mut part.position[0]).speed(0.05).prefix("X:"));
                                            ui.add(egui::DragValue::new(&mut part.position[1]).speed(0.05).prefix("Y:"));
                                            ui.add(egui::DragValue::new(&mut part.position[2]).speed(0.05).prefix("Z:"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Rot:");
                                            ui.add(egui::DragValue::new(&mut part.rotation_deg[0]).speed(1.0).prefix("X:").suffix("°"));
                                            ui.add(egui::DragValue::new(&mut part.rotation_deg[1]).speed(1.0).prefix("Y:").suffix("°"));
                                            ui.add(egui::DragValue::new(&mut part.rotation_deg[2]).speed(1.0).prefix("Z:").suffix("°"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Scale:");
                                            ui.add(egui::DragValue::new(&mut part.scale[0]).speed(0.05).prefix("X:"));
                                            ui.add(egui::DragValue::new(&mut part.scale[1]).speed(0.05).prefix("Y:"));
                                            ui.add(egui::DragValue::new(&mut part.scale[2]).speed(0.05).prefix("Z:"));
                                        });
                                    });

                                    ui.add_space(4.0);
                                    ui_theme::collapsible_card(ui, "forge_part_modifiers", "Modifiers", true, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            if ui.button("+ Taper").clicked() {
                                                part.modifiers.push(crate::modeler::ModifierDef::Taper { factor: 0.5, axis: "y".to_string() });
                                            }
                                            if ui.button("+ Twist").clicked() {
                                                part.modifiers.push(crate::modeler::ModifierDef::Twist { angle_deg: 90.0, axis: "y".to_string() });
                                            }
                                            if ui.button("+ Bend").clicked() {
                                                part.modifiers.push(crate::modeler::ModifierDef::Bend { angle_deg: 45.0, axis: "x".to_string() });
                                            }
                                            if ui.button("+ Noise").clicked() {
                                                part.modifiers.push(crate::modeler::ModifierDef::NoiseDisplace { frequency: 2.0, amplitude: 0.1, seed: 42 });
                                            }
                                            if ui.button("+ Flat").clicked() {
                                                part.modifiers.push(crate::modeler::ModifierDef::FlatShading);
                                            }
                                        });
                                        let mut mod_del = None;
                                        for (mi, m) in part.modifiers.iter_mut().enumerate() {
                                            ui.horizontal(|ui| {
                                                match m {
                                                    crate::modeler::ModifierDef::Taper { factor, axis: _ } => {
                                                        ui.label("Taper");
                                                        ui.add(egui::DragValue::new(factor).speed(0.05).prefix("f:"));
                                                    }
                                                    crate::modeler::ModifierDef::Twist { angle_deg, axis: _ } => {
                                                        ui.label("Twist");
                                                        ui.add(egui::DragValue::new(angle_deg).speed(1.0).suffix("°"));
                                                    }
                                                    crate::modeler::ModifierDef::Bend { angle_deg, axis: _ } => {
                                                        ui.label("Bend");
                                                        ui.add(egui::DragValue::new(angle_deg).speed(1.0).suffix("°"));
                                                    }
                                                    crate::modeler::ModifierDef::NoiseDisplace { frequency, amplitude, seed: _ } => {
                                                        ui.label("Noise");
                                                        ui.add(egui::DragValue::new(amplitude).speed(0.01).prefix("amp:"));
                                                        ui.add(egui::DragValue::new(frequency).speed(0.1).prefix("freq:"));
                                                    }
                                                    crate::modeler::ModifierDef::FlatShading => {
                                                        ui.label("Flat Shading");
                                                    }
                                                }
                                                if ui.small_button("✖").clicked() {
                                                    mod_del = Some(mi);
                                                }
                                            });
                                        }
                                        if let Some(m_idx) = mod_del {
                                            part.modifiers.remove(m_idx);
                                        }
                                    });
                                }
                            } else {
                                ui.label("Select a part from the left list to inspect.");
                            }

                            ui.add_space(10.0);
                            ui_theme::collapsible_card(ui, "forge_global_bake", "Bake & Export", true, |ui| {
                                ui.checkbox(&mut self.forge_recipe.normalize_pivot, "Normalize Bottom Pivot");

                                ui.add_space(6.0);
                                if ui.button("💾 Bake to .GLB").clicked() {
                                    let mesh = self.forge_recipe.build_mesh();
                                    let out_path = format!("assets/models/{}.glb", self.forge_recipe.name);
                                    match crate::glb_exporter::export_glb_to_file(&mesh, &self.forge_recipe.name, &out_path) {
                                        Ok(()) => {
                                            self.gpu.ensure_mesh(&out_path);
                                            self.logs.push(format!("Saved model to '{}'", out_path));
                                            self.add_toast(format!("Baked to '{}'", out_path), egui::Color32::from_rgb(34, 197, 94));
                                        }
                                        Err(e) => {
                                            self.logs.push(format!("Failed to bake model: {}", e));
                                            self.add_toast("Bake failed", egui::Color32::from_rgb(239, 68, 68));
                                        }
                                    }
                                }

                                if ui.button("🚀 Bake & Insert in Scene").clicked() {
                                    let mesh = self.forge_recipe.build_mesh();
                                    let out_path = format!("assets/models/{}.glb", self.forge_recipe.name);
                                    match crate::glb_exporter::export_glb_to_file(&mesh, &self.forge_recipe.name, &out_path) {
                                        Ok(()) => {
                                            self.gpu.ensure_mesh(&out_path);
                                            let ent_id = format!("{}_spawned", self.forge_recipe.name);
                                            let mut ent_builder = hecs::EntityBuilder::new();
                                            ent_builder.add(EntityMeta { id: ent_id.clone(), name: self.forge_recipe.name.clone(), parent: None });
                                            ent_builder.add(Transform {
                                                position: Vec3::new(0.0, 1.0, 0.0),
                                                rotation: Vec3::ZERO,
                                                scale: Vec3::ONE,
                                            });
                                            ent_builder.add(MeshRender {
                                                mesh_type: out_path.clone(),
                                                color: [1.0, 1.0, 1.0, 1.0],
                                                emissive: 0.0,
                                                wireframe: Some(false),
                                                texture: None,
                                                blend_mode: Some("opaque".to_string()),
                                                fresnel_color: None,
                                                fresnel_power: None,
                                                uv_scroll: None,
                                            });
                                            let spawned = self.world.spawn(ent_builder.build());
                                            self.selected_entity = Some(spawned);
                                            self.workspace = EditorWorkspace::Scene;
                                            self.logs.push(format!("Baked and inserted '{}' into active scene", out_path));
                                            self.add_toast(format!("Inserted '{}' into scene", ent_id), egui::Color32::from_rgb(56, 189, 248));
                                        }
                                        Err(e) => {
                                            self.logs.push(format!("Failed to bake and insert model: {}", e));
                                            self.add_toast("Bake failed", egui::Color32::from_rgb(239, 68, 68));
                                        }
                                    }
                                }
                            });
                            return;
                        }

                        ui.heading("Inspector");
                        ui.separator();

                        if let Some(entity) = self.selected_entity {
                            if self.world.contains(entity) {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    // 1. Identity
                                    ui_theme::collapsible_card(ui, "insp_identity", "Entity Identity", true, |ui| {
                                        let mut all_other_ids: Vec<String> = Vec::new();
                                        for other_ref in self.world.iter() {
                                            if other_ref.entity() != entity {
                                                if let Some(m) = other_ref.get::<&EntityMeta>() {
                                                    all_other_ids.push(m.id.clone());
                                                }
                                            }
                                        }

                                        if let Ok(mut meta) = self.world.get::<&mut EntityMeta>(entity) {
                                            ui.horizontal(|ui| {
                                                ui.label("ID:");
                                                ui.text_edit_singleline(&mut meta.id);
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Parent:");
                                                let current_parent_text = meta.parent.clone().unwrap_or_else(|| "None (Root)".to_string());
                                                egui::ComboBox::from_id_salt("parent_select")
                                                    .selected_text(&current_parent_text)
                                                    .show_ui(ui, |ui| {
                                                        let is_none = meta.parent.is_none();
                                                        if ui.selectable_label(is_none, "None (Root)").clicked() {
                                                            meta.parent = None;
                                                        }
                                                        for pid in &all_other_ids {
                                                            let is_selected = meta.parent.as_ref() == Some(pid);
                                                            if ui.selectable_label(is_selected, pid).clicked() {
                                                                meta.parent = Some(pid.clone());
                                                            }
                                                        }
                                                    });
                                            });
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // 2. Transform
                                    let mut transform_changed = false;
                                    ui_theme::collapsible_card(ui, "insp_transform", "Transform", true, |ui| {
                                        if let Ok(mut transform) = self.world.get::<&mut Transform>(entity) {
                                            let mut pos = [transform.position.x, transform.position.y, transform.position.z];
                                            if ui_theme::ui_vec3_colored(ui, "Position", &mut pos, [0.0, 0.0, 0.0], 0.05, None) {
                                                transform.position = glam::Vec3::new(pos[0], pos[1], pos[2]);
                                                transform_changed = true;
                                            }

                                            ui.add_space(4.0);

                                            let mut rot_deg = [
                                                transform.rotation.x.to_degrees(),
                                                transform.rotation.y.to_degrees(),
                                                transform.rotation.z.to_degrees(),
                                            ];
                                            if ui_theme::ui_vec3_colored(ui, "Rotation (Deg)", &mut rot_deg, [0.0, 0.0, 0.0], 1.0, None) {
                                                transform.rotation = glam::Vec3::new(
                                                    rot_deg[0].to_radians(),
                                                    rot_deg[1].to_radians(),
                                                    rot_deg[2].to_radians(),
                                                );
                                                transform_changed = true;
                                            }

                                            ui.add_space(4.0);

                                            let mut scl = [transform.scale.x, transform.scale.y, transform.scale.z];
                                            if ui_theme::ui_vec3_colored(ui, "Scale", &mut scl, [1.0, 1.0, 1.0], 0.05, Some(0.01..=100.0)) {
                                                transform.scale = glam::Vec3::new(scl[0], scl[1], scl[2]);
                                                transform_changed = true;
                                            }
                                        }
                                    });
                                    if transform_changed {
                                        self.sync_entity_to_physics(entity);
                                    }

                                    ui.add_space(4.0);

                                    // 3. Render & Material
                                    ui_theme::collapsible_card(ui, "insp_render", "Render & Material", true, |ui| {
                                        if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(entity) {
                                            ui.horizontal(|ui| {
                                                ui.label("Color:");
                                                ui.color_edit_button_rgba_unmultiplied(&mut mesh.color);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Emissive:");
                                                ui.add(egui::Slider::new(&mut mesh.emissive, 0.0..=5.0));
                                            });

                                            // Blend Mode
                                            ui.horizontal(|ui| {
                                                ui.label("Blend Mode:");
                                                let mut current_mode = mesh.blend_mode.clone().unwrap_or_else(|| "opaque".to_string());
                                                egui::ComboBox::from_id_salt("blend_mode_select")
                                                    .selected_text(&current_mode)
                                                    .show_ui(ui, |ui| {
                                                        if ui.selectable_value(&mut current_mode, "opaque".to_string(), "Opaque").clicked() {
                                                            mesh.blend_mode = Some("opaque".to_string());
                                                        }
                                                        if ui.selectable_value(&mut current_mode, "alpha".to_string(), "Alpha Blend").clicked() {
                                                            mesh.blend_mode = Some("alpha".to_string());
                                                        }
                                                        if ui.selectable_value(&mut current_mode, "additive".to_string(), "Additive").clicked() {
                                                            mesh.blend_mode = Some("additive".to_string());
                                                        }
                                                    });
                                            });

                                            // Fresnel
                                            let mut has_fresnel = mesh.fresnel_color.is_some();
                                            if ui.checkbox(&mut has_fresnel, "Fresnel Glow").changed() {
                                                if has_fresnel {
                                                    mesh.fresnel_color = Some([0.4, 0.8, 1.0]);
                                                    mesh.fresnel_power = Some(2.5);
                                                } else {
                                                    mesh.fresnel_color = None;
                                                    mesh.fresnel_power = None;
                                                }
                                            }
                                            if let Some(ref mut fc) = mesh.fresnel_color {
                                                ui.horizontal(|ui| {
                                                    ui.label("  Fresnel Color:");
                                                    ui.color_edit_button_rgb(fc);
                                                });
                                                let mut fp = mesh.fresnel_power.unwrap_or(2.5);
                                                if ui.add(egui::Slider::new(&mut fp, 0.5..=8.0).text("Power")).changed() {
                                                    mesh.fresnel_power = Some(fp);
                                                }
                                            }

                                            // Wireframe & UV Scroll
                                            let mut wire = mesh.wireframe.unwrap_or(false);
                                            if ui.checkbox(&mut wire, "Wireframe Overlay").changed() {
                                                mesh.wireframe = Some(wire);
                                            }

                                            ui.horizontal(|ui| {
                                                ui.label("UV Scroll:");
                                                let mut uv = mesh.uv_scroll.unwrap_or([0.0, 0.0]);
                                                let mut changed = false;
                                                changed |= ui.add(egui::DragValue::new(&mut uv[0]).speed(0.01).prefix("X: ")).changed();
                                                changed |= ui.add(egui::DragValue::new(&mut uv[1]).speed(0.01).prefix("Y: ")).changed();
                                                if changed {
                                                    mesh.uv_scroll = if uv[0] == 0.0 && uv[1] == 0.0 { None } else { Some(uv) };
                                                }
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
                                                        ui.selectable_value(&mut mesh.mesh_type, "torus".to_string(), "Torus");
                                                        ui.selectable_value(&mut mesh.mesh_type, "ring".to_string(), "Ring");
                                                        ui.selectable_value(&mut mesh.mesh_type, "cone".to_string(), "Cone");
                                                        ui.selectable_value(&mut mesh.mesh_type, "icosphere".to_string(), "Icosphere");
                                                    });
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Custom Path:");
                                                ui.text_edit_singleline(&mut mesh.mesh_type);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Texture:");
                                                let mut tex_str = mesh.texture.clone().unwrap_or_default();
                                                if ui.text_edit_singleline(&mut tex_str).changed() {
                                                    mesh.texture = if tex_str.trim().is_empty() {
                                                        None
                                                    } else {
                                                        Some(tex_str.trim().to_string())
                                                    };
                                                }
                                                if mesh.texture.is_some() && ui.button("X").on_hover_text("Clear Texture").clicked() {
                                                    mesh.texture = None;
                                                }
                                            });

                                            // Procedural Texture Generator in Material Inspector
                                            ui.separator();
                                            ui.label(egui::RichText::new("Texture Forge (Procedural)").size(11.0).strong());
                                            ui.horizontal_wrapped(|ui| {
                                                if ui.small_button("🎨 Planet").clicked() {
                                                    let name = format!("tex_planet_{}", entity.id());
                                                    if let Ok(path) = crate::texture_forge::save_procedural_texture("planet", &name, 256, 42, mesh.color) {
                                                        self.gpu.get_or_load_texture(&path);
                                                        mesh.texture = Some(path);
                                                    }
                                                }
                                                if ui.small_button("🎨 SciFi").clicked() {
                                                    let name = format!("tex_scifi_{}", entity.id());
                                                    if let Ok(path) = crate::texture_forge::save_procedural_texture("scifi_panel", &name, 256, 42, mesh.color) {
                                                        self.gpu.get_or_load_texture(&path);
                                                        mesh.texture = Some(path);
                                                    }
                                                }
                                                if ui.small_button("🎨 Plasma").clicked() {
                                                    let name = format!("tex_plasma_{}", entity.id());
                                                    if let Ok(path) = crate::texture_forge::save_procedural_texture("plasma", &name, 256, 42, mesh.color) {
                                                        self.gpu.get_or_load_texture(&path);
                                                        mesh.texture = Some(path);
                                                    }
                                                }
                                                if ui.small_button("🎨 Grid").clicked() {
                                                    let name = format!("tex_grid_{}", entity.id());
                                                    if let Ok(path) = crate::texture_forge::save_procedural_texture("grid_neon", &name, 256, 42, mesh.color) {
                                                        self.gpu.get_or_load_texture(&path);
                                                        mesh.texture = Some(path);
                                                    }
                                                }
                                                if ui.small_button("🎨 Rock").clicked() {
                                                    let name = format!("tex_rock_{}", entity.id());
                                                    if let Ok(path) = crate::texture_forge::save_procedural_texture("rock", &name, 256, 42, mesh.color) {
                                                        self.gpu.get_or_load_texture(&path);
                                                        mesh.texture = Some(path);
                                                    }
                                                }
                                            });
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // 4. Point Light
                                    ui_theme::collapsible_card(ui, "insp_light", "Point Light", true, |ui| {
                                        let has_light = self.world.get::<&ecs::PointLight>(entity).is_ok();
                                        let mut enable_light = has_light;
                                        if ui.checkbox(&mut enable_light, "Enable Light").changed() {
                                            if enable_light {
                                                let _ = self.world.insert_one(entity, ecs::PointLight {
                                                    color: [1.0, 0.9, 0.7],
                                                    intensity: 2.0,
                                                    radius: 8.0,
                                                });
                                            } else {
                                                let _ = self.world.remove_one::<ecs::PointLight>(entity);
                                            }
                                        }

                                        if let Ok(mut light) = self.world.get::<&mut ecs::PointLight>(entity) {
                                            ui.horizontal(|ui| {
                                                ui.label("Color:");
                                                let mut rgb = [light.color[0], light.color[1], light.color[2]];
                                                if ui.color_edit_button_rgb(&mut rgb).changed() {
                                                    light.color = rgb;
                                                }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Intensity:");
                                                ui.add(egui::DragValue::new(&mut light.intensity).speed(0.1).range(0.0..=50.0));
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label("Radius:");
                                                ui.add(egui::DragValue::new(&mut light.radius).speed(0.2).range(0.1..=100.0));
                                            });
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // 5. Physics
                                    ui_theme::collapsible_card(ui, "insp_physics", "Physics (Rapier3D)", true, |ui| {
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
                                                        is_sensor: None,
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

                                                if let Some(&c_handle) = body.colliders().first() {
                                                    if let Some(col) = self.physics.collider_set.get_mut(c_handle) {
                                                        let mut is_sensor = col.is_sensor();
                                                        if ui.checkbox(&mut is_sensor, "Is Trigger / Sensor").changed() {
                                                            col.set_sensor(is_sensor);
                                                        }
                                                    }
                                                }

                                                let mut mass = body.mass();
                                                if ui.add(egui::DragValue::new(&mut mass).speed(0.1).prefix("Mass: ").range(0.01..=1000.0)).changed() {
                                                    // mass updated
                                                }
                                            }
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // 6. Scripting
                                    ui_theme::collapsible_card(ui, "insp_scripting", "Scripting (Lua)", true, |ui| {
                                        let mut current_script = self.world.get::<&ScriptComponent>(entity).map(|s| s.path.clone()).unwrap_or_default();
                                        let mut script_changed = false;
                                        ui.horizontal(|ui| {
                                            ui.label("Path:");
                                            if ui.text_edit_singleline(&mut current_script).changed() {
                                                script_changed = true;
                                            }
                                        });

                                        if !current_script.is_empty() {
                                            if ui.button("[Edit] Edit Script").clicked() {
                                                script_to_open = Some(current_script.clone());
                                            }
                                        }

                                        if script_changed {
                                            if current_script.trim().is_empty() {
                                                let _ = self.world.remove_one::<ScriptComponent>(entity);
                                            } else {
                                                let mut comp = ScriptComponent::new(current_script);
                                                let _ = comp.init_or_reload(self.lua_ctx.clone());
                                                let _ = self.world.insert_one(entity, comp);
                                            }
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // 7. Animation Player
                                    ui_theme::collapsible_card(ui, "insp_animation", "[A] Animation Player", true, |ui| {
                                        let has_anim = self.world.get::<&crate::animation::AnimationPlayer>(entity).is_ok();
                                        let mut enable_anim = has_anim;
                                        if ui.checkbox(&mut enable_anim, "Enable Animation Player").changed() {
                                            if enable_anim {
                                                let mut player = crate::animation::AnimationPlayer::new();
                                                let mut default_clip = crate::animation::AnimationClip::new("idle", 2.0);
                                                if let Ok(trans) = self.world.get::<&Transform>(entity) {
                                                    default_clip.add_keyframe(crate::animation::Keyframe {
                                                        time: 0.0,
                                                        position: Some([trans.position.x, trans.position.y, trans.position.z]),
                                                        rotation: Some([trans.rotation.x, trans.rotation.y, trans.rotation.z]),
                                                        scale: Some([trans.scale.x, trans.scale.y, trans.scale.z]),
                                                        color: None,
                                                        emissive: None,
                                                    });
                                                }
                                                player.add_clip(default_clip);
                                                let _ = self.world.insert_one(entity, player);
                                            } else {
                                                let _ = self.world.remove_one::<crate::animation::AnimationPlayer>(entity);
                                            }
                                        }

                                        if let Ok(mut player) = self.world.get::<&mut crate::animation::AnimationPlayer>(entity) {
                                             let clip_names: Vec<String> = player.animations.keys().cloned().collect();
                                             let current_clip = player.active_clip.clone().unwrap_or_else(|| "None".to_string());

                                             ui.horizontal(|ui| {
                                                 ui.label("Clip:");
                                                 egui::ComboBox::from_id_salt("anim_clip_select")
                                                     .selected_text(&current_clip)
                                                     .show_ui(ui, |ui| {
                                                         for name in &clip_names {
                                                             let is_sel = player.active_clip.as_ref() == Some(name);
                                                             if ui.selectable_label(is_sel, name).clicked() {
                                                                 player.active_clip = Some(name.clone());
                                                                 player.current_time = 0.0;
                                                             }
                                                         }
                                                     });
                                             });

                                             let duration = if let Some(clip_name) = &player.active_clip {
                                                 player.animations.get(clip_name).map(|c| c.duration).unwrap_or(1.0)
                                             } else {
                                                 1.0
                                             };

                                             ui.horizontal(|ui| {
                                                 if player.playing {
                                                     if ui.button("⏸ Pause").clicked() {
                                                         player.pause();
                                                     }
                                                 } else {
                                                     if ui.button("▶ Play").clicked() {
                                                         player.resume();
                                                     }
                                                 }
                                                 if ui.button("⏹ Stop").clicked() {
                                                     player.stop();
                                                 }
                                             });

                                             ui.horizontal(|ui| {
                                                 ui.label("Time:");
                                                 let mut t = player.current_time;
                                                 if ui.add(egui::Slider::new(&mut t, 0.0..=duration).suffix(" s")).changed() {
                                                     player.current_time = t;
                                                 }
                                             });

                                             ui.horizontal(|ui| {
                                                 ui.checkbox(&mut player.looping, "Loop");
                                                 ui.label("Speed:");
                                                 ui.add(egui::Slider::new(&mut player.speed, 0.1..=3.0));
                                             });

                                             if ui.button("+ Capture Keyframe").on_hover_text("Record current Transform & Material to active clip at cursor time").clicked() {
                                                 let pos_rot_scale = if let Ok(t) = self.world.get::<&Transform>(entity) {
                                                     Some(([t.position.x, t.position.y, t.position.z], [t.rotation.x, t.rotation.y, t.rotation.z], [t.scale.x, t.scale.y, t.scale.z]))
                                                 } else {
                                                     None
                                                 };
                                                 let col_em = if let Ok(m) = self.world.get::<&MeshRender>(entity) {
                                                     Some((m.color, m.emissive))
                                                 } else {
                                                     None
                                                 };
                                                 if let Some((pos, rot, scl)) = pos_rot_scale {
                                                     let col = col_em.map(|(c, _)| c);
                                                     let em = col_em.map(|(_, e)| e);
                                                     player.capture_keyframe(pos, rot, scl, col, em);
                                                     self.logs.push(format!("Captured keyframe for entity at {:.2}s", player.current_time));
                                                 }
                                             }
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // 8. Prefab Export
                                    ui_theme::collapsible_card(ui, "insp_prefab", "Prefab Export", false, |ui| {
                                        if ui.button("[*] Save as Prefab").clicked() {
                                            if let Some(def) = assets::extract_entity_def(&self.world, &self.physics, entity) {
                                                let prefab_path = format!("assets/prefabs/{}.json", def.id);
                                                match assets::save_prefab_to_file(&prefab_path, &def) {
                                                    Ok(()) => {
                                                        self.logs.push(format!("Saved prefab to '{}'", prefab_path));
                                                    }
                                                    Err(e) => {
                                                        self.logs.push(format!("Failed to save prefab '{}': {}", prefab_path, e));
                                                    }
                                                }
                                            }
                                        }
                                    });
                                });
                            } else {
                                self.selected_entity = None;
                                ui.label("Selected entity no longer exists.");
                            }
                        } else {
                            ui.label("No entity selected.");
                            ui.label("Select an entity from the hierarchy or viewport to inspect and modify it.");
                        }

                        ui.add_space(8.0);
                        ui_theme::collapsible_card(ui, "insp_post_process", "Post-Processing (HDR)", false, |ui| {
                            ui.checkbox(&mut self.post_process_settings.bloom_enabled, "Bloom Glow");
                            ui.add(egui::Slider::new(&mut self.post_process_settings.bloom_intensity, 0.0..=3.0).text("Bloom Intensity"));
                            ui.add(egui::Slider::new(&mut self.post_process_settings.bloom_threshold, 0.1..=2.5).text("Threshold"));
                            ui.add(egui::Slider::new(&mut self.post_process_settings.vignette_intensity, 0.0..=1.0).text("Vignette"));
                            ui.add(egui::Slider::new(&mut self.post_process_settings.exposure, 0.1..=3.0).text("Exposure"));
                        });

                        ui.add_space(8.0);
                        ui_theme::collapsible_card(ui, "insp_environment", "Atmosphere & Skybox", false, |ui| {
                            egui::ComboBox::from_label("Sky Mode")
                                .selected_text(&self.environment_settings.sky_mode)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.environment_settings.sky_mode, "space".to_string(), "Space (Stars & Nebula)");
                                    ui.selectable_value(&mut self.environment_settings.sky_mode, "gradient".to_string(), "Gradient");
                                    ui.selectable_value(&mut self.environment_settings.sky_mode, "solid".to_string(), "Solid Color");
                                });
                            ui.horizontal(|ui| {
                                ui.label("Zenith:");
                                ui.color_edit_button_rgb(&mut self.environment_settings.zenith_color);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Horizon:");
                                ui.color_edit_button_rgb(&mut self.environment_settings.horizon_color);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Fog Color:");
                                ui.color_edit_button_rgb(&mut self.environment_settings.fog_color);
                            });
                            ui.add(egui::Slider::new(&mut self.environment_settings.fog_density, 0.0..=0.1).text("Fog Density"));
                            if let Ok(mut lock) = self.lua_ctx.environment_settings.write() {
                                *lock = self.environment_settings.clone();
                            }
                        });
                    });

                // 4. BOTTOM PANEL: CONSOLE, ASSETS BROWSER, SCRIPT EDITOR
                Panel::bottom("bottom_panel")
                    .resizable(true)
                    .default_size(200.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.bottom_tab, BottomTab::Console, "[_] Console & MCP");
                            ui.selectable_value(&mut self.bottom_tab, BottomTab::Assets, "[/] Assets Browser");
                            ui.selectable_value(&mut self.bottom_tab, BottomTab::ScriptEditor, "[<>] Script Editor");
                        });
                        ui.separator();

                        match self.bottom_tab {
                            BottomTab::Console => {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Filter:").strong());
                                    ui.selectable_value(&mut self.console_filter, ConsoleFilter::All, "All");
                                    ui.selectable_value(&mut self.console_filter, ConsoleFilter::Info, "Info");
                                    ui.selectable_value(&mut self.console_filter, ConsoleFilter::Error, "Lua / Error");
                                    ui.selectable_value(&mut self.console_filter, ConsoleFilter::Mcp, "MCP");

                                    ui.separator();
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
                                            let is_err = log.contains("ERROR") || log.contains("error") || log.contains("Failed");
                                            let is_mcp = log.contains("MCP");
                                            let is_info = !is_err && !is_mcp;

                                            let match_filter = match self.console_filter {
                                                ConsoleFilter::All => true,
                                                ConsoleFilter::Info => is_info,
                                                ConsoleFilter::Error => is_err,
                                                ConsoleFilter::Mcp => is_mcp,
                                            };

                                            if match_filter {
                                                if is_err {
                                                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), log);
                                                } else if is_mcp {
                                                    ui.colored_label(egui::Color32::from_rgb(56, 189, 248), log);
                                                } else {
                                                    ui.monospace(log);
                                                }
                                            }
                                        }
                                    });
                            }
                            BottomTab::Assets => {
                                ui.horizontal(|ui| {
                                    ui.label("Files in 'assets/':");
                                    if ui.button("[*] Refresh").clicked() {
                                        trigger_refresh_assets = true;
                                    }
                                });
                                ui.separator();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for path in &self.assets_cache.clone() {
                                        ui.horizontal(|ui| {
                                            if path.contains("prefabs") && path.ends_with(".json") {
                                                ui.label(format!("[prefab] {}", path));
                                                if ui.button("[+] Spawn Prefab").clicked() {
                                                    match assets::load_prefab_from_file(path) {
                                                        Ok(mut def) => {
                                                            self.entity_counter += 1;
                                                            def.id = format!("{}_{}", def.id, self.entity_counter);
                                                            def.position = [0.0, 2.0, 0.0];

                                                            let meta = EntityMeta {
                                id: def.id.clone(),
                                name: def.id.clone(),
                                parent: def.parent.clone(),
                            };
                                                            let transform = Transform::new(
                                                                Vec3::from(def.position),
                                                                Vec3::from(def.rotation),
                                                                Vec3::from(def.scale),
                                                            );
                                                            let mesh = MeshRender {
                                                                mesh_type: def.mesh.clone(),
                                                                color: def.color,
                                                                texture: def.texture.clone(),
                                                                emissive: def.emissive,
                                                                fresnel_color: def.fresnel_color,
                                                                fresnel_power: def.fresnel_power,
                                                                blend_mode: def.blend_mode.clone(),
                                                                wireframe: def.wireframe,
                                                                uv_scroll: def.uv_scroll,
                                                            };
                                                            let e = self.world.spawn((meta, transform, mesh));

                                                            if let Some(phys) = &def.physics {
                                                                let handle = self.physics.add_body(&transform, phys);
                                                                let _ = self.world.insert_one(e, RigidBodyComponent { handle });
                                                            }

                                                            if let Some(script_path) = &def.script {
                                                                let mut s = ScriptComponent::new(script_path.clone());
                                                                let _ = s.init_or_reload(self.lua_ctx.clone());
                                                                let _ = self.world.insert_one(e, s);
                                                            }

                                                            self.selected_entity = Some(e);
                                                            self.logs.push(format!("Spawned prefab '{}' as entity '{}'", path, def.id));
                                                        }
                                                        Err(e) => {
                                                            self.logs.push(format!("Failed to load prefab '{}': {}", path, e));
                                                        }
                                                    }
                                                }
                                            } else if path.ends_with(".json") {
                                                ui.label(format!("[json] {}", path));
                                                if ui.button("Load Scene").clicked() {
                                                    match assets::load_scene_from_file(path) {
                                                        Ok(scene) => {
                                                            assets::apply_scene_to_world(&scene, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
                                                            self.selected_entity = None;
                                                            self.logs.push(format!("Loaded scene from '{}'", path));
                                                        }
                                                        Err(e) => {
                                                            self.logs.push(format!("Failed to load scene '{}': {}", path, e));
                                                        }
                                                    }
                                                }
                                            } else if path.ends_with(".lua") {
                                                ui.label(format!("[lua]  {}", path));
                                                if ui.button("Open in Editor").clicked() {
                                                    script_to_open = Some(path.clone());
                                                }
                                                if let Some(entity) = self.selected_entity {
                                                    if ui.button("Assign to Selected").clicked() {
                                                        let mut comp = ScriptComponent::new(path.clone());
                                                        let _ = comp.init_or_reload(self.lua_ctx.clone());
                                                        let _ = self.world.insert_one(entity, comp);
                                                        self.logs.push(format!("Assigned script '{}' to selected entity", path));
                                                    }
                                                }
                                            } else if path.ends_with(".glb") || path.ends_with(".gltf") {
                                                ui.label(format!("[mesh] {}", path));
                                                if let Some(entity) = self.selected_entity {
                                                    if ui.button("Assign Mesh to Selected").clicked() {
                                                        if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(entity) {
                                                            mesh.mesh_type = path.clone();
                                                            self.logs.push(format!("Assigned mesh '{}' to selected entity", path));
                                                        }
                                                    }
                                                }
                                            } else if path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg") {
                                                ui.label(format!("[tex]  {}", path));
                                                if let Some(entity) = self.selected_entity {
                                                    if ui.button("Assign Texture to Selected").clicked() {
                                                        if let Ok(mut mesh) = self.world.get::<&mut MeshRender>(entity) {
                                                            mesh.texture = Some(path.clone());
                                                            self.logs.push(format!("Assigned texture '{}' to selected entity", path));
                                                        }
                                                    }
                                                }
                                            } else if path.ends_with(".wav") || path.ends_with(".ogg") {
                                                ui.label(format!("[sfx]  {}", path));
                                                if ui.button("Play Preview").clicked() {
                                                    if let Ok(audio) = self.audio.lock() {
                                                        let _ = audio.play_sound(path, 1.0);
                                                    }
                                                }
                                            } else {
                                                ui.label(format!("[file] {}", path));
                                            }
                                        });
                                    }
                                });
                            }
                            BottomTab::ScriptEditor => {
                                let is_modified = self.script_editor_content != self.script_editor_original_content;
                                ui.horizontal(|ui| {
                                    if let Some(path) = &self.script_editor_path {
                                        let title = if is_modified {
                                            format!("* {} [Modified]", path)
                                        } else {
                                            format!("Editing: {}", path)
                                        };
                                        let title_color = if is_modified {
                                            egui::Color32::from_rgb(250, 204, 21)
                                        } else {
                                            ui_theme::COLOR_TEXT_MAIN
                                        };
                                        ui.label(egui::RichText::new(title).strong().color(title_color));
                                        if ui.button("[Save] Save & Hot-Reload").clicked() {
                                            trigger_save_script = true;
                                        }
                                    } else {
                                        ui.label("(No script loaded. Open a .lua script from the Assets Browser or Inspector)");
                                    }
                                });
                                ui.separator();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.horizontal_top(|ui| {
                                        let line_count = self.script_editor_content.lines().count().max(1);
                                        let mut gutter = String::new();
                                        for i in 1..=line_count {
                                            use std::fmt::Write;
                                            let _ = writeln!(gutter, "{:>3}", i);
                                        }
                                        ui.label(
                                            egui::RichText::new(gutter)
                                                .monospace()
                                                .color(ui_theme::COLOR_TEXT_MUTED),
                                        );

                                        ui.add_space(4.0);

                                        ui.add(
                                            egui::TextEdit::multiline(&mut self.script_editor_content)
                                                .font(egui::TextStyle::Monospace)
                                                .code_editor()
                                                .desired_rows(12)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });
                                });
                            }
                        }
                    });

                // Central Panel for remaining screen space
                CentralPanel::default()
                    .frame(Frame::new().fill(egui::Color32::TRANSPARENT))
                    .show(ui, |ui| {
                        let painter = ui.painter();
                        let viewport_rect = ui.max_rect();
                        self.viewport_rect = viewport_rect;

                        let (
                            screen_rect,
                            pointer_pos,
                            is_primary_pressed,
                            is_primary_down,
                            is_primary_released,
                            is_secondary_down,
                            is_middle_down,
                            pointer_delta,
                            scroll_delta_y,
                            is_shift,
                            is_snap,
                        ) = ui.ctx().input(|i| {
                            (
                                i.raw.screen_rect.unwrap_or_else(|| {
                                    let scale = self.window.scale_factor() as f32;
                                    egui::Rect::from_min_size(
                                        egui::Pos2::ZERO,
                                        egui::vec2(
                                            self.gpu.config.width as f32 / scale.max(0.1),
                                            self.gpu.config.height as f32 / scale.max(0.1),
                                        ),
                                    )
                                }),
                                i.pointer.interact_pos().or(i.pointer.hover_pos()),
                                i.pointer.primary_pressed(),
                                i.pointer.primary_down(),
                                i.pointer.primary_released(),
                                i.pointer.secondary_down(),
                                i.pointer.middle_down(),
                                i.pointer.delta(),
                                i.smooth_scroll_delta.y,
                                i.modifiers.shift,
                                i.modifiers.ctrl || self.snap_enabled,
                            )
                        });
                        let screen_size = [screen_rect.width(), screen_rect.height()];

                        // 1. Raycast Selection & Gizmo Interaction
                        let mut hovered_axis = None;
                        let gizmo_len = 1.4f32;

                        if let Some(entity) = self.selected_entity {
                            let transform_data = self.world.get::<&Transform>(entity).map(|t| (t.position, t.rotation, t.scale)).ok();
                            if let Some((pos, rot, scale)) = transform_data {
                                // 1. Selection Outline
                                let corners = [
                                    Vec3::new(-0.5, -0.5, -0.5),
                                    Vec3::new( 0.5, -0.5, -0.5),
                                    Vec3::new( 0.5,  0.5, -0.5),
                                    Vec3::new(-0.5,  0.5, -0.5),
                                    Vec3::new(-0.5, -0.5,  0.5),
                                    Vec3::new( 0.5, -0.5,  0.5),
                                    Vec3::new( 0.5,  0.5,  0.5),
                                    Vec3::new(-0.5,  0.5,  0.5),
                                ];
                                let mat = glam::Mat4::from_scale_rotation_translation(
                                    scale,
                                    Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z),
                                    pos,
                                );
                                let mut screen_corners = [None; 8];
                                for i in 0..8 {
                                    let world_pt = mat.transform_point3(corners[i]);
                                    screen_corners[i] = self.camera.world_to_screen(world_pt, screen_size);
                                }

                                let outline_color = egui::Color32::from_rgb(80, 220, 255);
                                let outline_stroke = egui::Stroke::new(1.5, outline_color);

                                let edges = [
                                    (0, 1), (1, 2), (2, 3), (3, 0),
                                    (4, 5), (5, 6), (6, 7), (7, 4),
                                    (0, 4), (1, 5), (2, 6), (3, 7),
                                ];
                                for (a, b) in edges {
                                    if let (Some(pa), Some(pb)) = (screen_corners[a], screen_corners[b]) {
                                        painter.line_segment([egui::pos2(pa[0], pa[1]), egui::pos2(pb[0], pb[1])], outline_stroke);
                                    }
                                }

                                // 2. 3D Gizmo
                                let origin_pt = self.camera.world_to_screen(pos, screen_size);
                                if let Some(origin) = origin_pt {
                                    let origin_pos = egui::pos2(origin[0], origin[1]);
                                    painter.circle_filled(origin_pos, 4.0, egui::Color32::WHITE);

                                    let pt_x = self.camera.world_to_screen(pos + Vec3::X * gizmo_len, screen_size);
                                    let pt_y = self.camera.world_to_screen(pos + Vec3::Y * gizmo_len, screen_size);
                                    let pt_z = self.camera.world_to_screen(pos + Vec3::Z * gizmo_len, screen_size);

                                    let axes = [
                                        (GizmoAxis::X, pt_x, egui::Color32::from_rgb(235, 60, 60), "X"),
                                        (GizmoAxis::Y, pt_y, egui::Color32::from_rgb(60, 220, 60), "Y"),
                                        (GizmoAxis::Z, pt_z, egui::Color32::from_rgb(60, 130, 245), "Z"),
                                    ];

                                    // Hit test axes with generous 15px margin
                                    if let Some(cursor) = pointer_pos {
                                        if viewport_rect.contains(cursor) || self.active_gizmo_axis.is_some() {
                                            let mut closest_d = 15.0f32;
                                            for (axis, pt_opt, _, _) in &axes {
                                                if let Some(pt) = pt_opt {
                                                    let dist = dist_to_segment_2d([cursor.x, cursor.y], origin, *pt);
                                                    if dist <= closest_d {
                                                        closest_d = dist;
                                                        hovered_axis = Some(*axis);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    for (axis, pt_opt, base_color, label) in axes {
                                        if let Some(pt) = pt_opt {
                                            let end_pos = egui::pos2(pt[0], pt[1]);
                                            let is_active = self.active_gizmo_axis == Some(axis);
                                            let is_hovered = self.active_gizmo_axis.is_none() && hovered_axis == Some(axis);

                                            let color = if is_active || is_hovered {
                                                egui::Color32::from_rgb(255, 240, 50)
                                            } else {
                                                base_color
                                            };

                                            let stroke = egui::Stroke::new(if is_active || is_hovered { 3.5 } else { 2.5 }, color);

                                            match self.gizmo_mode {
                                                GizmoMode::Translate => {
                                                    painter.line_segment([origin_pos, end_pos], stroke);
                                                    painter.circle_filled(end_pos, 6.0, color);
                                                }
                                                GizmoMode::Rotate => {
                                                    painter.line_segment([origin_pos, end_pos], stroke);
                                                    painter.circle_stroke(end_pos, 7.0, stroke);
                                                }
                                                GizmoMode::Scale => {
                                                    painter.line_segment([origin_pos, end_pos], stroke);
                                                    painter.rect_filled(
                                                        egui::Rect::from_center_size(end_pos, egui::vec2(10.0, 10.0)),
                                                        2.0,
                                                        color,
                                                    );
                                                }
                                            }

                                            painter.text(
                                                end_pos + egui::vec2(6.0, -6.0),
                                                egui::Align2::LEFT_BOTTOM,
                                                label,
                                                egui::FontId::monospace(13.0),
                                                color,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        self.hovered_gizmo_axis = hovered_axis;

                        // Click Priority: Gizmo Axis BEFORE Raycast
                        if is_primary_pressed {
                            if let Some(cursor) = pointer_pos {
                                if viewport_rect.contains(cursor) {
                                    if let (Some(entity), Some(axis)) = (self.selected_entity, hovered_axis) {
                                        self.active_gizmo_axis = Some(axis);
                                        self.gizmo_drag_mouse_origin = Some([cursor.x, cursor.y]);
                                        if let Ok(transform) = self.world.get::<&Transform>(entity) {
                                            self.gizmo_drag_start_pos = Some(transform.position);
                                            self.gizmo_drag_start_rot = Some(transform.rotation);
                                            self.gizmo_drag_start_scale = Some(transform.scale);
                                        }
                                    } else if hovered_axis.is_none() {
                                        // Viewport Raycasting selection
                                        let (ray_origin, ray_dir) = self.camera.screen_to_ray([cursor.x, cursor.y], screen_size);
                                        let mut closest_hit: Option<(hecs::Entity, f32)> = None;

                                        for entity_ref in self.world.iter() {
                                            let entity = entity_ref.entity();
                                            if let Some(transform) = entity_ref.get::<&Transform>() {
                                                let half_s = transform.scale.abs() * 0.5;
                                                let min = transform.position - half_s;
                                                let max = transform.position + half_s;

                                                if let Some(t) = camera::ray_intersects_aabb(ray_origin, ray_dir, min, max) {
                                                    if t > 0.0 {
                                                        match closest_hit {
                                                            None => closest_hit = Some((entity, t)),
                                                            Some((_, cur_t)) if t < cur_t => closest_hit = Some((entity, t)),
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        self.selected_entity = closest_hit.map(|(e, _)| e);
                                    }
                                }
                            }
                        }

                        // Real-time Gizmo Dragging & Physics Sync
                        if is_primary_down {
                            if let (Some(entity), Some(axis), Some(start_m), Some(start_pos), Some(start_rot), Some(start_scale)) = (
                                self.selected_entity,
                                self.active_gizmo_axis,
                                self.gizmo_drag_mouse_origin,
                                self.gizmo_drag_start_pos,
                                self.gizmo_drag_start_rot,
                                self.gizmo_drag_start_scale,
                            ) {
                                if let Some(cursor) = pointer_pos {
                                    let axis_vec = match axis {
                                        GizmoAxis::X => Vec3::X,
                                        GizmoAxis::Y => Vec3::Y,
                                        GizmoAxis::Z => Vec3::Z,
                                    };

                                    let origin_2d = self.camera.world_to_screen(start_pos, screen_size);
                                    let tip_2d = self.camera.world_to_screen(start_pos + axis_vec * gizmo_len, screen_size);

                                    if let (Some(o), Some(a)) = (origin_2d, tip_2d) {
                                        let dir_x = a[0] - o[0];
                                        let dir_y = a[1] - o[1];
                                        let len = (dir_x * dir_x + dir_y * dir_y).sqrt().max(1.0);
                                        let ndir_x = dir_x / len;
                                        let ndir_y = dir_y / len;

                                        let total_dx = cursor.x - start_m[0];
                                        let total_dy = cursor.y - start_m[1];
                                        let proj_pixels = total_dx * ndir_x + total_dy * ndir_y;
                                        let delta_world = proj_pixels * (gizmo_len / len);

                                        match self.gizmo_mode {
                                            GizmoMode::Translate => {
                                                let mut new_pos = start_pos + axis_vec * delta_world;
                                                if is_snap {
                                                    new_pos.x = (new_pos.x * 2.0).round() * 0.5;
                                                    new_pos.y = (new_pos.y * 2.0).round() * 0.5;
                                                    new_pos.z = (new_pos.z * 2.0).round() * 0.5;
                                                }
                                                {
                                                    if let Ok(mut transform) = self.world.get::<&mut Transform>(entity) {
                                                        transform.position = new_pos;
                                                    }
                                                }
                                                self.sync_entity_to_physics(entity);
                                            }
                                            GizmoMode::Rotate => {
                                                let angle = (proj_pixels * 0.02) % (std::f32::consts::PI * 2.0);
                                                let final_angle = if is_snap {
                                                    let step = 15.0f32.to_radians();
                                                    (angle / step).round() * step
                                                } else {
                                                    angle
                                                };
                                                let new_rot = start_rot + axis_vec * final_angle;
                                                {
                                                    if let Ok(mut transform) = self.world.get::<&mut Transform>(entity) {
                                                        transform.rotation = new_rot;
                                                    }
                                                }
                                                self.sync_entity_to_physics(entity);
                                            }
                                            GizmoMode::Scale => {
                                                let scale_delta = proj_pixels * 0.015;
                                                let mut new_scale = (start_scale + axis_vec * scale_delta).max(Vec3::splat(0.05));
                                                if is_snap {
                                                    new_scale.x = (new_scale.x * 4.0).round() * 0.25;
                                                    new_scale.y = (new_scale.y * 4.0).round() * 0.25;
                                                    new_scale.z = (new_scale.z * 4.0).round() * 0.25;
                                                }
                                                if let Ok(mut transform) = self.world.get::<&mut Transform>(entity) {
                                                    transform.scale = new_scale;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Release Drag -> Push Undo History
                        if (is_primary_released || !is_primary_down) && self.active_gizmo_axis.is_some() {
                            self.push_history();
                            self.active_gizmo_axis = None;
                            self.gizmo_drag_mouse_origin = None;
                            self.gizmo_drag_start_pos = None;
                            self.gizmo_drag_start_rot = None;
                            self.gizmo_drag_start_scale = None;
                        }

                        // Camera Orbit, Pan & Zoom when not dragging gizmo
                        if self.active_gizmo_axis.is_none() {
                            if let Some(cursor) = pointer_pos {
                                if is_secondary_down || is_middle_down || viewport_rect.contains(cursor) {
                                    if is_secondary_down {
                                        if is_shift {
                                            self.camera.pan(pointer_delta.x, pointer_delta.y);
                                        } else {
                                            self.camera.orbit(pointer_delta.x, pointer_delta.y);
                                        }
                                    } else if is_middle_down {
                                        self.camera.pan(pointer_delta.x, pointer_delta.y);
                                    }

                                    if viewport_rect.contains(cursor) && scroll_delta_y.abs() > 0.0 {
                                        self.camera.zoom(scroll_delta_y * 0.05);
                                    }
                                }
                            }
                        }

                        // 3. Debug Colliders Wireframe (Rapier3D)
                        if self.debug_colliders {
                            let green_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 255, 60));
                            for (_handle, collider) in self.physics.collider_set.iter() {
                                let pos = collider.position();
                                let (rx, ry, rz) = pos.rotation.to_euler(glam::EulerRot::XYZ);
                                let trans_mat = glam::Mat4::from_translation(pos.translation)
                                    * glam::Mat4::from_euler(glam::EulerRot::XYZ, rx, ry, rz);

                                if let Some(cuboid) = collider.shape().as_cuboid() {
                                    let hx = cuboid.half_extents.x;
                                    let hy = cuboid.half_extents.y;
                                    let hz = cuboid.half_extents.z;

                                    let corners = [
                                        Vec3::new(-hx, -hy, -hz),
                                        Vec3::new( hx, -hy, -hz),
                                        Vec3::new( hx,  hy, -hz),
                                        Vec3::new(-hx,  hy, -hz),
                                        Vec3::new(-hx, -hy,  hz),
                                        Vec3::new( hx, -hy,  hz),
                                        Vec3::new( hx,  hy,  hz),
                                        Vec3::new(-hx,  hy,  hz),
                                    ];

                                    let mut scr = [None; 8];
                                    for i in 0..8 {
                                        let wp = trans_mat.transform_point3(corners[i]);
                                        scr[i] = self.camera.world_to_screen(wp, screen_size);
                                    }

                                    let edges = [
                                        (0, 1), (1, 2), (2, 3), (3, 0),
                                        (4, 5), (5, 6), (6, 7), (7, 4),
                                        (0, 4), (1, 5), (2, 6), (3, 7),
                                    ];
                                    for (a, b) in edges {
                                        if let (Some(pa), Some(pb)) = (scr[a], scr[b]) {
                                            painter.line_segment([egui::pos2(pa[0], pa[1]), egui::pos2(pb[0], pb[1])], green_stroke);
                                        }
                                    }
                                } else if let Some(ball) = collider.shape().as_ball() {
                                    let r = ball.radius;
                                    let segs = 16;
                                    for plane in 0..3 {
                                        let mut prev = None;
                                        for i in 0..=segs {
                                            let angle = 2.0 * std::f32::consts::PI * (i as f32 / segs as f32);
                                            let lp = match plane {
                                                0 => Vec3::new(r * angle.cos(), r * angle.sin(), 0.0),
                                                1 => Vec3::new(r * angle.cos(), 0.0, r * angle.sin()),
                                                _ => Vec3::new(0.0, r * angle.cos(), r * angle.sin()),
                                            };
                                            let wp = trans_mat.transform_point3(lp);
                                            if let Some(sp) = self.camera.world_to_screen(wp, screen_size) {
                                                let cur_pos = egui::pos2(sp[0], sp[1]);
                                                if let Some(p) = prev {
                                                    painter.line_segment([p, cur_pos], green_stroke);
                                                }
                                                prev = Some(cur_pos);
                                            } else {
                                                prev = None;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 4. In-Game HUD overlay from Lua (set_hud_text)
                        if let Ok(hud) = self.lua_ctx.hud_texts.read() {
                            if !hud.is_empty() {
                                let hud_area = egui::Area::new(ui.make_persistent_id("lua_hud_overlay"))
                                    .fixed_pos(egui::pos2(270.0, 50.0))
                                    .interactable(false);
                                hud_area.show(ui.ctx(), |ui| {
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_black_alpha(180))
                                        .corner_radius(6.0)
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            for (k, v) in hud.iter() {
                                                ui.colored_label(egui::Color32::from_rgb(100, 220, 255), format!("{}: {}", k, v));
                                            }
                                        });
                                });
                            }
                        }

                        // 5. In-Game Interactive HUD Buttons from Lua (set_hud_button)
                        if let Ok(btns) = self.lua_ctx.hud_buttons.read() {
                            for btn in btns.values() {
                                let btn_area = egui::Area::new(ui.make_persistent_id(format!("hud_btn_ed_{}", btn.id)))
                                    .fixed_pos(egui::pos2(btn.x, btn.y));
                                btn_area.show(ui.ctx(), |ui| {
                                    if ui.button(egui::RichText::new(&btn.text).size(14.0).strong()).clicked() {
                                        if let Ok(mut clicks) = self.lua_ctx.pending_button_clicks.lock() {
                                            clicks.push(btn.callback.clone());
                                        }
                                    }
                                });
                            }
                        }

                        // 5b. Declarative In-Game UI from Lua (game_ui)
                        if self.play_mode == PlayMode::Play {
                            if let Ok(elements) = self.lua_ctx.game_ui_elements.read() {
                                game_ui::render_declarative_ui(ui.ctx(), &painter, viewport_rect, &elements, &self.lua_ctx.pending_button_clicks);
                            }
                        }

                        // 5c. World Labels & Floating Texts (from Lua)
                        if let Ok(labels) = self.lua_ctx.world_labels.read() {
                            for label in labels.values() {
                                let mut found_pos = None;
                                for er in self.world.iter() {
                                    if let Some(meta) = er.get::<&EntityMeta>() {
                                        if meta.id == label.entity_id {
                                            if let Some((_, pos)) = world_transforms.get(&er.entity()) {
                                                found_pos = Some(*pos);
                                            } else if let Some(t) = er.get::<&Transform>() {
                                                found_pos = Some(t.position);
                                            }
                                            break;
                                        }
                                    }
                                }
                                if let Some(pos) = found_pos {
                                    let target = pos + glam::Vec3::new(0.0, label.offset_y, 0.0);
                                    if let Some([sx, sy]) = self.camera.world_to_screen(target, screen_size) {
                                        let p2 = egui::pos2(sx, sy);
                                        if viewport_rect.contains(p2) {
                                            let col = egui::Color32::from_rgb(
                                                (label.color[0] * 255.0) as u8,
                                                (label.color[1] * 255.0) as u8,
                                                (label.color[2] * 255.0) as u8,
                                            );
                                            painter.text(
                                                p2,
                                                egui::Align2::CENTER_CENTER,
                                                &label.text,
                                                egui::FontId::proportional(13.0),
                                                col,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        if let Ok(mut texts) = self.lua_ctx.floating_texts.lock() {
                            texts.retain_mut(|ft| {
                                ft.elapsed += dt;
                                ft.position[1] += dt * 0.8;
                                let alpha = (1.0 - ft.elapsed / ft.lifetime).clamp(0.0, 1.0);
                                if let Some([sx, sy]) = self.camera.world_to_screen(glam::Vec3::from(ft.position), screen_size) {
                                    let p2 = egui::pos2(sx, sy);
                                    if viewport_rect.contains(p2) {
                                        let col = egui::Color32::from_rgba_unmultiplied(
                                            (ft.color[0] * 255.0) as u8,
                                            (ft.color[1] * 255.0) as u8,
                                            (ft.color[2] * 255.0) as u8,
                                            (alpha * 255.0) as u8,
                                        );
                                        painter.text(
                                            p2,
                                            egui::Align2::CENTER_CENTER,
                                            &ft.text,
                                            egui::FontId::proportional(15.0),
                                            col,
                                        );
                                    }
                                }
                                ft.elapsed < ft.lifetime
                            });
                        }

                        let is_play_active = self.play_mode == PlayMode::Play;

                        // 6. Viewport Floating Overlay: Top-Left Capsule (Camera Mode & Colliders Toggle)
                        if !is_play_active {
                            let top_left_area = egui::Area::new(ui.make_persistent_id("viewport_top_left_capsule"))
                                .fixed_pos(egui::pos2(viewport_rect.min.x + 12.0, viewport_rect.min.y + 12.0));
                            top_left_area.show(ui.ctx(), |ui| {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgba_premultiplied(32, 35, 43, 220))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(46, 51, 64, 180)))
                                    .corner_radius(egui::CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let cam_mode_str = match self.camera.mode {
                                                CameraMode::Perspective => "3D",
                                                CameraMode::Orthographic => "2D",
                                            };
                                            if ui.selectable_label(false, cam_mode_str).clicked() {
                                                self.camera.toggle_mode();
                                            }
                                            ui.separator();
                                            ui.checkbox(&mut self.debug_colliders, "Colliders");
                                            ui.separator();
                                            let sky_label = match self.environment_settings.sky_mode.as_str() {
                                                "space" => "Sky: Space",
                                                "solid" => "Sky: Solid",
                                                _ => "Sky: Gradient",
                                            };
                                            egui::ComboBox::from_id_salt("viewport_sky_mode")
                                                .selected_text(sky_label)
                                                .width(95.0)
                                                .show_ui(ui, |ui| {
                                                    if ui.selectable_value(&mut self.environment_settings.sky_mode, "gradient".to_string(), "Gradient").changed() {
                                                        if let Ok(mut lock) = self.lua_ctx.environment_settings.write() {
                                                            *lock = self.environment_settings.clone();
                                                        }
                                                    }
                                                    if ui.selectable_value(&mut self.environment_settings.sky_mode, "space".to_string(), "Space").changed() {
                                                        if let Ok(mut lock) = self.lua_ctx.environment_settings.write() {
                                                            *lock = self.environment_settings.clone();
                                                        }
                                                    }
                                                    if ui.selectable_value(&mut self.environment_settings.sky_mode, "solid".to_string(), "Solid").changed() {
                                                        if let Ok(mut lock) = self.lua_ctx.environment_settings.write() {
                                                            *lock = self.environment_settings.clone();
                                                        }
                                                    }
                                                });
                                            ui.separator();
                                            let view_mode_str = match self.view_mode {
                                                renderer::ViewMode::Lit => "View: Lit",
                                                renderer::ViewMode::Unlit => "View: Unlit",
                                                renderer::ViewMode::Wireframe => "View: Wireframe",
                                            };
                                            egui::ComboBox::from_id_salt("viewport_view_mode")
                                                .selected_text(view_mode_str)
                                                .width(95.0)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut self.view_mode, renderer::ViewMode::Lit, "Lit");
                                                    ui.selectable_value(&mut self.view_mode, renderer::ViewMode::Unlit, "Unlit");
                                                    ui.selectable_value(&mut self.view_mode, renderer::ViewMode::Wireframe, "Wireframe");
                                                });
                                            ui.separator();
                                            if ui.selectable_label(self.show_profiler, "Profiler (F3)").clicked() {
                                                self.show_profiler = !self.show_profiler;
                                            }
                                        });
                                    });
                            });

                            // 7. Viewport Floating Overlay: Top-Right Capsule (FPS, Entity count & Orientation Gizmo)
                            let entity_count = self.world.iter().count();
                            let top_right_area = egui::Area::new(ui.make_persistent_id("viewport_top_right_capsule"))
                                .fixed_pos(egui::pos2((viewport_rect.max.x - 180.0).max(viewport_rect.min.x + 10.0), viewport_rect.min.y + 12.0));
                            top_right_area.show(ui.ctx(), |ui| {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgba_premultiplied(32, 35, 43, 220))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(46, 51, 64, 180)))
                                    .corner_radius(egui::CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{:.0} FPS | {} Ents", fps, entity_count))
                                                    .size(11.5)
                                                    .strong()
                                                    .color(ui_theme::COLOR_TEXT_MAIN),
                                            );
                                            ui.separator();

                                            let (rect, _resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                                            let center = rect.center();
                                            let p = ui.painter();

                                            // Camera orientation projection
                                            let forward = (self.camera.target - self.camera.eye).normalize_or_zero();
                                            let mut right = forward.cross(Vec3::Y);
                                            if right.length_squared() < 0.001 {
                                                right = Vec3::X;
                                            } else {
                                                right = right.normalize();
                                            }
                                            let up = right.cross(forward).normalize();

                                            let axes = [
                                                (Vec3::X, ui_theme::COLOR_AXIS_X, "X"),
                                                (Vec3::Y, ui_theme::COLOR_AXIS_Y, "Y"),
                                                (Vec3::Z, ui_theme::COLOR_AXIS_Z, "Z"),
                                            ];

                                            p.circle_filled(center, 13.0, egui::Color32::from_black_alpha(80));

                                            for (axis_dir, color, label) in &axes {
                                                let sx = axis_dir.dot(right) * 11.0;
                                                let sy = -axis_dir.dot(up) * 11.0;
                                                let tip = egui::pos2(center.x + sx, center.y + sy);
                                                p.line_segment([center, tip], egui::Stroke::new(2.0, *color));
                                                p.text(tip, egui::Align2::CENTER_CENTER, *label, egui::FontId::proportional(8.5), *color);
                                            }
                                        });
                                    });
                            });
                        }

                        // 7b. F3 Profiler Overlay
                        if self.show_profiler {
                            let profiler_area = egui::Area::new(ui.make_persistent_id("viewport_profiler_f3"))
                                .fixed_pos(egui::pos2((viewport_rect.max.x - 240.0).max(viewport_rect.min.x + 10.0), viewport_rect.min.y + 48.0));
                            profiler_area.show(ui.ctx(), |ui| {
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgba_premultiplied(20, 23, 30, 235))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 189, 248)))
                                    .corner_radius(egui::CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("PROFILER (F3)").strong().color(egui::Color32::from_rgb(56, 189, 248)));
                                            let avg_ms = if self.frametime_history.is_empty() {
                                                0.0
                                            } else {
                                                self.frametime_history.iter().sum::<f32>() / self.frametime_history.len() as f32
                                            };
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new(format!("{:.1}ms", avg_ms)).size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                                            });
                                        });
                                        ui.separator();

                                        // Sparkline Graph
                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(210.0, 50.0), egui::Sense::hover());
                                        let painter = ui.painter();
                                        painter.rect_filled(rect, egui::CornerRadius::same(4), egui::Color32::from_rgb(16, 18, 24));
                                        painter.rect_stroke(rect, egui::CornerRadius::same(4), egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(46, 51, 64, 150)), egui::StrokeKind::Inside);

                                        // Target reference lines: 16.6ms (60 FPS) and 33.3ms (30 FPS)
                                        let max_graph_ms = 40.0f32;
                                        let y_60 = rect.max.y - (16.6 / max_graph_ms) * rect.height();
                                        let y_30 = rect.max.y - (33.3 / max_graph_ms) * rect.height();
                                        painter.line_segment([egui::pos2(rect.min.x, y_60), egui::pos2(rect.max.x, y_60)], egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(60, 220, 100, 90)));
                                        painter.line_segment([egui::pos2(rect.min.x, y_30), egui::pos2(rect.max.x, y_30)], egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(240, 120, 40, 80)));
                                        painter.text(egui::pos2(rect.max.x - 2.0, y_60 - 2.0), egui::Align2::RIGHT_BOTTOM, "60 FPS", egui::FontId::proportional(8.0), egui::Color32::from_rgba_unmultiplied(60, 220, 100, 140));

                                        if self.frametime_history.len() >= 2 {
                                            let count = self.frametime_history.len();
                                            let step_x = rect.width() / (count - 1) as f32;
                                            let mut prev_pt: Option<egui::Pos2> = None;
                                            for (i, &ms) in self.frametime_history.iter().enumerate() {
                                                let clamped_ms = ms.clamp(0.0, max_graph_ms);
                                                let px = rect.min.x + i as f32 * step_x;
                                                let py = rect.max.y - (clamped_ms / max_graph_ms) * rect.height();
                                                let cur_pt = egui::pos2(px, py);
                                                if let Some(prev) = prev_pt {
                                                    let col = if ms <= 17.5 {
                                                        egui::Color32::from_rgb(56, 189, 248)
                                                    } else if ms <= 33.3 {
                                                        egui::Color32::from_rgb(250, 204, 21)
                                                    } else {
                                                        egui::Color32::from_rgb(248, 113, 113)
                                                    };
                                                    painter.line_segment([prev, cur_pt], egui::Stroke::new(1.5, col));
                                                }
                                                prev_pt = Some(cur_pt);
                                            }
                                        }

                                        ui.add_space(4.0);

                                        // Stats grid
                                        let bodies_count = self.physics.dynamic_bodies_count() + self.physics.fixed_bodies_count();
                                        let particles_count = self.particles.active_count();
                                        let lights_count = self.world.query::<&crate::ecs::PointLight>().iter().count();

                                        egui::Grid::new("profiler_stats_grid")
                                            .num_columns(2)
                                            .spacing([12.0, 3.0])
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new("FPS / Frametime:").size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                                                ui.label(egui::RichText::new(format!("{:.0} FPS ({:.2} ms)", self.fps, 1000.0 / self.fps.max(1.0))).size(11.0).strong());
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Draw Calls (Batches):").size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                                                ui.label(egui::RichText::new(format!("{}", self.last_draw_calls)).size(11.0).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Entities:").size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                                                ui.label(egui::RichText::new(format!("{}", self.world.len())).size(11.0).strong());
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Physics Bodies:").size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                                                ui.label(egui::RichText::new(format!("{}", bodies_count)).size(11.0).strong());
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Active Particles:").size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                                                ui.label(egui::RichText::new(format!("{}", particles_count)).size(11.0).strong());
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Point Lights:").size(11.0).color(egui::Color32::from_rgb(142, 149, 165)));
                                                ui.label(egui::RichText::new(format!("{}", lights_count)).size(11.0).strong());
                                                ui.end_row();
                                            });
                                    });
                            });
                        }

                        // 8. Viewport Floating Overlay: Bottom-Right Toast Notifications
                        let now = Instant::now();
                        self.toasts.retain(|(_, start, _)| now.duration_since(*start).as_secs_f32() < 2.5);

                        if !self.toasts.is_empty() {
                            let total_h = self.toasts.len() as f32 * 36.0;
                            let toast_area = egui::Area::new(ui.make_persistent_id("viewport_toast_container"))
                                .fixed_pos(egui::pos2((viewport_rect.max.x - 270.0).max(viewport_rect.min.x + 10.0), (viewport_rect.max.y - total_h - 14.0).max(viewport_rect.min.y + 10.0)));
                            toast_area.show(ui.ctx(), |ui| {
                                for (msg, start, color) in &self.toasts {
                                    let elapsed = now.duration_since(*start).as_secs_f32();
                                    let alpha = if elapsed > 2.0 {
                                        ((2.5 - elapsed) / 0.5 * 255.0).clamp(0.0, 255.0) as u8
                                    } else {
                                        255u8
                                    };
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgba_premultiplied(20, 22, 28, alpha))
                                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), alpha)))
                                        .corner_radius(egui::CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(10, 6))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.colored_label(
                                                    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), alpha),
                                                    "[i]",
                                                );
                                                ui.colored_label(
                                                    egui::Color32::from_rgba_premultiplied(230, 232, 238, alpha),
                                                    msg,
                                                );
                                            });
                                        });
                                    ui.add_space(3.0);
                                }
                            });
                        }
                    });

                // Command Palette Ctrl+K detection & Modal Window
                if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) {
                    self.command_palette_open = !self.command_palette_open;
                    if self.command_palette_open {
                        self.command_palette_query.clear();
                    }
                }

                if self.command_palette_open {
                    let mut close_palette = false;
                    let mut chosen_action: Option<&'static str> = None;
                    let screen_rect = ui.ctx().input(|i| i.raw.screen_rect).unwrap_or_else(|| {
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0))
                    });

                    egui::Window::new("command_palette_modal")
                        .fixed_pos(egui::pos2(screen_rect.center().x - 230.0, screen_rect.min.y + 60.0))
                        .fixed_size(egui::vec2(460.0, 320.0))
                        .title_bar(false)
                        .resizable(false)
                        .collapsible(false)
                        .frame(
                            egui::Frame::new()
                                .fill(ui_theme::COLOR_BG_PANEL)
                                .stroke(egui::Stroke::new(1.5, ui_theme::COLOR_ACCENT_PRIMARY))
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(egui::Margin::same(12))
                        )
                        .show(ui.ctx(), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Command Palette").strong().color(ui_theme::COLOR_ACCENT_PRIMARY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("Esc / Close").clicked() {
                                        close_palette = true;
                                    }
                                });
                            });
                            ui.separator();

                            let query_res = ui.add(
                                egui::TextEdit::singleline(&mut self.command_palette_query)
                                    .hint_text("Search actions, entities, primitives... (Esc to close)")
                                    .desired_width(f32::INFINITY)
                            );
                            query_res.request_focus();

                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                close_palette = true;
                            }

                            ui.separator();

                            let query = self.command_palette_query.to_lowercase();
                            egui::ScrollArea::vertical().max_height(230.0).show(ui, |ui| {
                                let quick_actions: &[(&str, &str)] = &[
                                    ("Add Cube Primitive", "cube"),
                                    ("Add Sphere Primitive", "sphere"),
                                    ("Add Cylinder Primitive", "cylinder"),
                                    ("Add Quad Primitive", "quad"),
                                    ("Add Plane Primitive", "plane"),
                                    ("Add Torus Primitive", "torus"),
                                    ("Add Ring Primitive", "ring"),
                                    ("Add Cone Primitive", "cone"),
                                    ("Add Icosphere Primitive", "icosphere"),
                                    ("Play Simulation", "play"),
                                    ("Pause Simulation", "pause"),
                                    ("Stop Simulation", "stop"),
                                    ("Save Scene (Ctrl+S)", "save"),
                                    ("Reload Scene", "reload"),
                                    ("Toggle 2D / 3D Camera", "toggle_cam"),
                                    ("Toggle Colliders Debug", "toggle_col"),
                                    ("Reset Camera View", "reset_cam"),
                                    ("Export Game (Standalone)", "export_game"),
                                ];

                                for (name, action_id) in quick_actions {
                                    if query.is_empty() || name.to_lowercase().contains(&query) {
                                        if ui.button(format!("> {}", name)).clicked() {
                                            chosen_action = Some(action_id);
                                            close_palette = true;
                                        }
                                    }
                                }

                                // Search scene entities
                                if !query.is_empty() {
                                    let mut entity_matches = Vec::new();
                                    for er in self.world.iter() {
                                        if let Some(m) = er.get::<&EntityMeta>() {
                                            if m.id.to_lowercase().contains(&query) {
                                                entity_matches.push((er.entity(), m.id.clone()));
                                            }
                                        }
                                    }

                                    if !entity_matches.is_empty() {
                                        ui.separator();
                                        ui.label(egui::RichText::new("Entities:").color(ui_theme::COLOR_TEXT_MUTED).small());
                                        for (e, id) in entity_matches {
                                            if ui.button(format!("* Select Entity: {}", id)).clicked() {
                                                self.selected_entity = Some(e);
                                                self.add_toast(format!("Selected '{}'", id), egui::Color32::from_rgb(56, 189, 248));
                                                close_palette = true;
                                            }
                                        }
                                    }
                                }
                            });
                        });

                    if let Some(act) = chosen_action {
                        match act {
                            "cube" => trigger_spawn_mesh = Some(("cube", [0.8, 0.4, 0.3, 1.0])),
                            "sphere" => trigger_spawn_mesh = Some(("sphere", [0.3, 0.7, 0.9, 1.0])),
                            "cylinder" => trigger_spawn_mesh = Some(("cylinder", [0.4, 0.8, 0.4, 1.0])),
                            "quad" => trigger_spawn_mesh = Some(("quad", [0.9, 0.8, 0.2, 1.0])),
                            "plane" => trigger_spawn_mesh = Some(("plane", [0.6, 0.6, 0.6, 1.0])),
                            "torus" => trigger_spawn_mesh = Some(("torus", [0.9, 0.4, 0.9, 1.0])),
                            "ring" => trigger_spawn_mesh = Some(("ring", [0.2, 0.9, 0.8, 1.0])),
                            "cone" => trigger_spawn_mesh = Some(("cone", [0.9, 0.6, 0.2, 1.0])),
                            "icosphere" => trigger_spawn_mesh = Some(("icosphere", [0.4, 0.8, 0.9, 1.0])),
                            "play" => {
                                if self.play_mode == PlayMode::Edit {
                                    self.play_snapshot = Some(assets::extract_scene_from_world(&self.world, &self.camera, &self.physics));
                                }
                                self.play_mode = PlayMode::Play;
                                self.add_toast("Play mode active", egui::Color32::from_rgb(34, 197, 94));
                            }
                            "pause" => {
                                self.play_mode = PlayMode::Pause;
                                self.add_toast("Simulation paused", egui::Color32::from_rgb(250, 204, 21));
                            }
                            "stop" => {
                                if let Some(snapshot) = self.play_snapshot.take() {
                                    assets::apply_scene_to_world(&snapshot, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
                                }
                                self.particles.clear();
                                self.lua_ctx.clear_timers_and_events();
                                self.play_mode = PlayMode::Edit;
                                self.add_toast("Simulation stopped & restored", egui::Color32::from_rgb(239, 68, 68));
                            }
                            "save" => trigger_save_scene = true,
                            "reload" => trigger_reload_scene = true,
                            "toggle_cam" => self.camera.toggle_mode(),
                            "toggle_col" => self.debug_colliders = !self.debug_colliders,
                            "reset_cam" => {
                                self.camera.eye = glam::Vec3::new(0.0, 3.0, 7.0);
                                self.camera.target = glam::Vec3::new(0.0, 1.0, 0.0);
                                self.camera.up = glam::Vec3::Y;
                                self.add_toast("Camera view reset", egui::Color32::from_rgb(56, 189, 248));
                            }
                            "export_game" => {
                                self.show_export_modal = true;
                            }
                            _ => {}
                        }
                    }

                    if close_palette {
                        self.command_palette_open = false;
                    }
                }

                // Export Game Modal
                if self.show_export_modal {
                    let screen_rect = ui.ctx().input(|i| i.raw.screen_rect).unwrap_or_else(|| {
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0))
                    });
                    let mut export_trigger = false;
                    let mut close_modal = false;

                    egui::Window::new("Export Game (Standalone)")
                        .fixed_pos(egui::pos2(screen_rect.center().x - 180.0, screen_rect.center().y - 120.0))
                        .fixed_size(egui::vec2(360.0, 220.0))
                        .resizable(false)
                        .collapsible(false)
                        .show(ui.ctx(), |ui| {
                            ui.heading("📦 Standalone Exporter");
                            ui.add_space(8.0);
                            ui.label("Game Title / Executable Name:");
                            ui.text_edit_singleline(&mut self.export_game_name);
                            ui.add_space(6.0);
                            ui.label("Entry Scene Path:");
                            ui.text_edit_singleline(&mut self.export_entry_scene);
                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                if ui.button(egui::RichText::new("🚀 Build & Export").strong()).clicked() {
                                    export_trigger = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    close_modal = true;
                                }
                            });
                        });

                    if close_modal {
                        self.show_export_modal = false;
                    }
                    if export_trigger {
                        match exporter::export_game(&self.export_game_name, &self.export_entry_scene, None) {
                            Ok(path) => {
                                let msg = format!("Game exported to '{}'!", path.display());
                                self.logs.push(msg.clone());
                                self.add_toast(&msg, egui::Color32::from_rgb(56, 189, 248));
                                self.show_export_modal = false;
                            }
                            Err(e) => {
                                let err = format!("Export failed: {}", e);
                                self.logs.push(err.clone());
                                self.add_toast(&err, egui::Color32::from_rgb(239, 68, 68));
                            }
                        }
                    }
                }
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
            self.add_toast("New empty scene created", egui::Color32::from_rgb(250, 204, 21));
        }

        if let Some(template_name) = trigger_template {
            self.push_history();
            match templates::create_template_scene(template_name) {
                Ok(scene) => {
                    self.world.clear();
                    self.physics.clear();
                    self.particles.clear();
                    self.selected_entity = None;
                    assets::apply_scene_to_world(&scene, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.lua_ctx));
                    if let Some(env) = &scene.environment {
                        self.environment_settings = env.clone();
                    }
                    let msg = format!("Loaded template '{}'", template_name);
                    self.logs.push(msg.clone());
                    self.add_toast(&msg, egui::Color32::from_rgb(56, 189, 248));
                }
                Err(e) => {
                    self.logs.push(format!("Error loading template '{}': {}", template_name, e));
                }
            }
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
                self.script_editor_path = Some(path.clone());
                self.script_editor_content = content.clone();
                self.script_editor_original_content = content;
                self.bottom_tab = BottomTab::ScriptEditor;
                self.add_toast(format!("Opened script '{}'", path), egui::Color32::from_rgb(56, 189, 248));
            } else {
                self.logs.push(format!("Could not read script file '{}'", path));
                self.add_toast(format!("Failed to open script '{}'", path), egui::Color32::from_rgb(239, 68, 68));
            }
        }

        if trigger_save_script {
            if let Some(path) = self.script_editor_path.clone() {
                match std::fs::write(&path, &self.script_editor_content) {
                    Ok(()) => {
                        self.script_editor_original_content = self.script_editor_content.clone();
                        let msg = format!("Script '{}' saved and hot-reloaded", path);
                        self.logs.push(msg.clone());
                        self.hot_reload.last_reload_status = Some(msg);
                        self.add_toast("Script saved & hot-reloaded", egui::Color32::from_rgb(34, 197, 94));
                        // Trigger reload on all entities using this script
                        for (meta, script) in self.world.query_mut::<(&EntityMeta, &mut ScriptComponent)>() {
                            if script.path == path {
                                let _ = script.init_or_reload(self.lua_ctx.clone());
                                self.logs.push(format!("Reloaded script for entity '{}'", meta.id));
                            }
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("Error saving script '{}': {}", path, e);
                        self.logs.push(err_msg.clone());
                        self.add_toast(err_msg, egui::Color32::from_rgb(239, 68, 68));
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

        // Update Post-Processing Uniform Buffer
        let pp_uniform = renderer::PostProcessUniform {
            bloom_enabled: if self.post_process_settings.bloom_enabled { 1 } else { 0 },
            bloom_intensity: self.post_process_settings.bloom_intensity,
            bloom_threshold: self.post_process_settings.bloom_threshold,
            vignette_intensity: self.post_process_settings.vignette_intensity,
            exposure: self.post_process_settings.exposure,
            time: self.total_time,
            screen_width: self.gpu.config.width as f32,
            screen_height: self.gpu.config.height as f32,
        };
        self.gpu.queue.write_buffer(
            &self.gpu.post_process.uniform_buffer,
            0,
            bytemuck::cast_slice(&[pp_uniform]),
        );

        // 1. 3D Scene Render Pass (HDR)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3D Scene Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.gpu.post_process.hdr_view,
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

            // 0. Render Procedural Skybox
            render_pass.set_pipeline(&self.gpu.skybox_pipeline);
            render_pass.set_bind_group(0, &self.gpu.camera_bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            // 1a. Render Opaque & Alpha Batches
            render_pass.set_pipeline(&self.gpu.render_pipeline);
            render_pass.set_bind_group(0, &self.gpu.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &self.gpu.lights_bind_group, &[]);

            for ((mesh_id, tex_opt, blend_mode), instances) in &instances_by_batch {
                if blend_mode == "additive" || instances.is_empty() {
                    continue;
                }
                if let Some(mesh_gpu) = self.gpu.meshes.get(mesh_id) {
                    let tex_bind = if let Some(tex) = tex_opt {
                        &self.gpu.get_texture(tex).bind_group
                    } else {
                        &self.gpu.default_white_texture.bind_group
                    };
                    render_pass.set_bind_group(1, tex_bind, &[]);

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

            // 1b. Render Additive Batches
            render_pass.set_pipeline(&self.gpu.additive_render_pipeline);
            render_pass.set_bind_group(0, &self.gpu.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &self.gpu.lights_bind_group, &[]);

            for ((mesh_id, tex_opt, blend_mode), instances) in &instances_by_batch {
                if blend_mode != "additive" || instances.is_empty() {
                    continue;
                }
                if let Some(mesh_gpu) = self.gpu.meshes.get(mesh_id) {
                    let tex_bind = if let Some(tex) = tex_opt {
                        &self.gpu.get_texture(tex).bind_group
                    } else {
                        &self.gpu.default_white_texture.bind_group
                    };
                    render_pass.set_bind_group(1, tex_bind, &[]);

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

            // Render Instanced Particles
            let particle_instances = self.particles.get_instances();
            if !particle_instances.is_empty() {
                self.gpu.ensure_mesh("quad");
                if let Some(mesh_gpu) = self.gpu.meshes.get("quad") {
                    render_pass.set_bind_group(1, &self.gpu.default_white_texture.bind_group, &[]);
                    let instance_buffer = create_instance_buffer(&self.gpu.device, &particle_instances);
                    render_pass.set_vertex_buffer(0, mesh_gpu.vertex_buffer.slice(..));
                    render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                    render_pass.set_index_buffer(
                        mesh_gpu.index_buffer.slice(..),
                        mesh_gpu.index_format,
                    );
                    render_pass.draw_indexed(
                        0..mesh_gpu.num_indices,
                        0,
                        0..particle_instances.len() as u32,
                    );
                }
            }

            // Render Dynamic Trails with Additive Blending
            if !trail_indices.is_empty() {
                let tv_buf = self.gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Trail Vertex Buffer"),
                    contents: bytemuck::cast_slice(&trail_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let ti_buf = self.gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Dynamic Trail Index Buffer"),
                    contents: bytemuck::cast_slice(&trail_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                let id_instance = InstanceRaw {
                    model: glam::Mat4::IDENTITY.to_cols_array_2d(),
                    color: [1.0, 1.0, 1.0, 1.0],
                    extra: [1.0, 0.0, 0.0, 0.0],
                    fresnel: [0.0, 0.0, 0.0, 0.0],
                    uv_offset: [0.0, 0.0],
                    uv_scale: [1.0, 1.0],
                };
                let inst_buf = create_instance_buffer(&self.gpu.device, &[id_instance]);
                render_pass.set_pipeline(&self.gpu.additive_render_pipeline);
                render_pass.set_bind_group(0, &self.gpu.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.gpu.default_white_texture.bind_group, &[]);
                render_pass.set_bind_group(2, &self.gpu.lights_bind_group, &[]);
                render_pass.set_vertex_buffer(0, tv_buf.slice(..));
                render_pass.set_vertex_buffer(1, inst_buf.slice(..));
                render_pass.set_index_buffer(ti_buf.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..trail_indices.len() as u32, 0, 0..1);
            }

            // Render Dynamic Debug Lines & Rings
            if !debug_line_vertices.is_empty() {
                let line_vbuf = self.gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Debug Lines Vertex Buffer"),
                    contents: bytemuck::cast_slice(&debug_line_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                render_pass.set_pipeline(&self.gpu.line_pipeline);
                render_pass.set_bind_group(0, &self.gpu.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, line_vbuf.slice(..));
                render_pass.draw(0..debug_line_vertices.len() as u32, 0..1);
            }

            // Render 3D Grid on Y=0 (editor mode only)
            if !self.is_standalone_play {
                render_pass.set_pipeline(&self.gpu.line_pipeline);
                render_pass.set_bind_group(0, &self.gpu.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.gpu.grid_mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.gpu.grid_mesh.index_buffer.slice(..),
                    self.gpu.grid_mesh.index_format,
                );
                render_pass.draw_indexed(
                    0..self.gpu.grid_mesh.num_indices,
                    0,
                    0..1,
                );
            }
        }

        // 2. Fullscreen HDR Post-Processing Pass (Bloom + ACES Tone Mapping + Vignette)
        {
            let mut pp_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Post Process Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pp_pass.set_pipeline(&self.gpu.post_process.pipeline);
            pp_pass.set_bind_group(0, &self.gpu.post_process.bind_group, &[]);
            pp_pass.draw(0..3, 0..1);
        }

        // 3. Egui Overlay Render Pass
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
        self.window.request_redraw();
    }
}

pub struct App {
    state: Option<EngineState>,
    is_play_mode: bool,
    target_scene: String,
    custom_title: Option<String>,
    custom_size: Option<(u32, u32)>,
}

impl App {
    pub fn new(
        is_play_mode: bool,
        target_scene: String,
        custom_title: Option<String>,
        custom_size: Option<(u32, u32)>,
    ) -> Self {
        Self {
            state: None,
            is_play_mode,
            target_scene,
            custom_title,
            custom_size,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if self.state.is_none() {
            let default_title = if self.is_play_mode {
                "Motor-dif | Game Mode"
            } else {
                "Motor-dif | AI-Native Lightweight 2D/3D Engine (Editor)"
            };
            let title = self.custom_title.as_deref().unwrap_or(default_title);
            let (w, h) = self.custom_size.unwrap_or((1280, 720));
            let window_attributes = Window::default_attributes()
                .with_title(title)
                .with_inner_size(winit::dpi::LogicalSize::new(w as f64, h as f64))
                .with_visible(true)
                .with_active(true)
                .with_resizable(true);
            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create winit window"),
            );
            window.set_visible(true);
            window.set_minimized(false);
            window.focus_window();
            window.request_user_attention(Some(winit::window::UserAttentionType::Critical));
            let state = pollster::block_on(EngineState::new(
                window.clone(),
                self.is_play_mode,
                self.target_scene.clone(),
            ));
            window.set_visible(true);
            window.set_minimized(false);
            window.focus_window();
            window.request_redraw();
            self.state = Some(state);
        } else if let Some(state) = &self.state {
            state.window.set_visible(true);
            state.window.set_minimized(false);
            state.window.focus_window();
            state.window.request_redraw();
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
    let args: Vec<String> = std::env::args().collect();
    let mut is_play_mode = false;
    let mut target_scene = "assets/scene.json".to_string();
    let mut custom_title = None;
    let mut custom_size = None;

    if args.len() <= 1 {
        if let Some(cfg) = exporter::check_auto_boot_game() {
            is_play_mode = true;
            target_scene = cfg.entry_scene;
            custom_title = Some(cfg.title);
            custom_size = Some((cfg.width, cfg.height));
        }
    } else {
        let mut i = 1;
        while i < args.len() {
            if args[i] == "--play" {
                is_play_mode = true;
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    target_scene = args[i + 1].clone();
                    i += 1;
                }
            } else if args[i] == "--export" {
                let name = if i + 1 < args.len() { &args[i + 1] } else { "MyGame" };
                let scene = if i + 2 < args.len() { &args[i + 2] } else { "assets/scene.json" };
                let out = if i + 3 < args.len() { Some(args[i + 3].as_str()) } else { Some("dist") };
                match exporter::export_game(name, scene, out) {
                    Ok(p) => {
                        println!("Standalone game exported successfully to: {:?}", p);
                        return;
                    }
                    Err(e) => {
                        eprintln!("Export error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if args[i] == "--scene" {
                if i + 1 < args.len() {
                    target_scene = args[i + 1].clone();
                    i += 1;
                }
            } else if args[i].ends_with(".json") {
                target_scene = args[i].clone();
            }
            i += 1;
        }
    }

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(is_play_mode, target_scene, custom_title, custom_size);
    event_loop.run_app(&mut app).expect("Error running event loop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_transforms_hierarchy_and_cycle() {
        let mut world = hecs::World::new();
        let parent_e = world.spawn((
            EntityMeta {
                id: "parent".to_string(),
                name: "Parent".to_string(),
                parent: None,
            },
            Transform::from_xyz(10.0, 0.0, 0.0),
        ));

        let child_e = world.spawn((
            EntityMeta {
                id: "child".to_string(),
                name: "Child".to_string(),
                parent: Some("parent".to_string()),
            },
            Transform::from_xyz(0.0, 5.0, 0.0),
        ));

        let transforms = compute_world_transforms(&world);
        let parent_pos = transforms.get(&parent_e).unwrap().1;
        let child_pos = transforms.get(&child_e).unwrap().1;

        assert_eq!(parent_pos, Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(child_pos, Vec3::new(10.0, 5.0, 0.0));

        // Test cycle safety (A -> B -> A)
        let mut world_cycle = hecs::World::new();
        let e_a = world_cycle.spawn((
            EntityMeta {
                id: "A".to_string(),
                name: "A".to_string(),
                parent: Some("B".to_string()),
            },
            Transform::from_xyz(1.0, 0.0, 0.0),
        ));
        let e_b = world_cycle.spawn((
            EntityMeta {
                id: "B".to_string(),
                name: "B".to_string(),
                parent: Some("A".to_string()),
            },
            Transform::from_xyz(0.0, 2.0, 0.0),
        ));

        // Must terminate without stack overflow or hanging!
        let cycle_transforms = compute_world_transforms(&world_cycle);
        assert!(cycle_transforms.contains_key(&e_a));
        assert!(cycle_transforms.contains_key(&e_b));
    }
}
