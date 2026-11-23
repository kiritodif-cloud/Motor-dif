use std::path::Path;
use crate::assets::{CameraDef, EntityDef, EnvironmentDef, PhysicsDef, PointLightDef, PostProcessDef, SceneDef};

pub fn create_template_scene(template_name: &str) -> Result<SceneDef, String> {
    match template_name.to_lowercase().as_str() {
        "empty_3d" => Ok(template_empty_3d()),
        "top_down_2d" => Ok(template_top_down_2d()),
        "platformer_3d" => Ok(template_platformer_3d()),
        "space_arena" => Ok(template_space_arena()),
        _ => Err(format!(
            "Plantilla desconocida '{}'. Disponibles: 'empty_3d', 'top_down_2d', 'platformer_3d', 'space_arena'",
            template_name
        )),
    }
}

pub fn generate_and_save_template(template_name: &str, save_path: impl AsRef<Path>) -> Result<SceneDef, String> {
    let scene = create_template_scene(template_name)?;
    crate::assets::save_scene_to_file(save_path.as_ref(), &scene)?;
    Ok(scene)
}

fn template_empty_3d() -> SceneDef {
    SceneDef {
        camera: CameraDef {
            mode: "3d".to_string(),
            position: [0.0, 3.5, 7.0],
            target: [0.0, 1.0, 0.0],
            ortho_size: 10.0,
        },
        post_process: Some(PostProcessDef::default()),
        environment: Some(EnvironmentDef {
            sky_mode: "gradient".to_string(),
            zenith_color: [0.071, 0.082, 0.118],
            horizon_color: [0.133, 0.153, 0.212],
            fog_color: [0.08, 0.09, 0.13],
            fog_density: 0.008,
        }),
        entities: vec![
            EntityDef {
                id: "ground_plane".to_string(),
                mesh: "plane".to_string(),
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [20.0, 1.0, 20.0],
                color: [0.18, 0.20, 0.26, 1.0],
                texture: None,
                parent: None,
                emissive: 0.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: Some(false),
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "fixed".to_string(),
                    collider: "cuboid".to_string(),
                    mass: 0.0,
                    restitution: 0.4,
                    is_sensor: None,
                }),
                point_light: None,
                animation_player: None,
            },
            EntityDef {
                id: "sun_light".to_string(),
                mesh: "sphere".to_string(),
                position: [0.0, 5.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [0.3, 0.3, 0.3],
                color: [1.0, 0.95, 0.85, 1.0],
                texture: None,
                parent: None,
                emissive: 3.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: None,
                uv_scroll: None,
                script: None,
                physics: None,
                point_light: Some(PointLightDef {
                    color: [1.0, 0.95, 0.85],
                    intensity: 4.5,
                    radius: 25.0,
                }),
                animation_player: None,
            },
            EntityDef {
                id: "starter_cube".to_string(),
                mesh: "cube".to_string(),
                position: [0.0, 1.5, 0.0],
                rotation: [0.0, 0.4, 0.0],
                scale: [1.2, 1.2, 1.2],
                color: [0.22, 0.75, 0.98, 1.0],
                texture: None,
                parent: None,
                emissive: 0.2,
                fresnel_color: Some([0.4, 0.85, 1.0]),
                fresnel_power: Some(2.5),
                blend_mode: Some("opaque".to_string()),
                wireframe: Some(false),
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "dynamic".to_string(),
                    collider: "cuboid".to_string(),
                    mass: 1.0,
                    restitution: 0.5,
                    is_sensor: None,
                }),
                point_light: None,
                animation_player: None,
            },
        ],
    }
}

fn template_top_down_2d() -> SceneDef {
    SceneDef {
        camera: CameraDef {
            mode: "2d".to_string(),
            position: [0.0, 12.0, 0.0],
            target: [0.0, 0.0, 0.0],
            ortho_size: 14.0,
        },
        post_process: Some(PostProcessDef::default()),
        environment: Some(EnvironmentDef {
            sky_mode: "solid".to_string(),
            zenith_color: [0.05, 0.06, 0.09],
            horizon_color: [0.05, 0.06, 0.09],
            fog_color: [0.05, 0.06, 0.09],
            fog_density: 0.0,
        }),
        entities: vec![
            EntityDef {
                id: "arena_floor".to_string(),
                mesh: "quad".to_string(),
                position: [0.0, 0.0, 0.0],
                rotation: [-1.5708, 0.0, 0.0],
                scale: [24.0, 24.0, 1.0],
                color: [0.12, 0.14, 0.18, 1.0],
                texture: None,
                parent: None,
                emissive: 0.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: Some(false),
                uv_scroll: None,
                script: None,
                physics: None,
                point_light: None,
                animation_player: None,
            },
            EntityDef {
                id: "player_ship".to_string(),
                mesh: "torus".to_string(),
                position: [0.0, 0.1, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 0.35],
                color: [0.2, 0.85, 0.4, 1.0],
                texture: None,
                parent: None,
                emissive: 1.5,
                fresnel_color: Some([0.3, 1.0, 0.6]),
                fresnel_power: Some(2.0),
                blend_mode: Some("opaque".to_string()),
                wireframe: None,
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "dynamic".to_string(),
                    collider: "ball".to_string(),
                    mass: 1.0,
                    restitution: 0.2,
                    is_sensor: None,
                }),
                point_light: Some(PointLightDef {
                    color: [0.3, 0.9, 0.5],
                    intensity: 3.5,
                    radius: 10.0,
                }),
                animation_player: None,
            },
            EntityDef {
                id: "top_down_light".to_string(),
                mesh: "sphere".to_string(),
                position: [0.0, 6.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [0.2, 0.2, 0.2],
                color: [1.0, 1.0, 1.0, 1.0],
                texture: None,
                parent: None,
                emissive: 2.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: None,
                uv_scroll: None,
                script: None,
                physics: None,
                point_light: Some(PointLightDef {
                    color: [0.9, 0.95, 1.0],
                    intensity: 4.0,
                    radius: 20.0,
                }),
                animation_player: None,
            },
        ],
    }
}

fn template_platformer_3d() -> SceneDef {
    SceneDef {
        camera: CameraDef {
            mode: "3d".to_string(),
            position: [0.0, 4.5, 12.0],
            target: [0.0, 2.0, 0.0],
            ortho_size: 10.0,
        },
        post_process: Some(PostProcessDef::default()),
        environment: Some(EnvironmentDef {
            sky_mode: "gradient".to_string(),
            zenith_color: [0.12, 0.16, 0.28],
            horizon_color: [0.25, 0.32, 0.45],
            fog_color: [0.20, 0.26, 0.38],
            fog_density: 0.007,
        }),
        entities: vec![
            EntityDef {
                id: "platform_base".to_string(),
                mesh: "cube".to_string(),
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [8.0, 0.8, 4.0],
                color: [0.25, 0.30, 0.38, 1.0],
                texture: None,
                parent: None,
                emissive: 0.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: Some(false),
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "fixed".to_string(),
                    collider: "cuboid".to_string(),
                    mass: 0.0,
                    restitution: 0.2,
                    is_sensor: None,
                }),
                point_light: None,
                animation_player: None,
            },
            EntityDef {
                id: "platform_step1".to_string(),
                mesh: "cube".to_string(),
                position: [4.5, 1.8, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [3.5, 0.6, 3.5],
                color: [0.30, 0.36, 0.46, 1.0],
                texture: None,
                parent: None,
                emissive: 0.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: Some(false),
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "fixed".to_string(),
                    collider: "cuboid".to_string(),
                    mass: 0.0,
                    restitution: 0.2,
                    is_sensor: None,
                }),
                point_light: None,
                animation_player: None,
            },
            EntityDef {
                id: "platform_step2".to_string(),
                mesh: "cube".to_string(),
                position: [8.5, 3.6, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [3.5, 0.6, 3.5],
                color: [0.35, 0.42, 0.54, 1.0],
                texture: None,
                parent: None,
                emissive: 0.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: Some(false),
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "fixed".to_string(),
                    collider: "cuboid".to_string(),
                    mass: 0.0,
                    restitution: 0.2,
                    is_sensor: None,
                }),
                point_light: None,
                animation_player: None,
            },
            EntityDef {
                id: "goal_orb".to_string(),
                mesh: "sphere".to_string(),
                position: [8.5, 5.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [0.8, 0.8, 0.8],
                color: [1.0, 0.8, 0.2, 1.0],
                texture: None,
                parent: None,
                emissive: 3.5,
                fresnel_color: Some([1.0, 0.9, 0.4]),
                fresnel_power: Some(2.0),
                blend_mode: Some("opaque".to_string()),
                wireframe: None,
                uv_scroll: None,
                script: None,
                physics: None,
                point_light: Some(PointLightDef {
                    color: [1.0, 0.8, 0.2],
                    intensity: 5.0,
                    radius: 12.0,
                }),
                animation_player: None,
            },
            EntityDef {
                id: "player_pawn".to_string(),
                mesh: "cylinder".to_string(),
                position: [0.0, 1.5, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [0.8, 1.4, 0.8],
                color: [0.95, 0.3, 0.3, 1.0],
                texture: None,
                parent: None,
                emissive: 0.4,
                fresnel_color: Some([1.0, 0.5, 0.5]),
                fresnel_power: Some(2.5),
                blend_mode: Some("opaque".to_string()),
                wireframe: None,
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "dynamic".to_string(),
                    collider: "cuboid".to_string(),
                    mass: 1.0,
                    restitution: 0.1,
                    is_sensor: None,
                }),
                point_light: None,
                animation_player: None,
            },
        ],
    }
}

fn template_space_arena() -> SceneDef {
    SceneDef {
        camera: CameraDef {
            mode: "3d".to_string(),
            position: [0.0, 5.0, 11.0],
            target: [0.0, 1.5, 0.0],
            ortho_size: 10.0,
        },
        post_process: Some(PostProcessDef {
            bloom_enabled: true,
            bloom_intensity: 1.3,
            bloom_threshold: 0.9,
            vignette_intensity: 0.35,
            exposure: 1.15,
        }),
        environment: Some(EnvironmentDef {
            sky_mode: "space".to_string(),
            zenith_color: [0.04, 0.06, 0.16],
            horizon_color: [0.015, 0.02, 0.05],
            fog_color: [0.03, 0.04, 0.12],
            fog_density: 0.01,
        }),
        entities: vec![
            EntityDef {
                id: "mothership_base".to_string(),
                mesh: "cylinder".to_string(),
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [12.0, 0.5, 12.0],
                color: [0.10, 0.12, 0.18, 1.0],
                texture: None,
                parent: None,
                emissive: 0.0,
                fresnel_color: None,
                fresnel_power: None,
                blend_mode: Some("opaque".to_string()),
                wireframe: Some(false),
                uv_scroll: None,
                script: None,
                physics: Some(PhysicsDef {
                    body: "fixed".to_string(),
                    collider: "cuboid".to_string(),
                    mass: 0.0,
                    restitution: 0.4,
                    is_sensor: None,
                }),
                point_light: None,
                animation_player: None,
            },
            EntityDef {
                id: "warp_beacon".to_string(),
                mesh: "icosphere".to_string(),
                position: [0.0, 2.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.2, 1.2, 1.2],
                color: [0.2, 0.8, 1.0, 1.0],
                texture: None,
                parent: None,
                emissive: 4.0,
                fresnel_color: Some([0.4, 0.95, 1.0]),
                fresnel_power: Some(3.0),
                blend_mode: Some("opaque".to_string()),
                wireframe: None,
                uv_scroll: None,
                script: None,
                physics: None,
                point_light: Some(PointLightDef {
                    color: [0.2, 0.8, 1.0],
                    intensity: 6.0,
                    radius: 20.0,
                }),
                animation_player: None,
            },
            EntityDef {
                id: "shield_ring".to_string(),
                mesh: "torus".to_string(),
                position: [0.0, 2.0, 0.0],
                rotation: [1.2, 0.0, 0.0],
                scale: [2.2, 2.2, 0.3],
                color: [0.4, 0.85, 1.0, 0.5],
                texture: None,
                parent: Some("warp_beacon".to_string()),
                emissive: 2.5,
                fresnel_color: Some([0.5, 0.9, 1.0]),
                fresnel_power: Some(2.0),
                blend_mode: Some("additive".to_string()),
                wireframe: None,
                uv_scroll: None,
                script: None,
                physics: None,
                point_light: None,
                animation_player: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_generation() {
        let names = ["empty_3d", "top_down_2d", "platformer_3d", "space_arena"];
        for name in names {
            let res = create_template_scene(name);
            assert!(res.is_ok(), "Failed to generate template '{}'", name);
            let scene = res.unwrap();
            assert!(!scene.entities.is_empty(), "Template '{}' has no entities", name);
        }
    }

    #[test]
    fn test_invalid_template_name() {
        let res = create_template_scene("non_existent_template");
        assert!(res.is_err());
    }
}
