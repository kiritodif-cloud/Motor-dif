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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshRender {
    pub mesh_type: String, // "cube", "quad", or path to .glb / .gltf
    pub color: [f32; 4],   // [r, g, b, a]
}

impl MeshRender {
    pub fn cube(color: [f32; 4]) -> Self {
        Self {
            mesh_type: "cube".to_string(),
            color,
        }
    }

    pub fn quad(color: [f32; 4]) -> Self {
        Self {
            mesh_type: "quad".to_string(),
            color,
        }
    }
}
