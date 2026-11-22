use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::collections::HashSet;
use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::camera::{Camera, CameraMode};
use crate::ecs::{EntityMeta, MeshRender, Transform};
use crate::physics::{PhysicsEngine, RigidBodyComponent};
use crate::scripting::ScriptComponent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDef {
    pub mode: String, // "3d" or "2d"
    pub position: [f32; 3],
    pub target: [f32; 3],
    #[serde(default = "default_ortho_size")]
    pub ortho_size: f32,
}

fn default_ortho_size() -> f32 {
    10.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsDef {
    pub body: String,      // "dynamic" or "fixed"
    pub collider: String,  // "cuboid" or "ball"
    #[serde(default = "default_mass")]
    pub mass: f32,
    #[serde(default = "default_restitution")]
    pub restitution: f32,
}

fn default_mass() -> f32 {
    1.0
}

fn default_restitution() -> f32 {
    0.4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDef {
    pub id: String,
    pub mesh: String,
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3],
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
    pub color: [f32; 4],
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub physics: Option<PhysicsDef>,
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDef {
    pub camera: CameraDef,
    pub entities: Vec<EntityDef>,
}

impl Default for SceneDef {
    fn default() -> Self {
        Self {
            camera: CameraDef {
                mode: "3d".to_string(),
                position: [0.0, 3.5, 9.0],
                target: [0.0, 0.0, 0.0],
                ortho_size: 10.0,
            },
            entities: vec![
                EntityDef {
                    id: "cube_red".to_string(),
                    mesh: "cube".to_string(),
                    position: [-2.5, 0.0, 0.0],
                    rotation: [0.0, 0.4, 0.0],
                    scale: [1.3, 1.3, 1.3],
                    color: [0.95, 0.25, 0.25, 1.0],
                    script: None,
                    physics: None,
                },
                EntityDef {
                    id: "cube_green".to_string(),
                    mesh: "cube".to_string(),
                    position: [0.0, 0.0, 0.0],
                    rotation: [0.3, 0.5, 0.0],
                    scale: [1.3, 1.3, 1.3],
                    color: [0.25, 0.85, 0.35, 1.0],
                    script: None,
                    physics: None,
                },
                EntityDef {
                    id: "cube_blue".to_string(),
                    mesh: "cube".to_string(),
                    position: [2.5, 0.0, 0.0],
                    rotation: [-0.2, 0.6, 0.0],
                    scale: [1.3, 1.3, 1.3],
                    color: [0.25, 0.45, 0.95, 1.0],
                    script: None,
                    physics: None,
                },
            ],
        }
    }
}

pub fn load_scene_from_file(path: impl AsRef<Path>) -> Result<SceneDef, String> {
    let p = path.as_ref();
    let content = fs::read_to_string(p)
        .map_err(|e| format!("Error reading scene file '{}': {}", p.display(), e))?;

    let scene: SceneDef = serde_json::from_str(&content)
        .map_err(|e| format!("JSON syntax error in '{}': {}", p.display(), e))?;

    Ok(scene)
}

pub fn apply_scene_to_world(
    scene: &SceneDef,
    world: &mut hecs::World,
    camera: &mut Camera,
    physics: &mut PhysicsEngine,
    pressed_keys: Option<&Arc<RwLock<HashSet<String>>>>,
) {
    // 1. Update Camera
    let is_2d = scene.camera.mode.to_lowercase() == "2d";
    camera.mode = if is_2d {
        CameraMode::Orthographic
    } else {
        CameraMode::Perspective
    };
    camera.eye = Vec3::from(scene.camera.position);
    camera.target = Vec3::from(scene.camera.target);
    camera.ortho_size = scene.camera.ortho_size;

    // 2. Clear physics and rebuild entities in hecs::World
    physics.clear();
    world.clear();
    for entity in &scene.entities {
        let meta = EntityMeta {
            id: entity.id.clone(),
            name: entity.id.clone(),
        };
        let transform = Transform::new(
            Vec3::from(entity.position),
            Vec3::from(entity.rotation),
            Vec3::from(entity.scale),
        );
        let mesh = MeshRender {
            mesh_type: entity.mesh.clone(),
            color: entity.color,
        };

        let e = world.spawn((meta, transform, mesh));

        if let Some(phys_def) = &entity.physics {
            let handle = physics.add_body(&transform, phys_def);
            let _ = world.insert_one(e, RigidBodyComponent { handle });
        }

        if let Some(script_path) = &entity.script {
            let mut script_comp = ScriptComponent::new(script_path.clone());
            if let Some(keys) = pressed_keys {
                let _ = script_comp.init_or_reload(keys.clone());
            }
            let _ = world.insert_one(e, script_comp);
        }
    }
}
