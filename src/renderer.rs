use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::camera::{Camera, CameraUniform};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3,
            2 => Float32x2,
            3 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub extra: [f32; 4], // extra[0] = emissive factor
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
            9 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBS,
        }
    }
}

pub struct MeshGpu {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub index_format: wgpu::IndexFormat,
}

pub struct DepthTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

pub struct TextureGpu {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
}

impl TextureGpu {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self, String> {
        let img = image::load_from_memory(bytes).map_err(|e| format!("Image decode error: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Self::from_rgba(device, queue, layout, &rgba, width, height, label)
    }

    pub fn from_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        rgba: &[u8],
        width: u32,
        height: u32,
        label: &str,
    ) -> Result<Self, String> {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width.max(1)),
                rows_per_image: Some(height.max(1)),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{}_sampler", label)),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some(&format!("{}_bind_group", label)),
        });

        Ok(Self {
            texture,
            view,
            sampler,
            bind_group,
        })
    }

    pub fn create_white_1x1(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let white_pixel = [255u8, 255u8, 255u8, 255u8];
        Self::from_rgba(device, queue, layout, &white_pixel, 1, 1, "default_white_1x1")
            .expect("Failed to create default 1x1 white texture")
    }
}

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub depth_texture: DepthTexture,
    pub render_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub grid_mesh: MeshGpu,
    pub wireframe_cube_mesh: MeshGpu,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub default_white_texture: TextureGpu,
    pub texture_cache: HashMap<String, TextureGpu>,
    pub meshes: HashMap<String, MeshGpu>,
    pub egui_renderer: egui_wgpu::Renderer,
    pub egui_winit_state: egui_winit::State,
}

const SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) vert_color: vec4<f32>,
};

struct InstanceInput {
    @location(4) model_matrix_0: vec4<f32>,
    @location(5) model_matrix_1: vec4<f32>,
    @location(6) model_matrix_2: vec4<f32>,
    @location(7) model_matrix_3: vec4<f32>,
    @location(8) inst_color: vec4<f32>,
    @location(9) inst_extra: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) emissive: f32,
    @location(4) world_pos: vec3<f32>,
};

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_pos = model_matrix * vec4<f32>(vertex.position, 1.0);
    let normal_mat = mat3x3<f32>(
        model_matrix[0].xyz,
        model_matrix[1].xyz,
        model_matrix[2].xyz,
    );
    let world_normal = normalize(normal_mat * vertex.normal);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_pos;
    out.world_normal = world_normal;
    out.color = vertex.vert_color * instance.inst_color;
    out.uv = vertex.uv;
    out.emissive = instance.inst_extra.x;
    out.world_pos = world_pos.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    let base_color = in.color * tex_color;

    if (base_color.a < 0.01) {
        discard;
    }

    // 1. Hemispheric Ambient Lighting (Sky/Ground)
    let sky_ambient = vec3<f32>(0.28, 0.32, 0.40);
    let ground_ambient = vec3<f32>(0.12, 0.13, 0.16);
    let hemi_blend = in.world_normal.y * 0.5 + 0.5;
    let ambient = mix(ground_ambient, sky_ambient, hemi_blend);

    // 2. Directional Diffuse Lighting
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.4));
    let n_dot_l = max(dot(in.world_normal, light_dir), 0.0);
    let light_color = vec3<f32>(1.0, 0.98, 0.92);
    let diffuse = light_color * n_dot_l * 0.70;

    // 3. Blinn-Phong Specular
    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);
    let half_dir = normalize(light_dir + view_dir);
    let n_dot_h = max(dot(in.world_normal, half_dir), 0.0);
    let spec_factor = pow(n_dot_h, 32.0) * step(0.001, n_dot_l) * 0.35;
    let specular = vec3<f32>(1.0, 1.0, 1.0) * spec_factor;

    // 4. Emissive Material Glow
    let emissive = base_color.rgb * in.emissive * 2.0;

    // 5. Combined Lighting
    let lit_color = base_color.rgb * (ambient + diffuse) + specular + emissive;

    // 6. Horizon Gradient & Soft Distance Fog
    let fog_color = vec3<f32>(0.08, 0.09, 0.12);
    let dist = length(camera.camera_pos.xyz - in.world_pos);
    let fog_factor = clamp((dist - 15.0) / 60.0, 0.0, 0.85);
    let final_color = mix(lit_color, fog_color, fog_factor);

    return vec4<f32>(final_color, base_color.a);
}
"#;

const LINE_SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct LineVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct LineVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_line(in: LineVertexInput) -> LineVertexOutput {
    var out: LineVertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_line(in: LineVertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

impl GpuContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();

        // Surface has 'static lifetime since window is Arc<Window>
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower, // Optimized for integrated graphics!
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("Failed to find suitable GPU adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Motor-dif GPU Device"),
                ..Default::default()
            })
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        // CRITICAL: TextureUsages::COPY_SRC enabled for screenshot / vision support!
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let depth_texture =
            DepthTexture::create_depth_texture(&device, config.width, config.height, "depth_texture");

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Standard Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        // Camera Uniform Buffer
        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Texture Bind Group Layout
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let default_white_texture =
            TextureGpu::create_white_1x1(&device, &queue, &texture_bind_group_layout);

        // Ensure procedural test textures exist
        let _ = generate_checker_texture(Path::new("assets/textures/checker.png"));
        let _ = generate_sprite_orb_texture(Path::new("assets/textures/sprite_orb.png"));

        // Pipeline Layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&texture_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc()), Some(InstanceRaw::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Allow transparent 2D sprites/quads to be double-sided
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        // Line pipeline for Grid, Wireframes and Debug Colliders
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader Module"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER_SOURCE.into()),
        });

        let line_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 0,
            });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_line"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_line"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let grid_mesh = create_grid_mesh(&device);
        let wireframe_cube_mesh = create_wireframe_cube_mesh(&device);

        let mut meshes = HashMap::new();
        meshes.insert("cube".to_string(), create_cube_mesh(&device));
        meshes.insert("quad".to_string(), create_quad_mesh(&device));
        meshes.insert("sphere".to_string(), create_sphere_mesh(&device, 16, 24));
        meshes.insert("cylinder".to_string(), create_cylinder_mesh(&device, 24));
        meshes.insert("plane".to_string(), create_plane_mesh(&device));

        // egui integration
        let egui_ctx = egui::Context::default();
        let egui_winit_state = egui_winit::State::new(
            egui_ctx,
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );

        Self {
            instance,
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
            render_pipeline,
            line_pipeline,
            grid_mesh,
            wireframe_cube_mesh,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            default_white_texture,
            texture_cache: HashMap::new(),
            meshes,
            egui_renderer,
            egui_winit_state,
        }
    }

    pub fn get_or_load_texture(&mut self, path: &str) -> &TextureGpu {
        if !self.texture_cache.contains_key(path) {
            match std::fs::read(path) {
                Ok(bytes) => {
                    match TextureGpu::from_bytes(
                        &self.device,
                        &self.queue,
                        &self.texture_bind_group_layout,
                        &bytes,
                        path,
                    ) {
                        Ok(tex) => {
                            self.texture_cache.insert(path.to_string(), tex);
                        }
                        Err(e) => {
                            eprintln!("[Renderer] Error decoding texture '{}': {}. Falling back to default.", path, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[Renderer] Warning: Could not read texture '{}': {}. Falling back to default.", path, e);
                }
            }
        }
        self.texture_cache.get(path).unwrap_or(&self.default_white_texture)
    }

    pub fn get_texture(&self, path: &str) -> &TextureGpu {
        self.texture_cache.get(path).unwrap_or(&self.default_white_texture)
    }

    pub fn ensure_mesh(&mut self, mesh_id: &str) {
        if self.meshes.contains_key(mesh_id) {
            return;
        }
        match mesh_id {
            "cube" => {
                self.meshes.insert("cube".to_string(), create_cube_mesh(&self.device));
            }
            "quad" => {
                self.meshes.insert("quad".to_string(), create_quad_mesh(&self.device));
            }
            "sphere" => {
                self.meshes.insert("sphere".to_string(), create_sphere_mesh(&self.device, 16, 24));
            }
            "cylinder" => {
                self.meshes.insert("cylinder".to_string(), create_cylinder_mesh(&self.device, 24));
            }
            "plane" => {
                self.meshes.insert("plane".to_string(), create_plane_mesh(&self.device));
            }
            _ => {
                match load_gltf_mesh(&self.device, mesh_id) {
                    Ok(mesh) => {
                        self.meshes.insert(mesh_id.to_string(), mesh);
                    }
                    Err(e) => {
                        eprintln!("glTF mesh load error for '{}': {}. Falling back to cube.", mesh_id, e);
                        let fallback = create_cube_mesh(&self.device);
                        self.meshes.insert(mesh_id.to_string(), fallback);
                    }
                }
            }
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = DepthTexture::create_depth_texture(
                &self.device,
                new_size.width,
                new_size.height,
                "depth_texture",
            );
        }
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        let mut uniform = CameraUniform::new();
        uniform.update(camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[uniform]),
        );
    }
}

pub fn create_cube_mesh(device: &wgpu::Device) -> MeshGpu {
    let white = [1.0, 1.0, 1.0, 1.0];
    let vertices = [
        // Front face (+Z)
        Vertex { position: [-0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [0.0, 1.0], color: white },
        Vertex { position: [ 0.5, -0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  0.0,  1.0], uv: [0.0, 0.0], color: white },
        // Back face (-Z)
        Vertex { position: [ 0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [0.0, 1.0], color: white },
        Vertex { position: [-0.5, -0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  0.0, -1.0], uv: [0.0, 0.0], color: white },
        // Top face (+Y)
        Vertex { position: [-0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0], uv: [0.0, 1.0], color: white },
        Vertex { position: [ 0.5,  0.5,  0.5], normal: [ 0.0,  1.0,  0.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [ 0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [-0.5,  0.5, -0.5], normal: [ 0.0,  1.0,  0.0], uv: [0.0, 0.0], color: white },
        // Bottom face (-Y)
        Vertex { position: [-0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0], uv: [0.0, 1.0], color: white },
        Vertex { position: [ 0.5, -0.5, -0.5], normal: [ 0.0, -1.0,  0.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [ 0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [-0.5, -0.5,  0.5], normal: [ 0.0, -1.0,  0.0], uv: [0.0, 0.0], color: white },
        // Right face (+X)
        Vertex { position: [ 0.5, -0.5,  0.5], normal: [ 1.0,  0.0,  0.0], uv: [0.0, 1.0], color: white },
        Vertex { position: [ 0.5, -0.5, -0.5], normal: [ 1.0,  0.0,  0.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [ 0.5,  0.5, -0.5], normal: [ 1.0,  0.0,  0.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [ 0.5,  0.5,  0.5], normal: [ 1.0,  0.0,  0.0], uv: [0.0, 0.0], color: white },
        // Left face (-X)
        Vertex { position: [-0.5, -0.5, -0.5], normal: [-1.0,  0.0,  0.0], uv: [0.0, 1.0], color: white },
        Vertex { position: [-0.5, -0.5,  0.5], normal: [-1.0,  0.0,  0.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [-0.5,  0.5,  0.5], normal: [-1.0,  0.0,  0.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [-0.5,  0.5, -0.5], normal: [-1.0,  0.0,  0.0], uv: [0.0, 0.0], color: white },
    ];

    let indices: [u16; 36] = [
         0,  1,  2,  0,  2,  3, // Front
         4,  5,  6,  4,  6,  7, // Back
         8,  9, 10,  8, 10, 11, // Top
        12, 13, 14, 12, 14, 15, // Bottom
        16, 17, 18, 16, 18, 19, // Right
        20, 21, 22, 20, 22, 23, // Left
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cube Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cube Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint16,
    }
}

pub fn create_quad_mesh(device: &wgpu::Device) -> MeshGpu {
    let white = [1.0, 1.0, 1.0, 1.0];
    let vertices = [
        Vertex { position: [-0.5, -0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0], color: white },
        Vertex { position: [ 0.5, -0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [ 0.5,  0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [-0.5,  0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0], color: white },
    ];

    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Quad Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Quad Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint16,
    }
}

pub fn create_grid_mesh(device: &wgpu::Device) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half_size = 30.0f32;
    let step = 1.0f32;
    let grid_color = [0.28, 0.32, 0.40, 0.4];
    let x_axis_color = [0.95, 0.25, 0.25, 0.9]; // Red X axis
    let z_axis_color = [0.25, 0.50, 0.95, 0.9]; // Blue Z axis

    let mut idx = 0u16;

    let mut z = -half_size;
    while z <= half_size + 0.001 {
        let is_center = z.abs() < 0.001;
        let color = if is_center { x_axis_color } else { grid_color };
        vertices.push(Vertex { position: [-half_size, 0.0, z], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color });
        vertices.push(Vertex { position: [ half_size, 0.0, z], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color });
        indices.push(idx);
        indices.push(idx + 1);
        idx += 2;
        z += step;
    }

    let mut x = -half_size;
    while x <= half_size + 0.001 {
        let is_center = x.abs() < 0.001;
        let color = if is_center { z_axis_color } else { grid_color };
        vertices.push(Vertex { position: [x, 0.0, -half_size], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color });
        vertices.push(Vertex { position: [x, 0.0,  half_size], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color });
        indices.push(idx);
        indices.push(idx + 1);
        idx += 2;
        x += step;
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Grid Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Grid Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint16,
    }
}

pub fn create_wireframe_cube_mesh(device: &wgpu::Device) -> MeshGpu {
    let yellow = [1.0, 0.9, 0.2, 0.9];
    let vertices = [
        Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
        Vertex { position: [ 0.5, -0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
        Vertex { position: [ 0.5,  0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
        Vertex { position: [-0.5,  0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
        Vertex { position: [-0.5, -0.5,  0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
        Vertex { position: [ 0.5, -0.5,  0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
        Vertex { position: [ 0.5,  0.5,  0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
        Vertex { position: [-0.5,  0.5,  0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: yellow },
    ];

    let indices: [u16; 24] = [
        0, 1, 1, 2, 2, 3, 3, 0, // back square
        4, 5, 5, 6, 6, 7, 7, 4, // front square
        0, 4, 1, 5, 2, 6, 3, 7, // connecting edges
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Wireframe Cube Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Wireframe Cube Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint16,
    }
}

pub fn create_sphere_mesh(device: &wgpu::Device, stacks: u32, slices: u32) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let radius = 0.5f32;
    let white = [1.0, 1.0, 1.0, 1.0];

    for i in 0..=stacks {
        let phi = std::f32::consts::PI * (i as f32 / stacks as f32);
        let v = i as f32 / stacks as f32;
        for j in 0..=slices {
            let theta = 2.0 * std::f32::consts::PI * (j as f32 / slices as f32);
            let u = j as f32 / slices as f32;
            let x = radius * phi.sin() * theta.cos();
            let y = radius * phi.cos();
            let z = radius * phi.sin() * theta.sin();
            let normal = [x / radius, y / radius, z / radius];
            vertices.push(Vertex {
                position: [x, y, z],
                normal,
                uv: [u, v],
                color: white,
            });
        }
    }

    for i in 0..stacks {
        for j in 0..slices {
            let first = (i * (slices + 1) + j) as u16;
            let second = first + (slices as u16) + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sphere Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sphere Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint16,
    }
}

pub fn create_cylinder_mesh(device: &wgpu::Device, segments: u32) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let radius = 0.5f32;
    let half_h = 0.5f32;
    let white = [1.0, 1.0, 1.0, 1.0];

    // Side vertices
    for i in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / segments as f32);
        let u = i as f32 / segments as f32;
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let x = radius * cos_t;
        let z = radius * sin_t;
        let normal = [cos_t, 0.0, sin_t];

        // Bottom vertex
        vertices.push(Vertex { position: [x, -half_h, z], normal, uv: [u, 1.0], color: white });
        // Top vertex
        vertices.push(Vertex { position: [x,  half_h, z], normal, uv: [u, 0.0], color: white });
    }

    for i in 0..segments {
        let b0 = (i * 2) as u16;
        let t0 = b0 + 1;
        let b1 = ((i + 1) * 2) as u16;
        let t1 = b1 + 1;

        indices.push(b0);
        indices.push(t0);
        indices.push(b1);

        indices.push(t0);
        indices.push(t1);
        indices.push(b1);
    }

    // Top cap
    let top_center_idx = vertices.len() as u16;
    vertices.push(Vertex { position: [0.0, half_h, 0.0], normal: [0.0, 1.0, 0.0], uv: [0.5, 0.5], color: white });
    let top_start_idx = vertices.len() as u16;
    for i in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / segments as f32);
        let u = (theta.cos() + 1.0) * 0.5;
        let v = (theta.sin() + 1.0) * 0.5;
        vertices.push(Vertex { position: [radius * theta.cos(), half_h, radius * theta.sin()], normal: [0.0, 1.0, 0.0], uv: [u, v], color: white });
    }
    for i in 0..segments {
        indices.push(top_center_idx);
        indices.push(top_start_idx + i as u16);
        indices.push(top_start_idx + i as u16 + 1);
    }

    // Bottom cap
    let bot_center_idx = vertices.len() as u16;
    vertices.push(Vertex { position: [0.0, -half_h, 0.0], normal: [0.0, -1.0, 0.0], uv: [0.5, 0.5], color: white });
    let bot_start_idx = vertices.len() as u16;
    for i in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / segments as f32);
        let u = (theta.cos() + 1.0) * 0.5;
        let v = (theta.sin() + 1.0) * 0.5;
        vertices.push(Vertex { position: [radius * theta.cos(), -half_h, radius * theta.sin()], normal: [0.0, -1.0, 0.0], uv: [u, v], color: white });
    }
    for i in 0..segments {
        indices.push(bot_center_idx);
        indices.push(bot_start_idx + i as u16 + 1);
        indices.push(bot_start_idx + i as u16);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cylinder Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cylinder Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint16,
    }
}

pub fn create_plane_mesh(device: &wgpu::Device) -> MeshGpu {
    let white = [1.0, 1.0, 1.0, 1.0];
    let vertices = [
        Vertex { position: [-0.5, 0.0, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], color: white },
        Vertex { position: [ 0.5, 0.0, -0.5], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0], color: white },
        Vertex { position: [ 0.5, 0.0,  0.5], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0], color: white },
        Vertex { position: [-0.5, 0.0,  0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0], color: white },
    ];

    let indices: [u16; 6] = [0, 2, 1, 0, 3, 2];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Plane Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Plane Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint16,
    }
}

pub fn load_gltf_mesh(device: &wgpu::Device, path: impl AsRef<Path>) -> Result<MeshGpu, String> {
    let p = path.as_ref();
    let (document, buffers, _images) = gltf::import(p)
        .map_err(|e| format!("Failed to import glTF model '{}': {}", p.display(), e))?;

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = if let Some(iter) = reader.read_positions() {
                iter.collect()
            } else {
                continue;
            };

            let normals: Vec<[f32; 3]> = if let Some(iter) = reader.read_normals() {
                iter.collect()
            } else {
                vec![[0.0, 1.0, 0.0]; positions.len()]
            };

            let tex_coords: Vec<[f32; 2]> = if let Some(iter) = reader.read_tex_coords(0) {
                iter.into_f32().collect()
            } else {
                vec![[0.0, 0.0]; positions.len()]
            };

            let colors: Vec<[f32; 4]> = if let Some(iter) = reader.read_colors(0) {
                iter.into_rgba_f32().collect()
            } else {
                vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]
            };

            let base_vertex = vertices.len() as u32;
            for i in 0..positions.len() {
                vertices.push(Vertex {
                    position: positions[i],
                    normal: if i < normals.len() { normals[i] } else { [0.0, 1.0, 0.0] },
                    uv: if i < tex_coords.len() { tex_coords[i] } else { [0.0, 0.0] },
                    color: if i < colors.len() { colors[i] } else { [1.0, 1.0, 1.0, 1.0] },
                });
            }

            if let Some(index_reader) = reader.read_indices() {
                for idx in index_reader.into_u32() {
                    indices.push(base_vertex + idx);
                }
            } else {
                for i in 0..positions.len() as u32 {
                    indices.push(base_vertex + i);
                }
            }
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err(format!("glTF model '{}' contains no vertices or indices", p.display()));
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Vertex Buffer", p.display())),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Index Buffer", p.display())),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Ok(MeshGpu {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
        index_format: wgpu::IndexFormat::Uint32,
    })
}

pub fn generate_checker_texture(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let width = 256u32;
    let height = 256u32;
    let mut img = image::RgbaImage::new(width, height);
    let tile_size = 32u32;

    for y in 0..height {
        for x in 0..width {
            let tx = x / tile_size;
            let ty = y / tile_size;
            let is_even = (tx + ty) % 2 == 0;
            let color = if is_even {
                image::Rgba([215, 222, 235, 255])
            } else {
                image::Rgba([115, 125, 145, 255])
            };
            img.put_pixel(x, y, color);
        }
    }

    img.save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to save checker.png: {}", e))
}

pub fn generate_sprite_orb_texture(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let width = 256u32;
    let height = 256u32;
    let mut img = image::RgbaImage::new(width, height);
    let cx = 128.0f32;
    let cy = 128.0f32;
    let max_radius = 120.0f32;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > max_radius {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            } else {
                let norm = dist / max_radius;
                let alpha = (1.0 - norm.powf(1.6)).clamp(0.0, 1.0);
                let a_byte = (alpha * 255.0) as u8;

                let r = (120.0 * (1.0 - norm * 0.8)) as u8;
                let g = (220.0 * (1.0 - norm * 0.4)) as u8;
                let b = 255u8;
                img.put_pixel(x, y, image::Rgba([r, g, b, a_byte]));
            }
        }
    }

    img.save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to save sprite_orb.png: {}", e))
}

pub fn create_instance_buffer(device: &wgpu::Device, instances: &[InstanceRaw]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(instances),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub fn capture_frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    surface_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> Result<String, String> {
    use base64::Engine;
    use image::{ImageBuffer, ImageFormat, Rgba};
    use std::io::Cursor;

    let unpadded_bytes_per_row = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let bytes_per_row = (unpadded_bytes_per_row + align - 1) & !(align - 1);
    let buffer_size = (bytes_per_row * height) as u64;

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Viewport Capture Staging Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Viewport Capture Encoder"),
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });

    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("GPU poll error: {:?}", e))?;

    rx.recv()
        .map_err(|e| format!("Channel receive error: {:?}", e))?
        .map_err(|e| format!("Buffer map async error: {:?}", e))?;

    let data = buffer_slice
        .get_mapped_range()
        .map_err(|e| format!("Map range error: {:?}", e))?;

    let is_bgra = matches!(
        surface_format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );

    let mut unpadded_bytes = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let row_start = (row * bytes_per_row) as usize;
        let row_end = row_start + (width * 4) as usize;
        let row_slice = &data[row_start..row_end];
        if is_bgra {
            for pixel in row_slice.chunks_exact(4) {
                unpadded_bytes.push(pixel[2]); // R
                unpadded_bytes.push(pixel[1]); // G
                unpadded_bytes.push(pixel[0]); // B
                unpadded_bytes.push(pixel[3]); // A
            }
        } else {
            unpadded_bytes.extend_from_slice(row_slice);
        }
    }
    drop(data);
    staging_buffer.unmap();

    let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, unpadded_bytes)
        .ok_or_else(|| "Failed to construct ImageBuffer from unpadded pixels".to_string())?;

    let mut png_bytes = Cursor::new(Vec::new());
    img.write_to(&mut png_bytes, ImageFormat::Png)
        .map_err(|e| format!("PNG encoding error: {}", e))?;

    let base64_png = base64::engine::general_purpose::STANDARD.encode(png_bytes.into_inner());
    Ok(base64_png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_generate_textures() {
        let checker_path = Path::new("assets/textures/checker.png");
        generate_checker_texture(checker_path).expect("Failed to generate checker.png");
        assert!(checker_path.exists());

        let orb_path = Path::new("assets/textures/sprite_orb.png");
        generate_sprite_orb_texture(orb_path).expect("Failed to generate sprite_orb.png");
        assert!(orb_path.exists());
    }
}
