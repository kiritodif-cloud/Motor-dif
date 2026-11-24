use std::collections::HashMap;
use std::f32::consts::PI;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use crate::ecs::{EntityMeta, Transform};
use crate::assets::EntityDef;

// ============================================================================
// 1. PROCEDURAL RIG DATA DEFINITIONS
// ============================================================================

fn default_walk_speed_scale() -> f32 { 5.0 }
fn default_stride_angle() -> f32 { 35.0 }
fn default_bob_amplitude() -> f32 { 0.08 }
fn default_breath_speed() -> f32 { 2.2 }
fn default_breath_amplitude() -> f32 { 0.03 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralRig {
    pub rig_type: String, // "biped" | "quadruped" | "hover_bot" | "turret"

    // Optional child entity IDs
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub torso: Option<String>,
    #[serde(default)]
    pub arm_l: Option<String>,
    #[serde(default)]
    pub arm_r: Option<String>,
    #[serde(default)]
    pub leg_l: Option<String>,
    #[serde(default)]
    pub leg_r: Option<String>,
    #[serde(default)]
    pub weapon: Option<String>,

    // Quadruped specific extra legs (optional)
    #[serde(default)]
    pub leg_fl: Option<String>,
    #[serde(default)]
    pub leg_fr: Option<String>,
    #[serde(default)]
    pub leg_bl: Option<String>,
    #[serde(default)]
    pub leg_br: Option<String>,

    // Locomotion & Idle Parameters
    #[serde(default = "default_walk_speed_scale")]
    pub walk_speed_scale: f32,
    #[serde(default = "default_stride_angle")]
    pub stride_angle_deg: f32,
    #[serde(default = "default_bob_amplitude")]
    pub bob_amplitude: f32,
    #[serde(default = "default_breath_speed")]
    pub breath_speed: f32,
    #[serde(default = "default_breath_amplitude")]
    pub breath_amplitude: f32,

    // Runtime state (skipped during serialization)
    #[serde(skip)]
    pub phase: f32,
    #[serde(skip)]
    pub last_pos: [f32; 3],
    #[serde(skip)]
    pub current_speed: f32,
    #[serde(skip)]
    pub look_target: Option<[f32; 3]>,
    #[serde(skip)]
    pub recoil_timer: f32,
    #[serde(skip)]
    pub recoil_strength: f32,
    #[serde(skip)]
    pub base_transforms: HashMap<String, Transform>,
    #[serde(skip)]
    pub initialized_pos: bool,
}

impl Default for ProceduralRig {
    fn default() -> Self {
        Self {
            rig_type: "biped".to_string(),
            head: None,
            torso: None,
            arm_l: None,
            arm_r: None,
            leg_l: None,
            leg_r: None,
            weapon: None,
            leg_fl: None,
            leg_fr: None,
            leg_bl: None,
            leg_br: None,
            walk_speed_scale: default_walk_speed_scale(),
            stride_angle_deg: default_stride_angle(),
            bob_amplitude: default_bob_amplitude(),
            breath_speed: default_breath_speed(),
            breath_amplitude: default_breath_amplitude(),
            phase: 0.0,
            last_pos: [0.0, 0.0, 0.0],
            current_speed: 0.0,
            look_target: None,
            recoil_timer: 0.0,
            recoil_strength: 1.0,
            base_transforms: HashMap::new(),
            initialized_pos: false,
        }
    }
}

impl ProceduralRig {
    pub fn new(rig_type: impl Into<String>) -> Self {
        Self {
            rig_type: rig_type.into(),
            ..Default::default()
        }
    }

    pub fn trigger_recoil(&mut self, strength: f32) {
        self.recoil_timer = 0.22;
        self.recoil_strength = strength.max(0.1);
    }

    pub fn set_look_target(&mut self, x: f32, y: f32, z: f32) {
        self.look_target = Some([x, y, z]);
    }

    pub fn clear_look_target(&mut self) {
        self.look_target = None;
    }
}

// ============================================================================
// 2. ANALYTICAL 2-BONE INVERSE KINEMATICS (IK)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IkResult {
    pub joint_pos: [f32; 3],
    pub rot1_euler: [f32; 3], // [pitch, yaw, roll] in radians
    pub rot2_euler: [f32; 3], // [pitch, yaw, roll] in radians
}

/// Solves 2-Bone Inverse Kinematics using the analytical Law of Cosines.
/// Safe against NaN, deterministic, zero iterations.
pub fn solve_two_bone_ik(
    root_pos: Vec3,
    target_pos: Vec3,
    pole_dir: Vec3,
    len1: f32,
    len2: f32,
) -> IkResult {
    let l1 = len1.max(0.01);
    let l2 = len2.max(0.01);

    let to_target = target_pos - root_pos;
    let raw_dist = to_target.length();

    let max_reach = (l1 + l2) * 0.9999;
    let min_reach = (l1 - l2).abs() + 0.0001;
    let d = raw_dist.clamp(min_reach, max_reach).max(0.001);

    let dir = if raw_dist > 1e-4 {
        to_target / raw_dist
    } else {
        Vec3::Z
    };

    // Law of cosines
    let cos_alpha = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let cos_beta = ((l1 * l1 + l2 * l2 - d * d) / (2.0 * l1 * l2)).clamp(-1.0, 1.0);

    let alpha = cos_alpha.acos();
    let _beta = cos_beta.acos();

    // Bend plane construction
    let mut plane_normal = dir.cross(pole_dir);
    if plane_normal.length_squared() < 1e-5 {
        plane_normal = dir.cross(Vec3::Y);
        if plane_normal.length_squared() < 1e-5 {
            plane_normal = dir.cross(Vec3::X);
        }
    }
    let plane_normal = plane_normal.normalize_or_zero();
    let bend_dir = plane_normal.cross(dir).normalize_or_zero();

    // Joint position
    let joint_pos = root_pos + dir * (l1 * alpha.cos()) + bend_dir * (l1 * alpha.sin());

    // Orientations for bone 1 and bone 2
    let b1 = (joint_pos - root_pos).normalize_or_zero();
    let pitch1 = -b1.y.atan2((b1.x * b1.x + b1.z * b1.z).sqrt().max(1e-4));
    let yaw1 = b1.x.atan2(b1.z.max(1e-4));

    let b2 = (target_pos - joint_pos).normalize_or_zero();
    let pitch2 = -b2.y.atan2((b2.x * b2.x + b2.z * b2.z).sqrt().max(1e-4));
    let yaw2 = b2.x.atan2(b2.z.max(1e-4));

    IkResult {
        joint_pos: [joint_pos.x, joint_pos.y, joint_pos.z],
        rot1_euler: [pitch1, yaw1, 0.0],
        rot2_euler: [pitch2, yaw2, 0.0],
    }
}

// ============================================================================
// 3. AUTOMATIC PER-FRAME EVALUATION (update_procedural_rigs)
// ============================================================================

/// Updates all ProceduralRig components in the World, modifying local transforms of children.
pub fn update_procedural_rigs(world: &mut hecs::World, dt: f32) {
    if dt <= 0.0 {
        return;
    }

    // Step 1: Map entity ID to Entity handle
    let mut id_to_entity: HashMap<String, hecs::Entity> = HashMap::new();
    for er in world.iter() {
        if let Some(meta) = er.get::<&EntityMeta>() {
            id_to_entity.insert(meta.id.clone(), er.entity());
        }
    }

    // Step 2: Collect rig root data and initialize base transforms
    struct RigWork {
        rig_entity: hecs::Entity,
        root_pos: Vec3,
        root_rot: Vec3,
        rig: ProceduralRig,
    }

    let mut rig_jobs: Vec<RigWork> = Vec::new();

    for er in world.iter() {
        let entity = er.entity();
        if let (Some(transform), Some(mut rig)) = (er.get::<&Transform>(), er.get::<&mut ProceduralRig>()) {
            let root_pos = transform.position;
            let root_rot = transform.rotation;

            if !rig.initialized_pos {
                rig.last_pos = [root_pos.x, root_pos.y, root_pos.z];
                rig.initialized_pos = true;
            }

            // Cache base transforms of children if not recorded yet
            let children_to_check: Vec<String> = [
                rig.head.as_ref(),
                rig.torso.as_ref(),
                rig.arm_l.as_ref(),
                rig.arm_r.as_ref(),
                rig.leg_l.as_ref(),
                rig.leg_r.as_ref(),
                rig.weapon.as_ref(),
                rig.leg_fl.as_ref(),
                rig.leg_fr.as_ref(),
                rig.leg_bl.as_ref(),
                rig.leg_br.as_ref(),
            ]
            .into_iter()
            .flatten()
            .cloned()
            .collect();

            for child_id in children_to_check {
                if !rig.base_transforms.contains_key(&child_id) {
                    if let Some(&child_e) = id_to_entity.get(&child_id) {
                        if let Ok(child_t) = world.get::<&Transform>(child_e) {
                            rig.base_transforms.insert(child_id, *child_t);
                        }
                    }
                }
            }

            rig_jobs.push(RigWork {
                rig_entity: entity,
                root_pos,
                root_rot,
                rig: rig.clone(),
            });
        }
    }

    // Step 3: Compute procedural animation and collect Transform modifications
    let mut transform_updates: Vec<(hecs::Entity, Transform)> = Vec::new();

    for mut job in rig_jobs {
        let rig = &mut job.rig;

        // Velocity & Speed tracking
        let dx = job.root_pos.x - rig.last_pos[0];
        let dz = job.root_pos.z - rig.last_pos[2];
        let dist = (dx * dx + dz * dz).sqrt();
        let inst_speed = dist / dt;
        rig.current_speed = rig.current_speed + (inst_speed - rig.current_speed) * (dt * 8.0).min(1.0);
        rig.last_pos = [job.root_pos.x, job.root_pos.y, job.root_pos.z];

        let walk_weight = (rig.current_speed / 1.2).clamp(0.0, 1.0);
        let idle_weight = 1.0 - walk_weight;

        // Phase advance
        if rig.current_speed > 0.05 {
            rig.phase += dt * (rig.current_speed * rig.walk_speed_scale).max(rig.breath_speed);
        } else {
            rig.phase += dt * rig.breath_speed;
        }
        if rig.phase > 1000.0 * PI {
            rig.phase = rig.phase % (2.0 * PI);
        }

        let phase = rig.phase;
        let stride = rig.stride_angle_deg.to_radians();

        // Helper to get base transform
        let get_base = |id: &Option<String>| -> Option<(hecs::Entity, Transform)> {
            if let Some(name) = id {
                if let Some(&ent) = id_to_entity.get(name) {
                    if let Some(base_t) = rig.base_transforms.get(name) {
                        return Some((ent, *base_t));
                    }
                }
            }
            None
        };

        // Recoil decay & factor
        let recoil_factor = if rig.recoil_timer > 0.0 {
            let f = (rig.recoil_timer / 0.22).clamp(0.0, 1.0) * rig.recoil_strength;
            rig.recoil_timer = (rig.recoil_timer - dt).max(0.0);
            f
        } else {
            0.0
        };

        match rig.rig_type.as_str() {
            "biped" => {
                // 1. Legs (Walk swing)
                if let Some((ent, mut t)) = get_base(&rig.leg_l) {
                    t.rotation.x += phase.sin() * stride * walk_weight;
                    transform_updates.push((ent, t));
                }
                if let Some((ent, mut t)) = get_base(&rig.leg_r) {
                    t.rotation.x -= phase.sin() * stride * walk_weight;
                    transform_updates.push((ent, t));
                }

                // 2. Arms (Counter-swing & idle sway)
                if let Some((ent, mut t)) = get_base(&rig.arm_l) {
                    t.rotation.x -= phase.sin() * (stride * 0.7) * walk_weight;
                    t.rotation.z += (phase * 1.3).cos() * 0.03 * idle_weight;
                    transform_updates.push((ent, t));
                }
                if let Some((ent, mut t)) = get_base(&rig.arm_r) {
                    t.rotation.x += phase.sin() * (stride * 0.7) * walk_weight;
                    t.rotation.z -= (phase * 1.3).cos() * 0.03 * idle_weight;
                    transform_updates.push((ent, t));
                }

                // 3. Torso (Bob, sway & breathing)
                if let Some((ent, mut t)) = get_base(&rig.torso) {
                    let walk_bob = (phase * 2.0).sin().abs() * rig.bob_amplitude * walk_weight;
                    let breath = phase.sin() * rig.breath_amplitude * idle_weight;
                    t.position.y += walk_bob + breath;
                    t.rotation.z += phase.sin() * 0.035 * walk_weight;
                    t.rotation.x -= recoil_factor * 0.08;
                    transform_updates.push((ent, t));
                }

                // 4. Weapon (Recoil kickback)
                if let Some((ent, mut t)) = get_base(&rig.weapon) {
                    t.position.z -= recoil_factor * 0.35;
                    t.rotation.x -= recoil_factor * 0.22;
                    transform_updates.push((ent, t));
                }

                // 5. Head (Look-At or gentle idle sway)
                if let Some((ent, mut t)) = get_base(&rig.head) {
                    if let Some(target) = rig.look_target {
                        let to_target = Vec3::new(target[0], target[1], target[2]) - job.root_pos;
                        let target_yaw = to_target.x.atan2(to_target.z) - job.root_rot.y;
                        let flat_d = (to_target.x * to_target.x + to_target.z * to_target.z).sqrt().max(0.1);
                        let target_pitch = -(to_target.y / flat_d).atan();

                        let clamp_yaw = target_yaw.clamp(-75.0_f32.to_radians(), 75.0_f32.to_radians());
                        let clamp_pitch = target_pitch.clamp(-45.0_f32.to_radians(), 45.0_f32.to_radians());

                        t.rotation.y = clamp_yaw;
                        t.rotation.x = clamp_pitch;
                    } else {
                        t.rotation.y += (phase * 0.7).sin() * 0.04 * idle_weight;
                        t.rotation.x += (phase * 1.4).cos() * 0.02 * idle_weight;
                    }
                    transform_updates.push((ent, t));
                }
            }
            "hover_bot" => {
                let hover_bob = (phase * 2.4).sin() * (rig.bob_amplitude * 1.4);
                let hover_roll = (phase * 1.8).cos() * 0.04;
                let forward_tilt = (rig.current_speed * 0.08).min(0.25);

                if let Some((ent, mut t)) = get_base(&rig.torso) {
                    t.position.y += hover_bob;
                    t.rotation.x += forward_tilt - recoil_factor * 0.1;
                    t.rotation.z += hover_roll;
                    transform_updates.push((ent, t));
                }
                if let Some((ent, mut t)) = get_base(&rig.weapon) {
                    t.position.z -= recoil_factor * 0.3;
                    transform_updates.push((ent, t));
                }
                if let Some((ent, mut t)) = get_base(&rig.head) {
                    if let Some(target) = rig.look_target {
                        let to_target = Vec3::new(target[0], target[1], target[2]) - job.root_pos;
                        let target_yaw = to_target.x.atan2(to_target.z) - job.root_rot.y;
                        t.rotation.y = target_yaw.clamp(-80.0_f32.to_radians(), 80.0_f32.to_radians());
                    }
                    transform_updates.push((ent, t));
                }
            }
            "quadruped" => {
                // Diagonal trot: Pair A (fl, br) and Pair B (fr, bl)
                let fl_id = rig.leg_fl.as_ref().or(rig.arm_l.as_ref());
                let fr_id = rig.leg_fr.as_ref().or(rig.arm_r.as_ref());
                let bl_id = rig.leg_bl.as_ref().or(rig.leg_l.as_ref());
                let br_id = rig.leg_br.as_ref().or(rig.leg_r.as_ref());

                if let Some(name) = fl_id {
                    if let Some((ent, mut t)) = get_base(&Some(name.clone())) {
                        t.rotation.x += phase.sin() * stride * walk_weight;
                        transform_updates.push((ent, t));
                    }
                }
                if let Some(name) = br_id {
                    if let Some((ent, mut t)) = get_base(&Some(name.clone())) {
                        t.rotation.x += phase.sin() * stride * walk_weight;
                        transform_updates.push((ent, t));
                    }
                }
                if let Some(name) = fr_id {
                    if let Some((ent, mut t)) = get_base(&Some(name.clone())) {
                        t.rotation.x -= phase.sin() * stride * walk_weight;
                        transform_updates.push((ent, t));
                    }
                }
                if let Some(name) = bl_id {
                    if let Some((ent, mut t)) = get_base(&Some(name.clone())) {
                        t.rotation.x -= phase.sin() * stride * walk_weight;
                        transform_updates.push((ent, t));
                    }
                }

                if let Some((ent, mut t)) = get_base(&rig.torso) {
                    let bob = (phase * 2.0).sin().abs() * rig.bob_amplitude * walk_weight;
                    t.position.y += bob + phase.sin() * rig.breath_amplitude * idle_weight;
                    transform_updates.push((ent, t));
                }
            }
            "turret" => {
                // Torso tracks yaw, Head/Weapon tracks pitch
                if let Some((ent, mut t)) = get_base(&rig.torso) {
                    if let Some(target) = rig.look_target {
                        let to_target = Vec3::new(target[0], target[1], target[2]) - job.root_pos;
                        let target_yaw = to_target.x.atan2(to_target.z) - job.root_rot.y;
                        t.rotation.y = target_yaw;
                    } else {
                        t.rotation.y = (phase * 0.6).sin() * 0.4;
                    }
                    transform_updates.push((ent, t));
                }
                if let Some((ent, mut t)) = get_base(&rig.weapon) {
                    if let Some(target) = rig.look_target {
                        let to_target = Vec3::new(target[0], target[1], target[2]) - job.root_pos;
                        let flat_d = (to_target.x * to_target.x + to_target.z * to_target.z).sqrt().max(0.1);
                        let target_pitch = -(to_target.y / flat_d).atan();
                        t.rotation.x = target_pitch.clamp(-35.0_f32.to_radians(), 35.0_f32.to_radians());
                    }
                    t.position.z -= recoil_factor * 0.45;
                    transform_updates.push((ent, t));
                }
            }
            _ => {}
        }

        // Write back updated rig state (phase, speed, timers) to root entity
        if let Ok(mut r) = world.get::<&mut ProceduralRig>(job.rig_entity) {
            r.phase = rig.phase;
            r.current_speed = rig.current_speed;
            r.last_pos = rig.last_pos;
            r.recoil_timer = rig.recoil_timer;
            r.recoil_strength = rig.recoil_strength;
            r.base_transforms = std::mem::take(&mut rig.base_transforms);
        }
    }

    // Step 4: Apply all updated transforms to child entities
    for (child_ent, new_transform) in transform_updates {
        if let Ok(mut t) = world.get::<&mut Transform>(child_ent) {
            *t = new_transform;
        }
    }
}

// ============================================================================
// 4. MODULAR CHARACTER PRESET GENERATOR
// ============================================================================

/// Builds a complete modular character hierarchy with ProceduralRig preconfigured.
pub fn build_modular_character(
    preset: &str,
    id_prefix: &str,
    base_pos: [f32; 3],
) -> Vec<EntityDef> {
    let p_clean = if id_prefix.trim().is_empty() { "character" } else { id_prefix.trim() };

    match preset.trim().to_lowercase().as_str() {
        "spider_drone" => build_preset_spider_drone(p_clean, base_pos),
        "defense_turret" => build_preset_defense_turret(p_clean, base_pos),
        "sci_fi_trooper" => build_preset_sci_fi_trooper(p_clean, base_pos),
        _ => build_preset_mech_biped(p_clean, base_pos),
    }
}

fn build_preset_mech_biped(prefix: &str, pos: [f32; 3]) -> Vec<EntityDef> {
    let root_id = format!("{}_root", prefix);
    let torso_id = format!("{}_torso", prefix);
    let head_id = format!("{}_head", prefix);
    let arm_l_id = format!("{}_arm_l", prefix);
    let arm_r_id = format!("{}_arm_r", prefix);
    let leg_l_id = format!("{}_leg_l", prefix);
    let leg_r_id = format!("{}_leg_r", prefix);
    let weapon_id = format!("{}_weapon", prefix);

    let rig = ProceduralRig {
        rig_type: "biped".to_string(),
        head: Some(head_id.clone()),
        torso: Some(torso_id.clone()),
        arm_l: Some(arm_l_id.clone()),
        arm_r: Some(arm_r_id.clone()),
        leg_l: Some(leg_l_id.clone()),
        leg_r: Some(leg_r_id.clone()),
        weapon: Some(weapon_id.clone()),
        walk_speed_scale: 5.2,
        stride_angle_deg: 36.0,
        bob_amplitude: 0.09,
        breath_speed: 2.2,
        breath_amplitude: 0.035,
        ..Default::default()
    };

    vec![
        // 1. Root Entity
        EntityDef {
            id: root_id.clone(),
            mesh: "cube".to_string(),
            position: pos,
            rotation: [0.0, 0.0, 0.0],
            scale: [0.2, 0.2, 0.2],
            texture: None,
            parent: None,
            color: [0.2, 0.8, 1.0, 0.0],
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(true),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: Some(rig),
            vfx_emitter: None,
        },
        // 2. Torso
        EntityDef {
            id: torso_id.clone(),
            mesh: "cube".to_string(),
            position: [0.0, 1.8, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.1, 1.25, 0.85],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.22, 0.26, 0.32, 1.0],
            emissive: 0.0,
            fresnel_color: Some([0.35, 0.5, 0.65]),
            fresnel_power: Some(2.5),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        // 3. Head (with glowing visor)
        EntityDef {
            id: head_id,
            mesh: "cube".to_string(),
            position: [0.0, 0.85, 0.12],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.65, 0.5, 0.6],
            texture: None,
            parent: Some(torso_id.clone()),
            color: [0.15, 0.9, 1.0, 1.0],
            emissive: 3.8,
            fresnel_color: Some([0.4, 0.95, 1.0]),
            fresnel_power: Some(2.2),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        // 4. Arm Left
        EntityDef {
            id: arm_l_id,
            mesh: "cube".to_string(),
            position: [-0.85, 0.25, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.3, 1.0, 0.3],
            texture: None,
            parent: Some(torso_id.clone()),
            color: [0.32, 0.38, 0.45, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        // 5. Arm Right
        EntityDef {
            id: arm_r_id.clone(),
            mesh: "cube".to_string(),
            position: [0.85, 0.25, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.3, 1.0, 0.3],
            texture: None,
            parent: Some(torso_id.clone()),
            color: [0.32, 0.38, 0.45, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        // 6. Weapon (Mounted on right arm)
        EntityDef {
            id: weapon_id,
            mesh: "cylinder".to_string(),
            position: [0.0, -0.6, 0.45],
            rotation: [90.0, 0.0, 0.0],
            scale: [0.2, 0.2, 1.1],
            texture: None,
            parent: Some(arm_r_id),
            color: [0.95, 0.45, 0.1, 1.0],
            emissive: 2.8,
            fresnel_color: Some([1.0, 0.6, 0.2]),
            fresnel_power: Some(2.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        // 7. Leg Left
        EntityDef {
            id: leg_l_id,
            mesh: "cube".to_string(),
            position: [-0.4, 0.6, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.35, 1.2, 0.4],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.24, 0.28, 0.35, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        // 8. Leg Right
        EntityDef {
            id: leg_r_id,
            mesh: "cube".to_string(),
            position: [0.4, 0.6, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.35, 1.2, 0.4],
            texture: None,
            parent: Some(root_id),
            color: [0.24, 0.28, 0.35, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
    ]
}

fn build_preset_sci_fi_trooper(prefix: &str, pos: [f32; 3]) -> Vec<EntityDef> {
    let root_id = format!("{}_root", prefix);
    let torso_id = format!("{}_torso", prefix);
    let head_id = format!("{}_head", prefix);
    let arm_l_id = format!("{}_arm_l", prefix);
    let arm_r_id = format!("{}_arm_r", prefix);
    let leg_l_id = format!("{}_leg_l", prefix);
    let leg_r_id = format!("{}_leg_r", prefix);
    let weapon_id = format!("{}_weapon", prefix);

    let rig = ProceduralRig {
        rig_type: "biped".to_string(),
        head: Some(head_id.clone()),
        torso: Some(torso_id.clone()),
        arm_l: Some(arm_l_id.clone()),
        arm_r: Some(arm_r_id.clone()),
        leg_l: Some(leg_l_id.clone()),
        leg_r: Some(leg_r_id.clone()),
        weapon: Some(weapon_id.clone()),
        walk_speed_scale: 5.6,
        stride_angle_deg: 32.0,
        bob_amplitude: 0.07,
        breath_speed: 2.4,
        breath_amplitude: 0.025,
        ..Default::default()
    };

    vec![
        EntityDef {
            id: root_id.clone(),
            mesh: "cube".to_string(),
            position: pos,
            rotation: [0.0, 0.0, 0.0],
            scale: [0.15, 0.15, 0.15],
            texture: None,
            parent: None,
            color: [0.2, 0.8, 0.3, 0.0],
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(true),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: Some(rig),
            vfx_emitter: None,
        },
        EntityDef {
            id: torso_id.clone(),
            mesh: "cube".to_string(),
            position: [0.0, 1.4, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.8, 0.9, 0.5],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.18, 0.22, 0.26, 1.0],
            emissive: 0.0,
            fresnel_color: Some([0.2, 0.8, 0.4]),
            fresnel_power: Some(2.8),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: head_id,
            mesh: "sphere".to_string(),
            position: [0.0, 0.65, 0.05],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.45, 0.45, 0.45],
            texture: None,
            parent: Some(torso_id.clone()),
            color: [0.95, 0.75, 0.15, 1.0],
            emissive: 3.2,
            fresnel_color: Some([1.0, 0.85, 0.3]),
            fresnel_power: Some(2.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: arm_l_id,
            mesh: "cylinder".to_string(),
            position: [-0.6, 0.15, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.2, 0.75, 0.2],
            texture: None,
            parent: Some(torso_id.clone()),
            color: [0.25, 0.3, 0.35, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: arm_r_id.clone(),
            mesh: "cylinder".to_string(),
            position: [0.6, 0.15, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.2, 0.75, 0.2],
            texture: None,
            parent: Some(torso_id.clone()),
            color: [0.25, 0.3, 0.35, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: weapon_id,
            mesh: "cube".to_string(),
            position: [0.0, -0.45, 0.35],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.15, 0.2, 0.85],
            texture: None,
            parent: Some(arm_r_id),
            color: [0.1, 0.9, 0.8, 1.0],
            emissive: 2.2,
            fresnel_color: Some([0.3, 0.95, 0.9]),
            fresnel_power: Some(2.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: leg_l_id,
            mesh: "cylinder".to_string(),
            position: [-0.3, 0.45, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.22, 0.9, 0.22],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.2, 0.24, 0.28, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: leg_r_id,
            mesh: "cylinder".to_string(),
            position: [0.3, 0.45, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.22, 0.9, 0.22],
            texture: None,
            parent: Some(root_id),
            color: [0.2, 0.24, 0.28, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
    ]
}

fn build_preset_spider_drone(prefix: &str, pos: [f32; 3]) -> Vec<EntityDef> {
    let root_id = format!("{}_root", prefix);
    let torso_id = format!("{}_torso", prefix);
    let eye_id = format!("{}_eye", prefix);
    let leg_fl = format!("{}_leg_fl", prefix);
    let leg_fr = format!("{}_leg_fr", prefix);
    let leg_bl = format!("{}_leg_bl", prefix);
    let leg_br = format!("{}_leg_br", prefix);

    let rig = ProceduralRig {
        rig_type: "quadruped".to_string(),
        torso: Some(torso_id.clone()),
        head: Some(eye_id.clone()),
        leg_fl: Some(leg_fl.clone()),
        leg_fr: Some(leg_fr.clone()),
        leg_bl: Some(leg_bl.clone()),
        leg_br: Some(leg_br.clone()),
        walk_speed_scale: 4.8,
        stride_angle_deg: 28.0,
        bob_amplitude: 0.06,
        breath_speed: 1.8,
        breath_amplitude: 0.02,
        ..Default::default()
    };

    vec![
        EntityDef {
            id: root_id.clone(),
            mesh: "cube".to_string(),
            position: pos,
            rotation: [0.0, 0.0, 0.0],
            scale: [0.2, 0.2, 0.2],
            texture: None,
            parent: None,
            color: [0.9, 0.2, 0.2, 0.0],
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(true),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: Some(rig),
            vfx_emitter: None,
        },
        EntityDef {
            id: torso_id.clone(),
            mesh: "cube".to_string(),
            position: [0.0, 0.75, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.3, 0.45, 1.3],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.15, 0.16, 0.18, 1.0],
            emissive: 0.0,
            fresnel_color: Some([0.8, 0.2, 0.3]),
            fresnel_power: Some(2.5),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: eye_id,
            mesh: "sphere".to_string(),
            position: [0.0, 0.1, 0.65],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.35, 0.35, 0.35],
            texture: None,
            parent: Some(torso_id),
            color: [0.95, 0.15, 0.2, 1.0],
            emissive: 4.2,
            fresnel_color: Some([1.0, 0.3, 0.4]),
            fresnel_power: Some(2.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: leg_fl,
            mesh: "cylinder".to_string(),
            position: [-0.75, 0.3, 0.65],
            rotation: [25.0, -35.0, 0.0],
            scale: [0.18, 0.8, 0.18],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.3, 0.32, 0.36, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: leg_fr,
            mesh: "cylinder".to_string(),
            position: [0.75, 0.3, 0.65],
            rotation: [25.0, 35.0, 0.0],
            scale: [0.18, 0.8, 0.18],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.3, 0.32, 0.36, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: leg_bl,
            mesh: "cylinder".to_string(),
            position: [-0.75, 0.3, -0.65],
            rotation: [-25.0, 35.0, 0.0],
            scale: [0.18, 0.8, 0.18],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.3, 0.32, 0.36, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: leg_br,
            mesh: "cylinder".to_string(),
            position: [0.75, 0.3, -0.65],
            rotation: [-25.0, -35.0, 0.0],
            scale: [0.18, 0.8, 0.18],
            texture: None,
            parent: Some(root_id),
            color: [0.3, 0.32, 0.36, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
    ]
}

fn build_preset_defense_turret(prefix: &str, pos: [f32; 3]) -> Vec<EntityDef> {
    let root_id = format!("{}_root", prefix);
    let base_id = format!("{}_base", prefix);
    let torso_id = format!("{}_ring", prefix);
    let head_id = format!("{}_dome", prefix);
    let weapon_id = format!("{}_cannons", prefix);

    let rig = ProceduralRig {
        rig_type: "turret".to_string(),
        torso: Some(torso_id.clone()),
        head: Some(head_id.clone()),
        weapon: Some(weapon_id.clone()),
        breath_speed: 1.5,
        ..Default::default()
    };

    vec![
        EntityDef {
            id: root_id.clone(),
            mesh: "cube".to_string(),
            position: pos,
            rotation: [0.0, 0.0, 0.0],
            scale: [0.2, 0.2, 0.2],
            texture: None,
            parent: None,
            color: [0.2, 0.5, 1.0, 0.0],
            emissive: 0.0,
            fresnel_color: None,
            fresnel_power: None,
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(true),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: Some(rig),
            vfx_emitter: None,
        },
        EntityDef {
            id: base_id,
            mesh: "cylinder".to_string(),
            position: [0.0, 0.35, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.4, 0.7, 1.4],
            texture: None,
            parent: Some(root_id.clone()),
            color: [0.2, 0.22, 0.26, 1.0],
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
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: torso_id.clone(),
            mesh: "cylinder".to_string(),
            position: [0.0, 0.85, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.95, 0.45, 0.95],
            texture: None,
            parent: Some(root_id),
            color: [0.28, 0.32, 0.38, 1.0],
            emissive: 0.0,
            fresnel_color: Some([0.4, 0.6, 0.8]),
            fresnel_power: Some(2.5),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: head_id,
            mesh: "sphere".to_string(),
            position: [0.0, 0.35, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.45, 0.35, 0.45],
            texture: None,
            parent: Some(torso_id.clone()),
            color: [0.15, 0.85, 1.0, 1.0],
            emissive: 3.5,
            fresnel_color: Some([0.4, 0.9, 1.0]),
            fresnel_power: Some(2.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
        EntityDef {
            id: weapon_id,
            mesh: "cylinder".to_string(),
            position: [0.0, 0.1, 0.6],
            rotation: [90.0, 0.0, 0.0],
            scale: [0.65, 0.2, 1.3],
            texture: None,
            parent: Some(torso_id),
            color: [0.9, 0.3, 0.1, 1.0],
            emissive: 2.2,
            fresnel_color: Some([1.0, 0.5, 0.2]),
            fresnel_power: Some(2.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        },
    ]
}

// ============================================================================
// 5. UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modular_character_presets_generation() {
        let presets = ["mech_biped", "sci_fi_trooper", "spider_drone", "defense_turret"];
        for p in &presets {
            let defs = build_modular_character(p, "unit_test", [0.0, 0.0, 0.0]);
            assert!(defs.len() >= 5, "Preset {} produced too few entities: {}", p, defs.len());

            // Root must have ProceduralRig
            let root = &defs[0];
            assert!(root.procedural_rig.is_some(), "Root of {} missing rig", p);

            // All other entities must have valid parent referencing root or child
            for child in &defs[1..] {
                assert!(child.parent.is_some(), "Child {} in {} missing parent", child.id, p);
            }
        }
    }

    #[test]
    fn test_two_bone_ik_analytical_solver_no_nan() {
        let root = Vec3::new(0.0, 2.0, 0.0);
        let pole = Vec3::new(0.0, 0.0, 1.0);
        let len1 = 1.0;
        let len2 = 1.0;

        // 1. Within reach
        let target_in = Vec3::new(0.0, 0.5, 0.5);
        let res_in = solve_two_bone_ik(root, target_in, pole, len1, len2);
        assert!(!res_in.joint_pos[0].is_nan() && !res_in.joint_pos[1].is_nan() && !res_in.joint_pos[2].is_nan());
        assert!(!res_in.rot1_euler[0].is_nan() && !res_in.rot2_euler[0].is_nan());

        // Joint should be distance len1 from root
        let j_vec = Vec3::from(res_in.joint_pos);
        let d1 = (j_vec - root).length();
        assert!((d1 - len1).abs() < 1e-3, "Bone1 length mismatch: {}", d1);

        // 2. Beyond reach (must not produce NaN)
        let target_far = Vec3::new(10.0, 20.0, 30.0);
        let res_far = solve_two_bone_ik(root, target_far, pole, len1, len2);
        assert!(!res_far.joint_pos[0].is_nan());
        assert!(!res_far.rot1_euler[0].is_nan());

        // 3. Exact root position (zero distance edge case)
        let res_zero = solve_two_bone_ik(root, root, pole, len1, len2);
        assert!(!res_zero.joint_pos[0].is_nan());
    }

    #[test]
    fn test_procedural_rig_locomotion_and_recoil() {
        let mut world = hecs::World::new();
        let defs = build_modular_character("mech_biped", "runner", [0.0, 0.0, 0.0]);

        // Spawn character in world
        for def in defs {
            let meta = EntityMeta {
                id: def.id.clone(),
                name: def.id.clone(),
                parent: def.parent.clone(),
            };
            let transform = Transform::new(
                Vec3::from(def.position),
                Vec3::from(def.rotation),
                Vec3::from(def.scale),
            );
            if let Some(rig) = def.procedural_rig {
                world.spawn((meta, transform, rig));
            } else {
                world.spawn((meta, transform));
            }
        }

        // Frame 1: Initial standing (speed ~ 0)
        update_procedural_rigs(&mut world, 0.016);

        // Verify rig initialized
        let mut root_ent = None;
        for er in world.iter() {
            if let Some(meta) = er.get::<&EntityMeta>() {
                if meta.id == "runner_root" {
                    root_ent = Some(er.entity());
                    break;
                }
            }
        }
        let root = root_ent.unwrap();
        let init_speed = world.get::<&ProceduralRig>(root).unwrap().current_speed;
        assert!(init_speed < 0.1);

        // Frame 2: Move root forward (simulate walking)
        {
            let mut t = world.get::<&mut Transform>(root).unwrap();
            t.position.z += 0.5; // moving fast
        }
        update_procedural_rigs(&mut world, 0.016);

        // Speed and phase should have increased
        let (speed_after, phase_after) = {
            let r = world.get::<&ProceduralRig>(root).unwrap();
            (r.current_speed, r.phase)
        };
        assert!(speed_after > 1.0, "Expected speed increase, got {}", speed_after);
        assert!(phase_after > 0.0, "Expected phase advance");

        // Trigger recoil
        {
            let mut r = world.get::<&mut ProceduralRig>(root).unwrap();
            r.trigger_recoil(1.5);
            assert!(r.recoil_timer > 0.0);
        }
        update_procedural_rigs(&mut world, 0.016);
    }

    #[test]
    fn test_generate_showcase_v19_rig_scene() {
        use crate::assets::{CameraDef, EnvironmentDef, PostProcessDef, PointLightDef, SceneDef};

        let mut entities = Vec::new();

        // 1. Central Arena Platform
        entities.push(EntityDef {
            id: "arena_base".to_string(),
            mesh: "cylinder".to_string(),
            position: [0.0, -0.1, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [18.0, 0.2, 18.0],
            texture: None,
            parent: None,
            color: [0.12, 0.15, 0.22, 1.0],
            emissive: 0.0,
            fresnel_color: Some([0.2, 0.4, 0.6]),
            fresnel_power: Some(3.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        });

        entities.push(EntityDef {
            id: "arena_inner".to_string(),
            mesh: "cylinder".to_string(),
            position: [0.0, 0.02, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [11.0, 0.1, 11.0],
            texture: None,
            parent: None,
            color: [0.16, 0.20, 0.30, 1.0],
            emissive: 0.0,
            fresnel_color: Some([0.3, 0.5, 0.7]),
            fresnel_power: Some(2.5),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        });

        // 2. Central Tracking Beacon
        entities.push(EntityDef {
            id: "rig_beacon".to_string(),
            mesh: "icosphere".to_string(),
            position: [0.0, 3.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.2, 1.2, 1.2],
            texture: None,
            parent: None,
            color: [0.2, 0.9, 1.0, 1.0],
            emissive: 4.0,
            fresnel_color: Some([0.5, 0.95, 1.0]),
            fresnel_power: Some(2.0),
            blend_mode: Some("opaque".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: Some(PointLightDef {
                color: [0.2, 0.85, 1.0],
                intensity: 5.5,
                radius: 24.0,
            }),
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        });

        entities.push(EntityDef {
            id: "rig_beacon_ring1".to_string(),
            mesh: "ring".to_string(),
            position: [0.0, 3.0, 0.0],
            rotation: [45.0, 0.0, 0.0],
            scale: [2.2, 2.2, 2.2],
            texture: None,
            parent: Some("rig_beacon".to_string()),
            color: [0.95, 0.3, 1.0, 1.0],
            emissive: 3.0,
            fresnel_color: Some([1.0, 0.4, 0.9]),
            fresnel_power: Some(2.0),
            blend_mode: Some("additive".to_string()),
            wireframe: Some(false),
            uv_scroll: None,
            script: None,
            physics: None,
            point_light: None,
            animation_player: None,
            procedural_rig: None,
            vfx_emitter: None,
        });

        // 3. Modular Character 1: Mech Biped (with showcase Lua script)
        let mut mech_defs = build_modular_character("mech_biped", "mech", [-6.0, 0.0, 6.0]);
        for def in &mut mech_defs {
            if def.id == "mech_root" {
                def.script = Some("assets/scripts/rig_showcase.lua".to_string());
            }
        }
        entities.extend(mech_defs);

        // 4. Modular Character 2: Spider Drone
        let spider_defs = build_modular_character("spider_drone", "spider", [5.5, 0.0, -4.5]);
        entities.extend(spider_defs);

        // 5. Modular Character 3: Defense Turret
        let turret_defs = build_modular_character("defense_turret", "turret", [-6.5, 0.0, -5.5]);
        entities.extend(turret_defs);

        // 6. Modular Character 4: Sci-Fi Trooper
        let trooper_defs = build_modular_character("sci_fi_trooper", "trooper", [5.5, 0.0, 5.0]);
        entities.extend(trooper_defs);

        // Scene Assembly
        let scene = SceneDef {
            camera: CameraDef {
                mode: "3d".to_string(),
                position: [0.0, 7.5, 17.5],
                target: [0.0, 2.0, 0.0],
                ortho_size: 10.0,
            },
            post_process: Some(PostProcessDef {
                bloom_enabled: true,
                bloom_intensity: 0.40,
                bloom_threshold: 0.75,
                vignette_intensity: 0.32,
                exposure: 1.05,
            }),
            environment: Some(EnvironmentDef {
                sky_mode: "gradient".to_string(),
                zenith_color: [0.06, 0.08, 0.14],
                horizon_color: [0.12, 0.16, 0.24],
                fog_color: [0.06, 0.08, 0.14],
                fog_density: 0.005,
                time_of_day: None,
            }),
            entities,
        };

        let json = serde_json::to_string_pretty(&scene).unwrap();
        std::fs::write("assets/showcase_v19_rig.json", json).unwrap();

        // Verify loaded cleanly
        let loaded = crate::assets::load_scene_from_file("assets/showcase_v19_rig.json");
        assert!(loaded.is_ok(), "Failed to load showcase_v19_rig.json: {:?}", loaded.err());
        let s = loaded.unwrap();
        assert!(s.entities.len() >= 25, "Expected at least 25 modular parts, got {}", s.entities.len());
    }
}
