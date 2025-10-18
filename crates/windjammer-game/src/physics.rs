//! Physics simulation using Rapier

use crate::math::Vec2;

#[cfg(feature = "3d")]
use crate::math::Vec3;

#[cfg(feature = "2d")]
pub use rapier2d::prelude::*;

#[cfg(feature = "3d")]
pub use rapier3d::prelude::*;

/// 2D Physics World wrapper
#[cfg(feature = "2d")]
pub struct PhysicsWorld {
    pub gravity: Vec2,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
}

#[cfg(feature = "2d")]
impl PhysicsWorld {
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity,
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
        }
    }

    pub fn step(&mut self) {
        let gravity = rapier2d::math::Vector::new(self.gravity.x, self.gravity.y);

        self.physics_pipeline.step(
            &gravity,
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
}

#[cfg(feature = "3d")]
pub struct PhysicsWorld {
    pub gravity: Vec3,
    // 3D physics implementation
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "2d")]
    #[test]
    fn test_physics_world_creation() {
        use super::*;
        let world = PhysicsWorld::new(Vec2::new(0.0, -9.81));
        assert_eq!(world.gravity.y, -9.81);
    }
}
