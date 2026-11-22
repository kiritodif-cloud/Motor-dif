use std::sync::{Arc, Mutex};
use rapier3d::prelude::*;
use glam::{EulerRot, Vec3};

pub use rapier3d::prelude::RigidBodyHandle;

use crate::assets::PhysicsDef;
use crate::ecs::Transform;

#[derive(Debug, Clone, Copy)]
pub struct RigidBodyComponent {
    pub handle: RigidBodyHandle,
}

#[derive(Debug, Clone)]
pub struct CollisionRecord {
    pub collider_1: ColliderHandle,
    pub collider_2: ColliderHandle,
    pub started: bool,
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
                CollisionEvent::Started(c1, c2, _) => {
                    list.push(CollisionRecord { collider_1: c1, collider_2: c2, started: true });
                }
                CollisionEvent::Stopped(c1, c2, _) => {
                    list.push(CollisionRecord { collider_1: c1, collider_2: c2, started: false });
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
        let is_dynamic = physics_def.body.to_lowercase() == "dynamic";
        let rb_builder = if is_dynamic {
            RigidBodyBuilder::dynamic()
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
        } else {
            // Default cuboid half-extents
            let hx = (transform.scale.x * 0.5).max(0.05);
            let hy = (transform.scale.y * 0.5).max(0.05);
            let hz = (transform.scale.z * 0.5).max(0.05);
            ColliderBuilder::cuboid(hx, hy, hz)
        };

        let collider = collider_builder
            .restitution(physics_def.restitution)
            .mass(physics_def.mass)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();

        self.collider_set
            .insert_with_parent(collider, handle, &mut self.rigid_body_set);

        handle
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
