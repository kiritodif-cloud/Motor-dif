use std::collections::VecDeque;
use glam::Vec3;
use crate::ecs::{EntityMeta, Transform};

#[derive(Clone, Debug)]
pub struct Tween {
    pub entity_id: String,
    pub start: Vec3,
    pub target: Vec3,
    pub duration: f32,
    pub elapsed: f32,
    pub is_scale: bool,
}

#[derive(Clone, Debug)]
pub struct TweenRequest {
    pub entity_id: String,
    pub target: Vec3,
    pub duration: f32,
    pub is_scale: bool,
}

#[derive(Default)]
pub struct TweenManager {
    pub active_tweens: Vec<Tween>,
    pub pending_requests: VecDeque<TweenRequest>,
}

impl TweenManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn queue_position_tween(&mut self, entity_id: String, target: Vec3, duration: f32) {
        self.pending_requests.push_back(TweenRequest {
            entity_id,
            target,
            duration: duration.max(0.001),
            is_scale: false,
        });
    }

    pub fn queue_scale_tween(&mut self, entity_id: String, target: Vec3, duration: f32) {
        self.pending_requests.push_back(TweenRequest {
            entity_id,
            target,
            duration: duration.max(0.001),
            is_scale: true,
        });
    }

    pub fn update(&mut self, dt: f32, world: &mut hecs::World) {
        // 1. Process pending requests to initialize start values
        while let Some(req) = self.pending_requests.pop_front() {
            let mut start_val = None;
            for entity_ref in world.iter() {
                if let (Some(meta), Some(trans)) = (entity_ref.get::<&EntityMeta>(), entity_ref.get::<&Transform>()) {
                    if meta.id == req.entity_id {
                        start_val = Some(if req.is_scale { trans.scale } else { trans.position });
                        break;
                    }
                }
            }
            if let Some(start) = start_val {
                self.active_tweens.retain(|t| !(t.entity_id == req.entity_id && t.is_scale == req.is_scale));
                self.active_tweens.push(Tween {
                    entity_id: req.entity_id,
                    start,
                    target: req.target,
                    duration: req.duration,
                    elapsed: 0.0,
                    is_scale: req.is_scale,
                });
            }
        }

        // 2. Update active tweens
        let mut i = 0;
        while i < self.active_tweens.len() {
            let (entity_id, is_scale, current, finished) = {
                let tween = &mut self.active_tweens[i];
                tween.elapsed += dt;
                let t = if tween.duration <= 1e-4 {
                    1.0
                } else {
                    (tween.elapsed / tween.duration).clamp(0.0, 1.0)
                };
                let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep ease_in_out
                let current = tween.start.lerp(tween.target, smooth_t);
                let finished = tween.elapsed >= tween.duration;
                (tween.entity_id.clone(), tween.is_scale, current, finished)
            };

            let target_entity = {
                let mut found = None;
                for entity_ref in world.iter() {
                    if let Some(meta) = entity_ref.get::<&EntityMeta>() {
                        if meta.id == entity_id {
                            found = Some(entity_ref.entity());
                            break;
                        }
                    }
                }
                found
            };

            if let Some(entity) = target_entity {
                if let Ok(mut trans) = world.get::<&mut Transform>(entity) {
                    if is_scale {
                        trans.scale = current;
                    } else {
                        trans.position = current;
                    }
                }
            }

            if finished {
                self.active_tweens.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tween_manager_lifecycle() {
        let mut world = hecs::World::new();
        let meta = EntityMeta {
            id: "cube1".to_string(),
            name: "cube1".to_string(),
            parent: None,
        };
        let trans = Transform::new(Vec3::ZERO, Vec3::ZERO, Vec3::ONE);
        world.spawn((meta, trans));

        let mut mgr = TweenManager::new();
        mgr.queue_position_tween("cube1".to_string(), Vec3::new(10.0, 0.0, 0.0), 1.0);

        // Update midway
        mgr.update(0.5, &mut world);
        assert_eq!(mgr.active_tweens.len(), 1);

        for trans in world.query::<&Transform>().iter() {
            assert!(trans.position.x > 4.0 && trans.position.x < 6.0);
        }

        // Update to completion
        mgr.update(0.6, &mut world);
        assert_eq!(mgr.active_tweens.len(), 0);

        for trans in world.query::<&Transform>().iter() {
            assert!((trans.position.x - 10.0).abs() < 1e-4);
        }
    }
}
