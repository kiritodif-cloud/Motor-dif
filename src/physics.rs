use std::sync::{Arc, Mutex};
use rapier3d::prelude::*;
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use glam::{EulerRot, Vec3};

pub use rapier3d::prelude::RigidBodyHandle;

use crate::assets::PhysicsDef;
use crate::ecs::Transform;

#[derive(Debug, Clone, Copy)]
pub struct RigidBodyComponent {
    pub handle: RigidBodyHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionLayer;
impl CollisionLayer {
    pub const DEFAULT: u32 = 1 << 0;
    pub const PLAYER: u32 = 1 << 1;
    pub const ENEMY: u32 = 1 << 2;
    pub const WORLD: u32 = 1 << 3;
    pub const PROJECTILE: u32 = 1 << 4;
    pub const TRIGGER: u32 = 1 << 5;
    pub const ALL: u32 = 0xFFFFFFFF;
}

#[derive(Debug, Clone)]
pub struct CharacterControllerComponent {
    pub controller: KinematicCharacterController,
    pub is_grounded: bool,
    pub coyote_timer: f32,
    pub max_coyote_time: f32,
}

impl Default for CharacterControllerComponent {
    fn default() -> Self {
        let mut controller = KinematicCharacterController::default();
        controller.autostep = Some(CharacterAutostep {
            max_height: CharacterLength::Absolute(0.4),
            min_width: CharacterLength::Relative(0.5),
            include_dynamic_bodies: false,
        });
        controller.max_slope_climb_angle = 45.0_f32.to_radians();
        controller.min_slope_slide_angle = 45.0_f32.to_radians();
        controller.snap_to_ground = Some(CharacterLength::Absolute(0.2));

        Self {
            controller,
            is_grounded: true,
            coyote_timer: 0.15,
            max_coyote_time: 0.15,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CollisionRecord {
    pub collider_1: ColliderHandle,
    pub collider_2: ColliderHandle,
    pub started: bool,
    pub is_sensor: bool,
}

pub struct CollisionCollector {
    pub events: Arc<Mutex<Vec<CollisionRecord>>>,
}

impl EventHandler for CollisionCollector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        if let Ok(mut list) = self.events.lock() {
            match event {
                CollisionEvent::Started(c1, c2, flags) => {
                    list.push(CollisionRecord {
                        collider_1: c1,
                        collider_2: c2,
                        started: true,
                        is_sensor: flags.contains(CollisionEventFlags::SENSOR),
                    });
                }
                CollisionEvent::Stopped(c1, c2, flags) => {
                    list.push(CollisionRecord {
                        collider_1: c1,
                        collider_2: c2,
                        started: false,
                        is_sensor: flags.contains(CollisionEventFlags::SENSOR),
                    });
                }
            }
        }
    }

    fn handle_contact_force_event(
        &self,
        _dt: f32,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &ContactPair,
        _total_force_magnitude: f32,
    ) {}

    fn handle_soft_body_tear_event(
        &self,
        _soft_bodies: &SoftBodySet,
        _event: &SoftBodyTearEvent,
    ) {}
}

pub struct PhysicsEngine {
    pub enabled: bool,
    pub gravity: Vec3,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub soft_body_set: SoftBodySet,
    pub ccd_solver: CCDSolver,
    pub fixed_time_step: f32,
    pub accumulator: f32,
    pub collision_events: Arc<Mutex<Vec<CollisionRecord>>>,
}

impl PhysicsEngine {
    pub fn new() -> Self {
        Self {
            enabled: true,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            soft_body_set: SoftBodySet::new(),
            ccd_solver: CCDSolver::new(),
            fixed_time_step: 1.0 / 60.0,
            accumulator: 0.0,
            collision_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn clear(&mut self) {
        self.rigid_body_set = RigidBodySet::new();
        self.collider_set = ColliderSet::new();
        self.island_manager = IslandManager::new();
        self.broad_phase = DefaultBroadPhase::new();
        self.narrow_phase = NarrowPhase::new();
        self.impulse_joint_set = ImpulseJointSet::new();
        self.multibody_joint_set = MultibodyJointSet::new();
        self.soft_body_set = SoftBodySet::new();
        self.ccd_solver = CCDSolver::new();
        self.accumulator = 0.0;
        if let Ok(mut events) = self.collision_events.lock() {
            events.clear();
        }
    }

    pub fn add_body(
        &mut self,
        transform: &Transform,
        physics_def: &PhysicsDef,
    ) -> RigidBodyHandle {
        let btype = physics_def.body.to_lowercase();
        let rb_builder = if btype == "dynamic" {
            RigidBodyBuilder::dynamic()
        } else if btype == "kinematic" {
            RigidBodyBuilder::kinematic_position_based()
        } else {
            RigidBodyBuilder::fixed()
        };

        let body = rb_builder
            .translation(transform.position)
            .rotation(transform.rotation)
            .build();

        let handle = self.rigid_body_set.insert(body);

        let collider_type = physics_def.collider.to_lowercase();
        let collider_builder = if collider_type == "ball" || collider_type == "sphere" {
            let radius = (transform.scale.x * 0.5).max(0.05);
            ColliderBuilder::ball(radius)
        } else if collider_type == "capsule" {
            let r = (transform.scale.x * 0.5).max(0.05);
            let hh = (transform.scale.y * 0.5 - r).max(0.05);
            ColliderBuilder::capsule_y(hh, r)
        } else {
            // Default cuboid half-extents
            let hx = (transform.scale.x * 0.5).max(0.05);
            let hy = (transform.scale.y * 0.5).max(0.05);
            let hz = (transform.scale.z * 0.5).max(0.05);
            ColliderBuilder::cuboid(hx, hy, hz)
        };

        let layer = physics_def.collision_layer.unwrap_or(CollisionLayer::DEFAULT);
        let mask = physics_def.collision_mask.unwrap_or(CollisionLayer::ALL);
        let groups = InteractionGroups::all()
            .with_memberships(Group::from_bits_truncate(layer))
            .with_filter(Group::from_bits_truncate(mask));

        let is_sensor = physics_def.is_sensor.unwrap_or(false);
        let collider = collider_builder
            .restitution(physics_def.restitution)
            .mass(physics_def.mass)
            .sensor(is_sensor)
            .collision_groups(groups)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();

        self.collider_set
            .insert_with_parent(collider, handle, &mut self.rigid_body_set);

        handle
    }

    pub fn add_trimesh_body(
        &mut self,
        transform: &Transform,
        vertices: Vec<Vec3>,
        indices: Vec<[u32; 3]>,
        is_sensor: bool,
    ) -> Option<RigidBodyHandle> {
        let body = RigidBodyBuilder::fixed()
            .translation(transform.position)
            .rotation(transform.rotation)
            .build();
        let handle = self.rigid_body_set.insert(body);
        if let Ok(builder) = ColliderBuilder::trimesh(vertices, indices) {
            let collider = builder
                .sensor(is_sensor)
                .active_events(ActiveEvents::COLLISION_EVENTS)
                .build();
            self.collider_set
                .insert_with_parent(collider, handle, &mut self.rigid_body_set);
            Some(handle)
        } else {
            None
        }
    }

    pub fn add_heightfield_body(
        &mut self,
        transform: &Transform,
        heights: &[f32],
        nrows: usize,
        ncols: usize,
        scale: Vec3,
    ) -> Option<RigidBodyHandle> {
        if nrows < 2 || ncols < 2 || heights.len() < nrows * ncols {
            return None;
        }
        let body = RigidBodyBuilder::fixed()
            .translation(transform.position)
            .rotation(transform.rotation)
            .build();
        let handle = self.rigid_body_set.insert(body);
        let mat = rapier3d::parry::utils::Array2::from_fn(nrows, ncols, |r, c| heights[r * ncols + c]);
        let collider = ColliderBuilder::heightfield(mat, scale)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();
        self.collider_set
            .insert_with_parent(collider, handle, &mut self.rigid_body_set);
        Some(handle)
    }

    pub fn add_joint(
        &mut self,
        body_a: RigidBodyHandle,
        body_b: RigidBodyHandle,
        joint_type: &str,
        local_anchor_a: Vec3,
        local_anchor_b: Vec3,
        axis: Vec3,
    ) -> Option<ImpulseJointHandle> {
        let jtype = joint_type.trim().to_lowercase();
        let joint: GenericJoint = match jtype.as_str() {
            "fixed" => FixedJointBuilder::new()
                .local_anchor1(local_anchor_a)
                .local_anchor2(local_anchor_b)
                .build()
                .into(),
            "revolute" | "hinge" => {
                let ax = if axis.length_squared() > 1e-6 {
                    axis.normalize()
                } else {
                    Vec3::Y
                };
                RevoluteJointBuilder::new(ax)
                    .local_anchor1(local_anchor_a)
                    .local_anchor2(local_anchor_b)
                    .build()
                    .into()
            }
            "spherical" | "ball" => SphericalJointBuilder::new()
                .local_anchor1(local_anchor_a)
                .local_anchor2(local_anchor_b)
                .build()
                .into(),
            _ => return None,
        };
        Some(self.impulse_joint_set.insert(body_a, body_b, joint, true))
    }

    pub fn step(&mut self, dt: f32) {
        if !self.enabled {
            return;
        }

        let collector = CollisionCollector {
            events: self.collision_events.clone(),
        };

        self.accumulator += dt.min(0.2);
        while self.accumulator >= self.fixed_time_step {
            self.accumulator -= self.fixed_time_step;
            self.physics_pipeline.step(
                self.gravity,
                &self.integration_parameters,
                &mut self.island_manager,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.rigid_body_set,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                &mut self.soft_body_set,
                &mut self.ccd_solver,
                &(),
                &collector,
            );
        }
    }

    pub fn raycast(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_dist: f32,
        layer_mask: u32,
    ) -> Option<(ColliderHandle, RayIntersection)> {
        let dir_norm = dir.normalize_or_zero();
        if dir_norm.length_squared() < 0.001 {
            return None;
        }
        let ray = Ray::new(origin, dir_norm);
        let groups = InteractionGroups::all()
            .with_memberships(Group::ALL)
            .with_filter(Group::from_bits_truncate(layer_mask));
        let filter = QueryFilter::default().groups(groups);
        let query_pipeline = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.rigid_body_set,
            &self.collider_set,
            filter,
        );
        query_pipeline.cast_ray_and_get_normal(&ray, max_dist, true)
    }

    pub fn overlap_sphere(
        &self,
        center: Vec3,
        radius: f32,
        layer_mask: u32,
    ) -> Vec<ColliderHandle> {
        let shape = Ball::new(radius.max(0.01));
        let shape_pos = Pose::from_translation(center);
        let groups = InteractionGroups::all()
            .with_memberships(Group::ALL)
            .with_filter(Group::from_bits_truncate(layer_mask));
        let filter = QueryFilter::default().groups(groups);
        let query_pipeline = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.rigid_body_set,
            &self.collider_set,
            filter,
        );
        query_pipeline
            .intersect_shape(shape_pos, &shape)
            .map(|(handle, _)| handle)
            .collect()
    }

    pub fn set_collision_groups(&mut self, collider_handle: ColliderHandle, layer: u32, mask: u32) {
        if let Some(collider) = self.collider_set.get_mut(collider_handle) {
            collider.set_collision_groups(
                InteractionGroups::all()
                    .with_memberships(Group::from_bits_truncate(layer))
                    .with_filter(Group::from_bits_truncate(mask)),
            );
        }
    }

    pub fn move_character(
        &mut self,
        body_handle: RigidBodyHandle,
        collider_handle: ColliderHandle,
        controller: &mut CharacterControllerComponent,
        desired_translation: Vec3,
        dt: f32,
    ) -> Vec3 {
        let Some(collider) = self.collider_set.get(collider_handle) else {
            return desired_translation;
        };
        let shape = collider.shape();
        let shape_pos = *collider.position();
        let filter = QueryFilter::default().exclude_rigid_body(body_handle);
        let query_pipeline = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.rigid_body_set,
            &self.collider_set,
            filter,
        );

        let movement = controller.controller.move_shape(
            dt,
            &query_pipeline,
            shape,
            &shape_pos,
            desired_translation,
            |_| {},
        );

        if movement.grounded {
            controller.is_grounded = true;
            controller.coyote_timer = controller.max_coyote_time;
        } else {
            controller.coyote_timer -= dt;
            controller.is_grounded = controller.coyote_timer > 0.0;
        }

        if let Some(body) = self.rigid_body_set.get_mut(body_handle) {
            let new_pos = body.translation() + movement.translation;
            body.set_next_kinematic_translation(new_pos);
        }

        movement.translation
    }

    pub fn sync_to_ecs(&self, world: &mut hecs::World) {
        for (transform, rb_comp) in world.query_mut::<(&mut Transform, &RigidBodyComponent)>() {
            if let Some(rb) = self.rigid_body_set.get(rb_comp.handle) {
                if rb.is_dynamic() {
                    transform.position = rb.translation();
                    let (rx, ry, rz) = rb.rotation().to_euler(EulerRot::XYZ);
                    transform.rotation = Vec3::new(rx, ry, rz);
                }
            }
        }
    }

    pub fn apply_force(&mut self, handle: RigidBodyHandle, force: Vec3) {
        if let Some(rb) = self.rigid_body_set.get_mut(handle) {
            rb.add_force(force, true);
        }
    }

    pub fn apply_impulse(&mut self, handle: RigidBodyHandle, impulse: Vec3) {
        if let Some(rb) = self.rigid_body_set.get_mut(handle) {
            rb.apply_impulse(impulse, true);
        }
    }

    pub fn remove_body(&mut self, handle: RigidBodyHandle) {
        self.rigid_body_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.soft_body_set,
            true,
        );
    }

    pub fn dynamic_bodies_count(&self) -> usize {
        self.rigid_body_set.iter().filter(|(_, b)| b.is_dynamic()).count()
    }

    pub fn fixed_bodies_count(&self) -> usize {
        self.rigid_body_set.iter().filter(|(_, b)| b.is_fixed()).count()
    }
}

impl Default for PhysicsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_controller_component_defaults_and_coyote_time() {
        let mut cc = CharacterControllerComponent::default();
        assert!(cc.is_grounded);
        assert!((cc.coyote_timer - 0.15).abs() < 1e-4);

        // Simulate coyote time countdown when airborne
        cc.coyote_timer -= 0.10;
        assert!(cc.coyote_timer > 0.0);
        cc.coyote_timer -= 0.10;
        assert!(cc.coyote_timer <= 0.0);
    }

    #[test]
    fn test_collision_layers_and_spatial_queries() {
        let mut engine = PhysicsEngine::new();

        // Floor (WORLD)
        let floor_trans = Transform::new(Vec3::new(0.0, 0.0, 0.0), Vec3::ZERO, Vec3::new(20.0, 0.2, 20.0));
        let floor_def = PhysicsDef {
            body: "fixed".to_string(),
            collider: "cuboid".to_string(),
            mass: 0.0,
            restitution: 0.0,
            is_sensor: None,
            collision_layer: Some(CollisionLayer::WORLD),
            collision_mask: Some(CollisionLayer::ALL),
        };
        engine.add_body(&floor_trans, &floor_def);

        // Player (PLAYER)
        let player_trans = Transform::new(Vec3::new(0.0, 2.0, 0.0), Vec3::ZERO, Vec3::new(1.0, 2.0, 1.0));
        let player_def = PhysicsDef {
            body: "dynamic".to_string(),
            collider: "capsule".to_string(),
            mass: 1.0,
            restitution: 0.0,
            is_sensor: None,
            collision_layer: Some(CollisionLayer::PLAYER),
            collision_mask: Some(CollisionLayer::WORLD | CollisionLayer::ENEMY),
        };
        let p_handle = engine.add_body(&player_trans, &player_def);
        assert!(engine.rigid_body_set.get(p_handle).is_some());

        // Step physics once
        engine.step(0.05);

        // Raycast hitting world
        let hit_world = engine.raycast(Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, -1.0, 0.0), 10.0, CollisionLayer::WORLD);
        assert!(hit_world.is_some());

        // Raycast filtered out by layer mask (only PROJECTILE)
        let hit_proj = engine.raycast(Vec3::new(5.0, 5.0, 5.0), Vec3::new(0.0, -1.0, 0.0), 10.0, CollisionLayer::PROJECTILE);
        assert!(hit_proj.is_none());

        // Overlap sphere around origin
        let overlaps = engine.overlap_sphere(Vec3::new(0.0, 0.1, 0.0), 2.0, CollisionLayer::ALL);
        assert!(!overlaps.is_empty());
    }

    #[test]
    fn test_character_controller_move_and_grounding() {
        let mut engine = PhysicsEngine::new();

        // Fixed floor
        let floor_trans = Transform::new(Vec3::new(0.0, 0.0, 0.0), Vec3::ZERO, Vec3::new(10.0, 0.2, 10.0));
        let floor_def = PhysicsDef {
            body: "fixed".to_string(),
            collider: "cuboid".to_string(),
            mass: 0.0,
            restitution: 0.0,
            is_sensor: None,
            collision_layer: Some(CollisionLayer::WORLD),
            collision_mask: Some(CollisionLayer::ALL),
        };
        engine.add_body(&floor_trans, &floor_def);

        // Kinematic character
        let char_trans = Transform::new(Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO, Vec3::new(1.0, 1.8, 1.0));
        let char_def = PhysicsDef {
            body: "kinematic".to_string(),
            collider: "capsule".to_string(),
            mass: 1.0,
            restitution: 0.0,
            is_sensor: None,
            collision_layer: Some(CollisionLayer::PLAYER),
            collision_mask: Some(CollisionLayer::WORLD),
        };
        let body_handle = engine.add_body(&char_trans, &char_def);
        let collider_handle = engine.rigid_body_set.get(body_handle).unwrap().colliders()[0];

        let mut controller = CharacterControllerComponent::default();
        // Move downwards into floor
        let moved = engine.move_character(body_handle, collider_handle, &mut controller, Vec3::new(0.0, -1.5, 0.0), 0.016);
        assert!(controller.is_grounded);
        assert!(moved.y < 0.0);
    }

    #[test]
    fn test_heightfield_and_joints() {
        let mut engine = PhysicsEngine::new();
        let heights = vec![0.0_f32; 9];
        let hf_handle = engine.add_heightfield_body(
            &Transform::default(),
            &heights,
            3,
            3,
            Vec3::new(10.0, 1.0, 10.0),
        );
        assert!(hf_handle.is_some());

        let dyn_def = PhysicsDef {
            body: "dynamic".to_string(),
            collider: "cuboid".to_string(),
            mass: 1.0,
            restitution: 0.2,
            is_sensor: None,
            collision_layer: None,
            collision_mask: None,
        };
        let b1 = engine.add_body(&Transform::from_xyz(0.0, 2.0, 0.0), &dyn_def);
        let b2 = engine.add_body(&Transform::from_xyz(1.0, 2.0, 0.0), &dyn_def);
        let j = engine.add_joint(b1, b2, "revolute", Vec3::ZERO, Vec3::new(-1.0, 0.0, 0.0), Vec3::Y);
        assert!(j.is_some());
    }
}
