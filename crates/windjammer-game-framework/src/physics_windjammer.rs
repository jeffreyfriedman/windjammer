//! Windjammer Physics System
//!
//! High-level physics API that hides Rapier internals.
//!
//! **Philosophy**: Zero crate leakage - no Rapier types exposed to Windjammer users.

use crate::math::{Quat, Vec3};
use rapier3d::prelude::*;
use std::collections::HashMap;

/// Physics material properties
#[derive(Clone, Debug)]
pub struct PhysicsMaterial {
    /// Friction coefficient (0.0 = ice, 1.0 = rubber)
    pub friction: f32,
    
    /// Restitution/bounciness (0.0 = no bounce, 1.0 = perfect bounce)
    pub restitution: f32,
    
    /// Density (kg/m³)
    pub density: f32,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            friction: 0.5,
            restitution: 0.3,
            density: 1000.0,
        }
    }
}

/// Collision shape types
#[derive(Clone, Debug)]
pub enum CollisionShape {
    /// Box shape (half extents)
    Box { half_extents: Vec3 },
    
    /// Sphere shape
    Sphere { radius: f32 },
    
    /// Capsule shape (cylinder with hemisphere caps)
    Capsule { half_height: f32, radius: f32 },
    
    /// Cylinder shape
    Cylinder { half_height: f32, radius: f32 },
}

/// Rigid body type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyType {
    /// Dynamic body (affected by forces)
    Dynamic,
    
    /// Kinematic body (moved by velocity, not forces)
    Kinematic,
    
    /// Static body (never moves)
    Static,
}

/// Rigid body handle (opaque)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) usize);

/// Collider handle (opaque)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ColliderHandle(pub(crate) usize);

/// Constraint/joint handle (opaque)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConstraintHandle(pub(crate) usize);

/// Rigid body builder
pub struct RigidBodyBuilder {
    body_type: BodyType,
    position: Vec3,
    rotation: Quat,
    linear_velocity: Vec3,
    angular_velocity: Vec3,
    mass: f32,
    lock_rotations: bool,
}

impl RigidBodyBuilder {
    /// Create a dynamic body
    pub fn dynamic() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            mass: 1.0,
            lock_rotations: false,
        }
    }
    
    /// Create a kinematic body
    pub fn kinematic() -> Self {
        Self {
            body_type: BodyType::Kinematic,
            ..Self::dynamic()
        }
    }
    
    /// Create a static body
    pub fn static_body() -> Self {
        Self {
            body_type: BodyType::Static,
            ..Self::dynamic()
        }
    }
    
    /// Set position
    pub fn position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }
    
    /// Set rotation
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }
    
    /// Set linear velocity
    pub fn linear_velocity(mut self, velocity: Vec3) -> Self {
        self.linear_velocity = velocity;
        self
    }
    
    /// Set angular velocity
    pub fn angular_velocity(mut self, velocity: Vec3) -> Self {
        self.angular_velocity = velocity;
        self
    }
    
    /// Set mass
    pub fn mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }
    
    /// Lock rotations (useful for characters)
    pub fn lock_rotations(mut self) -> Self {
        self.lock_rotations = true;
        self
    }
}

/// Collider builder
pub struct ColliderBuilder {
    shape: CollisionShape,
    material: PhysicsMaterial,
    is_sensor: bool,
}

impl ColliderBuilder {
    /// Create a box collider
    pub fn box_shape(half_extents: Vec3) -> Self {
        Self {
            shape: CollisionShape::Box { half_extents },
            material: PhysicsMaterial::default(),
            is_sensor: false,
        }
    }
    
    /// Create a sphere collider
    pub fn sphere(radius: f32) -> Self {
        Self {
            shape: CollisionShape::Sphere { radius },
            material: PhysicsMaterial::default(),
            is_sensor: false,
        }
    }
    
    /// Create a capsule collider
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self {
            shape: CollisionShape::Capsule { half_height, radius },
            material: PhysicsMaterial::default(),
            is_sensor: false,
        }
    }
    
    /// Set material
    pub fn material(mut self, material: PhysicsMaterial) -> Self {
        self.material = material;
        self
    }
    
    /// Set friction
    pub fn friction(mut self, friction: f32) -> Self {
        self.material.friction = friction;
        self
    }
    
    /// Set restitution
    pub fn restitution(mut self, restitution: f32) -> Self {
        self.material.restitution = restitution;
        self
    }
    
    /// Make this a sensor (triggers but doesn't collide)
    pub fn sensor(mut self) -> Self {
        self.is_sensor = true;
        self
    }
}

/// Raycast hit result
#[derive(Clone, Debug)]
pub struct RaycastHit {
    /// Hit point in world space
    pub point: Vec3,
    
    /// Hit normal
    pub normal: Vec3,
    
    /// Distance from ray origin
    pub distance: f32,
    
    /// Body that was hit
    pub body: BodyHandle,
}

/// Constraint types
pub enum ConstraintType {
    /// Fixed joint (weld)
    Fixed {
        anchor_a: Vec3,
        anchor_b: Vec3,
    },
    
    /// Hinge joint (revolute)
    Hinge {
        anchor_a: Vec3,
        anchor_b: Vec3,
        axis: Vec3,
        limits: Option<(f32, f32)>,
    },
    
    /// Spring/damper
    Spring {
        anchor_a: Vec3,
        anchor_b: Vec3,
        rest_length: f32,
        stiffness: f32,
        damping: f32,
    },
}

/// Physics world
pub struct PhysicsWorldWj {
    // Rapier internals (hidden from Windjammer users)
    gravity: Vector<f32>,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    
    // Handle mappings
    body_handles: HashMap<BodyHandle, RigidBodyHandle>,
    collider_handles: HashMap<ColliderHandle, rapier3d::prelude::ColliderHandle>,
    constraint_handles: HashMap<ConstraintHandle, ImpulseJointHandle>,
    next_body_id: usize,
    next_collider_id: usize,
    next_constraint_id: usize,
}

impl PhysicsWorldWj {
    /// Create a new physics world
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity: Vector::new(gravity.x, gravity.y, gravity.z),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            body_handles: HashMap::new(),
            collider_handles: HashMap::new(),
            constraint_handles: HashMap::new(),
            next_body_id: 0,
            next_collider_id: 0,
            next_constraint_id: 0,
        }
    }
    
    /// Step the simulation
    pub fn step(&mut self, delta: f32) {
        self.integration_parameters.dt = delta;
        
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }
    
    /// Create a rigid body
    pub fn create_body(&mut self, builder: RigidBodyBuilder) -> BodyHandle {
        let body_type = match builder.body_type {
            BodyType::Dynamic => RigidBodyType::Dynamic,
            BodyType::Kinematic => RigidBodyType::KinematicPositionBased,
            BodyType::Static => RigidBodyType::Fixed,
        };
        
        let mut rb = rapier3d::prelude::RigidBodyBuilder::new(body_type)
            .translation(Vector::new(builder.position.x, builder.position.y, builder.position.z))
            .rotation(Vector::new(builder.rotation.x, builder.rotation.y, builder.rotation.z))
            .linvel(Vector::new(builder.linear_velocity.x, builder.linear_velocity.y, builder.linear_velocity.z))
            .angvel(Vector::new(builder.angular_velocity.x, builder.angular_velocity.y, builder.angular_velocity.z))
            .build();
        
        if builder.lock_rotations {
            rb.lock_rotations(true, true);
        }
        
        let rapier_handle = self.rigid_body_set.insert(rb);
        
        let handle = BodyHandle(self.next_body_id);
        self.next_body_id += 1;
        self.body_handles.insert(handle, rapier_handle);
        
        handle
    }
    
    /// Create a collider attached to a body
    pub fn create_collider(&mut self, builder: ColliderBuilder, body: BodyHandle) -> ColliderHandle {
        let shape: SharedShape = match builder.shape {
            CollisionShape::Box { half_extents } => SharedShape::cuboid(
                half_extents.x,
                half_extents.y,
                half_extents.z,
            ),
            CollisionShape::Sphere { radius } => SharedShape::ball(radius),
            CollisionShape::Capsule { half_height, radius } => SharedShape::capsule_y(half_height, radius),
            CollisionShape::Cylinder { half_height, radius } => SharedShape::cylinder(half_height, radius),
        };
        
        let collider = rapier3d::prelude::ColliderBuilder::new(shape)
            .friction(builder.material.friction)
            .restitution(builder.material.restitution)
            .density(builder.material.density)
            .sensor(builder.is_sensor)
            .build();
        
        let rapier_body_handle = *self.body_handles.get(&body).expect("Invalid body handle");
        let rapier_collider_handle = self.collider_set.insert_with_parent(
            collider,
            rapier_body_handle,
            &mut self.rigid_body_set,
        );
        
        let handle = ColliderHandle(self.next_collider_id);
        self.next_collider_id += 1;
        self.collider_handles.insert(handle, rapier_collider_handle);
        
        handle
    }
    
    /// Get body position
    pub fn get_body_position(&self, handle: BodyHandle) -> Vec3 {
        let rapier_handle = self.body_handles.get(&handle).expect("Invalid body handle");
        let body = self.rigid_body_set.get(*rapier_handle).expect("Body not found");
        let pos = body.translation();
        Vec3::new(pos.x, pos.y, pos.z)
    }
    
    /// Get body rotation
    pub fn get_body_rotation(&self, handle: BodyHandle) -> Quat {
        let rapier_handle = self.body_handles.get(&handle).expect("Invalid body handle");
        let body = self.rigid_body_set.get(*rapier_handle).expect("Body not found");
        let rot = body.rotation();
        Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w)
    }
    
    /// Set body position
    pub fn set_body_position(&mut self, handle: BodyHandle, position: Vec3) {
        let rapier_handle = self.body_handles.get(&handle).expect("Invalid body handle");
        if let Some(body) = self.rigid_body_set.get_mut(*rapier_handle) {
            body.set_translation(Vector::new(position.x, position.y, position.z), true);
        }
    }
    
    /// Apply force to body
    pub fn apply_force(&mut self, handle: BodyHandle, force: Vec3) {
        let rapier_handle = self.body_handles.get(&handle).expect("Invalid body handle");
        if let Some(body) = self.rigid_body_set.get_mut(*rapier_handle) {
            body.add_force(Vector::new(force.x, force.y, force.z), true);
        }
    }
    
    /// Apply impulse to body
    pub fn apply_impulse(&mut self, handle: BodyHandle, impulse: Vec3) {
        let rapier_handle = self.body_handles.get(&handle).expect("Invalid body handle");
        if let Some(body) = self.rigid_body_set.get_mut(*rapier_handle) {
            body.apply_impulse(Vector::new(impulse.x, impulse.y, impulse.z), true);
        }
    }
    
    /// Raycast
    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RaycastHit> {
        let ray = Ray::new(
            Point::new(origin.x, origin.y, origin.z),
            Vector::new(direction.x, direction.y, direction.z),
        );
        
        let filter = QueryFilter::default();
        
        if let Some((handle, intersection)) = self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_distance,
            true,
            filter,
        ) {
            // Find our handle
            let body_handle = self.body_handles.iter()
                .find(|(_, &rapier_handle)| {
                    if let Some(collider) = self.collider_set.get(handle) {
                        if let Some(parent) = collider.parent() {
                            return parent == rapier_handle;
                        }
                    }
                    false
                })
                .map(|(wj_handle, _)| *wj_handle)?;
            
            let point = ray.point_at(intersection.toi);
            let normal = intersection.normal;
            
            Some(RaycastHit {
                point: Vec3::new(point.x, point.y, point.z),
                normal: Vec3::new(normal.x, normal.y, normal.z),
                distance: intersection.toi,
                body: body_handle,
            })
        } else {
            None
        }
    }
    
    /// Create a constraint/joint
    pub fn create_constraint(&mut self, constraint_type: ConstraintType, body_a: BodyHandle, body_b: BodyHandle) -> ConstraintHandle {
        let rapier_a = *self.body_handles.get(&body_a).expect("Invalid body handle");
        let rapier_b = *self.body_handles.get(&body_b).expect("Invalid body handle");
        
        let joint = match constraint_type {
            ConstraintType::Fixed { anchor_a, anchor_b } => {
                GenericJoint::new(
                    Isometry::translation(anchor_a.x, anchor_a.y, anchor_a.z),
                    Isometry::translation(anchor_b.x, anchor_b.y, anchor_b.z),
                )
            }
            ConstraintType::Hinge { anchor_a, anchor_b, axis, limits } => {
                let mut joint = RevoluteJointBuilder::new(Unit::new_normalize(Vector::new(axis.x, axis.y, axis.z)))
                    .local_anchor1(Point::new(anchor_a.x, anchor_a.y, anchor_a.z))
                    .local_anchor2(Point::new(anchor_b.x, anchor_b.y, anchor_b.z))
                    .build();
                
                if let Some((min, max)) = limits {
                    joint.set_limits([min, max]);
                }
                
                joint
            }
            ConstraintType::Spring { anchor_a, anchor_b, rest_length, stiffness, damping } => {
                // Rapier doesn't have built-in springs, so we'll use a generic joint with spring params
                let mut joint = GenericJoint::new(
                    Isometry::translation(anchor_a.x, anchor_a.y, anchor_a.z),
                    Isometry::translation(anchor_b.x, anchor_b.y, anchor_b.z),
                );
                
                // Configure as spring (simplified)
                joint.set_motor_position(JointAxis::X, rest_length, stiffness, damping);
                
                joint
            }
        };
        
        let rapier_handle = self.impulse_joint_set.insert(rapier_a, rapier_b, joint, true);
        
        let handle = ConstraintHandle(self.next_constraint_id);
        self.next_constraint_id += 1;
        self.constraint_handles.insert(handle, rapier_handle);
        
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_physics_world_creation() {
        let world = PhysicsWorldWj::new(Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(world.next_body_id, 0);
    }
    
    #[test]
    fn test_rigid_body_creation() {
        let mut world = PhysicsWorldWj::new(Vec3::new(0.0, -9.81, 0.0));
        
        let body = world.create_body(
            RigidBodyBuilder::dynamic()
                .position(Vec3::new(0.0, 10.0, 0.0))
                .mass(1.0)
        );
        
        let pos = world.get_body_position(body);
        assert!((pos.y - 10.0).abs() < 0.01);
    }
    
    #[test]
    fn test_collider_creation() {
        let mut world = PhysicsWorldWj::new(Vec3::new(0.0, -9.81, 0.0));
        
        let body = world.create_body(RigidBodyBuilder::dynamic());
        let _collider = world.create_collider(
            ColliderBuilder::sphere(1.0),
            body
        );
        
        // If we got here without panic, collider was created successfully
        assert!(true);
    }
    
    #[test]
    fn test_raycast() {
        let mut world = PhysicsWorldWj::new(Vec3::new(0.0, -9.81, 0.0));
        
        // Create a static body at origin
        let body = world.create_body(
            RigidBodyBuilder::static_body()
                .position(Vec3::new(0.0, 0.0, 0.0))
        );
        
        let _collider = world.create_collider(
            ColliderBuilder::sphere(1.0),
            body
        );
        
        // Update query pipeline
        world.query_pipeline.update(&world.rigid_body_set, &world.collider_set);
        
        // Raycast from above
        let hit = world.raycast(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            20.0
        );
        
        assert!(hit.is_some());
    }
}

