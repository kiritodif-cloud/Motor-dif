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

impl Default for CameraDef {
    fn default() -> Self {
        Self {
            mode: "3d".to_string(),
            position: [0.0, 5.0, 10.0],
            target: [0.0, 0.0, 0.0],
            ortho_size: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsDef {
    pub body: String,      // "dynamic" or "fixed"
    pub collider: String,  // "cuboid" or "ball"
    #[serde(default = "default_mass")]
    pub mass: f32,
    #[serde(default = "default_restitution")]
    pub restitution: f32,
    #[serde(default)]
    pub is_sensor: Option<bool>,
}

fn default_mass() -> f32 {
    1.0
}

fn default_restitution() -> f32 {
    0.4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointLightDef {
    pub color: [f32; 3],
    #[serde(default = "default_light_intensity")]
    pub intensity: f32,
    #[serde(default = "default_light_radius")]
    pub radius: f32,
}

fn default_light_intensity() -> f32 {
    1.0
}

fn default_light_radius() -> f32 {
    8.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfxEmitterDef {
    pub preset_or_recipe: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub looping: bool,
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default)]
    pub recipe: Option<crate::vfx_forge::VfxRecipe>,
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
    #[serde(default)]
    pub parent: Option<String>,
    pub color: [f32; 4],
    #[serde(default)]
    pub emissive: f32,
    #[serde(default)]
    pub fresnel_color: Option<[f32; 3]>,
    #[serde(default)]
    pub fresnel_power: Option<f32>,
    #[serde(default)]
    pub blend_mode: Option<String>,
    #[serde(default)]
    pub wireframe: Option<bool>,
    #[serde(default)]
    pub uv_scroll: Option<[f32; 2]>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub physics: Option<PhysicsDef>,
    #[serde(default)]
    pub point_light: Option<PointLightDef>,
    #[serde(default)]
    pub animation_player: Option<crate::animation::AnimationPlayer>,
    #[serde(default)]
    pub procedural_rig: Option<crate::procedural_rig::ProceduralRig>,
    #[serde(default)]
    pub vfx_emitter: Option<VfxEmitterDef>,
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessDef {
    #[serde(default = "default_true")]
    pub bloom_enabled: bool,
    #[serde(default = "default_bloom_intensity")]
    pub bloom_intensity: f32,
    #[serde(default = "default_bloom_threshold")]
    pub bloom_threshold: f32,
    #[serde(default = "default_vignette_intensity")]
    pub vignette_intensity: f32,
    #[serde(default = "default_exposure")]
    pub exposure: f32,
    #[serde(default = "default_zero")]
    pub chromatic_aberration: f32,
}

fn default_true() -> bool { true }
fn default_bloom_intensity() -> f32 { 0.8 }
fn default_bloom_threshold() -> f32 { 0.8 }
fn default_vignette_intensity() -> f32 { 0.35 }
fn default_exposure() -> f32 { 1.0 }
fn default_zero() -> f32 { 0.0 }

impl Default for PostProcessDef {
    fn default() -> Self {
        Self {
            bloom_enabled: true,
            bloom_intensity: 0.8,
            bloom_threshold: 0.8,
            vignette_intensity: 0.35,
            exposure: 1.0,
            chromatic_aberration: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDef {
    #[serde(default = "default_sky_mode")]
    pub sky_mode: String,
    #[serde(default = "default_zenith")]
    pub zenith_color: [f32; 3],
    #[serde(default = "default_horizon")]
    pub horizon_color: [f32; 3],
    #[serde(default = "default_fog_color")]
    pub fog_color: [f32; 3],
    #[serde(default)]
    pub fog_density: f32,
    #[serde(default)]
    pub time_of_day: Option<f32>,
    #[serde(default)]
    pub fog_mode: Option<String>,
    #[serde(default)]
    pub fog_start: Option<f32>,
    #[serde(default)]
    pub fog_end: Option<f32>,
}

fn default_sky_mode() -> String { "gradient".to_string() }
fn default_zenith() -> [f32; 3] { [0.071, 0.082, 0.118] }
fn default_horizon() -> [f32; 3] { [0.133, 0.153, 0.212] }
fn default_fog_color() -> [f32; 3] { [0.071, 0.082, 0.118] }

impl Default for EnvironmentDef {
    fn default() -> Self {
        Self {
            sky_mode: default_sky_mode(),
            zenith_color: default_zenith(),
            horizon_color: default_horizon(),
            fog_color: default_fog_color(),
            fog_density: 0.0,
            time_of_day: None,
            fog_mode: None,
            fog_start: None,
            fog_end: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDef {
    pub camera: CameraDef,
    #[serde(default)]
    pub post_process: Option<PostProcessDef>,
    #[serde(default)]
    pub environment: Option<EnvironmentDef>,
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
            post_process: Some(PostProcessDef::default()),
            environment: Some(EnvironmentDef::default()),
            entities: vec![
                EntityDef {
                    id: "cube_red".to_string(),
                    mesh: "cube".to_string(),
                    position: [-2.5, 0.0, 0.0],
                    rotation: [0.0, 0.4, 0.0],
                    scale: [1.3, 1.3, 1.3],
                    color: [0.95, 0.25, 0.25, 1.0],
                    texture: None,
                    parent: None,
                    emissive: 0.0,
                    fresnel_color: None,
                    fresnel_power: None,
                    blend_mode: None,
                    wireframe: None,
                    uv_scroll: None,
                    script: None,
                    physics: None,
                    point_light: None,
                    animation_player: None,
                    procedural_rig: None,
                    vfx_emitter: None,
                },
                EntityDef {
                    id: "cube_green".to_string(),
                    mesh: "cube".to_string(),
                    position: [0.0, 0.0, 0.0],
                    rotation: [0.3, 0.5, 0.0],
                    scale: [1.3, 1.3, 1.3],
                    color: [0.25, 0.85, 0.35, 1.0],
                    texture: None,
                    parent: None,
                    emissive: 0.0,
                    fresnel_color: None,
                    fresnel_power: None,
                    blend_mode: None,
                    wireframe: None,
                    uv_scroll: None,
                    script: None,
                    physics: None,
                    point_light: None,
                    animation_player: None,
                    procedural_rig: None,
                    vfx_emitter: None,
                },
                EntityDef {
                    id: "cube_blue".to_string(),
                    mesh: "cube".to_string(),
                    position: [2.5, 0.0, 0.0],
                    rotation: [-0.2, 0.6, 0.0],
                    scale: [1.3, 1.3, 1.3],
                    color: [0.25, 0.45, 0.95, 1.0],
                    texture: None,
                    parent: None,
                    emissive: 0.0,
                    fresnel_color: None,
                    fresnel_power: None,
                    blend_mode: None,
                    wireframe: None,
                    uv_scroll: None,
                    script: None,
                    physics: None,
                    point_light: None,
                    animation_player: None,
                    procedural_rig: None,
                    vfx_emitter: None,
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
    for er in world.iter() {
        if let (Some(meta), Some(transform), Some(mesh)) = (
            er.get::<&EntityMeta>(),
            er.get::<&Transform>(),
            er.get::<&MeshRender>(),
        ) {
            let script_opt = er.get::<&ScriptComponent>();
            let rb_opt = er.get::<&RigidBodyComponent>();
            let light_comp_opt = er.get::<&crate::ecs::PointLight>();
            let anim_comp_opt = er.get::<&crate::animation::AnimationPlayer>();
            let rig_comp_opt = er.get::<&crate::procedural_rig::ProceduralRig>();
            let vfx_comp_opt = er.get::<&crate::vfx_forge::VfxEmitterComponent>();

            let phys_def = rb_opt.as_deref().and_then(|rb| {
                physics.rigid_body_set.get(rb.handle).map(|body| {
                    let collider_shape = body.colliders().first().and_then(|h| physics.collider_set.get(*h)).map(|c| {
                        if c.shape().as_ball().is_some() {
                            "ball".to_string()
                        } else {
                            "cuboid".to_string()
                        }
                    }).unwrap_or_else(|| "cuboid".to_string());

                    let is_sensor = body.colliders().first().and_then(|h| physics.collider_set.get(*h)).map(|c| c.is_sensor());

                    PhysicsDef {
                        body: if body.is_dynamic() { "dynamic".to_string() } else { "fixed".to_string() },
                        collider: collider_shape,
                        mass: body.mass(),
                        restitution: 0.5,
                        is_sensor,
                    }
                })
            });

            let light_opt = light_comp_opt.as_deref().map(|l| PointLightDef {
                color: l.color,
                intensity: l.intensity,
                radius: l.radius,
            });

            let anim_opt = anim_comp_opt.as_deref().cloned();
            let rig_opt = rig_comp_opt.as_deref().cloned();
            let vfx_opt = vfx_comp_opt.as_deref().map(|v| VfxEmitterDef {
                preset_or_recipe: v.recipe.name.clone(),
                enabled: v.enabled,
                looping: v.looping,
                offset: v.offset,
                recipe: Some(v.recipe.clone()),
            });

            entities.push(EntityDef {
                id: meta.id.clone(),
                mesh: mesh.mesh_type.clone(),
                position: [transform.position.x, transform.position.y, transform.position.z],
                rotation: [transform.rotation.x, transform.rotation.y, transform.rotation.z],
                scale: [transform.scale.x, transform.scale.y, transform.scale.z],
                texture: mesh.texture.clone(),
                parent: meta.parent.clone(),
                color: mesh.color,
                emissive: mesh.emissive,
                fresnel_color: mesh.fresnel_color,
                fresnel_power: mesh.fresnel_power,
                blend_mode: mesh.blend_mode.clone(),
                wireframe: mesh.wireframe,
                uv_scroll: mesh.uv_scroll,
                script: script_opt.as_deref().map(|s| s.path.clone()),
                physics: phys_def,
                point_light: light_opt,
                animation_player: anim_opt,
                procedural_rig: rig_opt,
                vfx_emitter: vfx_opt,
            });
        }
    }

    SceneDef {
        camera: camera_def,
        post_process: None,
        environment: None,
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
            parent: entity.parent.clone(),
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
            fresnel_color: entity.fresnel_color,
            fresnel_power: entity.fresnel_power,
            blend_mode: entity.blend_mode.clone(),
            wireframe: entity.wireframe,
            uv_scroll: entity.uv_scroll,
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

        if let Some(light_def) = &entity.point_light {
            let _ = world.insert_one(
                e,
                crate::ecs::PointLight {
                    color: light_def.color,
                    intensity: light_def.intensity,
                    radius: light_def.radius,
                },
            );
        }

        if let Some(anim_player) = &entity.animation_player {
            let _ = world.insert_one(e, anim_player.clone());
        }

        if let Some(rig) = &entity.procedural_rig {
            let _ = world.insert_one(e, rig.clone());
        }

        if let Some(emitter_def) = &entity.vfx_emitter {
            let recipe = if let Some(r) = &emitter_def.recipe {
                r.clone()
            } else if let Some(preset) = crate::vfx_forge::get_vfx_preset(&emitter_def.preset_or_recipe) {
                preset
            } else if let Ok(custom) = serde_json::from_str::<crate::vfx_forge::VfxRecipe>(&emitter_def.preset_or_recipe) {
                custom
            } else {
                crate::vfx_forge::preset_fire()
            };
            let comp = crate::vfx_forge::VfxEmitterComponent {
                recipe,
                enabled: emitter_def.enabled,
                looping: emitter_def.looping,
                offset: emitter_def.offset,
                accumulator: 0.0,
                burst_fired: false,
            };
            let _ = world.insert_one(e, comp);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabDef {
    pub root: EntityDef,
    #[serde(default)]
    pub children: Vec<EntityDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum PrefabFormat {
    Hierarchy(PrefabDef),
    Single(EntityDef),
}

pub fn load_prefab_def_from_file<P: AsRef<Path>>(path: P) -> Result<PrefabDef, String> {
    let content = fs::read_to_string(path.as_ref())
        .map_err(|e| format!("Error reading prefab file: {}", e))?;
    match serde_json::from_str::<PrefabFormat>(&content) {
        Ok(PrefabFormat::Hierarchy(h)) => Ok(h),
        Ok(PrefabFormat::Single(root)) => Ok(PrefabDef {
            root,
            children: Vec::new(),
        }),
        Err(e) => Err(format!("Error parsing prefab JSON: {}", e)),
    }
}

pub fn load_prefab_from_file<P: AsRef<Path>>(path: P) -> Result<EntityDef, String> {
    let def = load_prefab_def_from_file(path)?;
    Ok(def.root)
}

pub fn save_prefab_to_file<P: AsRef<Path>>(path: P, def: &PrefabDef) -> Result<(), String> {
    let json = serde_json::to_string_pretty(def)
        .map_err(|e| format!("Error serializing prefab: {}", e))?;
    if let Some(parent) = path.as_ref().parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path.as_ref(), json)
        .map_err(|e| format!("Error writing prefab file: {}", e))?;
    Ok(())
}

pub fn save_entity_as_prefab_to_file<P: AsRef<Path>>(path: P, def: &EntityDef) -> Result<(), String> {
    save_prefab_to_file(path, &PrefabDef { root: def.clone(), children: Vec::new() })
}

pub fn extract_prefab_hierarchy(
    world: &hecs::World,
    physics: &PhysicsEngine,
    root_entity: hecs::Entity,
) -> Option<PrefabDef> {
    let mut root = extract_entity_def(world, physics, root_entity)?;
    let mut children = Vec::new();
    let root_id = root.id.clone();
    root.parent = None; // Root of prefab has no parent relative to prefab

    let mut pending_parents = vec![root_id];
    while let Some(current_parent_id) = pending_parents.pop() {
        for er in world.iter() {
            if er.entity() == root_entity {
                continue;
            }
            if let Some(meta) = er.get::<&EntityMeta>() {
                if meta.parent.as_deref() == Some(&current_parent_id) {
                    if let Some(child_def) = extract_entity_def(world, physics, er.entity()) {
                        pending_parents.push(child_def.id.clone());
                        children.push(child_def);
                    }
                }
            }
        }
    }

    Some(PrefabDef { root, children })
}

pub fn extract_entity_def(
    world: &hecs::World,
    physics: &PhysicsEngine,
    entity: hecs::Entity,
) -> Option<EntityDef> {
    let meta = world.get::<&EntityMeta>(entity).ok()?;
    let transform = world.get::<&Transform>(entity).ok()?;
    let mesh = world.get::<&MeshRender>(entity).ok()?;
    let script = world.get::<&ScriptComponent>(entity).ok().map(|s| s.path.clone());
    let anim_opt = world.get::<&crate::animation::AnimationPlayer>(entity).ok().map(|a| (*a).clone());

    let phys_def = if let Ok(rb) = world.get::<&RigidBodyComponent>(entity) {
        if let Some(body) = physics.rigid_body_set.get(rb.handle) {
            let body_type = if body.is_dynamic() { "dynamic" } else { "fixed" };
            let collider_type = if let Some(&c_handle) = body.colliders().first() {
                if let Some(collider) = physics.collider_set.get(c_handle) {
                    if collider.shape().as_ball().is_some() {
                        "ball"
                    } else {
                        "cuboid"
                    }
                } else {
                    "cuboid"
                }
            } else {
                "cuboid"
            };

            let restitution = if let Some(&c_handle) = body.colliders().first() {
                physics.collider_set.get(c_handle).map(|c| c.restitution()).unwrap_or(0.4)
            } else {
                0.4
            };

            let is_sensor = if let Some(&c_handle) = body.colliders().first() {
                physics.collider_set.get(c_handle).map(|c| c.is_sensor())
            } else {
                None
            };

            Some(PhysicsDef {
                body: body_type.to_string(),
                collider: collider_type.to_string(),
                mass: body.mass(),
                restitution,
                is_sensor,
            })
        } else {
            None
        }
    } else {
        None
    };

    let point_light = world.get::<&crate::ecs::PointLight>(entity).ok().map(|l| PointLightDef {
        color: l.color,
        intensity: l.intensity,
        radius: l.radius,
    });

    let rig_opt = world.get::<&crate::procedural_rig::ProceduralRig>(entity).ok().map(|r| (*r).clone());
    let vfx_opt = world.get::<&crate::vfx_forge::VfxEmitterComponent>(entity).ok().map(|v| VfxEmitterDef {
        preset_or_recipe: v.recipe.name.clone(),
        enabled: v.enabled,
        looping: v.looping,
        offset: v.offset,
        recipe: Some(v.recipe.clone()),
    });

    Some(EntityDef {
        id: meta.id.clone(),
        mesh: mesh.mesh_type.clone(),
        position: [transform.position.x, transform.position.y, transform.position.z],
        rotation: [transform.rotation.x, transform.rotation.y, transform.rotation.z],
        scale: [transform.scale.x, transform.scale.y, transform.scale.z],
        texture: mesh.texture.clone(),
        parent: meta.parent.clone(),
        color: mesh.color,
        emissive: mesh.emissive,
        fresnel_color: mesh.fresnel_color,
        fresnel_power: mesh.fresnel_power,
        blend_mode: mesh.blend_mode.clone(),
        wireframe: mesh.wireframe,
        uv_scroll: mesh.uv_scroll,
        script,
        physics: phys_def,
        point_light,
        animation_player: anim_opt,
        procedural_rig: rig_opt,
        vfx_emitter: vfx_opt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefab_serialization() {
        let prefab_path = Path::new("assets/prefabs/bouncing_orb.json");
        assert!(prefab_path.exists());

        let def = load_prefab_from_file(prefab_path).expect("Failed to load bouncing_orb.json");
        assert_eq!(def.id, "bouncing_orb");
        assert_eq!(def.mesh, "sphere");
        assert!(def.texture.is_some());
        assert!(def.physics.is_some());
        let phys = def.physics.unwrap();
        assert_eq!(phys.collider, "ball");
        assert!(phys.restitution > 0.9);
    }

    #[test]
    fn test_load_showcase_v10() {
        let scene = load_scene_from_file("assets/showcase_v10.json");
        assert!(scene.is_ok(), "Failed to load showcase_v10: {:?}", scene.err());
        let s = scene.unwrap();
        assert_eq!(s.entities.len(), 7);
    }

    #[test]
    fn test_entity_def_parent() {
        let def = EntityDef {
            id: "child".to_string(),
            mesh: "cube".to_string(),
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            texture: None,
            parent: Some("parent_node".to_string()),
            color: [1.0, 1.0, 1.0, 1.0],
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: Some(PointLightDef {
                color: [1.0, 0.5, 0.2],
                intensity: 3.5,
                radius: 12.0,
            }),
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("\"parent\":\"parent_node\""));
        assert!(json.contains("\"point_light\""));
        let decoded: EntityDef = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.parent, Some("parent_node".to_string()));
        assert_eq!(decoded.point_light.as_ref().unwrap().intensity, 3.5);
    }

    #[test]
    fn test_entity_def_material_properties() {
        let def = EntityDef {
            id: "styled_mat".to_string(),
            mesh: "torus".to_string(),
            position: [0.0, 1.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            texture: None,
            parent: None,
            color: [0.2, 0.7, 1.0, 1.0],
            emissive: 3.5,
            fresnel_color: Some([0.0, 1.0, 1.0]),
            fresnel_power: Some(3.0),
            blend_mode: Some("additive".to_string()),
            wireframe: Some(true),
            uv_scroll: Some([0.1, -0.2]),
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("\"blend_mode\":\"additive\""));
        assert!(json.contains("\"wireframe\":true"));
        assert!(json.contains("\"emissive\":3.5"));
        let decoded: EntityDef = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.blend_mode, Some("additive".to_string()));
        assert_eq!(decoded.wireframe, Some(true));
        assert_eq!(decoded.fresnel_power, Some(3.0));
    }

    #[test]
    fn test_entity_def_vfx_emitter() {
        let def = EntityDef {
            id: "magic_torch".to_string(),
            mesh: "cylinder".to_string(),
            position: [0.0, 0.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.2, 1.0, 0.2],
            texture: None,
            parent: None,
            color: [0.6, 0.4, 0.2, 1.0],
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: Some(VfxEmitterDef {
                preset_or_recipe: "fire".to_string(),
                enabled: true,
                looping: true,
                offset: [0.0, 0.5, 0.0],
                recipe: None,
            }),
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("\"vfx_emitter\""));
        assert!(json.contains("\"preset_or_recipe\":\"fire\""));
        let decoded: EntityDef = serde_json::from_str(&json).unwrap();
        assert!(decoded.vfx_emitter.is_some());
        let emitter = decoded.vfx_emitter.unwrap();
        assert_eq!(emitter.preset_or_recipe, "fire");
        assert!(emitter.enabled);
    }

    #[test]
    fn test_environment_def_serialization() {
        let env = EnvironmentDef {
            sky_mode: "space".to_string(),
            zenith_color: [0.05, 0.05, 0.15],
            horizon_color: [0.01, 0.01, 0.03],
            fog_color: [0.02, 0.02, 0.04],
            fog_density: 0.015,
            time_of_day: Some(14.5),
            fog_mode: Some("exponential".to_string()),
            fog_start: Some(5.0),
            fog_end: Some(80.0),
        };
        let json = serde_json::to_string(&env).unwrap();
        let decoded: EnvironmentDef = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.sky_mode, "space");
        assert_eq!(decoded.fog_density, 0.015);
        assert_eq!(decoded.time_of_day, Some(14.5));

        let def_env = EnvironmentDef::default();
        assert_eq!(def_env.sky_mode, "gradient");
        assert_eq!(def_env.zenith_color, [0.071, 0.082, 0.118]);
        assert_eq!(def_env.horizon_color, [0.133, 0.153, 0.212]);
    }

    #[test]
    fn test_load_showcase_v11() {
        let p = Path::new("assets/showcase_v11.json");
        assert!(p.exists());
        let scene = load_scene_from_file(p).expect("Failed to parse showcase_v11.json");
        assert_eq!(scene.entities.len(), 7);
        let core = scene.entities.iter().find(|e| e.id == "celestial_core").unwrap();
        assert!(core.animation_player.is_some());
        let anim_player = core.animation_player.as_ref().unwrap();
        assert_eq!(anim_player.active_clip, Some("pulse_loop".to_string()));
    }

    #[test]
    fn test_load_showcase_v12() {
        let p = Path::new("assets/showcase_v12.json");
        assert!(p.exists());
        let scene = load_scene_from_file(p).expect("Failed to parse showcase_v12.json");
        assert_eq!(scene.entities.len(), 12);
        let hero = scene.entities.iter().find(|e| e.id == "hero_agent").unwrap();
        assert_eq!(hero.script, Some("assets/scripts/showcase_v12.lua".to_string()));
    }

    #[test]
    fn test_load_showcase_v18_terrain() {
        let p = Path::new("assets/showcase_v18_terrain.json");
        assert!(p.exists());
        let scene = load_scene_from_file(p).expect("Failed to parse showcase_v18_terrain.json");
        assert!(scene.entities.len() >= 20);
        let terrain = scene.entities.iter().find(|e| e.id == "alien_terrain").unwrap();
        assert_eq!(terrain.mesh, "assets/models/terrain_demo.glb");
        let beacon = scene.entities.iter().find(|e| e.id == "plasma_beacon").unwrap();
        assert!(beacon.point_light.is_some());
        let scout = scene.entities.iter().find(|e| e.id == "terrain_scout").unwrap();
        assert_eq!(scout.script, Some("assets/scripts/terrain_scout.lua".to_string()));
    }

    #[test]
    fn test_load_showcase_v19_rig() {
        let p = Path::new("assets/showcase_v19_rig.json");
        if p.exists() {
            let scene = load_scene_from_file(p).expect("Failed to parse showcase_v19_rig.json");
            assert!(scene.entities.len() >= 25);
            let mech_root = scene.entities.iter().find(|e| e.id == "mech_root").unwrap();
            assert!(mech_root.procedural_rig.is_some());
            assert_eq!(mech_root.script, Some("assets/scripts/rig_showcase.lua".to_string()));
            let beacon = scene.entities.iter().find(|e| e.id == "rig_beacon").unwrap();
            assert!(beacon.point_light.is_some());
        }
    }

    #[test]
    fn test_load_showcase_v20_vfx() {
        let p = Path::new("assets/showcase_v20_vfx.json");
        if p.exists() {
            let scene = load_scene_from_file(p).expect("Failed to parse showcase_v20_vfx.json");
            assert!(scene.entities.len() >= 15);
            let portal = scene.entities.iter().find(|e| e.id == "portal_emitter").unwrap();
            assert!(portal.vfx_emitter.is_some());
            assert_eq!(portal.vfx_emitter.as_ref().unwrap().preset_or_recipe, "magic_portal");
            let torch = scene.entities.iter().find(|e| e.id == "torch_l_fire").unwrap();
            assert!(torch.vfx_emitter.is_some());
            assert_eq!(torch.vfx_emitter.as_ref().unwrap().preset_or_recipe, "fire");
            let core = scene.entities.iter().find(|e| e.id == "altar_core").unwrap();
            assert_eq!(core.script, Some("assets/scripts/vfx_showcase.lua".to_string()));
            assert!(core.point_light.is_some());
        }
    }

    #[test]
    fn test_prefab_hierarchy_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let prefab_file = temp_dir.join("test_turret.prefab.json");

        let root = EntityDef {
            id: "turret_base".to_string(),
            mesh: "cylinder".to_string(),
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            texture: None,
            parent: None,
            color: [0.2, 0.2, 0.2, 1.0],
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        };

        let child = EntityDef {
            id: "turret_cannon".to_string(),
            mesh: "cube".to_string(),
            position: [0.0, 1.5, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.3, 0.3, 1.2],
            texture: None,
            parent: Some("turret_base".to_string()),
            color: [0.8, 0.1, 0.1, 1.0],
            emissive: 0.5,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: None,
            wireframe: None,
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        };

        let prefab = PrefabDef {
            root,
            children: vec![child],
        };

        save_prefab_to_file(&prefab_file, &prefab).expect("Save prefab failed");
        let loaded = load_prefab_def_from_file(&prefab_file).expect("Load prefab failed");
        assert_eq!(loaded.root.id, "turret_base");
        assert_eq!(loaded.children.len(), 1);
        assert_eq!(loaded.children[0].id, "turret_cannon");
        assert_eq!(loaded.children[0].parent.as_deref(), Some("turret_base"));

        let _ = std::fs::remove_file(prefab_file);
    }
}
