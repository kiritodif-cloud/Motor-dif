use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use glam::{camera::rh, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::camera::{Camera, CameraUniform};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ShadowUniform {
    pub light_view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4],
    pub light_color: [f32; 4],
}

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
    pub extra: [f32; 4], // [emissive, uv_scroll.x, uv_scroll.y, wireframe_flag]
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub fresnel: [f32; 4], // [fresnel_color.r, fresnel_color.g, fresnel_color.b, fresnel_power]
    pub pbr_params: [f32; 4], // [metallic, roughness, normal_scale, emissive_intensity]
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBS: [wgpu::VertexAttribute; 10] = wgpu::vertex_attr_array![
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
            9 => Float32x4,
            10 => Float32x2,
            11 => Float32x2,
            12 => Float32x4,
            13 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewMode {
    Lit,
    Unlit,
    Wireframe,
}

impl Default for ViewMode {
    fn default() -> Self {
        Self::Lit
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightRaw {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub direction: [f32; 3],
    pub inner_cone: f32,
    pub outer_cone: f32,
    pub light_type: f32, // 0.0 = point, 1.0 = spot
    pub _pad: [f32; 2],
}

impl Default for LightRaw {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            radius: 0.0,
            color: [0.0; 3],
            intensity: 0.0,
            direction: [0.0, -1.0, 0.0],
            inner_cone: 0.0,
            outer_cone: 0.0,
            light_type: 0.0,
            _pad: [0.0; 2],
        }
    }
}

pub type PointLightRaw = LightRaw;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightsUniform {
    pub num_lights: u32,
    pub _pad: [u32; 3],
    pub lights: [LightRaw; 16],
}

impl Default for LightsUniform {
    fn default() -> Self {
        Self {
            num_lights: 0,
            _pad: [0; 3],
            lights: [LightRaw::default(); 16],
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialKey {
    pub diffuse: Option<String>,
    pub normal: Option<String>,
    pub metallic_roughness: Option<String>,
    pub emissive: Option<String>,
}

impl MaterialKey {
    pub fn simple(diffuse: Option<String>) -> Self {
        Self {
            diffuse,
            normal: None,
            metallic_roughness: None,
            emissive: None,
        }
    }
}

pub struct GpuTextureRaw {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl GpuTextureRaw {
    pub fn create_1x1(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pixel: [u8; 4],
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: 1,
            height: 1,
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
            &pixel,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
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
        Self { texture, view, sampler }
    }
}

pub fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    diffuse_view: &wgpu::TextureView,
    diffuse_sampler: &wgpu::Sampler,
    normal_view: &wgpu::TextureView,
    mr_view: &wgpu::TextureView,
    emissive_view: &wgpu::TextureView,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(diffuse_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(diffuse_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(normal_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(mr_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(emissive_view),
            },
        ],
        label: Some(label),
    })
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
        let dummy_normal = GpuTextureRaw::create_1x1(device, queue, [128, 128, 255, 255], "temp_normal");
        let dummy_mr = GpuTextureRaw::create_1x1(device, queue, [255, 128, 0, 255], "temp_mr");
        let dummy_black = GpuTextureRaw::create_1x1(device, queue, [0, 0, 0, 255], "temp_black");
        Self::from_bytes_pbr(
            device, queue, layout, bytes, &dummy_normal.view, &dummy_mr.view, &dummy_black.view, label,
        )
    }

    pub fn from_bytes_pbr(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        bytes: &[u8],
        normal_view: &wgpu::TextureView,
        mr_view: &wgpu::TextureView,
        emissive_view: &wgpu::TextureView,
        label: &str,
    ) -> Result<Self, String> {
        let img = image::load_from_memory(bytes).map_err(|e| format!("Image decode error: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Self::from_rgba_pbr(
            device, queue, layout, &rgba, width, height, normal_view, mr_view, emissive_view, label,
        )
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
        let dummy_normal = GpuTextureRaw::create_1x1(device, queue, [128, 128, 255, 255], "temp_normal");
        let dummy_mr = GpuTextureRaw::create_1x1(device, queue, [255, 128, 0, 255], "temp_mr");
        let dummy_black = GpuTextureRaw::create_1x1(device, queue, [0, 0, 0, 255], "temp_black");
        Self::from_rgba_pbr(
            device, queue, layout, rgba, width, height, &dummy_normal.view, &dummy_mr.view, &dummy_black.view, label,
        )
    }

    pub fn from_rgba_pbr(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        rgba: &[u8],
        width: u32,
        height: u32,
        normal_view: &wgpu::TextureView,
        mr_view: &wgpu::TextureView,
        emissive_view: &wgpu::TextureView,
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
        let is_nearest = label.contains("voxel") || label.contains("pixel") || label.contains("atlas") || label.ends_with("_nearest");
        let filter_mode = if is_nearest {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{}_sampler", label)),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = create_material_bind_group(
            device,
            layout,
            &view,
            &sampler,
            normal_view,
            mr_view,
            emissive_view,
            &format!("{}_bind_group", label),
        );

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
        normal_view: &wgpu::TextureView,
        mr_view: &wgpu::TextureView,
        emissive_view: &wgpu::TextureView,
    ) -> Self {
        let white_pixel = [255u8, 255u8, 255u8, 255u8];
        Self::from_rgba_pbr(
            device, queue, layout, &white_pixel, 1, 1, normal_view, mr_view, emissive_view, "default_white_1x1",
        ).expect("Failed to create default 1x1 white texture")
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
    pub translucent_render_pipeline: wgpu::RenderPipeline,
    pub additive_render_pipeline: wgpu::RenderPipeline,
    pub line_pipeline: wgpu::RenderPipeline,
    pub skybox_pipeline: wgpu::RenderPipeline,
    pub grid_mesh: MeshGpu,
    pub wireframe_cube_mesh: MeshGpu,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pub default_white_texture: TextureGpu,
    pub dummy_normal: GpuTextureRaw,
    pub dummy_mr: GpuTextureRaw,
    pub dummy_black: GpuTextureRaw,
    pub dummy_white: GpuTextureRaw,
    pub material_cache: HashMap<MaterialKey, wgpu::BindGroup>,
    pub lights_buffer: wgpu::Buffer,
    pub lights_bind_group_layout: wgpu::BindGroupLayout,
    pub lights_bind_group: wgpu::BindGroup,
    pub texture_cache: HashMap<String, TextureGpu>,
    pub meshes: HashMap<String, MeshGpu>,
    pub egui_renderer: egui_wgpu::Renderer,
    pub egui_winit_state: egui_winit::State,
    pub post_process: PostProcessPipeline,
    pub shadow_texture: DepthTexture,
    pub shadow_sampler: wgpu::Sampler,
    pub shadow_buffer: wgpu::Buffer,
    pub shadow_bind_group_layout: wgpu::BindGroupLayout,
    pub shadow_bind_group: wgpu::BindGroup,
    pub shadow_camera_bind_group_layout: wgpu::BindGroupLayout,
    pub shadow_camera_bind_group: wgpu::BindGroup,
    pub shadow_pipeline: wgpu::RenderPipeline,
}

const SHADOW_SHADER_SOURCE: &str = r#"
struct ShadowUniform {
    light_view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> shadow_uniform: ShadowUniform;

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
    @location(10) uv_offset: vec2<f32>,
    @location(11) uv_scale: vec2<f32>,
    @location(12) fresnel: vec4<f32>,
    @location(13) pbr_params: vec4<f32>,
};

struct VertexOutputShadow {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_shadow(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutputShadow {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let world_pos = model_matrix * vec4<f32>(vertex.position, 1.0);
    var out: VertexOutputShadow;
    out.clip_position = shadow_uniform.light_view_proj * world_pos;
    out.uv = vertex.uv * instance.uv_scale + instance.uv_offset;
    out.color = vertex.vert_color * instance.inst_color;
    return out;
}

@fragment
fn fs_shadow(in: VertexOutputShadow) {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    let alpha = in.color.a * tex_color.a;
    if (alpha < 0.5) {
        discard;
    }
}
"#;

const SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    fog_color: vec4<f32>,
    zenith_color: vec4<f32>,
    horizon_color: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    fog_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var t_normal: texture_2d<f32>;
@group(1) @binding(3)
var t_metallic_roughness: texture_2d<f32>;
@group(1) @binding(4)
var t_emissive: texture_2d<f32>;

struct LightGpu {
    position: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
    intensity: f32,
    direction: vec3<f32>,
    inner_cone: f32,
    outer_cone: f32,
    light_type: f32,
    _pad0: f32,
    _pad1: f32,
};

struct LightsUniform {
    num_lights: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    lights: array<LightGpu, 16>,
};

@group(2) @binding(0)
var<uniform> lights: LightsUniform;

struct ShadowUniform {
    light_view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
};

@group(3) @binding(0)
var t_shadow: texture_depth_2d;
@group(3) @binding(1)
var s_shadow: sampler_comparison;
@group(3) @binding(2)
var<uniform> shadow_uniform: ShadowUniform;

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
    @location(10) uv_offset: vec2<f32>,
    @location(11) uv_scale: vec2<f32>,
    @location(12) fresnel: vec4<f32>,
    @location(13) pbr_params: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) emissive: f32,
    @location(4) world_pos: vec3<f32>,
    @location(5) fresnel: vec4<f32>,
    @location(6) uv_scroll: vec2<f32>,
    @location(7) view_mode: f32,
    @location(8) shadow_pos: vec4<f32>,
    @location(9) pbr_params: vec4<f32>,
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
    out.uv = vertex.uv * instance.uv_scale + instance.uv_offset;
    out.emissive = instance.inst_extra.x;
    out.world_pos = world_pos.xyz;
    out.fresnel = instance.fresnel;
    out.uv_scroll = vec2<f32>(instance.inst_extra.y, instance.inst_extra.z);
    out.view_mode = instance.inst_extra.w;
    out.shadow_pos = shadow_uniform.light_view_proj * world_pos;
    out.pbr_params = instance.pbr_params;
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    let p2 = fract(p * vec2<f32>(123.34, 456.21));
    return fract(dot(p2, p2 + 45.32));
}

fn micro_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fetch_shadow(shadow_pos: vec4<f32>, n_dot_l: f32) -> f32 {
    let proj_coords = shadow_pos.xyz / shadow_pos.w;
    let uv = vec2<f32>(proj_coords.x * 0.5 + 0.5, 1.0 - (proj_coords.y * 0.5 + 0.5));
    let current_depth = proj_coords.z;

    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || current_depth > 1.0 || current_depth < 0.0) {
        return 1.0;
    }

    let bias = max(0.0035 * (1.0 - n_dot_l), 0.0008);
    let biased_depth = current_depth - bias;

    let texel_size = 1.0 / 2048.0;
    var shadow = 0.0;
    let weights = array<f32, 3>(1.0, 2.0, 1.0);
    var total_weight = 0.0;
    for (var x = -1; x <= 1; x = x + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            let w = weights[x + 1] * weights[y + 1];
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(t_shadow, s_shadow, uv + offset, biased_depth) * w;
            total_weight += w;
        }
    }
    return shadow / total_weight;
}

fn cotangent_frame(N: vec3<f32>, p: vec3<f32>, uv: vec2<f32>) -> mat3x3<f32> {
    let dp1 = dpdx(p);
    let dp2 = dpdy(p);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);

    let dp2perp = cross(dp2, N);
    let dp1perp = cross(N, dp1);

    let T = dp2perp * duv1.x + dp1perp * duv2.x;
    let B = dp2perp * duv1.y + dp1perp * duv2.y;

    let invmax = inverseSqrt(max(dot(T, T), dot(B, B)) + 1e-10);
    return mat3x3<f32>(T * invmax, B * invmax, N);
}

fn perturb_normal(N: vec3<f32>, V: vec3<f32>, uv: vec2<f32>, normal_map_sample: vec3<f32>, scale: f32) -> vec3<f32> {
    var map = normal_map_sample * 2.0 - 1.0;
    map = vec3<f32>(map.x * scale, map.y * scale, map.z);
    map = normalize(map);
    let TBN = cotangent_frame(N, -V, uv);
    return normalize(TBN * map);
}

const PI: f32 = 3.14159265359;

fn distribution_ggx(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    let denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (PI * denom * denom + 1e-5);
}

fn geometry_schlick_ggx(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k + 1e-5);
}

fn geometry_smith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    return geometry_schlick_ggx(NdotV, roughness) * geometry_schlick_ggx(NdotL, roughness);
}

fn fresnel_schlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let scrolled_uv = in.uv + in.uv_scroll * camera.camera_pos.w;
    let tex_color = textureSample(t_diffuse, s_diffuse, scrolled_uv);
    let base_color = in.color * tex_color;

    if (base_color.a < 0.5) {
        discard;
    }

    // View Modes: 0.0 = Lit, 1.0 = Unlit, 2.0 = Wireframe
    if (in.view_mode > 1.5) {
        let grid = abs(fract(in.uv * 8.0 - 0.5) - 0.5) / fwidth(in.uv * 8.0);
        let line = 1.0 - clamp(min(grid.x, grid.y), 0.0, 1.0);
        let wire_col = mix(vec3<f32>(0.07, 0.09, 0.13), vec3<f32>(0.22, 0.75, 0.98), line);
        return vec4<f32>(wire_col, 1.0);
    } else if (in.view_mode > 0.5) {
        return vec4<f32>(base_color.rgb + in.emissive * base_color.rgb * 2.0, base_color.a);
    }

    // Procedural surface micro-detail
    let n1 = micro_noise(in.world_pos.xz * 12.0);
    let n2 = micro_noise(in.world_pos.xz * 32.0);
    let detail_mod = 0.93 + 0.14 * (n1 * 0.7 + n2 * 0.3);
    let albedo = base_color.rgb * detail_mod;

    // Normal mapping
    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);
    let normal_sample = textureSample(t_normal, s_diffuse, scrolled_uv).xyz;
    let N = perturb_normal(in.world_normal, view_dir, scrolled_uv, normal_sample, in.pbr_params.z);

    // PBR Parameters (Metallic, Roughness, Emissive booster)
    let mr_sample = textureSample(t_metallic_roughness, s_diffuse, scrolled_uv);
    let metallic = clamp(mr_sample.b * in.pbr_params.x, 0.0, 1.0);
    let roughness = clamp(mr_sample.g * in.pbr_params.y, 0.04, 1.0);
    let emissive_sample = textureSample(t_emissive, s_diffuse, scrolled_uv).rgb;
    let emissive = base_color.rgb * in.emissive * 2.0 + emissive_sample * in.pbr_params.w;

    let F0 = mix(vec3<f32>(0.04), albedo, metallic);

    // 1. Hemispheric Ambient
    let hemi_factor = clamp(N.y * 0.5 + 0.5, 0.0, 1.0);
    let sky_ambient = mix(vec3<f32>(0.26, 0.30, 0.38), camera.zenith_color.rgb * 0.45, 0.5);
    let ground_ambient = mix(vec3<f32>(0.11, 0.12, 0.14), camera.horizon_color.rgb * 0.25, 0.5);
    let ambient = mix(ground_ambient, sky_ambient, hemi_factor) * albedo * (1.0 - metallic * 0.6);

    // 2. Directional Sun/Light with PCF 3x3 Shadows & PBR Cook-Torrance
    var light_dir = shadow_uniform.light_dir.xyz;
    if (dot(light_dir, light_dir) < 0.001) {
        light_dir = normalize(vec3<f32>(0.5, 0.8, 0.4));
    } else {
        light_dir = normalize(light_dir);
    }
    let light_color = shadow_uniform.light_color.rgb;
    let light_intensity = max(shadow_uniform.light_color.w, 0.01);
    let NdotL = max(dot(N, light_dir), 0.0);
    let shadow_factor = fetch_shadow(in.shadow_pos, NdotL);

    var direct_light = vec3<f32>(0.0);
    if (NdotL > 0.0) {
        let H = normalize(light_dir + view_dir);
        let NdotV = max(dot(N, view_dir), 0.001);
        let NDF = distribution_ggx(N, H, roughness);
        let G = geometry_smith(N, view_dir, light_dir, roughness);
        let F = fresnel_schlick(max(dot(H, view_dir), 0.0), F0);

        let kS = F;
        let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);
        let numerator = NDF * G * F;
        let denominator = 4.0 * NdotV * NdotL + 0.0001;
        let specular = numerator / denominator;

        direct_light = (kD * albedo / PI + specular) * light_color * (light_intensity * NdotL * shadow_factor);
    }

    // 3. Dynamic Lights (Point & Spot, up to 16)
    var point_lights_contrib = vec3<f32>(0.0);
    let count = min(lights.num_lights, 16u);
    for (var i = 0u; i < count; i = i + 1u) {
        let l = lights.lights[i];
        let light_vec = l.position - in.world_pos;
        let d = length(light_vec);
        if (d < l.radius && d > 0.0001) {
            let l_dir = light_vec / d;
            let norm_d = clamp(d / max(l.radius, 0.001), 0.0, 1.0);
            var atten = (1.0 - norm_d * norm_d) * (1.0 - norm_d * norm_d) * l.intensity;

            // Spot light cone
            if (l.light_type > 0.5) {
                let spot_dir = normalize(l.direction);
                let cos_angle = dot(-l_dir, spot_dir);
                let spot_atten = clamp((cos_angle - l.outer_cone) / max(l.inner_cone - l.outer_cone, 0.001), 0.0, 1.0);
                atten *= spot_atten * spot_atten;
            }

            let l_NdotL = max(dot(N, l_dir), 0.0);
            if (l_NdotL > 0.0 && atten > 0.0001) {
                let H = normalize(l_dir + view_dir);
                let NdotV = max(dot(N, view_dir), 0.001);
                let NDF = distribution_ggx(N, H, roughness);
                let G = geometry_smith(N, view_dir, l_dir, roughness);
                let F = fresnel_schlick(max(dot(H, view_dir), 0.0), F0);

                let kS = F;
                let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);
                let numerator = NDF * G * F;
                let denominator = 4.0 * NdotV * l_NdotL + 0.0001;
                let specular = numerator / denominator;

                point_lights_contrib += (kD * albedo / PI + specular) * l.color * (atten * l_NdotL);
            }
        }
    }

    // 4. Fresnel Rim Glow
    var fresnel_contrib = vec3<f32>(0.0);
    if (in.fresnel.w > 0.0) {
        let n_dot_v = max(dot(N, view_dir), 0.0);
        let rim = pow(1.0 - n_dot_v, in.fresnel.w);
        fresnel_contrib = in.fresnel.rgb * rim;
    }

    // 5. Total Lit Color
    let lit_color = ambient + direct_light + point_lights_contrib + emissive + fresnel_contrib;

    // 6. Atmospheric Fog (Exponential or Linear)
    let dist = length(camera.camera_pos.xyz - in.world_pos);
    let fog_density = camera.fog_color.a;
    var final_color = lit_color;
    if (fog_density > 0.0001) {
        var fog_factor = 0.0;
        if (camera.fog_params.x > 0.5) {
            let f_start = camera.fog_params.y;
            let f_end = max(camera.fog_params.z, f_start + 0.1);
            fog_factor = clamp((dist - f_start) / (f_end - f_start), 0.0, 1.0);
        } else {
            fog_factor = clamp(1.0 - exp(-dist * fog_density), 0.0, 1.0);
        }
        final_color = mix(lit_color, camera.fog_color.rgb, fog_factor);
    }

    return vec4<f32>(final_color, base_color.a);
}

@fragment
fn fs_translucent(in: VertexOutput) -> @location(0) vec4<f32> {
    let scrolled_uv = in.uv + in.uv_scroll * camera.camera_pos.w;
    let tex_color = textureSample(t_diffuse, s_diffuse, scrolled_uv);
    let base_color = in.color * tex_color;

    if (base_color.a < 0.01) {
        discard;
    }

    let emissive = base_color.rgb * in.emissive * 2.0;

    // View Modes: 0.0 = Lit, 1.0 = Unlit, 2.0 = Wireframe
    if (in.view_mode > 1.5) {
        let grid = abs(fract(in.uv * 8.0 - 0.5) - 0.5) / fwidth(in.uv * 8.0);
        let line = 1.0 - clamp(min(grid.x, grid.y), 0.0, 1.0);
        let wire_col = mix(vec3<f32>(0.07, 0.09, 0.13), vec3<f32>(0.22, 0.75, 0.98), line);
        return vec4<f32>(wire_col, base_color.a);
    } else if (in.view_mode > 0.5) {
        return vec4<f32>(base_color.rgb + emissive, base_color.a);
    }

    let hemi_factor = clamp(in.world_normal.y * 0.5 + 0.5, 0.0, 1.0);
    let sky_ambient = mix(vec3<f32>(0.26, 0.30, 0.38), camera.zenith_color.rgb * 0.45, 0.5);
    let ground_ambient = mix(vec3<f32>(0.11, 0.12, 0.14), camera.horizon_color.rgb * 0.25, 0.5);
    let ambient = mix(ground_ambient, sky_ambient, hemi_factor);

    var light_dir = shadow_uniform.light_dir.xyz;
    if (dot(light_dir, light_dir) < 0.001) {
        light_dir = normalize(vec3<f32>(0.5, 0.8, 0.4));
    } else {
        light_dir = normalize(light_dir);
    }
    let light_color = shadow_uniform.light_color.rgb;
    let light_intensity = max(shadow_uniform.light_color.w, 0.01);
    let n_dot_l = max(dot(in.world_normal, light_dir), 0.0);
    let shadow_factor = fetch_shadow(in.shadow_pos, n_dot_l);
    let diffuse = light_color * n_dot_l * (0.70 * light_intensity) * shadow_factor;

    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);
    let half_dir = normalize(light_dir + view_dir);
    let n_dot_h = max(dot(in.world_normal, half_dir), 0.0);
    let spec_factor = pow(n_dot_h, 32.0) * step(0.001, n_dot_l) * (0.40 * light_intensity) * shadow_factor;
    let specular = light_color * spec_factor;

    var fresnel_contrib = vec3<f32>(0.0);
    if (in.fresnel.w > 0.0) {
        let n_dot_v = max(dot(in.world_normal, view_dir), 0.0);
        let rim = pow(1.0 - n_dot_v, in.fresnel.w);
        fresnel_contrib = in.fresnel.rgb * rim;
    }

    let lit_color = base_color.rgb * (ambient + diffuse) + specular + emissive + fresnel_contrib;

    let dist = length(camera.camera_pos.xyz - in.world_pos);
    let fog_density = camera.fog_color.a;
    var final_color = lit_color;
    if (fog_density > 0.0001) {
        var fog_factor = 0.0;
        if (camera.fog_params.x > 0.5) {
            let f_start = camera.fog_params.y;
            let f_end = max(camera.fog_params.z, f_start + 0.1);
            fog_factor = clamp((dist - f_start) / (f_end - f_start), 0.0, 1.0);
        } else {
            fog_factor = clamp(1.0 - exp(-dist * fog_density), 0.0, 1.0);
        }
        final_color = mix(lit_color, camera.fog_color.rgb, fog_factor);
    }

    return vec4<f32>(final_color, base_color.a);
}
"#;

const LINE_SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    fog_color: vec4<f32>,
    zenith_color: vec4<f32>,
    horizon_color: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    fog_params: vec4<f32>,
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

pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub fn create_hdr_texture(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("HDR Render Target"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PostProcessUniform {
    pub bloom_enabled: u32,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub vignette_intensity: f32,
    pub exposure: f32,
    pub time: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub chromatic_aberration: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl Default for PostProcessUniform {
    fn default() -> Self {
        Self {
            bloom_enabled: 1,
            bloom_intensity: 0.8,
            bloom_threshold: 0.8,
            vignette_intensity: 0.35,
            exposure: 1.0,
            time: 0.0,
            screen_width: 1280.0,
            screen_height: 720.0,
            chromatic_aberration: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

pub struct PostProcessPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,
    pub hdr_texture: wgpu::Texture,
    pub hdr_view: wgpu::TextureView,
}

const POST_PROCESS_SHADER_SOURCE: &str = r#"
struct PostProcessUniform {
    bloom_enabled: u32,
    bloom_intensity: f32,
    bloom_threshold: f32,
    vignette_intensity: f32,
    exposure: f32,
    time: f32,
    screen_width: f32,
    screen_height: f32,
    chromatic_aberration: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;
@group(0) @binding(1)
var hdr_sampler: sampler;
@group(0) @binding(2)
var<uniform> pp: PostProcessUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_post_process(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    out.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn aces_filmic_saturated(color: vec3<f32>) -> vec3<f32> {
    let per_channel = aces_filmic(color);
    let peak = max(color.r, max(color.g, color.b));
    if (peak <= 0.0001) {
        return vec3<f32>(0.0);
    }
    let mapped_peak = (peak * (2.51 * peak + 0.03)) / (peak * (2.43 * peak + 0.59) + 0.14);
    let luma_based = clamp(color * (mapped_peak / peak), vec3<f32>(0.0), vec3<f32>(1.0));
    return mix(per_channel, luma_based, 0.65);
}

@fragment
fn fs_post_process(in: VertexOutput) -> @location(0) vec4<f32> {
    var base_color = textureSample(hdr_texture, hdr_sampler, in.uv).rgb;

    // Dynamic Chromatic Aberration
    if (pp.chromatic_aberration > 0.0) {
        let dir = in.uv - vec2<f32>(0.5, 0.5);
        let dist = length(dir);
        let ca_offset = dir * (dist * pp.chromatic_aberration * 0.05);
        let col_r = textureSample(hdr_texture, hdr_sampler, in.uv - ca_offset).r;
        let col_b = textureSample(hdr_texture, hdr_sampler, in.uv + ca_offset).b;
        base_color = vec3<f32>(col_r, base_color.g, col_b);
    }

    var final_color = base_color * pp.exposure;

    if (pp.bloom_enabled != 0u && pp.bloom_intensity > 0.0) {
        let tex_offset = vec2<f32>(1.5 / max(pp.screen_width, 1.0), 1.5 / max(pp.screen_height, 1.0));
        var bloom_sum = vec3<f32>(0.0);

        // 9-tap kernel sampling with Gaussian weights
        // Center
        let c_bright = max(base_color - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0));
        bloom_sum += c_bright * 0.25;

        // Cross
        let c_up = textureSample(hdr_texture, hdr_sampler, in.uv + vec2<f32>(0.0, tex_offset.y)).rgb;
        let c_dn = textureSample(hdr_texture, hdr_sampler, in.uv - vec2<f32>(0.0, tex_offset.y)).rgb;
        let c_lf = textureSample(hdr_texture, hdr_sampler, in.uv - vec2<f32>(tex_offset.x, 0.0)).rgb;
        let c_rt = textureSample(hdr_texture, hdr_sampler, in.uv + vec2<f32>(tex_offset.x, 0.0)).rgb;

        bloom_sum += max(c_up - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.125;
        bloom_sum += max(c_dn - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.125;
        bloom_sum += max(c_lf - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.125;
        bloom_sum += max(c_rt - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.125;

        // Diagonal taps with wider spread for soft bloom halo
        let diag_offset = tex_offset * 2.0;
        let c_ul = textureSample(hdr_texture, hdr_sampler, in.uv + vec2<f32>(-diag_offset.x, diag_offset.y)).rgb;
        let c_ur = textureSample(hdr_texture, hdr_sampler, in.uv + vec2<f32>(diag_offset.x, diag_offset.y)).rgb;
        let c_dl = textureSample(hdr_texture, hdr_sampler, in.uv + vec2<f32>(-diag_offset.x, -diag_offset.y)).rgb;
        let c_dr = textureSample(hdr_texture, hdr_sampler, in.uv + vec2<f32>(diag_offset.x, -diag_offset.y)).rgb;

        bloom_sum += max(c_ul - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.0625;
        bloom_sum += max(c_ur - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.0625;
        bloom_sum += max(c_dl - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.0625;
        bloom_sum += max(c_dr - vec3<f32>(pp.bloom_threshold), vec3<f32>(0.0)) * 0.0625;

        final_color += bloom_sum * pp.bloom_intensity;
    }

    // ACES Filmic Tone Mapping with Emissive Saturation Preservation
    final_color = aces_filmic_saturated(final_color);

    // Subtle Vignette
    if (pp.vignette_intensity > 0.0) {
        let dist = distance(in.uv, vec2<f32>(0.5, 0.5));
        let vig = clamp(1.0 - smoothstep(0.2, 0.8, dist) * pp.vignette_intensity, 0.0, 1.0);
        final_color *= vig;
    }

    return vec4<f32>(final_color, 1.0);
}
"#;

const SKYBOX_SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    fog_color: vec4<f32>,
    zenith_color: vec4<f32>,
    horizon_color: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    fog_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct SkyVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) in_vertex_index: u32) -> SkyVertexOutput {
    var out: SkyVertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32(in_vertex_index & 2u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.999999, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

fn hash31(p: vec3<f32>) -> f32 {
    let p3 = fract(p * 0.1031);
    let d = dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * d);
}

@fragment
fn fs_sky(in: SkyVertexOutput) -> @location(0) vec4<f32> {
    let ndc = vec4<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0, 1.0, 1.0);
    let unprojected = camera.inv_view_proj * ndc;
    let ray_dir = normalize(unprojected.xyz / unprojected.w);

    let sky_mode = i32(camera.zenith_color.w + 0.5);
    let time = camera.camera_pos.w;

    if (sky_mode == 0) {
        // Solid color
        return vec4<f32>(camera.horizon_color.rgb, 1.0);
    } else if (sky_mode == 1) {
        // Smooth zenith / horizon gradient with sun/moon and stars
        let h = clamp(ray_dir.y * 0.5 + 0.5, 0.0, 1.0);
        let grad = mix(camera.horizon_color.rgb, camera.zenith_color.rgb, pow(h, 0.7));
        var color = grad;

        let sun_dir = normalize(camera.sun_dir.xyz);
        let sun_intensity = camera.sun_dir.w;
        let sun_dot = max(dot(ray_dir, sun_dir), 0.0);
        let sun_disk = pow(sun_dot, 800.0) * 4.0 * sun_intensity;
        let sun_glow = pow(sun_dot, 16.0) * 0.6 * sun_intensity;
        let sun_color = camera.sun_color.rgb;

        let moon_dir = -sun_dir;
        let moon_dot = max(dot(ray_dir, moon_dir), 0.0);
        let night_factor = 1.0 - clamp(sun_intensity * 1.5, 0.0, 1.0);
        let moon_disk = pow(moon_dot, 900.0) * 2.5 * night_factor;
        let moon_glow = pow(moon_dot, 20.0) * 0.3 * night_factor;
        let moon_color = vec3<f32>(0.75, 0.85, 1.0);

        let stars_vis = camera.sun_color.w;
        if (stars_vis > 0.01) {
            let star_cell = floor(ray_dir * 180.0);
            let star_rand = hash31(star_cell);
            if (star_rand > 0.985) {
                let star_pos = (star_cell + 0.5) / 180.0;
                let dist_to_center = length(ray_dir - normalize(star_pos)) * 180.0;
                let twinkle = 0.6 + 0.4 * sin(time * 3.5 + star_rand * 62.8);
                let intensity = exp(-dist_to_center * dist_to_center * 16.0) * twinkle * stars_vis;
                color += vec3<f32>(0.9, 0.95, 1.0) * intensity * 1.8;
            }
        }

        color += (sun_disk + sun_glow) * sun_color + (moon_disk + moon_glow) * moon_color;
        return vec4<f32>(color, 1.0);
    } else {
        // Space with 3D starfield & faint nebula
        let h = clamp(ray_dir.y * 0.5 + 0.5, 0.0, 1.0);
        let nebula_freq = ray_dir * 3.0;
        let nebula = sin(nebula_freq.x + time * 0.05) * cos(nebula_freq.y) * sin(nebula_freq.z);
        let cosmic_base = mix(camera.horizon_color.rgb, camera.zenith_color.rgb, h);
        var color = cosmic_base + vec3<f32>(0.02, 0.04, 0.08) * max(nebula, 0.0);

        let star_cell = floor(ray_dir * 180.0);
        let star_rand = hash31(star_cell);
        if (star_rand > 0.982) {
            let star_pos = (star_cell + 0.5) / 180.0;
            let dist_to_center = length(ray_dir - normalize(star_pos)) * 180.0;
            let star_size = (star_rand - 0.982) / 0.018;
            let twinkle = 0.6 + 0.4 * sin(time * 3.5 + star_rand * 62.8);
            let intensity = exp(-dist_to_center * dist_to_center * 16.0) * star_size * twinkle;
            let star_tint = mix(vec3<f32>(0.8, 0.9, 1.0), vec3<f32>(1.0, 0.9, 0.7), fract(star_rand * 17.0));
            color += star_tint * intensity * 2.5;
        }

        return vec4<f32>(color, 1.0);
    }
}
"#;

impl PostProcessPipeline {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let (hdr_texture, hdr_view) = create_hdr_texture(device, width, height);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("HDR Post Process Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let uniform = PostProcessUniform {
            screen_width: width as f32,
            screen_height: height as f32,
            ..Default::default()
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Post Process Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Post Process Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Process Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post Process Shader"),
            source: wgpu::ShaderSource::Wgsl(POST_PROCESS_SHADER_SOURCE.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Post Process Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Post Process Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_post_process"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_post_process"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            sampler,
            hdr_texture,
            hdr_view,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (hdr_texture, hdr_view) = create_hdr_texture(device, width, height);
        self.hdr_texture = hdr_texture;
        self.hdr_view = hdr_view;
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Process Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        });
    }
}

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

        // Texture Bind Group Layout (5 slots: Diffuse, Sampler, Normal, Metallic-Roughness, Emissive)
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
            });

        let dummy_normal = GpuTextureRaw::create_1x1(&device, &queue, [128, 128, 255, 255], "dummy_normal_1x1");
        let dummy_mr = GpuTextureRaw::create_1x1(&device, &queue, [255, 128, 0, 255], "dummy_mr_1x1");
        let dummy_black = GpuTextureRaw::create_1x1(&device, &queue, [0, 0, 0, 255], "dummy_black_1x1");
        let dummy_white = GpuTextureRaw::create_1x1(&device, &queue, [255, 255, 255, 255], "dummy_white_1x1");

        let default_white_texture = TextureGpu::create_white_1x1(
            &device,
            &queue,
            &texture_bind_group_layout,
            &dummy_normal.view,
            &dummy_mr.view,
            &dummy_black.view,
        );

        // Ensure procedural test textures exist
        let _ = generate_checker_texture(Path::new("assets/textures/checker.png"));
        let _ = generate_sprite_orb_texture(Path::new("assets/textures/sprite_orb.png"));
        let _ = generate_test_spritesheet(Path::new("assets/textures/spritesheet_test.png"));

        // Point Lights Uniform Buffer
        let lights_uniform = LightsUniform::default();
        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lights Buffer"),
            contents: bytemuck::cast_slice(&[lights_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lights_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("lights_bind_group_layout"),
            });

        let lights_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &lights_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_buffer.as_entire_binding(),
            }],
            label: Some("lights_bind_group"),
        });

        // Shadow Mapping Resources
        let shadow_texture = DepthTexture::create_depth_texture(&device, 2048, 2048, "shadow_depth_texture");

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_uniform = ShadowUniform {
            light_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.5, 0.8, 0.4, 0.0],
            light_color: [1.0, 0.98, 0.92, 1.0],
        };
        let shadow_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_buffer"),
            contents: bytemuck::cast_slice(&[shadow_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shadow_camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("shadow_camera_bind_group_layout"),
            });

        let shadow_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &shadow_camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_buffer.as_entire_binding(),
            }],
            label: Some("shadow_camera_bind_group"),
        });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("shadow_bind_group_layout"),
            });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: shadow_buffer.as_entire_binding(),
                },
            ],
            label: Some("shadow_bind_group"),
        });

        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader Module"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER_SOURCE.into()),
        });

        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[
                Some(&shadow_camera_bind_group_layout),
                Some(&texture_bind_group_layout),
            ],
            immediate_size: 0,
        });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Render Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_shadow"),
                buffers: &[Some(Vertex::desc()), Some(InstanceRaw::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shadow_shader,
                entry_point: Some("fs_shadow"),
                targets: &[],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Pipeline Layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&texture_bind_group_layout),
                    Some(&lights_bind_group_layout),
                    Some(&shadow_bind_group_layout),
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
                    format: HDR_FORMAT,
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

        let translucent_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D Translucent Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc()), Some(InstanceRaw::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_translucent"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
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

        let additive_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D Additive Render Pipeline"),
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
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
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
                    format: HDR_FORMAT,
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

        // Skybox Procedural Render Pipeline
        let sky_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skybox Shader Module"),
            source: wgpu::ShaderSource::Wgsl(SKYBOX_SHADER_SOURCE.into()),
        });

        let skybox_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skybox Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_bind_group_layout)],
            immediate_size: 0,
        });

        let skybox_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox Render Pipeline"),
            layout: Some(&skybox_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sky_shader,
                entry_point: Some("vs_sky"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sky_shader,
                entry_point: Some("fs_sky"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
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
            multisample: wgpu::MultisampleState::default(),
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
        meshes.insert("torus".to_string(), create_torus_mesh(&device, 0.8, 0.25, 20, 32));
        meshes.insert("ring".to_string(), create_ring_mesh(&device, 0.5, 1.0, 32));
        meshes.insert("cone".to_string(), create_cone_mesh(&device, 0.8, 1.6, 24));
        meshes.insert("icosphere".to_string(), create_icosphere_mesh(&device, 1.0, 2));
        meshes.insert("basalt_pillar".to_string(), create_basalt_pillar_mesh(&device));
        meshes.insert("jagged_rock".to_string(), create_jagged_rock_mesh(&device));
        meshes.insert("crystal_cluster".to_string(), create_crystal_cluster_mesh(&device));
        meshes.insert("sci_fi_crate".to_string(), create_sci_fi_crate_mesh(&device));
        meshes.insert("sword".to_string(), create_sword_mesh(&device));
        meshes.insert("pickaxe".to_string(), create_pickaxe_mesh(&device));

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

        let post_process = PostProcessPipeline::new(&device, config.format, config.width, config.height);

        Self {
            instance,
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
            render_pipeline,
            translucent_render_pipeline,
            additive_render_pipeline,
            line_pipeline,
            skybox_pipeline,
            grid_mesh,
            wireframe_cube_mesh,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            default_white_texture,
            dummy_normal,
            dummy_mr,
            dummy_black,
            dummy_white,
            material_cache: HashMap::new(),
            lights_buffer,
            lights_bind_group_layout,
            lights_bind_group,
            texture_cache: HashMap::new(),
            meshes,
            egui_renderer,
            egui_winit_state,
            post_process,
            shadow_texture,
            shadow_sampler,
            shadow_buffer,
            shadow_bind_group_layout,
            shadow_bind_group,
            shadow_camera_bind_group_layout,
            shadow_camera_bind_group,
            shadow_pipeline,
        }
    }

    pub fn update_shadow_camera(&self, camera_target: Vec3, light_dir: Vec3, light_color: Vec3, light_intensity: f32) {
        let norm_light_dir = if light_dir.length_squared() > 0.001 {
            light_dir.normalize()
        } else {
            Vec3::new(0.5, 0.8, 0.4).normalize()
        };
        let target_xz = Vec3::new(camera_target.x, 0.0, camera_target.z);
        let light_pos = target_xz + norm_light_dir * 50.0;
        let view = rh::view::look_at_mat4(light_pos, target_xz, Vec3::Y);
        let proj = rh::proj::directx::orthographic(-45.0, 45.0, -45.0, 45.0, 1.0, 120.0);
        let light_view_proj = (proj * view).to_cols_array_2d();

        let uniform = ShadowUniform {
            light_view_proj,
            light_dir: [norm_light_dir.x, norm_light_dir.y, norm_light_dir.z, 0.0],
            light_color: [light_color.x, light_color.y, light_color.z, light_intensity],
        };
        self.queue.write_buffer(&self.shadow_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn get_or_load_texture(&mut self, path: &str) -> &TextureGpu {
        if !self.texture_cache.contains_key(path) {
            match std::fs::read(path) {
                Ok(bytes) => {
                    match TextureGpu::from_bytes_pbr(
                        &self.device,
                        &self.queue,
                        &self.texture_bind_group_layout,
                        &bytes,
                        &self.dummy_normal.view,
                        &self.dummy_mr.view,
                        &self.dummy_black.view,
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

    pub fn get_or_create_material(&mut self, key: &MaterialKey) -> &wgpu::BindGroup {
        if key.normal.is_none() && key.metallic_roughness.is_none() && key.emissive.is_none() {
            if let Some(diffuse) = &key.diffuse {
                return &self.get_or_load_texture(diffuse).bind_group;
            } else {
                return &self.default_white_texture.bind_group;
            }
        }

        if !self.material_cache.contains_key(key) {
            if let Some(diffuse) = &key.diffuse {
                let _ = self.get_or_load_texture(diffuse);
            }
            if let Some(normal) = &key.normal {
                let _ = self.get_or_load_texture(normal);
            }
            if let Some(mr) = &key.metallic_roughness {
                let _ = self.get_or_load_texture(mr);
            }
            if let Some(emissive) = &key.emissive {
                let _ = self.get_or_load_texture(emissive);
            }

            let (diffuse_view, diffuse_sampler) = if let Some(diffuse) = &key.diffuse {
                let tex = self.get_texture(diffuse);
                (&tex.view, &tex.sampler)
            } else {
                (&self.dummy_white.view, &self.dummy_white.sampler)
            };

            let normal_view = if let Some(normal) = &key.normal {
                &self.get_texture(normal).view
            } else {
                &self.dummy_normal.view
            };

            let mr_view = if let Some(mr) = &key.metallic_roughness {
                &self.get_texture(mr).view
            } else {
                &self.dummy_mr.view
            };

            let emissive_view = if let Some(emissive) = &key.emissive {
                &self.get_texture(emissive).view
            } else {
                &self.dummy_black.view
            };

            let bind_group = create_material_bind_group(
                &self.device,
                &self.texture_bind_group_layout,
                diffuse_view,
                diffuse_sampler,
                normal_view,
                mr_view,
                emissive_view,
                "pbr_material_bind_group",
            );
            self.material_cache.insert(key.clone(), bind_group);
        }

        self.material_cache.get(key).unwrap()
    }

    pub fn get_material(&self, key: &MaterialKey) -> &wgpu::BindGroup {
        if key.normal.is_none() && key.metallic_roughness.is_none() && key.emissive.is_none() {
            if let Some(diffuse) = &key.diffuse {
                return &self.get_texture(diffuse).bind_group;
            } else {
                return &self.default_white_texture.bind_group;
            }
        }
        self.material_cache.get(key).unwrap_or(&self.default_white_texture.bind_group)
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
            "torus" => {
                self.meshes.insert("torus".to_string(), create_torus_mesh(&self.device, 0.8, 0.25, 20, 32));
            }
            "ring" => {
                self.meshes.insert("ring".to_string(), create_ring_mesh(&self.device, 0.5, 1.0, 32));
            }
            "cone" => {
                self.meshes.insert("cone".to_string(), create_cone_mesh(&self.device, 0.8, 1.6, 24));
            }
            "icosphere" => {
                self.meshes.insert("icosphere".to_string(), create_icosphere_mesh(&self.device, 1.0, 2));
            }
            "basalt_pillar" => {
                self.meshes.insert("basalt_pillar".to_string(), create_basalt_pillar_mesh(&self.device));
            }
            "jagged_rock" => {
                self.meshes.insert("jagged_rock".to_string(), create_jagged_rock_mesh(&self.device));
            }
            "crystal_cluster" => {
                self.meshes.insert("crystal_cluster".to_string(), create_crystal_cluster_mesh(&self.device));
            }
            "sci_fi_crate" => {
                self.meshes.insert("sci_fi_crate".to_string(), create_sci_fi_crate_mesh(&self.device));
            }
            "sword" => {
                self.meshes.insert("sword".to_string(), create_sword_mesh(&self.device));
            }
            "pickaxe" => {
                self.meshes.insert("pickaxe".to_string(), create_pickaxe_mesh(&self.device));
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

    pub fn insert_forge_mesh(&mut self, mesh_id: &str, mesh: &crate::modeler::ForgeMesh) {
        let vertices: Vec<Vertex> = (0..mesh.positions.len())
            .map(|i| Vertex {
                position: mesh.positions[i],
                normal: mesh.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
                uv: mesh.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
                color: mesh.colors.get(i).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]),
            })
            .collect();
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Forge Mesh Vertex Buffer: {}", mesh_id)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Forge Mesh Index Buffer: {}", mesh_id)),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.meshes.insert(
            mesh_id.to_string(),
            MeshGpu {
                vertex_buffer,
                index_buffer,
                num_indices: mesh.indices.len() as u32,
                index_format: wgpu::IndexFormat::Uint32,
            },
        );
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
            self.post_process.resize(&self.device, new_size.width, new_size.height);
        }
    }

    pub fn update_camera(&mut self, camera: &Camera) {
        self.update_camera_with_time(camera, 1.0);
    }

    pub fn update_camera_with_time(&mut self, camera: &Camera, time: f32) {
        let mut uniform = CameraUniform::new();
        uniform.update_with_time(camera, time);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[uniform]),
        );
    }

    pub fn update_camera_with_environment(
        &mut self,
        camera: &Camera,
        time: f32,
        sky_mode: &str,
        zenith: [f32; 3],
        horizon: [f32; 3],
        fog_color: [f32; 3],
        fog_density: f32,
        sun_dir: [f32; 3],
        sun_color: [f32; 3],
        sun_intensity: f32,
        stars_visibility: f32,
    ) {
        self.update_camera_with_environment_fog(
            camera,
            time,
            sky_mode,
            zenith,
            horizon,
            fog_color,
            fog_density,
            "exponential",
            0.0,
            100.0,
            sun_dir,
            sun_color,
            sun_intensity,
            stars_visibility,
        );
    }

    pub fn update_camera_with_environment_fog(
        &mut self,
        camera: &Camera,
        time: f32,
        sky_mode: &str,
        zenith: [f32; 3],
        horizon: [f32; 3],
        fog_color: [f32; 3],
        fog_density: f32,
        fog_mode: &str,
        fog_start: f32,
        fog_end: f32,
        sun_dir: [f32; 3],
        sun_color: [f32; 3],
        sun_intensity: f32,
        stars_visibility: f32,
    ) {
        let mut uniform = CameraUniform::new();
        uniform.update_with_environment_fog(
            camera,
            time,
            sky_mode,
            zenith,
            horizon,
            fog_color,
            fog_density,
            fog_mode,
            fog_start,
            fog_end,
            sun_dir,
            sun_color,
            sun_intensity,
            stars_visibility,
        );
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[uniform]),
        );
    }

    pub fn update_lights(&self, lights: &LightsUniform) {
        self.queue.write_buffer(
            &self.lights_buffer,
            0,
            bytemuck::cast_slice(&[*lights]),
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

pub fn create_torus_mesh(device: &wgpu::Device, major_radius: f32, minor_radius: f32, radial_segments: usize, tubular_segments: usize) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let white = [1.0, 1.0, 1.0, 1.0];

    for j in 0..=radial_segments {
        let v = j as f32 / radial_segments as f32;
        let theta = v * 2.0 * std::f32::consts::PI;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        for i in 0..=tubular_segments {
            let u = i as f32 / tubular_segments as f32;
            let phi = u * 2.0 * std::f32::consts::PI;
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            let x = (major_radius + minor_radius * cos_phi) * cos_theta;
            let y = minor_radius * sin_phi;
            let z = (major_radius + minor_radius * cos_phi) * sin_theta;

            let nx = cos_phi * cos_theta;
            let ny = sin_phi;
            let nz = cos_phi * sin_theta;

            vertices.push(Vertex {
                position: [x, y, z],
                normal: [nx, ny, nz],
                uv: [u, v],
                color: white,
            });
        }
    }

    for j in 0..radial_segments {
        for i in 0..tubular_segments {
            let first = (j * (tubular_segments + 1) + i) as u16;
            let second = ((j + 1) * (tubular_segments + 1) + i) as u16;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Torus Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Torus Index Buffer"),
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

pub fn create_ring_mesh(device: &wgpu::Device, inner_radius: f32, outer_radius: f32, segments: usize) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let white = [1.0, 1.0, 1.0, 1.0];

    for i in 0..=segments {
        let u = i as f32 / segments as f32;
        let theta = u * 2.0 * std::f32::consts::PI;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        vertices.push(Vertex {
            position: [inner_radius * cos_t, 0.0, inner_radius * sin_t],
            normal: [0.0, 1.0, 0.0],
            uv: [u, 0.0],
            color: white,
        });

        vertices.push(Vertex {
            position: [outer_radius * cos_t, 0.0, outer_radius * sin_t],
            normal: [0.0, 1.0, 0.0],
            uv: [u, 1.0],
            color: white,
        });
    }

    for i in 0..segments {
        let in0 = (i * 2) as u16;
        let out0 = in0 + 1;
        let in1 = ((i + 1) * 2) as u16;
        let out1 = in1 + 1;

        indices.push(in0);
        indices.push(out0);
        indices.push(in1);

        indices.push(out0);
        indices.push(out1);
        indices.push(in1);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Ring Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Ring Index Buffer"),
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

pub fn create_cone_mesh(device: &wgpu::Device, radius: f32, height: f32, segments: usize) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let white = [1.0, 1.0, 1.0, 1.0];
    let half_h = height * 0.5;

    let tip_pos = [0.0, half_h, 0.0];
    let slope = (radius / height).atan();
    let cos_slope = slope.cos();
    let sin_slope = slope.sin();

    for i in 0..=segments {
        let u = i as f32 / segments as f32;
        let theta = u * 2.0 * std::f32::consts::PI;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let norm = [cos_t * cos_slope, sin_slope, sin_t * cos_slope];

        vertices.push(Vertex {
            position: tip_pos,
            normal: norm,
            uv: [u, 0.0],
            color: white,
        });

        vertices.push(Vertex {
            position: [radius * cos_t, -half_h, radius * sin_t],
            normal: norm,
            uv: [u, 1.0],
            color: white,
        });
    }

    for i in 0..segments {
        let t0 = (i * 2) as u16;
        let b0 = t0 + 1;
        let b1 = ((i + 1) * 2 + 1) as u16;

        indices.push(t0);
        indices.push(b0);
        indices.push(b1);
    }

    let base_center_idx = vertices.len() as u16;
    vertices.push(Vertex {
        position: [0.0, -half_h, 0.0],
        normal: [0.0, -1.0, 0.0],
        uv: [0.5, 0.5],
        color: white,
    });
    let base_start_idx = vertices.len() as u16;
    for i in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * (i as f32 / segments as f32);
        let u = (theta.cos() + 1.0) * 0.5;
        let v = (theta.sin() + 1.0) * 0.5;
        vertices.push(Vertex {
            position: [radius * theta.cos(), -half_h, radius * theta.sin()],
            normal: [0.0, -1.0, 0.0],
            uv: [u, v],
            color: white,
        });
    }
    for i in 0..segments {
        indices.push(base_center_idx);
        indices.push(base_start_idx + i as u16 + 1);
        indices.push(base_start_idx + i as u16);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cone Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cone Index Buffer"),
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

pub fn create_icosphere_mesh(device: &wgpu::Device, radius: f32, subdivisions: usize) -> MeshGpu {
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;
    let white = [1.0, 1.0, 1.0, 1.0];

    let mut positions = vec![
        [-1.0,  t,  0.0],
        [ 1.0,  t,  0.0],
        [-1.0, -t,  0.0],
        [ 1.0, -t,  0.0],
        [ 0.0, -1.0,  t],
        [ 0.0,  1.0,  t],
        [ 0.0, -1.0, -t],
        [ 0.0,  1.0, -t],
        [  t,  0.0, -1.0],
        [  t,  0.0,  1.0],
        [ -t,  0.0, -1.0],
        [ -t,  0.0,  1.0],
    ];

    for p in &mut positions {
        let len = (p[0]*p[0] + p[1]*p[1] + p[2]*p[2]).sqrt();
        p[0] /= len;
        p[1] /= len;
        p[2] /= len;
    }

    let mut faces = vec![
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];

    let mut midpoint_cache: std::collections::HashMap<(u16, u16), u16> = std::collections::HashMap::new();

    let mut get_midpoint = |p1: u16, p2: u16, positions: &mut Vec<[f32; 3]>| -> u16 {
        let key = if p1 < p2 { (p1, p2) } else { (p2, p1) };
        if let Some(&idx) = midpoint_cache.get(&key) {
            return idx;
        }
        let pos1 = positions[p1 as usize];
        let pos2 = positions[p2 as usize];
        let mid = [
            (pos1[0] + pos2[0]) * 0.5,
            (pos1[1] + pos2[1]) * 0.5,
            (pos1[2] + pos2[2]) * 0.5,
        ];
        let len = (mid[0]*mid[0] + mid[1]*mid[1] + mid[2]*mid[2]).sqrt();
        let normalized = [mid[0] / len, mid[1] / len, mid[2] / len];
        let new_idx = positions.len() as u16;
        positions.push(normalized);
        midpoint_cache.insert(key, new_idx);
        new_idx
    };

    for _ in 0..subdivisions.min(4) {
        let mut new_faces = Vec::new();
        for tri in &faces {
            let a = tri[0] as u16;
            let b = tri[1] as u16;
            let c = tri[2] as u16;

            let ab = get_midpoint(a, b, &mut positions);
            let bc = get_midpoint(b, c, &mut positions);
            let ca = get_midpoint(c, a, &mut positions);

            new_faces.push([a as usize, ab as usize, ca as usize]);
            new_faces.push([b as usize, bc as usize, ab as usize]);
            new_faces.push([c as usize, ca as usize, bc as usize]);
            new_faces.push([ab as usize, bc as usize, ca as usize]);
        }
        faces = new_faces;
    }

    let mut vertices = Vec::with_capacity(positions.len());
    for p in &positions {
        let nx = p[0];
        let ny = p[1];
        let nz = p[2];
        let u = 0.5 + nz.atan2(nx) / (2.0 * std::f32::consts::PI);
        let v = 0.5 - ny.asin() / std::f32::consts::PI;
        vertices.push(Vertex {
            position: [nx * radius, ny * radius, nz * radius],
            normal: [nx, ny, nz],
            uv: [u, v],
            color: white,
        });
    }

    let mut indices = Vec::with_capacity(faces.len() * 3);
    for f in faces {
        indices.push(f[0] as u16);
        indices.push(f[1] as u16);
        indices.push(f[2] as u16);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Icosphere Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Icosphere Index Buffer"),
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

fn add_flat_triangle(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    uv0: [f32; 2],
    uv1: [f32; 2],
    uv2: [f32; 2],
    color: [f32; 4],
) {
    let edge1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let edge2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let cx = edge1[1] * edge2[2] - edge1[2] * edge2[1];
    let cy = edge1[2] * edge2[0] - edge1[0] * edge2[2];
    let cz = edge1[0] * edge2[1] - edge1[1] * edge2[0];
    let len = (cx * cx + cy * cy + cz * cz).sqrt().max(1e-6);
    let normal = [cx / len, cy / len, cz / len];

    let base_idx = vertices.len() as u16;
    vertices.push(Vertex { position: p0, normal, uv: uv0, color });
    vertices.push(Vertex { position: p1, normal, uv: uv1, color });
    vertices.push(Vertex { position: p2, normal, uv: uv2, color });
    indices.push(base_idx);
    indices.push(base_idx + 1);
    indices.push(base_idx + 2);
}

fn add_flat_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    p3: [f32; 3],
    color: [f32; 4],
) {
    add_flat_triangle(vertices, indices, p0, p1, p2, [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], color);
    add_flat_triangle(vertices, indices, p0, p2, p3, [0.0, 0.0], [1.0, 1.0], [0.0, 1.0], color);
}

pub fn create_basalt_pillar_mesh(device: &wgpu::Device) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let white = [1.0, 1.0, 1.0, 1.0];
    let sides = 6;
    let r_bot = 0.65f32;
    let r_top = 0.55f32;
    let h = 3.2f32;
    let cap_h = 3.35f32;

    let mut bot_pts = Vec::new();
    let mut top_pts = Vec::new();
    for i in 0..sides {
        let angle = i as f32 * std::f32::consts::TAU / sides as f32;
        let c = angle.cos();
        let s = angle.sin();
        bot_pts.push([r_bot * c, 0.0, r_bot * s]);
        top_pts.push([r_top * c, h, r_top * s]);
    }

    for i in 0..sides {
        let next = (i + 1) % sides;
        add_flat_quad(&mut vertices, &mut indices, bot_pts[i], bot_pts[next], top_pts[next], top_pts[i], white);
    }

    let apex = [0.0, cap_h, 0.0];
    for i in 0..sides {
        let next = (i + 1) % sides;
        add_flat_triangle(&mut vertices, &mut indices, top_pts[i], top_pts[next], apex, [0.5, 0.5], [1.0, 0.5], [0.5, 1.0], white);
    }

    let base_center = [0.0, 0.0, 0.0];
    for i in 0..sides {
        let next = (i + 1) % sides;
        add_flat_triangle(&mut vertices, &mut indices, bot_pts[next], bot_pts[i], base_center, [0.5, 0.5], [1.0, 0.5], [0.5, 1.0], white);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Basalt Pillar Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Basalt Pillar Index Buffer"),
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

pub fn create_jagged_rock_mesh(device: &wgpu::Device) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let white = [1.0, 1.0, 1.0, 1.0];
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;

    let mut base_pts = vec![
        [-1.0,  t,  0.0], [ 1.0,  t,  0.0], [-1.0, -t,  0.0], [ 1.0, -t,  0.0],
        [ 0.0, -1.0,  t], [ 0.0,  1.0,  t], [ 0.0, -1.0, -t], [ 0.0,  1.0, -t],
        [  t,  0.0, -1.0], [  t,  0.0,  1.0], [ -t,  0.0, -1.0], [ -t,  0.0,  1.0],
    ];

    for (i, p) in base_pts.iter_mut().enumerate() {
        let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-6);
        let scale = 1.0 + 0.35 * ((i as f32 * 4.67 + 1.2).sin());
        p[0] = (p[0] / len) * scale * 1.1;
        p[1] = ((p[1] / len) * scale * 0.9 + 0.6).max(0.0);
        p[2] = (p[2] / len) * scale * 1.1;
    }

    let faces: [[usize; 3]; 20] = [
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];

    for f in faces {
        add_flat_triangle(&mut vertices, &mut indices, base_pts[f[0]], base_pts[f[1]], base_pts[f[2]], [0.0, 0.0], [1.0, 0.0], [0.5, 1.0], white);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Jagged Rock Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Jagged Rock Index Buffer"),
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

pub fn create_crystal_cluster_mesh(device: &wgpu::Device) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let white = [1.0, 1.0, 1.0, 1.0];

    let mut add_shard = |center: [f32; 3], tilt_axis: [f32; 3], tilt_angle_deg: f32, radius: f32, height: f32| {
        let sides = 6;
        let rad = tilt_angle_deg.to_radians();
        let sin_t = rad.sin();
        let cos_t = rad.cos();

        let rotate = |p: [f32; 3]| -> [f32; 3] {
            let along = p[0] * tilt_axis[0] + p[2] * tilt_axis[2];
            let perp = p[0] * (-tilt_axis[2]) + p[2] * tilt_axis[0];
            let rotated_along = along * cos_t - p[1] * sin_t;
            let rotated_y = along * sin_t + p[1] * cos_t;
            [
                rotated_along * tilt_axis[0] + perp * (-tilt_axis[2]) + center[0],
                rotated_y + center[1],
                rotated_along * tilt_axis[2] + perp * tilt_axis[0] + center[2],
            ]
        };

        let mut bot = Vec::new();
        let mut mid = Vec::new();
        let mid_h = height * 0.75;
        for i in 0..sides {
            let a = i as f32 * std::f32::consts::TAU / sides as f32;
            let c = a.cos() * radius;
            let s = a.sin() * radius;
            bot.push(rotate([c, 0.0, s]));
            mid.push(rotate([c * 0.95, mid_h, s * 0.95]));
        }
        let tip = rotate([0.0, height, 0.0]);

        for i in 0..sides {
            let next = (i + 1) % sides;
            add_flat_quad(&mut vertices, &mut indices, bot[i], bot[next], mid[next], mid[i], white);
            add_flat_triangle(&mut vertices, &mut indices, mid[i], mid[next], tip, [0.0, 0.0], [1.0, 0.0], [0.5, 1.0], white);
        }
    };

    add_shard([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 6.0, 0.42, 2.7);
    add_shard([-0.42, 0.0, 0.25], [-0.8, 0.0, 0.6], 26.0, 0.28, 1.8);
    add_shard([0.38, 0.0, -0.22], [0.7, 0.0, -0.4], 32.0, 0.22, 1.3);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Crystal Cluster Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Crystal Cluster Index Buffer"),
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

pub fn create_sci_fi_crate_mesh(device: &wgpu::Device) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let white = [1.0, 1.0, 1.0, 1.0];

    let half_w = 0.6f32;
    let bevel = 0.12f32;
    let b = half_w - bevel;

    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, half_w], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -half_w], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([half_w, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        ([-half_w, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, half_w, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -half_w, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ];

    for (pos, right, up) in faces {
        let p0 = [pos[0] - right[0]*b - up[0]*b, pos[1] - right[1]*b - up[1]*b, pos[2] - right[2]*b - up[2]*b];
        let p1 = [pos[0] + right[0]*b - up[0]*b, pos[1] + right[1]*b - up[1]*b, pos[2] + right[2]*b - up[2]*b];
        let p2 = [pos[0] + right[0]*b + up[0]*b, pos[1] + right[1]*b + up[1]*b, pos[2] + right[2]*b + up[2]*b];
        let p3 = [pos[0] - right[0]*b + up[0]*b, pos[1] - right[1]*b + up[1]*b, pos[2] - right[2]*b + up[2]*b];
        add_flat_quad(&mut vertices, &mut indices, p0, p1, p2, p3, white);

        let q0 = [pos[0] - right[0]*half_w - up[0]*b, pos[1] - right[1]*half_w - up[1]*b, pos[2] - right[2]*half_w - up[2]*b];
        let q1 = [pos[0] + right[0]*half_w - up[0]*b, pos[1] + right[1]*half_w - up[1]*b, pos[2] + right[2]*half_w - up[2]*b];
        let q2 = [pos[0] + right[0]*half_w + up[0]*b, pos[1] + right[1]*half_w + up[1]*b, pos[2] + right[2]*half_w + up[2]*b];
        let q3 = [pos[0] - right[0]*half_w + up[0]*b, pos[1] - right[1]*half_w + up[1]*b, pos[2] - right[2]*half_w + up[2]*b];

        add_flat_quad(&mut vertices, &mut indices, p1, q1, q2, p2, white);
        add_flat_quad(&mut vertices, &mut indices, p3, q3, q0, p0, white);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sci-Fi Crate Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sci-Fi Crate Index Buffer"),
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

fn push_box_mesh(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    center: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
) {
    let hw = size[0] * 0.5;
    let hh = size[1] * 0.5;
    let hd = size[2] * 0.5;
    let cx = center[0];
    let cy = center[1];
    let cz = center[2];

    let base = vertices.len() as u16;
    let faces = [
        ([cx-hw, cy-hh, cz+hd], [cx+hw, cy-hh, cz+hd], [cx+hw, cy+hh, cz+hd], [cx-hw, cy+hh, cz+hd], [0.0, 0.0, 1.0]),
        ([cx+hw, cy-hh, cz-hd], [cx-hw, cy-hh, cz-hd], [cx-hw, cy+hh, cz-hd], [cx+hw, cy+hh, cz-hd], [0.0, 0.0, -1.0]),
        ([cx-hw, cy+hh, cz+hd], [cx+hw, cy+hh, cz+hd], [cx+hw, cy+hh, cz-hd], [cx-hw, cy+hh, cz-hd], [0.0, 1.0, 0.0]),
        ([cx-hw, cy-hh, cz-hd], [cx+hw, cy-hh, cz-hd], [cx+hw, cy-hh, cz+hd], [cx-hw, cy-hh, cz+hd], [0.0, -1.0, 0.0]),
        ([cx+hw, cy-hh, cz+hd], [cx+hw, cy-hh, cz-hd], [cx+hw, cy+hh, cz-hd], [cx+hw, cy+hh, cz+hd], [1.0, 0.0, 0.0]),
        ([cx-hw, cy-hh, cz-hd], [cx-hw, cy-hh, cz+hd], [cx-hw, cy+hh, cz+hd], [cx-hw, cy+hh, cz-hd], [-1.0, 0.0, 0.0]),
    ];

    for (f_idx, (p0, p1, p2, p3, n)) in faces.iter().enumerate() {
        let b = base + (f_idx as u16) * 4;
        vertices.push(Vertex { position: *p0, normal: *n, uv: [0.0, 1.0], color });
        vertices.push(Vertex { position: *p1, normal: *n, uv: [1.0, 1.0], color });
        vertices.push(Vertex { position: *p2, normal: *n, uv: [1.0, 0.0], color });
        vertices.push(Vertex { position: *p3, normal: *n, uv: [0.0, 0.0], color });
        indices.extend_from_slice(&[b, b+1, b+2, b, b+2, b+3]);
    }
}

pub fn create_sword_mesh(device: &wgpu::Device) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let steel = [0.90, 0.92, 0.96, 1.0];
    let gold = [0.92, 0.75, 0.25, 1.0];
    let brown = [0.45, 0.28, 0.16, 1.0];

    // Handle
    push_box_mesh(&mut vertices, &mut indices, [0.0, -0.15, 0.0], [0.05, 0.26, 0.05], brown);
    // Pommel
    push_box_mesh(&mut vertices, &mut indices, [0.0, -0.30, 0.0], [0.08, 0.06, 0.08], gold);
    // Crossguard
    push_box_mesh(&mut vertices, &mut indices, [0.0, 0.0, 0.0], [0.34, 0.05, 0.07], gold);
    // Blade
    push_box_mesh(&mut vertices, &mut indices, [0.0, 0.45, 0.0], [0.08, 0.85, 0.02], steel);
    // Tip
    push_box_mesh(&mut vertices, &mut indices, [0.0, 0.92, 0.0], [0.04, 0.12, 0.015], steel);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sword Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sword Index Buffer"),
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

pub fn create_pickaxe_mesh(device: &wgpu::Device) -> MeshGpu {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let wood = [0.55, 0.38, 0.22, 1.0];
    let iron = [0.70, 0.72, 0.78, 1.0];

    // Shaft / handle
    push_box_mesh(&mut vertices, &mut indices, [0.0, -0.18, 0.0], [0.05, 0.75, 0.05], wood);
    // Center connector
    push_box_mesh(&mut vertices, &mut indices, [0.0, 0.22, 0.0], [0.08, 0.10, 0.08], iron);
    // Left pick
    push_box_mesh(&mut vertices, &mut indices, [-0.20, 0.20, 0.0], [0.34, 0.06, 0.05], iron);
    // Right pick
    push_box_mesh(&mut vertices, &mut indices, [0.20, 0.20, 0.0], [0.34, 0.06, 0.05], iron);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Pickaxe Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Pickaxe Index Buffer"),
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

pub fn generate_test_spritesheet(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let width = 128u32;
    let height = 128u32;
    let mut img = image::RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let col = x >= width / 2;
            let row = y >= height / 2;
            let color = match (col, row) {
                (false, false) => [240u8, 60u8, 60u8, 255u8],   // Frame 0 (top-left): Red
                (true, false) => [60u8, 220u8, 60u8, 255u8],   // Frame 1 (top-right): Green
                (false, true) => [60u8, 120u8, 240u8, 255u8],  // Frame 2 (bottom-left): Blue
                (true, true) => [240u8, 220u8, 40u8, 255u8],   // Frame 3 (bottom-right): Yellow
            };
            img.put_pixel(x, y, image::Rgba(color));
        }
    }

    img.save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to save spritesheet_test.png: {}", e))
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

        let sheet_path = Path::new("assets/textures/spritesheet_test.png");
        generate_test_spritesheet(sheet_path).expect("Failed to generate spritesheet_test.png");
        assert!(sheet_path.exists());
    }

    #[test]
    fn test_lights_uniform_layout() {
        use std::mem::size_of;
        assert_eq!(size_of::<PointLightRaw>(), 64);
        assert_eq!(size_of::<LightsUniform>(), 1040);
        assert_eq!(size_of::<LightsUniform>() % 16, 0);
        assert_eq!(size_of::<PostProcessUniform>(), 48);
        assert_eq!(size_of::<PostProcessUniform>() % 16, 0);
        assert_eq!(size_of::<ShadowUniform>(), 96);
        assert_eq!(size_of::<ShadowUniform>() % 16, 0);
        assert_eq!(size_of::<CameraUniform>(), 240);
        assert_eq!(size_of::<CameraUniform>() % 16, 0);
    }
}
