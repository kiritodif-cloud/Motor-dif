use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::ecs::{MeshRender, Transform};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f32,
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub rotation: Option<[f32; 3]>,
    #[serde(default)]
    pub scale: Option<[f32; 3]>,
    #[serde(default)]
    pub color: Option<[f32; 4]>,
    #[serde(default)]
    pub emissive: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct InterpolatedState {
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
    pub color: Option<[f32; 4]>,
    pub emissive: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    #[serde(default)]
    pub keyframes: Vec<Keyframe>,
}

impl AnimationClip {
    pub fn new(name: impl Into<String>, duration: f32) -> Self {
        Self {
            name: name.into(),
            duration: duration.max(0.001),
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(&mut self, kf: Keyframe) {
        self.keyframes.push(kf);
        self.sort_keyframes();
    }

    pub fn sort_keyframes(&mut self) {
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    }

    pub fn sample(&self, time: f32) -> InterpolatedState {
        let mut state = InterpolatedState::default();
        if self.keyframes.is_empty() {
            return state;
        }

        let t = if self.duration > 0.0 {
            time.clamp(0.0, self.duration)
        } else {
            0.0
        };

        // 1. Position interpolation
        state.position = self.sample_vec3(t, |kf| kf.position);
        // 2. Rotation interpolation (Euler rx, ry, rz)
        state.rotation = self.sample_vec3(t, |kf| kf.rotation);
        // 3. Scale interpolation
        state.scale = self.sample_vec3(t, |kf| kf.scale);
        // 4. Color interpolation (RGBA)
        state.color = self.sample_vec4(t, |kf| kf.color);
        // 5. Emissive interpolation (f32)
        state.emissive = self.sample_scalar(t, |kf| kf.emissive);

        state
    }

    fn sample_vec3<F>(&self, t: f32, extractor: F) -> Option<[f32; 3]>
    where
        F: Fn(&Keyframe) -> Option<[f32; 3]>,
    {
        let matching: Vec<(f32, [f32; 3])> = self
            .keyframes
            .iter()
            .filter_map(|kf| extractor(kf).map(|val| (kf.time, val)))
            .collect();

        if matching.is_empty() {
            return None;
        }
        if matching.len() == 1 || t <= matching.first().unwrap().0 {
            return Some(matching.first().unwrap().1);
        }
        if t >= matching.last().unwrap().0 {
            return Some(matching.last().unwrap().1);
        }

        for i in 0..matching.len() - 1 {
            let (t0, v0) = matching[i];
            let (t1, v1) = matching[i + 1];
            if t >= t0 && t <= t1 {
                let factor = if (t1 - t0).abs() > 0.0001 {
                    (t - t0) / (t1 - t0)
                } else {
                    0.0
                };
                return Some([
                    v0[0] + (v1[0] - v0[0]) * factor,
                    v0[1] + (v1[1] - v0[1]) * factor,
                    v0[2] + (v1[2] - v0[2]) * factor,
                ]);
            }
        }
        Some(matching.last().unwrap().1)
    }

    fn sample_vec4<F>(&self, t: f32, extractor: F) -> Option<[f32; 4]>
    where
        F: Fn(&Keyframe) -> Option<[f32; 4]>,
    {
        let matching: Vec<(f32, [f32; 4])> = self
            .keyframes
            .iter()
            .filter_map(|kf| extractor(kf).map(|val| (kf.time, val)))
            .collect();

        if matching.is_empty() {
            return None;
        }
        if matching.len() == 1 || t <= matching.first().unwrap().0 {
            return Some(matching.first().unwrap().1);
        }
        if t >= matching.last().unwrap().0 {
            return Some(matching.last().unwrap().1);
        }

        for i in 0..matching.len() - 1 {
            let (t0, v0) = matching[i];
            let (t1, v1) = matching[i + 1];
            if t >= t0 && t <= t1 {
                let factor = if (t1 - t0).abs() > 0.0001 {
                    (t - t0) / (t1 - t0)
                } else {
                    0.0
                };
                return Some([
                    v0[0] + (v1[0] - v0[0]) * factor,
                    v0[1] + (v1[1] - v0[1]) * factor,
                    v0[2] + (v1[2] - v0[2]) * factor,
                    v0[3] + (v1[3] - v0[3]) * factor,
                ]);
            }
        }
        Some(matching.last().unwrap().1)
    }

    fn sample_scalar<F>(&self, t: f32, extractor: F) -> Option<f32>
    where
        F: Fn(&Keyframe) -> Option<f32>,
    {
        let matching: Vec<(f32, f32)> = self
            .keyframes
            .iter()
            .filter_map(|kf| extractor(kf).map(|val| (kf.time, val)))
            .collect();

        if matching.is_empty() {
            return None;
        }
        if matching.len() == 1 || t <= matching.first().unwrap().0 {
            return Some(matching.first().unwrap().1);
        }
        if t >= matching.last().unwrap().0 {
            return Some(matching.last().unwrap().1);
        }

        for i in 0..matching.len() - 1 {
            let (t0, v0) = matching[i];
            let (t1, v1) = matching[i + 1];
            if t >= t0 && t <= t1 {
                let factor = if (t1 - t0).abs() > 0.0001 {
                    (t - t0) / (t1 - t0)
                } else {
                    0.0
                };
                return Some(v0 + (v1 - v0) * factor);
            }
        }
        Some(matching.last().unwrap().1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationPlayer {
    #[serde(default)]
    pub animations: HashMap<String, AnimationClip>,
    #[serde(default)]
    pub active_clip: Option<String>,
    #[serde(default)]
    pub current_time: f32,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default = "default_true")]
    pub looping: bool,
    #[serde(default = "default_true")]
    pub playing: bool,

    // Crossfade blending state
    #[serde(skip)]
    pub previous_clip: Option<String>,
    #[serde(skip)]
    pub previous_time: f32,
    #[serde(skip)]
    pub blend_duration: f32,
    #[serde(skip)]
    pub blend_elapsed: f32,
}

fn default_speed() -> f32 {
    1.0
}
fn default_true() -> bool {
    true
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            animations: HashMap::new(),
            active_clip: None,
            current_time: 0.0,
            speed: 1.0,
            looping: true,
            playing: true,
            previous_clip: None,
            previous_time: 0.0,
            blend_duration: 0.0,
            blend_elapsed: 0.0,
        }
    }
}

pub fn blend_states(a: &InterpolatedState, b: &InterpolatedState, factor: f32) -> InterpolatedState {
    let f = factor.clamp(0.0, 1.0);
    let inv_f = 1.0 - f;

    let position = match (a.position, b.position) {
        (Some(p1), Some(p2)) => Some([
            p1[0] * inv_f + p2[0] * f,
            p1[1] * inv_f + p2[1] * f,
            p1[2] * inv_f + p2[2] * f,
        ]),
        (Some(p), None) | (None, Some(p)) => Some(p),
        (None, None) => None,
    };

    let rotation = match (a.rotation, b.rotation) {
        (Some(r1), Some(r2)) => Some([
            r1[0] * inv_f + r2[0] * f,
            r1[1] * inv_f + r2[1] * f,
            r1[2] * inv_f + r2[2] * f,
        ]),
        (Some(r), None) | (None, Some(r)) => Some(r),
        (None, None) => None,
    };

    let scale = match (a.scale, b.scale) {
        (Some(s1), Some(s2)) => Some([
            s1[0] * inv_f + s2[0] * f,
            s1[1] * inv_f + s2[1] * f,
            s1[2] * inv_f + s2[2] * f,
        ]),
        (Some(s), None) | (None, Some(s)) => Some(s),
        (None, None) => None,
    };

    let color = match (a.color, b.color) {
        (Some(c1), Some(c2)) => Some([
            c1[0] * inv_f + c2[0] * f,
            c1[1] * inv_f + c2[1] * f,
            c1[2] * inv_f + c2[2] * f,
            c1[3] * inv_f + c2[3] * f,
        ]),
        (Some(c), None) | (None, Some(c)) => Some(c),
        (None, None) => None,
    };

    let emissive = match (a.emissive, b.emissive) {
        (Some(e1), Some(e2)) => Some(e1 * inv_f + e2 * f),
        (Some(e), None) | (None, Some(e)) => Some(e),
        (None, None) => None,
    };

    InterpolatedState {
        position,
        rotation,
        scale,
        color,
        emissive,
    }
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_clip(&mut self, clip: AnimationClip) {
        let name = clip.name.clone();
        if self.active_clip.is_none() {
            self.active_clip = Some(name.clone());
        }
        self.animations.insert(name, clip);
    }

    pub fn play(&mut self, clip_name: &str, looping: bool) -> bool {
        self.play_with_blend(clip_name, looping, 0.0)
    }

    pub fn play_with_blend(&mut self, clip_name: &str, looping: bool, blend_duration: f32) -> bool {
        if self.animations.contains_key(clip_name) {
            if blend_duration > 0.0 && self.playing && self.active_clip.is_some() {
                self.previous_clip = self.active_clip.clone();
                self.previous_time = self.current_time;
                self.blend_duration = blend_duration;
                self.blend_elapsed = 0.0;
            } else {
                self.previous_clip = None;
                self.blend_duration = 0.0;
                self.blend_elapsed = 0.0;
            }
            self.active_clip = Some(clip_name.to_string());
            self.looping = looping;
            self.playing = true;
            self.current_time = 0.0;
            true
        } else {
            false
        }
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.current_time = 0.0;
        self.previous_clip = None;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn resume(&mut self) {
        self.playing = true;
    }

    pub fn is_playing(&self) -> bool {
        self.playing && self.active_clip.is_some()
    }

    pub fn play_preset(&mut self, preset_name: &str) -> bool {
        let clean = preset_name.to_lowercase();
        let looping = clean != "recoil";
        if !self.animations.contains_key(&clean) {
            if let Some(clip) = create_preset_clip(&clean) {
                self.add_clip(clip);
            } else {
                return false;
            }
        }
        self.play(&clean, looping)
    }

    pub fn update(&mut self, dt: f32) -> Option<InterpolatedState> {
        let clip_name = self.active_clip.as_ref()?;
        let clip = self.animations.get(clip_name)?;
        let duration = clip.duration.max(0.001);

        if self.playing {
            self.current_time += dt * self.speed;
            if self.current_time >= duration {
                if self.looping {
                    self.current_time %= duration;
                } else {
                    self.current_time = duration;
                    self.playing = false;
                }
            } else if self.current_time < 0.0 {
                if self.looping {
                    self.current_time = duration - ((-self.current_time) % duration);
                } else {
                    self.current_time = 0.0;
                    self.playing = false;
                }
            }
        }

        let mut target_state = clip.sample(self.current_time);

        // Linear crossfade blending if active
        if self.blend_duration > 0.0 && self.blend_elapsed < self.blend_duration {
            if let Some(prev_name) = &self.previous_clip {
                if let Some(prev_clip) = self.animations.get(prev_name) {
                    let prev_duration = prev_clip.duration.max(0.001);
                    self.previous_time += dt * self.speed;
                    if self.previous_time >= prev_duration {
                        self.previous_time %= prev_duration;
                    }
                    let prev_state = prev_clip.sample(self.previous_time);
                    self.blend_elapsed += dt;
                    let factor = (self.blend_elapsed / self.blend_duration).clamp(0.0, 1.0);

                    target_state = blend_states(&prev_state, &target_state, factor);
                    if self.blend_elapsed >= self.blend_duration {
                        self.previous_clip = None;
                    }
                }
            }
        } else {
            self.previous_clip = None;
        }

        Some(target_state)
    }

    pub fn attach_gltf_animations(&mut self, path: impl AsRef<std::path::Path>) -> Result<usize, String> {
        let loaded = load_gltf_clips(path)?;
        let count = loaded.len();
        for (_name, clip) in loaded {
            self.add_clip(clip);
        }
        Ok(count)
    }

    pub fn capture_keyframe(
        &mut self,
        pos: [f32; 3],
        rot: [f32; 3],
        scl: [f32; 3],
        color: Option<[f32; 4]>,
        emissive: Option<f32>,
    ) {
        if let Some(clip_name) = self.active_clip.as_ref() {
            if let Some(clip) = self.animations.get_mut(clip_name) {
                let t = self.current_time;
                // If a keyframe already exists very close in time, replace it
                if let Some(existing) = clip.keyframes.iter_mut().find(|k| (k.time - t).abs() < 0.02) {
                    existing.position = Some(pos);
                    existing.rotation = Some(rot);
                    existing.scale = Some(scl);
                    if color.is_some() {
                        existing.color = color;
                    }
                    if emissive.is_some() {
                        existing.emissive = emissive;
                    }
                } else {
                    clip.add_keyframe(Keyframe {
                        time: t,
                        position: Some(pos),
                        rotation: Some(rot),
                        scale: Some(scl),
                        color,
                        emissive,
                    });
                }
            }
        }
    }

    pub fn apply_to(
        state: &InterpolatedState,
        transform: &mut Transform,
        mesh_render: Option<&mut MeshRender>,
    ) {
        if let Some(pos) = state.position {
            transform.position = glam::Vec3::new(pos[0], pos[1], pos[2]);
        }
        if let Some(rot) = state.rotation {
            transform.rotation = glam::Vec3::new(rot[0], rot[1], rot[2]);
        }
        if let Some(scl) = state.scale {
            transform.scale = glam::Vec3::new(scl[0], scl[1], scl[2]);
        }
        if let Some(mr) = mesh_render {
            if let Some(col) = state.color {
                mr.color = col;
            }
            if let Some(em) = state.emissive {
                mr.emissive = em;
            }
        }
    }
}

pub fn create_preset_clip(preset_name: &str) -> Option<AnimationClip> {
    match preset_name.to_lowercase().as_str() {
        "hover_idle" | "hover" => {
            let mut clip = AnimationClip::new("hover_idle", 2.0);
            clip.add_keyframe(Keyframe {
                time: 0.0,
                position: Some([0.0, 0.0, 0.0]),
                rotation: None,
                scale: None,
                color: None,
                emissive: None,
            });
            clip.add_keyframe(Keyframe {
                time: 1.0,
                position: Some([0.0, 0.2, 0.0]),
                rotation: None,
                scale: None,
                color: None,
                emissive: None,
            });
            clip.add_keyframe(Keyframe {
                time: 2.0,
                position: Some([0.0, 0.0, 0.0]),
                rotation: None,
                scale: None,
                color: None,
                emissive: None,
            });
            Some(clip)
        }
        "spin_y" | "spin" => {
            let mut clip = AnimationClip::new("spin_y", 1.5);
            clip.add_keyframe(Keyframe {
                time: 0.0,
                position: None,
                rotation: Some([0.0, 0.0, 0.0]),
                scale: None,
                color: None,
                emissive: None,
            });
            clip.add_keyframe(Keyframe {
                time: 0.75,
                position: None,
                rotation: Some([0.0, 180.0, 0.0]),
                scale: None,
                color: None,
                emissive: None,
            });
            clip.add_keyframe(Keyframe {
                time: 1.5,
                position: None,
                rotation: Some([0.0, 360.0, 0.0]),
                scale: None,
                color: None,
                emissive: None,
            });
            Some(clip)
        }
        "pulse_glow" | "pulse" => {
            let mut clip = AnimationClip::new("pulse_glow", 0.8);
            clip.add_keyframe(Keyframe {
                time: 0.0,
                position: None,
                rotation: None,
                scale: Some([1.0, 1.0, 1.0]),
                color: None,
                emissive: Some(0.0),
            });
            clip.add_keyframe(Keyframe {
                time: 0.4,
                position: None,
                rotation: None,
                scale: Some([1.25, 1.25, 1.25]),
                color: None,
                emissive: Some(2.0),
            });
            clip.add_keyframe(Keyframe {
                time: 0.8,
                position: None,
                rotation: None,
                scale: Some([1.0, 1.0, 1.0]),
                color: None,
                emissive: Some(0.0),
            });
            Some(clip)
        }
        "recoil" => {
            let mut clip = AnimationClip::new("recoil", 0.25);
            clip.add_keyframe(Keyframe {
                time: 0.0,
                position: Some([0.0, 0.0, 0.0]),
                rotation: None,
                scale: None,
                color: None,
                emissive: None,
            });
            clip.add_keyframe(Keyframe {
                time: 0.06,
                position: Some([0.0, 0.0, -0.4]),
                rotation: None,
                scale: None,
                color: None,
                emissive: None,
            });
            clip.add_keyframe(Keyframe {
                time: 0.25,
                position: Some([0.0, 0.0, 0.0]),
                rotation: None,
                scale: None,
                color: None,
                emissive: None,
            });
            Some(clip)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfSkinData {
    pub name: String,
    pub joints: Vec<usize>,
    pub inverse_bind_matrices: Vec<[[f32; 4]; 4]>,
}

pub fn load_gltf_skins(path: impl AsRef<std::path::Path>) -> Result<Vec<GltfSkinData>, String> {
    let p = path.as_ref();
    let (document, buffers, _) = gltf::import(p)
        .map_err(|e| format!("Failed to import glTF '{}': {}", p.display(), e))?;

    let mut skins = Vec::new();
    for skin in document.skins() {
        let name = skin.name().unwrap_or("default_skin").to_string();
        let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
        let ibms: Vec<[[f32; 4]; 4]> = if let Some(iter) = reader.read_inverse_bind_matrices() {
            iter.collect()
        } else {
            Vec::new()
        };
        let joints: Vec<usize> = skin.joints().map(|j| j.index()).collect();
        skins.push(GltfSkinData {
            name,
            joints,
            inverse_bind_matrices: ibms,
        });
    }
    Ok(skins)
}

pub fn load_gltf_clips(path: impl AsRef<std::path::Path>) -> Result<HashMap<String, AnimationClip>, String> {
    let p = path.as_ref();
    let (document, buffers, _) = gltf::import(p)
        .map_err(|e| format!("Failed to import glTF '{}': {}", p.display(), e))?;

    let mut clips = HashMap::new();
    for anim in document.animations() {
        let anim_name = anim.name().unwrap_or("default").to_string();
        let mut clip = AnimationClip::new(&anim_name, 0.0);

        for channel in anim.channels() {
            let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
            let inputs: Vec<f32> = if let Some(inputs) = reader.read_inputs() {
                inputs.collect()
            } else {
                continue;
            };

            for &t in &inputs {
                if t > clip.duration {
                    clip.duration = t;
                }
            }

            match reader.read_outputs() {
                Some(gltf::animation::util::ReadOutputs::Translations(trans)) => {
                    for (time, pos) in inputs.iter().zip(trans) {
                        clip.add_keyframe(Keyframe {
                            time: *time,
                            position: Some(pos),
                            rotation: None,
                            scale: None,
                            color: None,
                            emissive: None,
                        });
                    }
                }
                Some(gltf::animation::util::ReadOutputs::Rotations(rotations)) => {
                    let quats: Vec<[f32; 4]> = rotations.into_f32().collect();
                    for (time, q) in inputs.iter().zip(quats) {
                        let quat = glam::Quat::from_array(q);
                        let (rx, ry, rz) = quat.to_euler(glam::EulerRot::XYZ);
                        clip.add_keyframe(Keyframe {
                            time: *time,
                            position: None,
                            rotation: Some([rx, ry, rz]),
                            scale: None,
                            color: None,
                            emissive: None,
                        });
                    }
                }
                Some(gltf::animation::util::ReadOutputs::Scales(scales)) => {
                    for (time, s) in inputs.iter().zip(scales) {
                        clip.add_keyframe(Keyframe {
                            time: *time,
                            position: None,
                            rotation: None,
                            scale: Some(s),
                            color: None,
                            emissive: None,
                        });
                    }
                }
                _ => {}
            }
        }
        clips.insert(anim_name, clip);
    }
    Ok(clips)
}

#[derive(Debug, Clone)]
pub struct SkinMeshData {
    pub base_positions: Vec<[f32; 3]>,
    pub base_normals: Vec<[f32; 3]>,
    pub joint_indices: Vec<[u16; 4]>,
    pub joint_weights: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub inverse_bind_matrices: Vec<glam::Mat4>,
}

pub fn compute_skinned_vertices(
    skin: &SkinMeshData,
    joint_matrices: &[glam::Mat4],
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let n = skin.base_positions.len();
    let mut out_pos = Vec::with_capacity(n);
    let mut out_norm = Vec::with_capacity(n);

    for i in 0..n {
        let p = glam::Vec3::from(skin.base_positions[i]);
        let nrm = glam::Vec3::from(skin.base_normals[i]);
        let ji = skin.joint_indices[i];
        let jw = skin.joint_weights[i];
        let w_sum = jw[0] + jw[1] + jw[2] + jw[3];

        if w_sum < 1e-5 || joint_matrices.is_empty() {
            out_pos.push(skin.base_positions[i]);
            out_norm.push(skin.base_normals[i]);
            continue;
        }

        let mut skinned_p = glam::Vec3::ZERO;
        let mut skinned_n = glam::Vec3::ZERO;
        for k in 0..4 {
            let w = jw[k] / w_sum;
            if w > 0.0 {
                let idx = (ji[k] as usize).min(joint_matrices.len() - 1);
                let m = joint_matrices[idx];
                skinned_p += m.transform_point3(p) * w;
                skinned_n += m.transform_vector3(nrm) * w;
            }
        }
        let nn = skinned_n.normalize_or_zero();
        out_pos.push([skinned_p.x, skinned_p.y, skinned_p.z]);
        out_norm.push(if nn.length_squared() > 0.0 { [nn.x, nn.y, nn.z] } else { skin.base_normals[i] });
    }
    (out_pos, out_norm)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimStateTransition {
    pub from: String,
    pub to: String,
    pub param: String,
    pub threshold: f32,
    pub greater: bool,
    pub blend_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimStateMachine {
    pub current_state: String,
    #[serde(default)]
    pub params: HashMap<String, f32>,
    #[serde(default)]
    pub transitions: Vec<AnimStateTransition>,
}

impl AnimStateMachine {
    pub fn new(initial_state: impl Into<String>) -> Self {
        Self {
            current_state: initial_state.into(),
            params: HashMap::new(),
            transitions: Vec::new(),
        }
    }

    pub fn set_param(&mut self, name: &str, value: f32) {
        self.params.insert(name.to_string(), value);
    }

    pub fn add_transition(&mut self, t: AnimStateTransition) {
        self.transitions.push(t);
    }

    pub fn evaluate(&mut self) -> Option<(String, f32)> {
        for t in &self.transitions {
            if t.from == self.current_state || t.from == "*" {
                let val = self.params.get(&t.param).copied().unwrap_or(0.0);
                let matched = if t.greater { val > t.threshold } else { val < t.threshold };
                if matched && t.to != self.current_state {
                    self.current_state = t.to.clone();
                    return Some((t.to.clone(), t.blend_time));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_clip_sample_interpolation() {
        let mut clip = AnimationClip::new("pulse", 2.0);
        clip.add_keyframe(Keyframe {
            time: 0.0,
            position: Some([0.0, 0.0, 0.0]),
            rotation: Some([0.0, 0.0, 0.0]),
            scale: Some([1.0, 1.0, 1.0]),
            color: Some([1.0, 0.0, 0.0, 1.0]),
            emissive: Some(0.0),
        });
        clip.add_keyframe(Keyframe {
            time: 2.0,
            position: Some([10.0, 20.0, 30.0]),
            rotation: Some([3.14, 0.0, 1.57]),
            scale: Some([2.0, 2.0, 2.0]),
            color: Some([0.0, 1.0, 0.0, 1.0]),
            emissive: Some(4.0),
        });

        // Sample at midpoint t = 1.0
        let state = clip.sample(1.0);
        assert_eq!(state.position, Some([5.0, 10.0, 15.0]));
        assert_eq!(state.scale, Some([1.5, 1.5, 1.5]));
        assert_eq!(state.emissive, Some(2.0));
        let col = state.color.unwrap();
        assert!((col[0] - 0.5).abs() < 0.001);
        assert!((col[1] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_animation_player_loop_and_stop() {
        let mut clip = AnimationClip::new("bounce", 1.0);
        clip.add_keyframe(Keyframe {
            time: 0.0,
            position: Some([0.0, 0.0, 0.0]),
            rotation: None,
            scale: None,
            color: None,
            emissive: None,
        });
        clip.add_keyframe(Keyframe {
            time: 1.0,
            position: Some([0.0, 5.0, 0.0]),
            rotation: None,
            scale: None,
            color: None,
            emissive: None,
        });

        let mut player = AnimationPlayer::new();
        player.add_clip(clip);
        player.play("bounce", true);
        assert!(player.is_playing());

        // Step by 0.5s -> midpoint
        let s1 = player.update(0.5).unwrap();
        assert_eq!(s1.position, Some([0.0, 2.5, 0.0]));
        assert_eq!(player.current_time, 0.5);

        // Step by 0.6s -> 1.1s -> looped to 0.1s
        let _s2 = player.update(0.6).unwrap();
        assert!((player.current_time - 0.1).abs() < 0.001);
        assert!(player.is_playing());

        // Stop
        player.stop();
        assert!(!player.is_playing());
        assert_eq!(player.current_time, 0.0);
    }

    #[test]
    fn test_animation_presets_and_player_play_preset() {
        let hover = create_preset_clip("hover_idle").unwrap();
        assert_eq!(hover.name, "hover_idle");
        assert_eq!(hover.duration, 2.0);

        let spin = create_preset_clip("spin_y").unwrap();
        assert_eq!(spin.duration, 1.5);

        let pulse = create_preset_clip("pulse_glow").unwrap();
        assert_eq!(pulse.duration, 0.8);

        let recoil = create_preset_clip("recoil").unwrap();
        assert_eq!(recoil.duration, 0.25);

        let mut player = AnimationPlayer::new();
        assert!(player.play_preset("pulse_glow"));
        assert!(player.is_playing());
        assert_eq!(player.active_clip.as_deref(), Some("pulse_glow"));

        // Step by 0.4s (midpoint of pulse)
        let s = player.update(0.4).unwrap();
        assert_eq!(s.scale, Some([1.25, 1.25, 1.25]));
        assert_eq!(s.emissive, Some(2.0));
    }

    #[test]
    fn test_animation_crossfade_blending() {
        let mut clip_a = AnimationClip::new("clip_a", 2.0);
        clip_a.add_keyframe(Keyframe {
            time: 0.0,
            position: Some([0.0, 0.0, 0.0]),
            rotation: None,
            scale: None,
            color: None,
            emissive: None,
        });
        clip_a.add_keyframe(Keyframe {
            time: 2.0,
            position: Some([0.0, 0.0, 0.0]),
            rotation: None,
            scale: None,
            color: None,
            emissive: None,
        });

        let mut clip_b = AnimationClip::new("clip_b", 2.0);
        clip_b.add_keyframe(Keyframe {
            time: 0.0,
            position: Some([10.0, 20.0, 30.0]),
            rotation: None,
            scale: None,
            color: None,
            emissive: None,
        });
        clip_b.add_keyframe(Keyframe {
            time: 2.0,
            position: Some([10.0, 20.0, 30.0]),
            rotation: None,
            scale: None,
            color: None,
            emissive: None,
        });

        let mut player = AnimationPlayer::new();
        player.add_clip(clip_a);
        player.add_clip(clip_b);

        // Start playing A
        player.play("clip_a", true);
        let _ = player.update(0.1);

        // Crossfade to B over 1.0s
        player.play_with_blend("clip_b", true, 1.0);
        assert_eq!(player.previous_clip.as_deref(), Some("clip_a"));
        assert_eq!(player.active_clip.as_deref(), Some("clip_b"));

        // Halfway through blend (0.5s): position should be exactly 50% between A and B
        let mid = player.update(0.5).unwrap();
        let pos = mid.position.unwrap();
        assert!((pos[0] - 5.0).abs() < 0.01);
        assert!((pos[1] - 10.0).abs() < 0.01);
        assert!((pos[2] - 15.0).abs() < 0.01);

        // Complete the blend (another 0.5s): should be 100% B
        let end = player.update(0.5).unwrap();
        let end_pos = end.position.unwrap();
        assert!((end_pos[0] - 10.0).abs() < 0.01);
        assert!((end_pos[1] - 20.0).abs() < 0.01);
        assert!((end_pos[2] - 30.0).abs() < 0.01);
        assert_eq!(player.previous_clip, None);
    }

    #[test]
    fn test_skeletal_skinning_and_state_machine() {
        let skin = SkinMeshData {
            base_positions: vec![[1.0, 0.0, 0.0]],
            base_normals: vec![[0.0, 1.0, 0.0]],
            joint_indices: vec![[0, 1, 0, 0]],
            joint_weights: vec![[0.5, 0.5, 0.0, 0.0]],
            uvs: vec![[0.0, 0.0]],
            colors: vec![[1.0; 4]],
            indices: vec![0],
            inverse_bind_matrices: vec![glam::Mat4::IDENTITY; 2],
        };
        let joints = [
            glam::Mat4::from_translation(glam::Vec3::new(0.0, 2.0, 0.0)),
            glam::Mat4::from_translation(glam::Vec3::new(0.0, 4.0, 0.0)),
        ];
        let (pos, _) = compute_skinned_vertices(&skin, &joints);
        assert!((pos[0][0] - 1.0).abs() < 1e-4);
        assert!((pos[0][1] - 3.0).abs() < 1e-4);

        let mut sm = AnimStateMachine::new("idle");
        sm.add_transition(AnimStateTransition {
            from: "idle".to_string(),
            to: "run".to_string(),
            param: "speed".to_string(),
            threshold: 0.5,
            greater: true,
            blend_time: 0.2,
        });
        sm.set_param("speed", 0.2);
        assert!(sm.evaluate().is_none());
        sm.set_param("speed", 1.5);
        let next = sm.evaluate();
        assert_eq!(next, Some(("run".to_string(), 0.2)));
    }
}
