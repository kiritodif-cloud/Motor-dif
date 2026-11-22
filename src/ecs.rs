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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshRender {
    pub mesh_type: String, // "cube", "quad", "sphere", "cylinder", "plane", or path to .glb / .gltf
    #[serde(default)]
    pub texture: Option<String>,
    pub color: [f32; 4],   // [r, g, b, a]
    pub emissive: f32,     // 0.0 to 1.0
}

impl MeshRender {
    pub fn new(mesh_type: impl Into<String>, color: [f32; 4], emissive: f32) -> Self {
        Self {
            mesh_type: mesh_type.into(),
            texture: None,
            color,
            emissive,
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
        }
    }

    pub fn quad(color: [f32; 4]) -> Self {
        Self {
            mesh_type: "quad".to_string(),
            texture: None,
            color,
            emissive: 0.0,
        }
    }
}
