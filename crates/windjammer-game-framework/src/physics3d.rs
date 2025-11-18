//! 3D Physics System using Rapier3D
//!
//! Provides a high-level API for 3D physics simulation.

#[cfg(feature = "3d")]
use rapier3d::prelude::*;

use crate::math::{Quat, Vec3};
use std::collections::HashMap;

// Re-export handle types for external use
#[cfg(feature = "3d")]
pub use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};

/// 3D Physics World
#[cfg(feature = "3d")]
pub struct PhysicsWorld3D {
    /// Rapier rigid body set
    pub rigid_body_set: RigidBodySet,
    
    /// Rapier collider set
    pub collider_set: ColliderSet,
    
    /// Gravity vector
    pub gravity: Vec3,
    
    /// Integration parameters
    pub integration_parameters: IntegrationParameters,
    
    /// Physics pipeline
    pub physics_pipeline: PhysicsPipeline,
    
    /// Island manager
    pub island_manager: IslandManager,
    
    /// Broad phase
    pub broad_phase: BroadPhase,
    
    /// Narrow phase
    pub narrow_phase: NarrowPhase,
    
    /// Impulse joint set
    pub impulse_joint_set: ImpulseJointSet,
    
    /// Multibody joint set
    pub multibody_joint_set: MultibodyJointSet,
    
    /// CCD solver
    pub ccd_solver: CCDSolver,
    
    /// Query pipeline (for raycasts, etc.)
    pub query_pipeline: QueryPipeline,
    
    /// Entity to rigid body handle mapping
    entity_to_body: HashMap<u64, RigidBodyHandle>,
    
    /// Entity to collider handle mapping
    entity_to_collider: HashMap<u64, ColliderHandle>,
}

#[cfg(feature = "3d")]
impl PhysicsWorld3D {
    /// Create a new 3D physics world
    pub fn new(gravity: Vec3) -> Self {
        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            gravity,
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            entity_to_body: HashMap::new(),
            entity_to_collider: HashMap::new(),
        }
    }
    
    /// Step the physics simulation
    pub fn step(&mut self, delta: f32) {
        self.integration_parameters.dt = delta;
        
        let gravity_vector = vector![self.gravity.x, self.gravity.y, self.gravity.z];
        
        self.physics_pipeline.step(
            &gravity_vector,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None, // query pipeline update
            &(), // hooks
            &(), // events
        );
        
        // Update query pipeline
        self.query_pipeline.update(&self.rigid_body_set, &self.collider_set);
    }
    
    /// Create a dynamic rigid body
    pub fn create_dynamic_body(
        &mut self,
        entity_id: u64,
        position: Vec3,
        rotation: Quat,
    ) -> RigidBodyHandle {
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![position.x, position.y, position.z])
            .rotation(vector![rotation.x, rotation.y, rotation.z, rotation.w])
            .build();
        
        let handle = self.rigid_body_set.insert(rigid_body);
        self.entity_to_body.insert(entity_id, handle);
        handle
    }
    
    /// Create a static rigid body
    pub fn create_static_body(
        &mut self,
        entity_id: u64,
        position: Vec3,
        rotation: Quat,
    ) -> RigidBodyHandle {
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(vector![position.x, position.y, position.z])
            .rotation(vector![rotation.x, rotation.y, rotation.z, rotation.w])
            .build();
        
        let handle = self.rigid_body_set.insert(rigid_body);
        self.entity_to_body.insert(entity_id, handle);
        handle
    }
    
    /// Create a kinematic rigid body
    pub fn create_kinematic_body(
        &mut self,
        entity_id: u64,
        position: Vec3,
        rotation: Quat,
    ) -> RigidBodyHandle {
        let rigid_body = RigidBodyBuilder::kinematic_position_based()
            .translation(vector![position.x, position.y, position.z])
            .rotation(vector![rotation.x, rotation.y, rotation.z, rotation.w])
            .build();
        
        let handle = self.rigid_body_set.insert(rigid_body);
        self.entity_to_body.insert(entity_id, handle);
        handle
    }
    
    /// Add a box collider to a rigid body
    pub fn add_box_collider(
        &mut self,
        entity_id: u64,
        body_handle: RigidBodyHandle,
        half_extents: Vec3,
    ) -> ColliderHandle {
        let collider = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .build();
        
        let handle = self.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut self.rigid_body_set,
        );
        
        self.entity_to_collider.insert(entity_id, handle);
        handle
    }
    
    /// Add a sphere collider to a rigid body
    pub fn add_sphere_collider(
        &mut self,
        entity_id: u64,
        body_handle: RigidBodyHandle,
        radius: f32,
    ) -> ColliderHandle {
        let collider = ColliderBuilder::ball(radius).build();
        
        let handle = self.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut self.rigid_body_set,
        );
        
        self.entity_to_collider.insert(entity_id, handle);
        handle
    }
    
    /// Add a capsule collider to a rigid body
    pub fn add_capsule_collider(
        &mut self,
        entity_id: u64,
        body_handle: RigidBodyHandle,
        half_height: f32,
        radius: f32,
    ) -> ColliderHandle {
        let collider = ColliderBuilder::capsule_y(half_height, radius).build();
        
        let handle = self.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut self.rigid_body_set,
        );
        
        self.entity_to_collider.insert(entity_id, handle);
        handle
    }
    
    /// Add a cylinder collider to a rigid body
    pub fn add_cylinder_collider(
        &mut self,
        entity_id: u64,
        body_handle: RigidBodyHandle,
        half_height: f32,
        radius: f32,
    ) -> ColliderHandle {
        let collider = ColliderBuilder::cylinder(half_height, radius).build();
        
        let handle = self.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut self.rigid_body_set,
        );
        
        self.entity_to_collider.insert(entity_id, handle);
        handle
    }
    
    /// Add a mesh collider to a rigid body
    pub fn add_mesh_collider(
        &mut self,
        entity_id: u64,
        body_handle: RigidBodyHandle,
        vertices: Vec<Vec3>,
        indices: Vec<[u32; 3]>,
    ) -> ColliderHandle {
        let rapier_vertices: Vec<Point<Real>> = vertices
            .iter()
            .map(|v| Point::new(v.x, v.y, v.z))
            .collect();
        
        let collider = ColliderBuilder::trimesh(rapier_vertices, indices).build();
        
        let handle = self.collider_set.insert_with_parent(
            collider,
            body_handle,
            &mut self.rigid_body_set,
        );
        
        self.entity_to_collider.insert(entity_id, handle);
        handle
    }
    
    /// Get rigid body handle for entity
    pub fn get_body_handle(&self, entity_id: u64) -> Option<RigidBodyHandle> {
        self.entity_to_body.get(&entity_id).copied()
    }
    
    /// Get collider handle for entity
    pub fn get_collider_handle(&self, entity_id: u64) -> Option<ColliderHandle> {
        self.entity_to_collider.get(&entity_id).copied()
    }
    
    /// Get rigid body position and rotation
    pub fn get_body_transform(&self, handle: RigidBodyHandle) -> Option<(Vec3, Quat)> {
        self.rigid_body_set.get(handle).map(|body| {
            let pos = body.translation();
            let rot = body.rotation();
            
            (
                Vec3::new(pos.x, pos.y, pos.z),
                Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w),
            )
        })
    }
    
    /// Set rigid body position and rotation
    pub fn set_body_transform(&mut self, handle: RigidBodyHandle, position: Vec3, rotation: Quat) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.set_translation(vector![position.x, position.y, position.z], true);
            body.set_rotation(vector![rotation.x, rotation.y, rotation.z, rotation.w], true);
        }
    }
    
    /// Apply force to rigid body
    pub fn apply_force(&mut self, handle: RigidBodyHandle, force: Vec3) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.add_force(vector![force.x, force.y, force.z], true);
        }
    }
    
    /// Apply impulse to rigid body
    pub fn apply_impulse(&mut self, handle: RigidBodyHandle, impulse: Vec3) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);
        }
    }
    
    /// Apply torque to rigid body
    pub fn apply_torque(&mut self, handle: RigidBodyHandle, torque: Vec3) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.add_torque(vector![torque.x, torque.y, torque.z], true);
        }
    }
    
    /// Set rigid body velocity
    pub fn set_velocity(&mut self, handle: RigidBodyHandle, velocity: Vec3) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.set_linvel(vector![velocity.x, velocity.y, velocity.z], true);
        }
    }
    
    /// Get rigid body velocity
    pub fn get_velocity(&self, handle: RigidBodyHandle) -> Option<Vec3> {
        self.rigid_body_set.get(handle).map(|body| {
            let vel = body.linvel();
            Vec3::new(vel.x, vel.y, vel.z)
        })
    }
    
    /// Raycast
    pub fn raycast(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        solid: bool,
    ) -> Option<RaycastHit> {
        let ray = Ray::new(
            Point::new(origin.x, origin.y, origin.z),
            vector![direction.x, direction.y, direction.z],
        );
        
        self.query_pipeline
            .cast_ray(
                &self.rigid_body_set,
                &self.collider_set,
                &ray,
                max_distance,
                solid,
                QueryFilter::default(),
            )
            .map(|(handle, toi)| {
                let point = ray.point_at(toi);
                RaycastHit {
                    collider_handle: handle,
                    distance: toi,
                    point: Vec3::new(point.x, point.y, point.z),
                    normal: Vec3::ZERO, // Would need to compute from collider
                }
            })
    }
    
    /// Remove rigid body
    pub fn remove_body(&mut self, entity_id: u64) {
        if let Some(handle) = self.entity_to_body.remove(&entity_id) {
            self.rigid_body_set.remove(
                handle,
                &mut self.island_manager,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                true,
            );
        }
    }
    
    /// Remove collider
    pub fn remove_collider(&mut self, entity_id: u64) {
        if let Some(handle) = self.entity_to_collider.remove(&entity_id) {
            self.collider_set.remove(
                handle,
                &mut self.island_manager,
                &mut self.rigid_body_set,
                true,
            );
        }
    }
    
    /// Set rigid body mass
    pub fn set_mass(&mut self, handle: RigidBodyHandle, mass: f32) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.set_additional_mass(mass, true);
        }
    }
    
    /// Set linear damping
    pub fn set_linear_damping(&mut self, handle: RigidBodyHandle, damping: f32) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.set_linear_damping(damping);
        }
    }
    
    /// Set angular damping
    pub fn set_angular_damping(&mut self, handle: RigidBodyHandle, damping: f32) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.set_angular_damping(damping);
        }
    }
    
    /// Enable or disable a rigid body
    pub fn set_enabled(&mut self, handle: RigidBodyHandle, enabled: bool) {
        if let Some(body) = self.rigid_body_set.get_mut(handle) {
            body.set_enabled(enabled);
        }
    }
    
    /// Get rigid body position
    pub fn get_body_position(&self, handle: RigidBodyHandle) -> Option<Vec3> {
        self.rigid_body_set.get(handle).map(|body| {
            let pos = body.translation();
            Vec3::new(pos.x, pos.y, pos.z)
        })
    }
    
    /// Get rigid body rotation
    pub fn get_body_rotation(&self, handle: RigidBodyHandle) -> Option<Quat> {
        self.rigid_body_set.get(handle).map(|body| {
            let rot = body.rotation();
            Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w)
        })
    }
}

/// Raycast hit result
#[cfg(feature = "3d")]
#[derive(Debug, Clone)]
pub struct RaycastHit {
    /// Collider that was hit
    pub collider_handle: ColliderHandle,
    
    /// Distance to hit point
    pub distance: f32,
    
    /// Hit point in world space
    pub point: Vec3,
    
    /// Surface normal at hit point
    pub normal: Vec3,
}

#[cfg(not(feature = "3d"))]
pub struct PhysicsWorld3D;

#[cfg(not(feature = "3d"))]
impl PhysicsWorld3D {
    pub fn new(_gravity: Vec3) -> Self {
        panic!("3D physics requires the '3d' feature to be enabled");
    }
}

#[cfg(all(test, feature = "3d"))]
mod tests {
    use super::*;
    
    #[test]
    fn test_physics_world_creation() {
        let world = PhysicsWorld3D::new(Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(world.gravity, Vec3::new(0.0, -9.81, 0.0));
    }
    
    #[test]
    fn test_create_dynamic_body() {
        let mut world = PhysicsWorld3D::new(Vec3::new(0.0, -9.81, 0.0));
        let handle = world.create_dynamic_body(
            1,
            Vec3::new(0.0, 10.0, 0.0),
            Quat::IDENTITY,
        );
        
        assert!(world.rigid_body_set.get(handle).is_some());
        assert_eq!(world.get_body_handle(1), Some(handle));
    }
    
    #[test]
    fn test_add_colliders() {
        let mut world = PhysicsWorld3D::new(Vec3::new(0.0, -9.81, 0.0));
        let body_handle = world.create_dynamic_body(
            1,
            Vec3::ZERO,
            Quat::IDENTITY,
        );
        
        // Box collider
        let box_handle = world.add_box_collider(1, body_handle, Vec3::new(1.0, 1.0, 1.0));
        assert!(world.collider_set.get(box_handle).is_some());
        
        // Sphere collider
        let sphere_handle = world.add_sphere_collider(2, body_handle, 1.0);
        assert!(world.collider_set.get(sphere_handle).is_some());
    }
    
    #[test]
    fn test_physics_step() {
        let mut world = PhysicsWorld3D::new(Vec3::new(0.0, -9.81, 0.0));
        let handle = world.create_dynamic_body(
            1,
            Vec3::new(0.0, 10.0, 0.0),
            Quat::IDENTITY,
        );
        world.add_sphere_collider(1, handle, 1.0);
        
        // Step simulation
        world.step(1.0 / 60.0);
        
        // Body should have fallen due to gravity
        if let Some((pos, _)) = world.get_body_transform(handle) {
            assert!(pos.y < 10.0);
        }
    }
}
