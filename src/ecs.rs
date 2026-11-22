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

#[cfg(test)]
mod tests {
    use super::*;

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
}
