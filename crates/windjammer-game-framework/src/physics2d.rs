//! 2D Physics System using Rapier2D
//!
//! Provides a high-level API for 2D physics simulation integrated with the ECS.

use crate::ecs::{Entity, World};
use crate::math::Vec2;
use rapier2d::prelude::*;
use rapier2d::dynamics::RigidBodyType as RapierRigidBodyType;
use std::collections::HashMap;

/// Physics world for 2D simulation
pub struct PhysicsWorld2D {
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

impl PhysicsWorld2D {
    /// Create a new physics world with default gravity
    pub fn new() -> Self {
        Self::with_gravity(Vec2::new(0.0, -9.81))
    }
    
    /// Create a new physics world with custom gravity
    pub fn with_gravity(gravity: Vec2) -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: vector![gravity.x as Real, gravity.y as Real],
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
    pub fn set_gravity(&mut self, gravity: Vec2) {
        self.gravity = vector![gravity.x as Real, gravity.y as Real];
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
    pub fn add_collider(&mut self, collider: Collider, body_handle: RigidBodyHandle) -> ColliderHandle {
        self.colliders.insert_with_parent(collider, body_handle, &mut self.bodies)
    }
    
    /// Get rigid body handle for an entity
    pub fn get_body_handle(&self, entity: Entity) -> Option<RigidBodyHandle> {
        self.entity_to_body.get(&entity).copied()
    }
    
    /// Get entity for a rigid body handle
    pub fn get_entity(&self, handle: RigidBodyHandle) -> Option<Entity> {
        self.body_to_entity.get(&handle).copied()
    }
    
    /// Get rigid body for an entity
    pub fn get_body(&self, entity: Entity) -> Option<&RigidBody> {
        let handle = self.entity_to_body.get(&entity)?;
        self.bodies.get(*handle)
    }
    
    /// Get mutable rigid body for an entity
    pub fn get_body_mut(&mut self, entity: Entity) -> Option<&mut RigidBody> {
        let handle = self.entity_to_body.get(&entity)?;
        self.bodies.get_mut(*handle)
    }
    
    /// Raycast in the physics world
    pub fn raycast(&self, origin: Vec2, direction: Vec2, max_distance: f64) -> Option<(Entity, f64)> {
        let ray = Ray::new(
            point![origin.x as Real, origin.y as Real],
            vector![direction.x as Real, direction.y as Real],
        );
        
        self.query_pipeline.cast_ray(
            &self.bodies,
            &self.colliders,
            &ray,
            max_distance as Real,
            true,
            QueryFilter::default(),
        ).and_then(|(handle, toi)| {
            let body_handle = self.colliders.get(handle)?.parent()?;
            let entity = self.body_to_entity.get(&body_handle)?;
            Some((*entity, toi as f64))
        })
    }
}

impl Default for PhysicsWorld2D {
    fn default() -> Self {
        Self::new()
    }
}

/// ECS Component: Rigid body type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigidBodyType {
    /// Dynamic body (affected by forces)
    Dynamic,
    /// Fixed body (immovable)
    Fixed,
    /// Kinematic body (controlled by velocity)
    Kinematic,
}

/// ECS Component: 2D Rigid Body
#[derive(Debug, Clone)]
pub struct RigidBody2D {
    pub body_type: RigidBodyType,
    pub position: Vec2,
    pub rotation: f64,
    pub velocity: Vec2,
    pub angular_velocity: f64,
    pub mass: f64,
    pub restitution: f64,  // Bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    pub friction: f64,     // Surface friction (0.0 = ice, 1.0 = sticky)
}

impl RigidBody2D {
    pub fn new_dynamic(position: Vec2) -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            position,
            rotation: 0.0,
            velocity: Vec2::new(0.0, 0.0),
            angular_velocity: 0.0,
            mass: 1.0,
            restitution: 0.3,
            friction: 0.5,
        }
    }
    
    pub fn new_fixed(position: Vec2) -> Self {
        Self {
            body_type: RigidBodyType::Fixed,
            position,
            rotation: 0.0,
            velocity: Vec2::new(0.0, 0.0),
            angular_velocity: 0.0,
            mass: 0.0,  // Fixed bodies have infinite mass
            restitution: 0.3,
            friction: 0.5,
        }
    }
    
    pub fn new_kinematic(position: Vec2) -> Self {
        Self {
            body_type: RigidBodyType::Kinematic,
            position,
            rotation: 0.0,
            velocity: Vec2::new(0.0, 0.0),
            angular_velocity: 0.0,
            mass: 0.0,  // Kinematic bodies have infinite mass
            restitution: 0.3,
            friction: 0.5,
        }
    }
    
    /// Convert to Rapier rigid body
    pub(crate) fn to_rapier(&self) -> RigidBody {
        let body_type = match self.body_type {
            RigidBodyType::Dynamic => RapierRigidBodyType::Dynamic,
            RigidBodyType::Fixed => RapierRigidBodyType::Fixed,
            RigidBodyType::Kinematic => RapierRigidBodyType::KinematicVelocityBased,
        };
        
        RigidBodyBuilder::new(body_type)
            .translation(vector![self.position.x as Real, self.position.y as Real])
            .rotation(self.rotation as Real)
            .linvel(vector![self.velocity.x as Real, self.velocity.y as Real])
            .angvel(self.angular_velocity as Real)
            .build()
    }
}

/// ECS Component: 2D Collider shape
#[derive(Debug, Clone)]
pub enum ColliderShape2D {
    /// Rectangle collider
    Box { width: f64, height: f64 },
    /// Circle collider
    Circle { radius: f64 },
    /// Capsule collider (rounded rectangle)
    Capsule { half_height: f64, radius: f64 },
}

/// ECS Component: 2D Collider
#[derive(Debug, Clone)]
pub struct Collider2D {
    pub shape: ColliderShape2D,
    pub is_sensor: bool,  // If true, detects collisions but doesn't physically interact
}

impl Collider2D {
    pub fn box_collider(width: f64, height: f64) -> Self {
        Self {
            shape: ColliderShape2D::Box { width, height },
            is_sensor: false,
        }
    }
    
    pub fn circle_collider(radius: f64) -> Self {
        Self {
            shape: ColliderShape2D::Circle { radius },
            is_sensor: false,
        }
    }
    
    pub fn capsule_collider(half_height: f64, radius: f64) -> Self {
        Self {
            shape: ColliderShape2D::Capsule { half_height, radius },
            is_sensor: false,
        }
    }
    
    /// Convert to Rapier collider
    pub(crate) fn to_rapier(&self, rigid_body: &RigidBody2D) -> Collider {
        let shape: SharedShape = match self.shape {
            ColliderShape2D::Box { width, height } => {
                SharedShape::cuboid((width / 2.0) as Real, (height / 2.0) as Real)
            }
            ColliderShape2D::Circle { radius } => {
                SharedShape::ball(radius as Real)
            }
            ColliderShape2D::Capsule { half_height, radius } => {
                SharedShape::capsule_y(half_height as Real, radius as Real)
            }
        };
        
        ColliderBuilder::new(shape)
            .restitution(rigid_body.restitution as Real)
            .friction(rigid_body.friction as Real)
            .sensor(self.is_sensor)
            .build()
    }
}

/// System: Sync ECS transforms with physics bodies
pub fn sync_physics_transforms(world: &mut World, physics: &PhysicsWorld2D) {
    // TODO: Implement once we have Transform component in ECS
    // For now, we'll update RigidBody2D components directly
}

/// System: Sync physics bodies with ECS transforms
pub fn sync_transforms_to_physics(world: &World, physics: &mut PhysicsWorld2D) {
    // TODO: Implement once we have Transform component in ECS
}

