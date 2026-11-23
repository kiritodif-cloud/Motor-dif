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
        }
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
        if self.animations.contains_key(clip_name) {
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

        Some(clip.sample(self.current_time))
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
}
