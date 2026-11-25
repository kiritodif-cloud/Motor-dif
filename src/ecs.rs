use glam::{EulerRot, Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3, // Euler angles in radians (rx, ry, rz)
    pub scale: Vec3,
}

impl Transform {
    pub fn new(position: Vec3, rotation: Vec3, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_euler(EulerRot::YXZ, self.rotation.y, self.rotation.x, self.rotation.z)
            * Mat4::from_scale(self.scale)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabInstance {
    pub source: String,
}

impl EntityMeta {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent: None,
        }
    }

    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PointLight {
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            radius: 8.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpotLight {
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub direction: [f32; 3],
    #[serde(default = "default_inner_cone")]
    pub inner_angle: f32,
    #[serde(default = "default_outer_cone")]
    pub outer_angle: f32,
}

fn default_inner_cone() -> f32 { 0.4 }
fn default_outer_cone() -> f32 { 0.65 }

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            radius: 12.0,
            direction: [0.0, -1.0, 0.0],
            inner_angle: 0.4,
            outer_angle: 0.65,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PbrMaterial {
    #[serde(default)]
    pub normal_map: Option<String>,
    #[serde(default)]
    pub metallic_roughness_map: Option<String>,
    #[serde(default)]
    pub emissive_map: Option<String>,
    #[serde(default = "default_pbr_zero")]
    pub metallic: f32,
    #[serde(default = "default_pbr_half")]
    pub roughness: f32,
    #[serde(default = "default_pbr_one")]
    pub normal_scale: f32,
    #[serde(default = "default_pbr_one")]
    pub emissive_intensity: f32,
}

fn default_pbr_zero() -> f32 { 0.0 }
fn default_pbr_half() -> f32 { 0.5 }
fn default_pbr_one() -> f32 { 1.0 }

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            normal_map: None,
            metallic_roughness_map: None,
            emissive_map: None,
            metallic: 0.0,
            roughness: 0.5,
            normal_scale: 1.0,
            emissive_intensity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshRender {
    pub mesh_type: String, // "cube", "quad", "sphere", "cylinder", "plane", or path to .glb / .gltf
    #[serde(default)]
    pub texture: Option<String>,
    pub color: [f32; 4],   // [r, g, b, a]
    pub emissive: f32,     // 0.0 to 5.0
    #[serde(default)]
    pub fresnel_color: Option<[f32; 3]>,
    #[serde(default)]
    pub fresnel_power: Option<f32>,
    #[serde(default)]
    pub blend_mode: Option<String>, // "opaque" | "alpha" | "additive"
    #[serde(default)]
    pub wireframe: Option<bool>,
    #[serde(default)]
    pub uv_scroll: Option<[f32; 2]>,
}

impl MeshRender {
    pub fn new(mesh_type: impl Into<String>, color: [f32; 4], emissive: f32) -> Self {
        Self {
            mesh_type: mesh_type.into(),
            texture: None,
            color,
            emissive,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
        }
    }

    pub fn with_texture(mut self, texture: impl Into<String>) -> Self {
        self.texture = Some(texture.into());
        self
    }

    pub fn cube(color: [f32; 4]) -> Self {
        Self {
            mesh_type: "cube".to_string(),
            texture: None,
            color,
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
        }
    }

    pub fn quad(color: [f32; 4]) -> Self {
        Self {
            mesh_type: "quad".to_string(),
            texture: None,
            color,
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteAnimation {
    pub cols: u32,
    pub rows: u32,
    pub current_frame: u32,
}

impl SpriteAnimation {
    pub fn new(cols: u32, rows: u32, current_frame: u32) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            current_frame,
        }
    }

    pub fn uv_offset_and_scale(&self) -> ([f32; 2], [f32; 2]) {
        let cols = self.cols.max(1) as f32;
        let rows = self.rows.max(1) as f32;
        let u_scale = 1.0 / cols;
        let v_scale = 1.0 / rows;
        let col = (self.current_frame % self.cols.max(1)) as f32;
        let row = ((self.current_frame / self.cols.max(1)) % self.rows.max(1)) as f32;
        ([col * u_scale, row * v_scale], [u_scale, v_scale])
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TrailPoint {
    pub position: Vec3,
    pub time: f32,
}

#[derive(Debug, Clone)]
pub struct TrailComponent {
    pub points: std::collections::VecDeque<TrailPoint>,
    pub max_points: usize,
    pub lifetime: f32,
    pub width: f32,
    pub color: [f32; 4],
}

impl TrailComponent {
    pub fn new(color: [f32; 4], lifetime: f32, width: f32) -> Self {
        Self {
            points: std::collections::VecDeque::new(),
            max_points: 64,
            lifetime,
            width,
            color,
        }
    }

    pub fn push_point(&mut self, pos: Vec3, current_time: f32) {
        if let Some(last) = self.points.back() {
            if (last.position - pos).length_squared() < 0.001 {
                return;
            }
        }
        self.points.push_back(TrailPoint {
            position: pos,
            time: current_time,
        });
        while self.points.len() > self.max_points {
            self.points.pop_front();
        }
    }

    pub fn prune(&mut self, current_time: f32) {
        while let Some(front) = self.points.front() {
            if current_time - front.time > self.lifetime {
                self.points.pop_front();
            } else {
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraTarget {
    pub target_id: String,
    #[serde(default = "default_rtt_width")]
    pub width: u32,
    #[serde(default = "default_rtt_height")]
    pub height: u32,
    #[serde(default = "default_rtt_fov")]
    pub fov: f32,
    #[serde(default = "default_rtt_hz")]
    pub update_hz: f32,
    #[serde(default = "default_rtt_true")]
    pub enabled: bool,
}

fn default_rtt_width() -> u32 { 640 }
fn default_rtt_height() -> u32 { 360 }
fn default_rtt_fov() -> f32 { 60.0 }
fn default_rtt_hz() -> f32 { 15.0 }
fn default_rtt_true() -> bool { true }

impl Default for CameraTarget {
    fn default() -> Self {
        Self {
            target_id: "default_rtt".to_string(),
            width: 640,
            height: 360,
            fov: 60.0,
            update_hz: 15.0,
            enabled: true,
        }
    }
}

impl CameraTarget {
    pub fn new(target_id: impl Into<String>, width: u32, height: u32, fov: f32, update_hz: f32) -> Self {
        Self {
            target_id: target_id.into(),
            width: width.max(1),
            height: height.max(1),
            fov: fov.clamp(10.0, 150.0),
            update_hz: update_hz.max(0.1),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaterialOverrideRtt {
    pub target_id: String,
    #[serde(default)]
    pub as_emissive: bool,
}

impl Default for MaterialOverrideRtt {
    fn default() -> Self {
        Self {
            target_id: "default_rtt".to_string(),
            as_emissive: false,
        }
    }
}

impl MaterialOverrideRtt {
    pub fn new(target_id: impl Into<String>, as_emissive: bool) -> Self {
        Self {
            target_id: target_id.into(),
            as_emissive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtt_components() {
        let cam = CameraTarget::new("cam_sec_1", 800, 600, 75.0, 30.0);
        assert_eq!(cam.target_id, "cam_sec_1");
        assert_eq!(cam.width, 800);
        assert_eq!(cam.height, 600);
        assert_eq!(cam.fov, 75.0);
        assert_eq!(cam.update_hz, 30.0);
        assert!(cam.enabled);

        let mat = MaterialOverrideRtt::new("cam_sec_1", true);
        assert_eq!(mat.target_id, "cam_sec_1");
        assert!(mat.as_emissive);
    }

    #[test]
    fn test_sprite_animation_math() {
        let anim = SpriteAnimation::new(4, 2, 0);
        let (offset, scale) = anim.uv_offset_and_scale();
        assert_eq!(scale, [0.25, 0.5]);
        assert_eq!(offset, [0.0, 0.0]);

        // Frame 5 -> col 1, row 1
        let anim_f5 = SpriteAnimation::new(4, 2, 5);
        let (offset5, scale5) = anim_f5.uv_offset_and_scale();
        assert_eq!(scale5, [0.25, 0.5]);
        assert_eq!(offset5, [0.25, 0.5]);
    }

    #[test]
    fn test_trail_component_push_and_prune() {
        let mut trail = TrailComponent::new([1.0, 0.5, 0.0, 1.0], 1.0, 0.3);
        trail.push_point(Vec3::new(0.0, 0.0, 0.0), 0.0);
        trail.push_point(Vec3::new(1.0, 0.0, 0.0), 0.2);
        trail.push_point(Vec3::new(2.0, 0.0, 0.0), 0.5);
        assert_eq!(trail.points.len(), 3);

        trail.prune(1.1);
        assert_eq!(trail.points.len(), 2);
        assert_eq!(trail.points.front().unwrap().position, Vec3::new(1.0, 0.0, 0.0));
    }
}
