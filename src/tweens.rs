use std::collections::VecDeque;
use glam::Vec3;
use crate::ecs::{EntityMeta, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutCubic,
    BounceOut,
}

impl Easing {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().replace('-', "_").as_str() {
            "ease_in_quad" | "in_quad" => Easing::EaseInQuad,
            "ease_out_quad" | "out_quad" => Easing::EaseOutQuad,
            "ease_in_out_cubic" | "in_out_cubic" | "smoothstep" => Easing::EaseInOutCubic,
            "bounce_out" | "bounce" => Easing::BounceOut,
            _ => Easing::Linear,
        }
    }

    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => t * (2.0 - t),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let f = 2.0 * t - 2.0;
                    0.5 * f * f * f + 1.0
                }
            }
            Easing::BounceOut => {
                let n1 = 7.5625;
                let d1 = 2.75;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t2 = t - 1.5 / d1;
                    n1 * t2 * t2 + 0.75
                } else if t < 2.5 / d1 {
                    let t2 = t - 2.25 / d1;
                    n1 * t2 * t2 + 0.9375
                } else {
                    let t2 = t - 2.625 / d1;
                    n1 * t2 * t2 + 0.984375
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiTween {
    pub element_id: String,
    pub property: String,
    pub start: f32,
    pub target: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: Easing,
}

pub fn get_ui_element_prop(elem: &crate::game_ui::UiElement, prop: &str) -> Option<f32> {
    match elem {
        crate::game_ui::UiElement::Panel { offset_x, offset_y, width, height, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => Some(*offset_x),
            "offset_y" | "y" => Some(*offset_y),
            "width" | "w" => Some(*width),
            "height" | "h" => Some(*height),
            _ => None,
        },
        crate::game_ui::UiElement::Bar { offset_x, offset_y, width, height, progress, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => Some(*offset_x),
            "offset_y" | "y" => Some(*offset_y),
            "width" | "w" => Some(*width),
            "height" | "h" => Some(*height),
            "progress" | "val" | "value" => Some(*progress),
            _ => None,
        },
        crate::game_ui::UiElement::Label { offset_x, offset_y, font_size, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => Some(*offset_x),
            "offset_y" | "y" => Some(*offset_y),
            "font_size" | "size" => Some(*font_size),
            _ => None,
        },
        crate::game_ui::UiElement::Button { offset_x, offset_y, width, height, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => Some(*offset_x),
            "offset_y" | "y" => Some(*offset_y),
            "width" | "w" => Some(*width),
            "height" | "h" => Some(*height),
            _ => None,
        },
        crate::game_ui::UiElement::Image { x, y, width, height, .. } => match prop.to_lowercase().as_str() {
            "x" | "offset_x" => Some(*x),
            "y" | "offset_y" => Some(*y),
            "width" | "w" => Some(*width),
            "height" | "h" => Some(*height),
            _ => None,
        },
        crate::game_ui::UiElement::Slot { x, y, size, .. } => match prop.to_lowercase().as_str() {
            "x" | "offset_x" => Some(*x),
            "y" | "offset_y" => Some(*y),
            "size" | "width" | "height" => Some(*size),
            _ => None,
        },
        crate::game_ui::UiElement::RttView { x, y, width, height, .. } => match prop.to_lowercase().as_str() {
            "x" | "offset_x" => Some(*x),
            "y" | "offset_y" => Some(*y),
            "width" | "w" => Some(*width),
            "height" | "h" => Some(*height),
            _ => None,
        },
    }
}

pub fn set_ui_element_prop(elem: &mut crate::game_ui::UiElement, prop: &str, val: f32) {
    match elem {
        crate::game_ui::UiElement::Panel { offset_x, offset_y, width, height, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => *offset_x = val,
            "offset_y" | "y" => *offset_y = val,
            "width" | "w" => *width = val,
            "height" | "h" => *height = val,
            _ => {},
        },
        crate::game_ui::UiElement::Bar { offset_x, offset_y, width, height, progress, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => *offset_x = val,
            "offset_y" | "y" => *offset_y = val,
            "width" | "w" => *width = val,
            "height" | "h" => *height = val,
            "progress" | "val" | "value" => *progress = val.clamp(0.0, 1.0),
            _ => {},
        },
        crate::game_ui::UiElement::Label { offset_x, offset_y, font_size, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => *offset_x = val,
            "offset_y" | "y" => *offset_y = val,
            "font_size" | "size" => *font_size = val.max(1.0),
            _ => {},
        },
        crate::game_ui::UiElement::Button { offset_x, offset_y, width, height, .. } => match prop.to_lowercase().as_str() {
            "offset_x" | "x" => *offset_x = val,
            "offset_y" | "y" => *offset_y = val,
            "width" | "w" => *width = val,
            "height" | "h" => *height = val,
            _ => {},
        },
        crate::game_ui::UiElement::Image { x, y, width, height, .. } => match prop.to_lowercase().as_str() {
            "x" | "offset_x" => *x = val,
            "y" | "offset_y" => *y = val,
            "width" | "w" => *width = val,
            "height" | "h" => *height = val,
            _ => {},
        },
        crate::game_ui::UiElement::Slot { x, y, size, .. } => match prop.to_lowercase().as_str() {
            "x" | "offset_x" => *x = val,
            "y" | "offset_y" => *y = val,
            "size" | "width" | "height" => *size = val.max(1.0),
            _ => {},
        },
        crate::game_ui::UiElement::RttView { x, y, width, height, .. } => match prop.to_lowercase().as_str() {
            "x" | "offset_x" => *x = val,
            "y" | "offset_y" => *y = val,
            "width" | "w" => *width = val,
            "height" | "h" => *height = val,
            _ => {},
        },
    }
}

pub fn update_ui_tweens(tweens: &mut Vec<UiTween>, elements: &mut [crate::game_ui::UiElement], dt: f32) {
    let mut i = 0;
    while i < tweens.len() {
        let tween = &mut tweens[i];
        tween.elapsed += dt;
        let t = if tween.duration <= 1e-4 {
            1.0
        } else {
            (tween.elapsed / tween.duration).clamp(0.0, 1.0)
        };
        let eased_t = tween.easing.apply(t);
        let cur_val = tween.start + (tween.target - tween.start) * eased_t;

        for elem in elements.iter_mut() {
            if elem.id() == tween.element_id {
                set_ui_element_prop(elem, &tween.property, cur_val);
            }
        }

        if tween.elapsed >= tween.duration {
            tweens.remove(i);
        } else {
            i += 1;
        }
    }
}

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
    fn test_easing_curves() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);

        assert_eq!(Easing::EaseInQuad.apply(0.5), 0.25);
        assert_eq!(Easing::EaseOutQuad.apply(0.5), 0.75);

        assert_eq!(Easing::EaseInOutCubic.apply(0.0), 0.0);
        assert_eq!(Easing::EaseInOutCubic.apply(0.5), 0.5);
        assert_eq!(Easing::EaseInOutCubic.apply(1.0), 1.0);

        assert_eq!(Easing::BounceOut.apply(0.0), 0.0);
        assert_eq!(Easing::BounceOut.apply(1.0), 1.0);
    }

    #[test]
    fn test_ui_property_tween() {
        let mut elements = vec![crate::game_ui::UiElement::Bar {
            id: "health_bar".to_string(),
            anchor: "bottom_center".to_string(),
            offset_x: 0.0,
            offset_y: 20.0,
            width: 200.0,
            height: 24.0,
            progress: 1.0,
            label: "HP".to_string(),
            color: [0.9, 0.2, 0.2],
        }];

        let mut tweens = vec![UiTween {
            element_id: "health_bar".to_string(),
            property: "progress".to_string(),
            start: 1.0,
            target: 0.2,
            duration: 1.0,
            elapsed: 0.0,
            easing: Easing::Linear,
        }];

        // Midway (0.5s)
        update_ui_tweens(&mut tweens, &mut elements, 0.5);
        assert_eq!(tweens.len(), 1);
        if let crate::game_ui::UiElement::Bar { progress, .. } = &elements[0] {
            assert!((*progress - 0.6).abs() < 1e-3);
        } else {
            panic!("Expected bar");
        }

        // Completion (0.5s more)
        update_ui_tweens(&mut tweens, &mut elements, 0.5);
        assert_eq!(tweens.len(), 0);
        if let crate::game_ui::UiElement::Bar { progress, .. } = &elements[0] {
            assert!((*progress - 0.2).abs() < 1e-3);
        } else {
            panic!("Expected bar");
        }
    }

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
