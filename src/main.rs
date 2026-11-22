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

use assets::{CameraDef, EntityDef, SceneDef};
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
}

impl EngineState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let aspect = size.width as f32 / size.height.max(1) as f32;
        let mut camera = Camera::new_3d(Vec3::new(0.0, 3.5, 9.0), Vec3::ZERO, aspect);

        let mut gpu = GpuContext::new(window.clone()).await;
        gpu.update_camera(&camera);

        // Ensure assets directories exist
        if !Path::new("assets/scripts").exists() {
            let _ = std::fs::create_dir_all("assets/scripts");
        }
        if !Path::new("assets/models").exists() {
            let _ = std::fs::create_dir_all("assets/models");
        }

        let pressed_keys = Arc::new(RwLock::new(HashSet::new()));

        // Initialize world from assets/scene.json or default
        let mut world = hecs::World::new();
        let mut physics = PhysicsEngine::new();
        let mut logs = Vec::new();
        logs.push("Motor-dif engine initialized.".to_string());

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

        // Initialize hot-reload watcher
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

        // Initialize MCP Server with async channel
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let mcp_handle = mcp_server::start_mcp_server(cmd_tx, 3030);
        let mcp_server_addr = mcp_handle.bound_addr.to_string();
        logs.push(format!("MCP Server: running at http://{}/mcp and stdio", mcp_server_addr));

        logs.push("Renderer: wgpu + WGSL directional lighting (Low-PBR).".to_string());
        logs.push("Depth Buffer: 32-bit Float enabled.".to_string());
        logs.push("Texture usage: RENDER_ATTACHMENT | COPY_SRC.".to_string());
        logs.push("Scripting: mlua (Lua 5.4 vendored) ready.".to_string());
        logs.push("Physics: Rapier3D (fixed 60Hz step) active.".to_string());

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
        }
    }

    pub fn handle_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
        let response = self.gpu.egui_winit_state.on_window_event(&self.window, event);
        if response.consumed {
            return;
        }

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

                let is_pressed = *state == winit::event::ElementState::Pressed;
                if let Ok(mut set) = self.pressed_keys.write() {
                    for k in keys_to_update {
                        if is_pressed {
                            set.insert(k);
                        } else {
                            set.remove(&k);
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

    pub fn reload_scene(&mut self) {
        match assets::load_scene_from_file("assets/scene.json") {
            Ok(scene) => {
                assets::apply_scene_to_world(&scene, &mut self.world, &mut self.camera, &mut self.physics, Some(&self.pressed_keys));
                self.hot_reload.last_reload_error = None;
                let msg = format!("Hot-Reload: Scene successfully reloaded ({} entities)", self.world.len());
                self.logs.push(msg.clone());
                self.hot_reload.last_reload_status = Some(msg);
            }
            Err(err) => {
                // Resilience: NEVER crash the engine on malformed JSON or missing path!
                self.hot_reload.last_reload_error = Some(err.clone());
                self.logs.push(format!("Hot-Reload ERROR: {}", err));
            }
        }
    }

    pub fn process_mcp_commands(&mut self) {
        while let Ok(cmd) = self.mcp_cmd_rx.try_recv() {
            match cmd {
                McpCommand::InspectWorld { respond_to } => {
                    let camera_def = CameraDef {
                        mode: match self.camera.mode {
                            CameraMode::Perspective => "3d".to_string(),
                            CameraMode::Orthographic => "2d".to_string(),
                        },
                        position: [self.camera.eye.x, self.camera.eye.y, self.camera.eye.z],
                        target: [self.camera.target.x, self.camera.target.y, self.camera.target.z],
                        ortho_size: self.camera.ortho_size,
                    };

                    let mut entities = Vec::new();
                    for (meta, transform, mesh, script, rb_opt) in self.world.query::<(&EntityMeta, &Transform, &MeshRender, Option<&ScriptComponent>, Option<&RigidBodyComponent>)>().iter() {
                        let phys_def = rb_opt.and_then(|rb| {
                            self.physics.rigid_body_set.get(rb.handle).map(|body| {
                                assets::PhysicsDef {
                                    body: if body.is_dynamic() { "dynamic".to_string() } else { "fixed".to_string() },
                                    collider: "cuboid".to_string(),
                                    mass: body.mass(),
                                    restitution: 0.5,
                                }
                            })
                        });

                        entities.push(EntityDef {
                            id: meta.id.clone(),
                            mesh: mesh.mesh_type.clone(),
                            position: [transform.position.x, transform.position.y, transform.position.z],
                            rotation: [transform.rotation.x, transform.rotation.y, transform.rotation.z],
                            scale: [transform.scale.x, transform.scale.y, transform.scale.z],
                            color: mesh.color,
                            script: script.map(|s| s.path.clone()),
                            physics: phys_def,
                        });
                    }

                    let _ = respond_to.send(WorldInspection {
                        camera: camera_def,
                        entities,
                    });
                }
                McpCommand::UpsertEntity { entity, respond_to } => {
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

        // Calculate smooth FPS
        self.frame_counter += 1;
        if self.fps_timer.elapsed().as_secs_f32() >= 0.5 {
            self.fps = self.frame_counter as f32 / self.fps_timer.elapsed().as_secs_f32();
            self.frame_counter = 0;
            self.fps_timer = Instant::now();
        }

        // Process MCP commands
        self.process_mcp_commands();

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

        // Run Lua scripts for entities with ScriptComponent
        for (meta, transform, script) in self.world.query_mut::<(&EntityMeta, &mut Transform, &mut ScriptComponent)>() {
            if let Err(e) = script.update(transform, dt) {
                let err_msg = format!("Lua runtime error on '{}': {}", meta.id, e);
                if self.last_lua_error.as_ref() != Some(&err_msg) {
                    self.logs.push(err_msg.clone());
                    self.last_lua_error = Some(err_msg);
                }
            }
        }

        // Apply queued forces & impulses from Lua scripts to Physics
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

        // Step Physics simulation and sync with hecs Transform
        self.physics.step(dt);
        self.physics.sync_to_ecs(&mut self.world);

        // Subtle rotation animation in 3D for entities without scripts and without physics
        for (transform, has_script, has_physics) in self.world.query_mut::<(&mut Transform, Option<&ScriptComponent>, Option<&RigidBodyComponent>)>() {
            if has_script.is_none() && has_physics.is_none() {
                transform.rotation.y += 0.8 * dt;
                transform.rotation.x += 0.4 * dt;
            }
        }

        // Update camera in GPU
        self.gpu.update_camera(&self.camera);

        // Collect instances by mesh type
        let mut instances_by_mesh: HashMap<String, Vec<InstanceRaw>> = HashMap::new();
        for (transform, mesh) in self.world.query::<(&Transform, &MeshRender)>().iter() {
            let raw = InstanceRaw {
                model: transform.matrix().to_cols_array_2d(),
                color: mesh.color,
            };
            instances_by_mesh.entry(mesh.mesh_type.clone()).or_default().push(raw);
        }

        // Ensure all meshes requested by entities are loaded in GPU
        for mesh_id in instances_by_mesh.keys() {
            self.gpu.ensure_mesh(mesh_id);
        }

        // Get surface frame
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
        let entity_count = self.world.len();
        let fps = self.fps;
        let logs = &mut self.logs;
        let camera = &mut self.camera;
        let hot_reload_error = self.hot_reload.last_reload_error.clone();
        let hot_reload_status = self.hot_reload.last_reload_status.clone();
        let mcp_addr = self.mcp_server_addr.clone();
        let lua_error = self.last_lua_error.clone();
        let active_keys: Vec<String> = self.pressed_keys.read().unwrap().iter().cloned().collect();
        let physics_enabled = self.physics.enabled;
        let dynamic_count = self.physics.dynamic_bodies_count();
        let fixed_count = self.physics.fixed_bodies_count();

        let mut trigger_manual_reload = false;
        let mut trigger_toggle_physics = false;

        let full_output = self
            .gpu
            .egui_winit_state
            .egui_ctx()
            .run_ui(raw_input, |ui| {
                egui::Window::new("Motor-dif Inspector")
                    .default_pos([16.0, 16.0])
                    .default_size([360.0, 560.0])
                    .resizable(true)
                    .show(ui.ctx(), |ui| {
                        ui.heading("Motor-dif (AI-Native Engine)");
                        ui.label(format!("FPS: {:.1}", fps));
                        ui.label(format!("Entidades activas (hecs): {}", entity_count));
                        ui.separator();

                        // MCP Server Status
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(100, 200, 255), "[OK] Servidor MCP Activo");
                        });
                        ui.monospace(format!("Endpoint: http://{}/mcp", mcp_addr));
                        ui.label("Herramientas: inspect_world, upsert_entity, get_engine_logs, capture_viewport");
                        ui.separator();

                        // Hot-Reload Indicator & Resilience
                        ui.horizontal(|ui| {
                            if let Some(err) = &hot_reload_error {
                                ui.colored_label(egui::Color32::from_rgb(255, 90, 90), "[!] Error en Hot-Reload");
                                ui.label(err);
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(90, 220, 90), "[OK] Hot-Reload Activo");
                                if let Some(status) = &hot_reload_status {
                                    ui.label(status);
                                }
                            }
                        });

                        if ui.button("[R] Forzar Recarga (assets/scene.json)").clicked() {
                            trigger_manual_reload = true;
                        }

                        ui.separator();

                        // Lua Scripting Status
                        ui.horizontal(|ui| {
                            if let Some(err) = &lua_error {
                                ui.colored_label(egui::Color32::from_rgb(255, 90, 90), "[!] Error en Script Lua");
                                ui.label(err);
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(90, 220, 90), "[OK] Scripting Lua Activo (mlua 5.4)");
                            }
                        });
                        if !active_keys.is_empty() {
                            ui.label(format!("Teclas presionadas: {:?}", active_keys));
                        }

                        ui.separator();

                        // Physics Section
                        ui.horizontal(|ui| {
                            if physics_enabled {
                                ui.colored_label(egui::Color32::from_rgb(90, 220, 90), "[OK] Fisicas Activas (Rapier3D 60Hz)");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(255, 180, 50), "[PAUSA] Fisicas Pausadas");
                            }
                        });

                        ui.horizontal(|ui| {
                            let play_label = if physics_enabled {
                                "[||] Pausar Fisicas"
                            } else {
                                "[>] Reanudar Fisicas"
                            };
                            if ui.button(play_label).clicked() {
                                trigger_toggle_physics = true;
                            }
                            if ui.button("[R] Reiniciar Simulacion").clicked() {
                                trigger_manual_reload = true;
                            }
                        });

                        ui.label(format!("Cuerpos: {} dinamicos, {} estaticos", dynamic_count, fixed_count));

                        ui.separator();

                        // Camera Controls
                        ui.horizontal(|ui| {
                            let mode_str = match camera.mode {
                                CameraMode::Perspective => "3D (Perspectiva)",
                                CameraMode::Orthographic => "2D (Ortografica)",
                            };
                            ui.label(format!("Modo de Camara: {}", mode_str));
                        });

                        let toggle_label = match camera.mode {
                            CameraMode::Perspective => "Alternar a 2D (Ortografica)",
                            CameraMode::Orthographic => "Alternar a 3D (Perspectiva)",
                        };
                        if ui.button(toggle_label).clicked() {
                            camera.toggle_mode();
                            logs.push(format!("Camara cambiada a {:?}", camera.mode));
                        }

                        ui.separator();
                        ui.label("Pipeline & Graficos:");
                        ui.label("- Depth Buffer: 32-bit Float");
                        ui.label("- Iluminacion: Direccional ligera WGSL");
                        ui.label("- Textura de render: COPY_SRC habilitado");
                        ui.label("- Vision Bridge: PNG Base64 (256-byte align)");

                        ui.separator();
                        ui.collapsing("Consola de Logs del Motor", |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(140.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for log in logs.iter() {
                                        ui.monospace(log);
                                    }
                                });
                        });
                    });
            });

        if trigger_manual_reload {
            self.reload_scene();
        }

        if trigger_toggle_physics {
            self.physics.enabled = !self.physics.enabled;
            let status = if self.physics.enabled { "Reanudadas" } else { "Pausadas" };
            self.logs.push(format!("Físicas: {}", status));
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

            // Render all mesh instances (cube, quad, external glTF models)
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
                .with_title("Motor-dif | AI-Native Lightweight 2D/3D Engine")
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
