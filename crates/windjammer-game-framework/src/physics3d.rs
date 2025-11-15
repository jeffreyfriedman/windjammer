//! 3D Physics System using Rapier3D
//!
//! Provides a high-level API for 3D physics simulation integrated with the ECS.

use crate::ecs::Entity;
use crate::math::Vec3;
use rapier3d::prelude::*;
use rapier3d::dynamics::RigidBodyType as RapierRigidBodyType;
use rapier3d::na::{Quaternion, UnitQuaternion};
use std::collections::HashMap;

/// Physics world for 3D simulation
pub struct PhysicsWorld3D {
    /// Rapier physics pipeline
    pub(crate) pipeline: PhysicsPipeline,
    /// Gravity vector
    pub(crate) gravity: Vector<Real>,
    /// Integration parameters
    pub(crate) integration_parameters: IntegrationParameters,
    /// Island manager
    pub(crate) islands: IslandManager,
    /// Broad phase
    pub(crate) broad_phase: BroadPhase,
    /// Narrow phase
    pub(crate) narrow_phase: NarrowPhase,
    /// Rigid body set
    pub(crate) bodies: RigidBodySet,
    /// Collider set
    pub(crate) colliders: ColliderSet,
    /// Impulse joint set
    pub(crate) impulse_joints: ImpulseJointSet,
    /// Multibody joint set
    pub(crate) multibody_joints: MultibodyJointSet,
    /// CCD solver
    pub(crate) ccd_solver: CCDSolver,
    /// Query pipeline
    pub(crate) query_pipeline: QueryPipeline,
    
    /// Map from ECS entities to Rapier rigid body handles
    entity_to_body: HashMap<Entity, RigidBodyHandle>,
    /// Map from Rapier rigid body handles to ECS entities
    body_to_entity: HashMap<RigidBodyHandle, Entity>,
}

impl PhysicsWorld3D {
    /// Create a new physics world with default gravity (Earth-like: -9.81 m/s² on Y axis)
    pub fn new() -> Self {
        Self::with_gravity(Vec3::new(0.0, -9.81, 0.0))
    }
    
    /// Create a new physics world with custom gravity
    pub fn with_gravity(gravity: Vec3) -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: vector![gravity.x as Real, gravity.y as Real, gravity.z as Real],
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),
        }
    }
    
    /// Set gravity
    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = vector![gravity.x as Real, gravity.y as Real, gravity.z as Real];
    }
    
    /// Step the physics simulation
    pub fn step(&mut self) {
        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }
    
    /// Add a rigid body for an entity
    pub fn add_rigid_body(&mut self, entity: Entity, body: RigidBody) -> RigidBodyHandle {
        let handle = self.bodies.insert(body);
        self.entity_to_body.insert(entity, handle);
        self.body_to_entity.insert(handle, entity);
        handle
    }
    
    /// Add a collider to a rigid body
    pub fn add_collider(
        &mut self,
        collider: Collider,
        body_handle: RigidBodyHandle,
    ) -> ColliderHandle {
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies)
    }
    
    /// Get the rigid body handle for an entity
    pub fn get_body_handle(&self, entity: Entity) -> Option<RigidBodyHandle> {
        self.entity_to_body.get(&entity).copied()
    }
    
    /// Get the entity for a rigid body handle
    pub fn get_entity(&self, handle: RigidBodyHandle) -> Option<Entity> {
        self.body_to_entity.get(&handle).copied()
    }
    
    /// Get a rigid body by entity
    pub fn get_body(&self, entity: Entity) -> Option<&RigidBody> {
        self.entity_to_body
            .get(&entity)
            .and_then(|handle| self.bodies.get(*handle))
    }
    
    /// Get a mutable rigid body by entity
    pub fn get_body_mut(&mut self, entity: Entity) -> Option<&mut RigidBody> {
        self.entity_to_body
            .get(&entity)
            .and_then(|handle| self.bodies.get_mut(*handle))
    }
    
    /// Remove a rigid body for an entity
    pub fn remove_body(&mut self, entity: Entity) {
        if let Some(handle) = self.entity_to_body.remove(&entity) {
            self.body_to_entity.remove(&handle);
            self.bodies.remove(
                handle,
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            );
        }
    }
    
    /// Get position of a rigid body
    pub fn get_position(&self, entity: Entity) -> Option<Vec3> {
        self.get_body(entity).map(|body| {
            let pos = body.translation();
            Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32)
        })
    }
    
    /// Set position of a rigid body
    pub fn set_position(&mut self, entity: Entity, position: Vec3, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.set_translation(
                vector![position.x as Real, position.y as Real, position.z as Real],
                wake_up,
            );
        }
    }
    
    /// Get rotation of a rigid body (as quaternion components: x, y, z, w)
    pub fn get_rotation(&self, entity: Entity) -> Option<(f32, f32, f32, f32)> {
        self.get_body(entity).map(|body| {
            let rot = body.rotation();
            (rot.i as f32, rot.j as f32, rot.k as f32, rot.w as f32)
        })
    }
    
    /// Set rotation of a rigid body (from quaternion components: x, y, z, w)
    pub fn set_rotation(&mut self, entity: Entity, x: f32, y: f32, z: f32, w: f32, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.set_rotation(
                UnitQuaternion::from_quaternion(Quaternion::new(
                    w as Real,
                    x as Real,
                    y as Real,
                    z as Real,
                )),
                wake_up,
            );
        }
    }
    
    /// Get linear velocity of a rigid body
    pub fn get_linear_velocity(&self, entity: Entity) -> Option<Vec3> {
        self.get_body(entity).map(|body| {
            let vel = body.linvel();
            Vec3::new(vel.x as f32, vel.y as f32, vel.z as f32)
        })
    }
    
    /// Set linear velocity of a rigid body
    pub fn set_linear_velocity(&mut self, entity: Entity, velocity: Vec3, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.set_linvel(
                vector![velocity.x as Real, velocity.y as Real, velocity.z as Real],
                wake_up,
            );
        }
    }
    
    /// Get angular velocity of a rigid body
    pub fn get_angular_velocity(&self, entity: Entity) -> Option<Vec3> {
        self.get_body(entity).map(|body| {
            let vel = body.angvel();
            Vec3::new(vel.x as f32, vel.y as f32, vel.z as f32)
        })
    }
    
    /// Set angular velocity of a rigid body
    pub fn set_angular_velocity(&mut self, entity: Entity, velocity: Vec3, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.set_angvel(
                vector![velocity.x as Real, velocity.y as Real, velocity.z as Real],
                wake_up,
            );
        }
    }
    
    /// Apply an impulse to a rigid body
    pub fn apply_impulse(&mut self, entity: Entity, impulse: Vec3, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.apply_impulse(
                vector![impulse.x as Real, impulse.y as Real, impulse.z as Real],
                wake_up,
            );
        }
    }
    
    /// Apply a force to a rigid body
    pub fn apply_force(&mut self, entity: Entity, force: Vec3, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.add_force(
                vector![force.x as Real, force.y as Real, force.z as Real],
                wake_up,
            );
        }
    }
    
    /// Apply a torque impulse to a rigid body
    pub fn apply_torque_impulse(&mut self, entity: Entity, torque: Vec3, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.apply_torque_impulse(
                vector![torque.x as Real, torque.y as Real, torque.z as Real],
                wake_up,
            );
        }
    }
    
    /// Apply a torque to a rigid body
    pub fn apply_torque(&mut self, entity: Entity, torque: Vec3, wake_up: bool) {
        if let Some(body) = self.get_body_mut(entity) {
            body.add_torque(
                vector![torque.x as Real, torque.y as Real, torque.z as Real],
                wake_up,
            );
        }
    }
    
    /// Raycast in the physics world
    pub fn raycast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        solid: bool,
    ) -> Option<(Entity, Vec3, Vec3, f32)> {
        let ray = Ray::new(
            point![origin.x as Real, origin.y as Real, origin.z as Real],
            vector![direction.x as Real, direction.y as Real, direction.z as Real],
        );
        
        let filter = QueryFilter::default();
        
        self.query_pipeline
            .cast_ray(
                &self.bodies,
                &self.colliders,
                &ray,
                max_distance as Real,
                solid,
                filter,
            )
            .and_then(|(handle, toi)| {
                self.colliders.get(handle).and_then(|collider| {
                    collider.parent().and_then(|body_handle| {
                        self.get_entity(body_handle).map(|entity| {
                            let hit_point = ray.point_at(toi);
                            let normal = if let Some(normal) = self.query_pipeline.cast_ray_and_get_normal(
                                &self.bodies,
                                &self.colliders,
                                &ray,
                                max_distance as Real,
                                solid,
                                filter,
                            ) {
                                normal.1.normal
                            } else {
                                vector![0.0, 1.0, 0.0]
                            };
                            
                            (
                                entity,
                                Vec3::new(hit_point.x as f32, hit_point.y as f32, hit_point.z as f32),
                                Vec3::new(normal.x as f32, normal.y as f32, normal.z as f32),
                                toi as f32,
                            )
                        })
                    })
                })
            })
    }
    
    /// Check if two entities are colliding
    pub fn is_colliding(&self, entity1: Entity, entity2: Entity) -> bool {
        if let (Some(handle1), Some(handle2)) = (
            self.get_body_handle(entity1),
            self.get_body_handle(entity2),
        ) {
            // Check all colliders attached to both bodies
            for (collider1_handle, collider1) in self.colliders.iter() {
                if collider1.parent() != Some(handle1) {
                    continue;
                }
                
                for (collider2_handle, collider2) in self.colliders.iter() {
                    if collider2.parent() != Some(handle2) {
                        continue;
                    }
                    
                    // Check if colliders are in contact
                    if self.narrow_phase.contact_pair(collider1_handle, collider2_handle).is_some() {
                        return true;
                    }
                }
            }
        }
        
        false
    }
}

impl Default for PhysicsWorld3D {
    fn default() -> Self {
        Self::new()
    }
}

/// Rigid body type for 3D physics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigidBodyType3D {
    /// Dynamic body (affected by forces and gravity)
    Dynamic,
    /// Fixed body (immovable, like terrain)
    Fixed,
    /// Kinematic body (position-based, not affected by forces)
    KinematicPositionBased,
    /// Kinematic body (velocity-based)
    KinematicVelocityBased,
}

impl From<RigidBodyType3D> for RapierRigidBodyType {
    fn from(ty: RigidBodyType3D) -> Self {
        match ty {
            RigidBodyType3D::Dynamic => RapierRigidBodyType::Dynamic,
            RigidBodyType3D::Fixed => RapierRigidBodyType::Fixed,
            RigidBodyType3D::KinematicPositionBased => RapierRigidBodyType::KinematicPositionBased,
            RigidBodyType3D::KinematicVelocityBased => RapierRigidBodyType::KinematicVelocityBased,
        }
    }
}

/// Collider shape for 3D physics
#[derive(Debug, Clone)]
pub enum ColliderShape3D {
    /// Sphere collider
    Ball(f32),
    /// Box collider (half-extents)
    Cuboid(f32, f32, f32),
    /// Capsule collider (half-height, radius)
    Capsule(f32, f32),
    /// Cylinder collider (half-height, radius)
    Cylinder(f32, f32),
    /// Cone collider (half-height, radius)
    Cone(f32, f32),
}

impl ColliderShape3D {
    /// Create a Rapier collider from this shape
    pub fn to_rapier(&self) -> Collider {
        match *self {
            ColliderShape3D::Ball(radius) => ColliderBuilder::ball(radius as Real).build(),
            ColliderShape3D::Cuboid(hx, hy, hz) => {
                ColliderBuilder::cuboid(hx as Real, hy as Real, hz as Real).build()
            }
            ColliderShape3D::Capsule(half_height, radius) => {
                ColliderBuilder::capsule_y(half_height as Real, radius as Real).build()
            }
            ColliderShape3D::Cylinder(half_height, radius) => {
                ColliderBuilder::cylinder(half_height as Real, radius as Real).build()
            }
            ColliderShape3D::Cone(half_height, radius) => {
                ColliderBuilder::cone(half_height as Real, radius as Real).build()
            }
        }
    }
}

/// Component for 3D rigid bodies
#[derive(Debug, Clone)]
pub struct RigidBody3D {
    pub body_type: RigidBodyType3D,
    pub mass: f32,
    pub restitution: f32,  // Bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    pub friction: f32,     // Surface friction (0.0 = ice, 1.0 = rubber)
    pub linear_damping: f32,  // Air resistance for linear motion
    pub angular_damping: f32, // Air resistance for rotation
    pub can_sleep: bool,   // Whether the body can be put to sleep for optimization
}

impl Default for RigidBody3D {
    fn default() -> Self {
        Self {
            body_type: RigidBodyType3D::Dynamic,
            mass: 1.0,
            restitution: 0.5,
            friction: 0.5,
            linear_damping: 0.0,
            angular_damping: 0.0,
            can_sleep: true,
        }
    }
}

impl RigidBody3D {
    /// Create a new dynamic rigid body
    pub fn new_dynamic() -> Self {
        Self {
            body_type: RigidBodyType3D::Dynamic,
            ..Default::default()
        }
    }
    
    /// Create a new fixed (static) rigid body
    pub fn new_fixed() -> Self {
        Self {
            body_type: RigidBodyType3D::Fixed,
            ..Default::default()
        }
    }
    
    /// Create a new kinematic rigid body
    pub fn new_kinematic() -> Self {
        Self {
            body_type: RigidBodyType3D::KinematicPositionBased,
            ..Default::default()
        }
    }
    
    /// Convert to a Rapier rigid body
    pub fn to_rapier(&self, position: Vec3) -> RigidBody {
        let mut builder = RigidBodyBuilder::new(self.body_type.into())
            .translation(vector![position.x as Real, position.y as Real, position.z as Real])
            .can_sleep(self.can_sleep)
            .linear_damping(self.linear_damping as Real)
            .angular_damping(self.angular_damping as Real);
        
        // Only set mass for dynamic bodies
        if self.body_type == RigidBodyType3D::Dynamic {
            builder = builder.additional_mass(self.mass as Real);
        }
        
        builder.build()
    }
}

/// Component for 3D colliders
#[derive(Debug, Clone)]
pub struct Collider3D {
    pub shape: ColliderShape3D,
    pub density: f32,
    pub restitution: f32,
    pub friction: f32,
    pub is_sensor: bool,  // If true, collider detects collisions but doesn't physically interact
}

impl Default for Collider3D {
    fn default() -> Self {
        Self {
            shape: ColliderShape3D::Ball(0.5),
            density: 1.0,
            restitution: 0.5,
            friction: 0.5,
            is_sensor: false,
        }
    }
}

impl Collider3D {
    /// Create a new ball collider
    pub fn new_ball(radius: f32) -> Self {
        Self {
            shape: ColliderShape3D::Ball(radius),
            ..Default::default()
        }
    }
    
    /// Create a new box collider
    pub fn new_cuboid(hx: f32, hy: f32, hz: f32) -> Self {
        Self {
            shape: ColliderShape3D::Cuboid(hx, hy, hz),
            ..Default::default()
        }
    }
    
    /// Create a new capsule collider
    pub fn new_capsule(half_height: f32, radius: f32) -> Self {
        Self {
            shape: ColliderShape3D::Capsule(half_height, radius),
            ..Default::default()
        }
    }
    
    /// Convert to a Rapier collider
    pub fn to_rapier(&self) -> Collider {
        let mut collider = self.shape.to_rapier();
        collider.set_density(self.density as Real);
        collider.set_restitution(self.restitution as Real);
        collider.set_friction(self.friction as Real);
        collider.set_sensor(self.is_sensor);
        collider
    }
}

/// Raycast hit result for 3D
#[derive(Debug, Clone)]
pub struct RaycastHit3D {
    pub entity: Entity,
    pub position: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

