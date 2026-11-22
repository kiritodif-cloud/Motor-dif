use std::fs;
use std::path::Path;
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
    #[serde(default)]
    pub texture: Option<String>,
    pub color: [f32; 4],
    #[serde(default)]
    pub emissive: f32,
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
                    texture: None,
                    emissive: 0.0,
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
                    texture: None,
                    emissive: 0.0,
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
                    texture: None,
                    emissive: 0.0,
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

pub fn save_scene_to_file(path: impl AsRef<Path>, scene: &SceneDef) -> Result<(), String> {
    let p = path.as_ref();
    let json = serde_json::to_string_pretty(scene)
        .map_err(|e| format!("Error serializing scene to JSON: {}", e))?;

    fs::write(p, json)
        .map_err(|e| format!("Error writing scene file '{}': {}", p.display(), e))?;

    Ok(())
}

pub fn extract_scene_from_world(
    world: &hecs::World,
    camera: &Camera,
    physics: &PhysicsEngine,
) -> SceneDef {
    let camera_def = CameraDef {
        mode: match camera.mode {
            CameraMode::Perspective => "3d".to_string(),
            CameraMode::Orthographic => "2d".to_string(),
        },
        position: [camera.eye.x, camera.eye.y, camera.eye.z],
        target: [camera.target.x, camera.target.y, camera.target.z],
        ortho_size: camera.ortho_size,
    };

    let mut entities = Vec::new();
    for (meta, transform, mesh, script_opt, rb_opt) in world
        .query::<(
            &EntityMeta,
            &Transform,
            &MeshRender,
            Option<&ScriptComponent>,
            Option<&RigidBodyComponent>,
        )>()
        .iter()
    {
        let phys_def = rb_opt.and_then(|rb| {
            physics.rigid_body_set.get(rb.handle).map(|body| {
                // Find collider associated with body
                let collider_shape = body.colliders().first().and_then(|h| physics.collider_set.get(*h)).map(|c| {
                    if c.shape().as_ball().is_some() {
                        "ball".to_string()
                    } else {
                        "cuboid".to_string()
                    }
                }).unwrap_or_else(|| "cuboid".to_string());

                PhysicsDef {
                    body: if body.is_dynamic() { "dynamic".to_string() } else { "fixed".to_string() },
                    collider: collider_shape,
                    mass: body.mass(),
                    restitution: 0.5,
                }
            })
        });

        entities.push(EntityDef {
            id: meta.id.clone(),
            mesh: mesh.mesh_type.clone(),
            position: [transform.position.x, transform.position.y, transform.position.z],
            rotation: [transform.rotation.x, transform.rotation.y, transform.rotation.z],
            scale: [transform.scale.x, transform.scale.y, transform.scale.z],
            texture: mesh.texture.clone(),
            color: mesh.color,
            emissive: mesh.emissive,
            script: script_opt.map(|s| s.path.clone()),
            physics: phys_def,
        });
    }

    SceneDef {
        camera: camera_def,
        entities,
    }
}

pub fn apply_scene_to_world(
    scene: &SceneDef,
    world: &mut hecs::World,
    camera: &mut Camera,
    physics: &mut PhysicsEngine,
    lua_ctx: Option<&crate::scripting::LuaEngineContext>,
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
            texture: entity.texture.clone(),
            color: entity.color,
            emissive: entity.emissive,
        };

        let e = world.spawn((meta, transform, mesh));

        if let Some(phys_def) = &entity.physics {
            let handle = physics.add_body(&transform, phys_def);
            let _ = world.insert_one(e, RigidBodyComponent { handle });
        }

        if let Some(script_path) = &entity.script {
            let mut script_comp = ScriptComponent::new(script_path.clone());
            if let Some(ctx) = lua_ctx {
                let _ = script_comp.init_or_reload(ctx.clone());
            }
            let _ = world.insert_one(e, script_comp);
        }
    }
}
