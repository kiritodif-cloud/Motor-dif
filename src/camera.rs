use bytemuck::{Pod, Zeroable};
use glam::{camera::rh, Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraMode {
    #[serde(rename = "3d")]
    Perspective,
    #[serde(rename = "2d")]
    Orthographic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub mode: CameraMode,
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub ortho_size: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new_3d(eye: Vec3, target: Vec3, aspect: f32) -> Self {
        Self {
            mode: CameraMode::Perspective,
            eye,
            target,
            up: Vec3::Y,
            aspect,
            fovy: 45.0_f32.to_radians(),
            ortho_size: 10.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn new_2d(eye: Vec3, target: Vec3, ortho_size: f32, aspect: f32) -> Self {
        Self {
            mode: CameraMode::Orthographic,
            eye,
            target,
            up: Vec3::Y,
            aspect,
            fovy: 45.0_f32.to_radians(),
            ortho_size,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn toggle_mode(&mut self) {
        match self.mode {
            CameraMode::Perspective => {
                self.mode = CameraMode::Orthographic;
                self.eye = Vec3::new(0.0, 0.0, 15.0);
                self.target = Vec3::ZERO;
            }
            CameraMode::Orthographic => {
                self.mode = CameraMode::Perspective;
                self.eye = Vec3::new(0.0, 3.5, 9.0);
                self.target = Vec3::new(0.0, 0.0, 0.0);
            }
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = rh::view::look_at_mat4(self.eye, self.target, self.up);
        let proj = match self.mode {
            CameraMode::Perspective => rh::proj::directx::perspective(
                self.fovy,
                self.aspect.max(0.001),
                self.znear,
                self.zfar,
            ),
            CameraMode::Orthographic => {
                let half_h = self.ortho_size * 0.5;
                let half_w = half_h * self.aspect.max(0.001);
                rh::proj::directx::orthographic(
                    -half_w,
                    half_w,
                    -half_h,
                    half_h,
                    self.znear,
                    self.zfar,
                )
            }
        };
        proj * view
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0; 4],
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
        self.camera_pos = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new()
    }
}
