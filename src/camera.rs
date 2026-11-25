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
    #[serde(skip)]
    pub shake_intensity: f32,
    #[serde(skip)]
    pub shake_duration: f32,
    #[serde(skip)]
    pub shake_timer: f32,
    #[serde(skip)]
    pub shake_offset: Vec3,
    #[serde(skip)]
    pub shake_frequency: f32,
    #[serde(skip)]
    pub follow_target: Option<(String, Vec3, f32)>,
    #[serde(skip)]
    pub lerp_cinematic: Option<CameraLerpState>,
    #[serde(skip)]
    pub orbit_cinematic: Option<CameraOrbitState>,
}

#[derive(Debug, Clone)]
pub struct CameraLerpState {
    pub start_eye: Vec3,
    pub target_eye: Vec3,
    pub start_target: Vec3,
    pub target_target: Vec3,
    pub duration: f32,
    pub elapsed: f32,
}

#[derive(Debug, Clone)]
pub struct CameraOrbitState {
    pub target_id: Option<String>,
    pub distance: f32,
    pub pitch_deg: f32,
    pub yaw_speed: f32,
    pub current_yaw: f32,
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
            shake_intensity: 0.0,
            shake_duration: 0.0,
            shake_timer: 0.0,
            shake_offset: Vec3::ZERO,
            shake_frequency: 25.0,
            follow_target: None,
            lerp_cinematic: None,
            orbit_cinematic: None,
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
            shake_intensity: 0.0,
            shake_duration: 0.0,
            shake_timer: 0.0,
            shake_offset: Vec3::ZERO,
            shake_frequency: 25.0,
            follow_target: None,
            lerp_cinematic: None,
            orbit_cinematic: None,
        }
    }

    pub fn start_lerp_to(&mut self, target_eye: Vec3, target_target: Vec3, duration: f32) {
        self.lerp_cinematic = Some(CameraLerpState {
            start_eye: self.eye,
            target_eye,
            start_target: self.target,
            target_target,
            duration: duration.max(0.01),
            elapsed: 0.0,
        });
        self.orbit_cinematic = None;
    }

    pub fn start_orbit_target(&mut self, target_id: Option<String>, distance: f32, pitch_deg: f32, yaw_speed: f32) {
        let initial_yaw = (self.eye.z - self.target.z).atan2(self.eye.x - self.target.x);
        self.orbit_cinematic = Some(CameraOrbitState {
            target_id,
            distance: distance.max(0.5),
            pitch_deg: pitch_deg.clamp(-85.0, 85.0),
            yaw_speed,
            current_yaw: initial_yaw,
        });
        self.lerp_cinematic = None;
    }

    pub fn stop_cinematics(&mut self) {
        self.lerp_cinematic = None;
        self.orbit_cinematic = None;
    }

    pub fn update_cinematics(&mut self, dt: f32, entity_world_positions: &std::collections::HashMap<String, Vec3>) {
        if let Some(ref mut lerp) = self.lerp_cinematic {
            lerp.elapsed += dt;
            let t = (lerp.elapsed / lerp.duration).clamp(0.0, 1.0);
            let smooth_t = t * t * (3.0 - 2.0 * t);
            self.eye = lerp.start_eye.lerp(lerp.target_eye, smooth_t);
            self.target = lerp.start_target.lerp(lerp.target_target, smooth_t);

            if lerp.elapsed >= lerp.duration {
                self.lerp_cinematic = None;
            }
        } else if let Some(ref mut orb) = self.orbit_cinematic {
            orb.current_yaw += orb.yaw_speed * dt;
            let center = if let Some(ref id) = orb.target_id {
                entity_world_positions.get(id).copied().unwrap_or(Vec3::ZERO)
            } else {
                self.target
            };
            self.target = center;
            let pitch_rad = orb.pitch_deg.to_radians();
            let cos_p = pitch_rad.cos();
            let sin_p = pitch_rad.sin();
            let x = center.x + orb.distance * cos_p * orb.current_yaw.cos();
            let y = center.y + orb.distance * sin_p;
            let z = center.z + orb.distance * cos_p * orb.current_yaw.sin();
            self.eye = Vec3::new(x, y, z);
        }
    }

    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.shake_with_freq(intensity, duration, 25.0);
    }

    pub fn shake_with_freq(&mut self, intensity: f32, duration: f32, frequency: f32) {
        self.shake_intensity = intensity;
        self.shake_duration = duration.max(0.01);
        self.shake_timer = self.shake_duration;
        self.shake_frequency = frequency.max(1.0);
    }

    pub fn update_shake(&mut self, dt: f32) {
        if self.shake_timer > 0.0 {
            self.shake_timer -= dt;
            let progress = (self.shake_timer / self.shake_duration).clamp(0.0, 1.0);
            // Exponential ease-out decay curve
            let decay = progress * progress * (3.0 - 2.0 * progress);
            let current_amp = self.shake_intensity * decay;
            let t = (self.shake_duration - self.shake_timer) * self.shake_frequency;

            // Multi-harmonic smooth pseudo-noise
            let r1 = (t * 1.11).sin() * 0.65 + (t * 2.37).sin() * 0.35;
            let r2 = (t * 1.43 + 1.2).cos() * 0.65 + (t * 2.81 + 0.8).sin() * 0.35;
            let r3 = (t * 1.79 + 2.1).sin() * 0.6 + (t * 3.19 + 1.5).cos() * 0.4;
            self.shake_offset = Vec3::new(r1, r2, r3) * current_amp;
        } else {
            self.shake_timer = 0.0;
            self.shake_offset = Vec3::ZERO;
        }
    }

    pub fn follow_entity(&mut self, target_id: Option<String>, offset: Vec3, smoothness: f32) {
        if let Some(id) = target_id {
            let clean = id.trim().to_string();
            if !clean.is_empty() {
                self.follow_target = Some((clean, offset, smoothness.clamp(0.01, 1.0)));
            } else {
                self.follow_target = None;
            }
        } else {
            self.follow_target = None;
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
        let eff_eye = self.eye + self.shake_offset;
        let eff_target = self.target + self.shake_offset;
        let view = rh::view::look_at_mat4(eff_eye, eff_target, self.up);
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

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        if self.mode == CameraMode::Orthographic {
            self.pan(delta_x * 5.0, delta_y * 5.0);
            return;
        }
        let offset = self.eye - self.target;
        let radius = offset.length().max(0.1);
        let mut yaw = offset.z.atan2(offset.x);
        let mut pitch = (offset.y / radius).clamp(-0.9999, 0.9999).asin();

        yaw += delta_x * 0.006;
        pitch = (pitch + delta_y * 0.006).clamp(-1.52, 1.52);

        let cos_pitch = pitch.cos();
        let new_offset = Vec3::new(
            radius * cos_pitch * yaw.cos(),
            radius * pitch.sin(),
            radius * cos_pitch * yaw.sin(),
        );
        self.eye = self.target + new_offset;
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.eye).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let dist = (self.eye - self.target).length().max(1.0);
        let factor = match self.mode {
            CameraMode::Perspective => dist * 0.0018,
            CameraMode::Orthographic => self.ortho_size * 0.0018,
        };

        let move_vec = (-right * delta_x + up * delta_y) * factor;
        self.eye += move_vec;
        self.target += move_vec;
    }

    pub fn zoom(&mut self, delta_y: f32) {
        match self.mode {
            CameraMode::Perspective => {
                let offset = self.eye - self.target;
                let dist = offset.length();
                let new_dist = (dist * (1.0 - delta_y * 0.08)).clamp(0.5, 200.0);
                self.eye = self.target + offset.normalize_or_zero() * new_dist;
            }
            CameraMode::Orthographic => {
                self.ortho_size = (self.ortho_size * (1.0 - delta_y * 0.08)).clamp(1.0, 100.0);
            }
        }
    }

    pub fn screen_to_ray(&self, screen_pos: [f32; 2], screen_size: [f32; 2]) -> (Vec3, Vec3) {
        let w = screen_size[0].max(1.0);
        let h = screen_size[1].max(1.0);

        let ndc_x = (2.0 * screen_pos[0] / w) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_pos[1] / h);

        let inv_vp = self.build_view_projection_matrix().inverse();

        match self.mode {
            CameraMode::Perspective => {
                let near_pt = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
                let far_pt = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
                let dir = (far_pt - near_pt).normalize_or_zero();
                (near_pt, dir)
            }
            CameraMode::Orthographic => {
                let near_pt = inv_vp.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
                let dir = (self.target - self.eye).normalize_or_zero();
                (near_pt, dir)
            }
        }
    }

    pub fn intersect_horizontal_plane(&self, screen_pos: [f32; 2], screen_size: [f32; 2], plane_y: f32) -> Option<Vec3> {
        let (origin, dir) = self.screen_to_ray(screen_pos, screen_size);
        if dir.y.abs() < 1e-6 {
            return None;
        }
        let t = (plane_y - origin.y) / dir.y;
        if t < 0.0 {
            return None;
        }
        Some(origin + dir * t)
    }

    pub fn world_to_screen(&self, world_pos: Vec3, screen_size: [f32; 2]) -> Option<[f32; 2]> {
        let vp = self.build_view_projection_matrix();
        let clip = vp * glam::Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let screen_x = (ndc.x + 1.0) * 0.5 * screen_size[0];
        let screen_y = (1.0 - ndc.y) * 0.5 * screen_size[1];
        Some([screen_x, screen_y])
    }
}

pub fn ray_intersects_aabb(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut tmin = (min.x - origin.x) / if dir.x.abs() > 1e-6 { dir.x } else { 1e-6 * dir.x.signum() };
    let mut tmax = (max.x - origin.x) / if dir.x.abs() > 1e-6 { dir.x } else { 1e-6 * dir.x.signum() };
    if tmin > tmax { std::mem::swap(&mut tmin, &mut tmax); }

    let mut tymin = (min.y - origin.y) / if dir.y.abs() > 1e-6 { dir.y } else { 1e-6 * dir.y.signum() };
    let mut tymax = (max.y - origin.y) / if dir.y.abs() > 1e-6 { dir.y } else { 1e-6 * dir.y.signum() };
    if tymin > tymax { std::mem::swap(&mut tymin, &mut tymax); }

    if (tmin > tymax) || (tymin > tmax) {
        return None;
    }

    if tymin > tmin { tmin = tymin; }
    if tymax < tmax { tmax = tymax; }

    let mut tzmin = (min.z - origin.z) / if dir.z.abs() > 1e-6 { dir.z } else { 1e-6 * dir.z.signum() };
    let mut tzmax = (max.z - origin.z) / if dir.z.abs() > 1e-6 { dir.z } else { 1e-6 * dir.z.signum() };
    if tzmin > tzmax { std::mem::swap(&mut tzmin, &mut tzmax); }

    if (tmin > tzmax) || (tzmin > tmax) {
        return None;
    }

    if tzmin > tmin { tmin = tzmin; }
    if tzmax < tmax { tmax = tzmax; }

    if tmax < 0.0 {
        return None;
    }
    Some(if tmin < 0.0 { tmax } else { tmin })
}

pub fn ray_intersects_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }
    let t = -b - discriminant.sqrt();
    if t > 0.0 {
        Some(t)
    } else {
        let t2 = -b + discriminant.sqrt();
        if t2 > 0.0 { Some(t2) } else { None }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub fog_color: [f32; 4],
    pub zenith_color: [f32; 4],
    pub horizon_color: [f32; 4],
    pub sun_dir: [f32; 4],
    pub sun_color: [f32; 4],
    pub fog_params: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            inv_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0; 4],
            fog_color: [0.071, 0.082, 0.118, 0.0],
            zenith_color: [0.071, 0.082, 0.118, 1.0],
            horizon_color: [0.133, 0.153, 0.212, 0.0],
            sun_dir: [0.5, 0.8, 0.4, 1.0],
            sun_color: [1.0, 0.98, 0.92, 0.0],
            fog_params: [0.0, 0.0, 100.0, 0.5],
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.update_with_time(camera, 1.0);
    }

    pub fn update_with_time(&mut self, camera: &Camera, time: f32) {
        let vp = camera.build_view_projection_matrix();
        self.view_proj = vp.to_cols_array_2d();
        self.inv_view_proj = vp.inverse().to_cols_array_2d();
        let eff_eye = camera.eye + camera.shake_offset;
        self.camera_pos = [eff_eye.x, eff_eye.y, eff_eye.z, time];
    }

    pub fn update_with_environment(
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
        self.update_with_environment_fog(
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

    pub fn update_with_environment_fog(
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
        self.update_with_time(camera, time);
        let mode_val = match sky_mode.to_lowercase().as_str() {
            "solid" => 0.0,
            "gradient" => 1.0,
            _ => 2.0, // "space"
        };
        let f_mode = if fog_mode.to_lowercase() == "linear" { 1.0 } else { 0.0 };
        self.fog_color = [fog_color[0], fog_color[1], fog_color[2], fog_density];
        self.zenith_color = [zenith[0], zenith[1], zenith[2], mode_val];
        self.horizon_color = [horizon[0], horizon[1], horizon[2], 0.0];
        self.sun_dir = [sun_dir[0], sun_dir[1], sun_dir[2], sun_intensity];
        self.sun_color = [sun_color[0], sun_color[1], sun_color[2], stars_visibility];
        self.fog_params = [f_mode, fog_start, fog_end, 0.5];
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_camera_lerp_cinematics() {
        let mut cam = Camera::new_3d(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 1.77);
        cam.start_lerp_to(Vec3::new(0.0, 5.0, 20.0), Vec3::new(0.0, 2.0, 0.0), 1.0);
        assert!(cam.lerp_cinematic.is_some());

        let entity_pos = HashMap::new();
        cam.update_cinematics(0.5, &entity_pos);
        // Halfway smoothstep is 0.5
        assert!((cam.eye.y - 2.5).abs() < 0.2);
        assert!((cam.eye.z - 15.0).abs() < 0.2);

        cam.update_cinematics(0.6, &entity_pos);
        assert!(cam.lerp_cinematic.is_none());
        assert!((cam.eye.y - 5.0).abs() < 1e-4);
        assert!((cam.eye.z - 20.0).abs() < 1e-4);
    }

    #[test]
    fn test_camera_orbit_cinematics() {
        let mut cam = Camera::new_3d(Vec3::new(5.0, 0.0, 0.0), Vec3::ZERO, 1.77);
        cam.start_orbit_target(None, 5.0, 0.0, 1.0);
        assert!(cam.orbit_cinematic.is_some());

        let entity_pos = HashMap::new();
        cam.update_cinematics(std::f32::consts::PI * 0.5, &entity_pos);
        // After 90 deg rotation, eye should be near (0, 0, 5)
        assert!(cam.eye.x.abs() < 0.1);
        assert!((cam.eye.z - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_camera_manual_orbit_yaw_direction() {
        let mut cam = Camera::new_3d(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 1.77);

        // Move mouse to the right (dx > 0) -> camera view turns right (forward x > 0)
        cam.orbit(50.0, 0.0);
        let forward_after_right = (cam.target - cam.eye).normalize();
        assert!(forward_after_right.x > 0.0, "Expected forward vector x > 0 when turning right, got {}", forward_after_right.x);

        // Reset and move mouse to the left (dx < 0) -> camera view turns left (forward x < 0)
        let mut cam_left = Camera::new_3d(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 1.77);
        cam_left.orbit(-50.0, 0.0);
        let forward_after_left = (cam_left.target - cam_left.eye).normalize();
        assert!(forward_after_left.x < 0.0, "Expected forward vector x < 0 when turning left, got {}", forward_after_left.x);
    }

    #[test]
    fn test_frustum_culling_sphere() {
        let cam = Camera::new_3d(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 16.0 / 9.0);
        let frustum = Frustum::from_view_proj(cam.build_view_projection_matrix());
        assert!(frustum.intersects_sphere(Vec3::ZERO, 1.0));
        assert!(frustum.intersects_sphere(Vec3::new(1.5, 0.0, 0.0), 1.0));
        // Far behind camera (+Z > 10)
        assert!(!frustum.intersects_sphere(Vec3::new(0.0, 0.0, 25.0), 1.0));
        // Far to the side
        assert!(!frustum.intersects_sphere(Vec3::new(50.0, 0.0, 0.0), 1.0));
    }
}

pub struct Frustum {
    pub planes: [glam::Vec4; 6],
}

impl Frustum {
    pub fn from_view_proj(vp: Mat4) -> Self {
        let r0 = vp.row(0);
        let r1 = vp.row(1);
        let r2 = vp.row(2);
        let r3 = vp.row(3);
        let mut planes = [
            r3 + r0, // Left
            r3 - r0, // Right
            r3 + r1, // Bottom
            r3 - r1, // Top
            r2,      // Near (wgpu [0, 1] NDC depth)
            r3 - r2, // Far
        ];
        for p in &mut planes {
            let len = Vec3::new(p.x, p.y, p.z).length();
            if len > 1e-6 {
                *p /= len;
            }
        }
        Self { planes }
    }

    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for p in &self.planes {
            let dist = p.x * center.x + p.y * center.y + p.z * center.z + p.w;
            if dist < -radius {
                return false;
            }
        }
        true
    }
}
