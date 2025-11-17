// Ragdoll Physics System
// Provides realistic physics-based character animation for death, knockback, and interactions.

use crate::animation::Skeleton;
use crate::math::{Quat, Vec3};
use crate::physics3d::{ColliderHandle, PhysicsWorld3D, RigidBodyHandle};
use std::collections::HashMap;

/// Ragdoll configuration for a character
#[derive(Debug, Clone)]
pub struct RagdollConfig {
    /// Total mass of the ragdoll (distributed across bones)
    pub total_mass: f32,
    /// Density for auto-calculated masses
    pub density: f32,
    /// Angular damping (0.0 = no damping, 1.0 = full damping)
    pub angular_damping: f32,
    /// Linear damping (0.0 = no damping, 1.0 = full damping)
    pub linear_damping: f32,
    /// Whether to use CCD (Continuous Collision Detection)
    pub use_ccd: bool,
    /// Collision groups for ragdoll bodies
    pub collision_group: u32,
    /// Joint stiffness (0.0 = loose, 1.0 = rigid)
    pub joint_stiffness: f32,
    /// Joint damping
    pub joint_damping: f32,
}

impl Default for RagdollConfig {
    fn default() -> Self {
        Self {
            total_mass: 70.0, // Average human mass in kg
            density: 1000.0,  // kg/m³ (similar to water)
            angular_damping: 0.5,
            linear_damping: 0.1,
            use_ccd: true,
            collision_group: 1,
            joint_stiffness: 0.3,
            joint_damping: 0.5,
        }
    }
}

impl RagdollConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_mass(mut self, mass: f32) -> Self {
        self.total_mass = mass;
        self
    }

    pub fn density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    pub fn angular_damping(mut self, damping: f32) -> Self {
        self.angular_damping = damping.clamp(0.0, 1.0);
        self
    }

    pub fn linear_damping(mut self, damping: f32) -> Self {
        self.linear_damping = damping.clamp(0.0, 1.0);
        self
    }

    pub fn use_ccd(mut self, use_ccd: bool) -> Self {
        self.use_ccd = use_ccd;
        self
    }

    pub fn collision_group(mut self, group: u32) -> Self {
        self.collision_group = group;
        self
    }

    pub fn joint_stiffness(mut self, stiffness: f32) -> Self {
        self.joint_stiffness = stiffness.clamp(0.0, 1.0);
        self
    }

    pub fn joint_damping(mut self, damping: f32) -> Self {
        self.joint_damping = damping.clamp(0.0, 1.0);
        self
    }
}

/// Bone shape for ragdoll colliders
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoneShape {
    /// Capsule collider (radius, half_height)
    Capsule { radius: f32, half_height: f32 },
    /// Box collider (half_extents)
    Box { half_extents: Vec3 },
    /// Sphere collider (radius)
    Sphere { radius: f32 },
}

impl BoneShape {
    pub fn capsule(radius: f32, half_height: f32) -> Self {
        Self::Capsule {
            radius,
            half_height,
        }
    }

    pub fn box_shape(half_extents: Vec3) -> Self {
        Self::Box { half_extents }
    }

    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    /// Estimate volume for mass calculation
    pub fn volume(&self) -> f32 {
        match self {
            BoneShape::Capsule {
                radius,
                half_height,
            } => {
                // Volume = cylinder + 2 hemispheres
                let cylinder_volume = std::f32::consts::PI * radius * radius * (half_height * 2.0);
                let sphere_volume = (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3);
                cylinder_volume + sphere_volume
            }
            BoneShape::Box { half_extents } => {
                // Volume = length * width * height
                (half_extents.x * 2.0) * (half_extents.y * 2.0) * (half_extents.z * 2.0)
            }
            BoneShape::Sphere { radius } => {
                // Volume = (4/3) * π * r³
                (4.0 / 3.0) * std::f32::consts::PI * radius.powi(3)
            }
        }
    }
}

/// Joint configuration for connecting ragdoll bones
#[derive(Debug, Clone)]
pub struct RagdollJoint {
    /// Parent bone index
    pub parent_bone: usize,
    /// Child bone index
    pub child_bone: usize,
    /// Anchor point on parent (local space)
    pub parent_anchor: Vec3,
    /// Anchor point on child (local space)
    pub child_anchor: Vec3,
    /// Joint axis (for hinge joints)
    pub axis: Vec3,
    /// Minimum angle (degrees)
    pub min_angle: f32,
    /// Maximum angle (degrees)
    pub max_angle: f32,
    /// Joint type
    pub joint_type: JointType,
}

/// Type of joint between bones
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointType {
    /// Ball and socket joint (3 DOF)
    Spherical,
    /// Hinge joint (1 DOF)
    Revolute,
    /// Fixed joint (0 DOF)
    Fixed,
}

impl RagdollJoint {
    pub fn spherical(
        parent_bone: usize,
        child_bone: usize,
        parent_anchor: Vec3,
        child_anchor: Vec3,
    ) -> Self {
        Self {
            parent_bone,
            child_bone,
            parent_anchor,
            child_anchor,
            axis: Vec3::new(0.0, 1.0, 0.0),
            min_angle: -180.0,
            max_angle: 180.0,
            joint_type: JointType::Spherical,
        }
    }

    pub fn revolute(
        parent_bone: usize,
        child_bone: usize,
        parent_anchor: Vec3,
        child_anchor: Vec3,
        axis: Vec3,
        min_angle: f32,
        max_angle: f32,
    ) -> Self {
        Self {
            parent_bone,
            child_bone,
            parent_anchor,
            child_anchor,
            axis,
            min_angle,
            max_angle,
            joint_type: JointType::Revolute,
        }
    }

    pub fn fixed(
        parent_bone: usize,
        child_bone: usize,
        parent_anchor: Vec3,
        child_anchor: Vec3,
    ) -> Self {
        Self {
            parent_bone,
            child_bone,
            parent_anchor,
            child_anchor,
            axis: Vec3::new(0.0, 1.0, 0.0),
            min_angle: 0.0,
            max_angle: 0.0,
            joint_type: JointType::Fixed,
        }
    }
}

/// Ragdoll bone definition
#[derive(Debug, Clone)]
pub struct RagdollBone {
    /// Bone name (matches skeleton bone name)
    pub name: String,
    /// Bone index in skeleton
    pub bone_index: usize,
    /// Collider shape
    pub shape: BoneShape,
    /// Mass (kg)
    pub mass: f32,
    /// Rigid body handle (set when created)
    pub body_handle: Option<RigidBodyHandle>,
    /// Collider handle (set when created)
    pub collider_handle: Option<ColliderHandle>,
}

impl RagdollBone {
    pub fn new(name: String, bone_index: usize, shape: BoneShape, mass: f32) -> Self {
        Self {
            name,
            bone_index,
            shape,
            mass,
            body_handle: None,
            collider_handle: None,
        }
    }
}

/// Ragdoll instance
pub struct Ragdoll {
    /// Entity ID this ragdoll belongs to
    pub entity_id: u64,
    /// Configuration
    pub config: RagdollConfig,
    /// Bones in the ragdoll
    pub bones: Vec<RagdollBone>,
    /// Joints connecting bones
    pub joints: Vec<RagdollJoint>,
    /// Whether the ragdoll is active
    pub is_active: bool,
    /// Original skeleton (for reference)
    pub skeleton: Option<Skeleton>,
}

impl Ragdoll {
    /// Create a new ragdoll
    pub fn new(entity_id: u64, config: RagdollConfig) -> Self {
        Self {
            entity_id,
            config,
            bones: Vec::new(),
            joints: Vec::new(),
            is_active: false,
            skeleton: None,
        }
    }

    /// Add a bone to the ragdoll
    pub fn add_bone(&mut self, bone: RagdollBone) {
        self.bones.push(bone);
    }

    /// Add a joint to the ragdoll
    pub fn add_joint(&mut self, joint: RagdollJoint) {
        self.joints.push(joint);
    }

    /// Create physics bodies for all bones
    pub fn create_physics_bodies(&mut self, physics: &mut PhysicsWorld3D, positions: &[Vec3]) {
        for (i, bone) in self.bones.iter_mut().enumerate() {
            let position = if i < positions.len() {
                positions[i]
            } else {
                Vec3::new(0.0, 0.0, 0.0)
            };

            // Create dynamic rigid body
            let body_handle =
                physics.create_dynamic_body(self.entity_id, position, Quat::IDENTITY);

            // Create collider based on shape
            let collider_handle = match bone.shape {
                BoneShape::Capsule {
                    radius,
                    half_height,
                } => physics.add_capsule_collider(self.entity_id, body_handle, half_height, radius),
                BoneShape::Box { half_extents } => {
                    physics.add_box_collider(self.entity_id, body_handle, half_extents)
                }
                BoneShape::Sphere { radius } => {
                    physics.add_sphere_collider(self.entity_id, body_handle, radius)
                }
            };

            bone.body_handle = Some(body_handle);
            bone.collider_handle = Some(collider_handle);

            // Set mass and damping
            physics.set_mass(body_handle, bone.mass);
            physics.set_linear_damping(body_handle, self.config.linear_damping);
            physics.set_angular_damping(body_handle, self.config.angular_damping);
        }
    }

    /// Activate the ragdoll (switch from animated to physics-driven)
    pub fn activate(&mut self, physics: &mut PhysicsWorld3D) {
        if self.is_active {
            return;
        }

        // Enable all rigid bodies
        for bone in &self.bones {
            if let Some(body_handle) = bone.body_handle {
                physics.set_enabled(body_handle, true);
            }
        }

        self.is_active = true;
    }

    /// Deactivate the ragdoll (switch back to animated)
    pub fn deactivate(&mut self, physics: &mut PhysicsWorld3D) {
        if !self.is_active {
            return;
        }

        // Disable all rigid bodies
        for bone in &self.bones {
            if let Some(body_handle) = bone.body_handle {
                physics.set_enabled(body_handle, false);
            }
        }

        self.is_active = false;
    }

    /// Apply an impulse to a specific bone
    pub fn apply_impulse(&mut self, physics: &mut PhysicsWorld3D, bone_index: usize, impulse: Vec3) {
        if bone_index < self.bones.len() {
            if let Some(body_handle) = self.bones[bone_index].body_handle {
                physics.apply_impulse(body_handle, impulse);
            }
        }
    }

    /// Apply a force to a specific bone
    pub fn apply_force(&mut self, physics: &mut PhysicsWorld3D, bone_index: usize, force: Vec3) {
        if bone_index < self.bones.len() {
            if let Some(body_handle) = self.bones[bone_index].body_handle {
                physics.apply_force(body_handle, force);
            }
        }
    }

    /// Get the position of a bone
    pub fn get_bone_position(&self, physics: &PhysicsWorld3D, bone_index: usize) -> Option<Vec3> {
        if bone_index < self.bones.len() {
            if let Some(body_handle) = self.bones[bone_index].body_handle {
                return physics.get_body_position(body_handle);
            }
        }
        None
    }

    /// Get the rotation of a bone
    pub fn get_bone_rotation(&self, physics: &PhysicsWorld3D, bone_index: usize) -> Option<Quat> {
        if bone_index < self.bones.len() {
            if let Some(body_handle) = self.bones[bone_index].body_handle {
                return physics.get_body_rotation(body_handle);
            }
        }
        None
    }

    /// Get the number of bones
    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    /// Get the number of joints
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }
}

/// Ragdoll builder for common character setups
pub struct RagdollBuilder {
    config: RagdollConfig,
    bones: Vec<RagdollBone>,
    joints: Vec<RagdollJoint>,
}

impl RagdollBuilder {
    pub fn new() -> Self {
        Self {
            config: RagdollConfig::default(),
            bones: Vec::new(),
            joints: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: RagdollConfig) -> Self {
        self.config = config;
        self
    }

    pub fn add_bone(mut self, bone: RagdollBone) -> Self {
        self.bones.push(bone);
        self
    }

    pub fn add_joint(mut self, joint: RagdollJoint) -> Self {
        self.joints.push(joint);
        self
    }

    pub fn build(self, entity_id: u64) -> Ragdoll {
        let mut ragdoll = Ragdoll::new(entity_id, self.config);
        for bone in self.bones {
            ragdoll.add_bone(bone);
        }
        for joint in self.joints {
            ragdoll.add_joint(joint);
        }
        ragdoll
    }

    /// Create a humanoid ragdoll (standard biped)
    pub fn humanoid(entity_id: u64) -> Ragdoll {
        let config = RagdollConfig::default();
        let mut ragdoll = Ragdoll::new(entity_id, config);

        // Spine/Torso
        ragdoll.add_bone(RagdollBone::new(
            "spine".to_string(),
            0,
            BoneShape::capsule(0.15, 0.3),
            20.0,
        ));

        // Head
        ragdoll.add_bone(RagdollBone::new(
            "head".to_string(),
            1,
            BoneShape::sphere(0.12),
            5.0,
        ));

        // Left Upper Arm
        ragdoll.add_bone(RagdollBone::new(
            "left_upper_arm".to_string(),
            2,
            BoneShape::capsule(0.06, 0.15),
            3.0,
        ));

        // Left Lower Arm
        ragdoll.add_bone(RagdollBone::new(
            "left_lower_arm".to_string(),
            3,
            BoneShape::capsule(0.05, 0.15),
            2.0,
        ));

        // Right Upper Arm
        ragdoll.add_bone(RagdollBone::new(
            "right_upper_arm".to_string(),
            4,
            BoneShape::capsule(0.06, 0.15),
            3.0,
        ));

        // Right Lower Arm
        ragdoll.add_bone(RagdollBone::new(
            "right_lower_arm".to_string(),
            5,
            BoneShape::capsule(0.05, 0.15),
            2.0,
        ));

        // Left Upper Leg
        ragdoll.add_bone(RagdollBone::new(
            "left_upper_leg".to_string(),
            6,
            BoneShape::capsule(0.08, 0.25),
            8.0,
        ));

        // Left Lower Leg
        ragdoll.add_bone(RagdollBone::new(
            "left_lower_leg".to_string(),
            7,
            BoneShape::capsule(0.06, 0.25),
            5.0,
        ));

        // Right Upper Leg
        ragdoll.add_bone(RagdollBone::new(
            "right_upper_leg".to_string(),
            8,
            BoneShape::capsule(0.08, 0.25),
            8.0,
        ));

        // Right Lower Leg
        ragdoll.add_bone(RagdollBone::new(
            "right_lower_leg".to_string(),
            9,
            BoneShape::capsule(0.06, 0.25),
            5.0,
        ));

        // Add joints (simplified - in production, these would have proper anchors and limits)
        // Spine to Head
        ragdoll.add_joint(RagdollJoint::spherical(
            0,
            1,
            Vec3::new(0.0, 0.3, 0.0),
            Vec3::new(0.0, -0.12, 0.0),
        ));

        // Spine to Left Upper Arm
        ragdoll.add_joint(RagdollJoint::spherical(
            0,
            2,
            Vec3::new(-0.15, 0.2, 0.0),
            Vec3::new(0.0, 0.15, 0.0),
        ));

        // Left Upper Arm to Left Lower Arm (elbow)
        ragdoll.add_joint(RagdollJoint::revolute(
            2,
            3,
            Vec3::new(0.0, -0.15, 0.0),
            Vec3::new(0.0, 0.15, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            0.0,
            150.0,
        ));

        // Spine to Right Upper Arm
        ragdoll.add_joint(RagdollJoint::spherical(
            0,
            4,
            Vec3::new(0.15, 0.2, 0.0),
            Vec3::new(0.0, 0.15, 0.0),
        ));

        // Right Upper Arm to Right Lower Arm (elbow)
        ragdoll.add_joint(RagdollJoint::revolute(
            4,
            5,
            Vec3::new(0.0, -0.15, 0.0),
            Vec3::new(0.0, 0.15, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            0.0,
            150.0,
        ));

        // Spine to Left Upper Leg
        ragdoll.add_joint(RagdollJoint::spherical(
            0,
            6,
            Vec3::new(-0.1, -0.3, 0.0),
            Vec3::new(0.0, 0.25, 0.0),
        ));

        // Left Upper Leg to Left Lower Leg (knee)
        ragdoll.add_joint(RagdollJoint::revolute(
            6,
            7,
            Vec3::new(0.0, -0.25, 0.0),
            Vec3::new(0.0, 0.25, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            -150.0,
            0.0,
        ));

        // Spine to Right Upper Leg
        ragdoll.add_joint(RagdollJoint::spherical(
            0,
            8,
            Vec3::new(0.1, -0.3, 0.0),
            Vec3::new(0.0, 0.25, 0.0),
        ));

        // Right Upper Leg to Right Lower Leg (knee)
        ragdoll.add_joint(RagdollJoint::revolute(
            8,
            9,
            Vec3::new(0.0, -0.25, 0.0),
            Vec3::new(0.0, 0.25, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            -150.0,
            0.0,
        ));

        ragdoll
    }
}

impl Default for RagdollBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Ragdoll manager for handling multiple ragdolls
pub struct RagdollManager {
    ragdolls: HashMap<u64, Ragdoll>,
}

impl RagdollManager {
    pub fn new() -> Self {
        Self {
            ragdolls: HashMap::new(),
        }
    }

    /// Add a ragdoll to the manager
    pub fn add_ragdoll(&mut self, ragdoll: Ragdoll) {
        self.ragdolls.insert(ragdoll.entity_id, ragdoll);
    }

    /// Get a ragdoll by entity ID
    pub fn get_ragdoll(&self, entity_id: u64) -> Option<&Ragdoll> {
        self.ragdolls.get(&entity_id)
    }

    /// Get a mutable ragdoll by entity ID
    pub fn get_ragdoll_mut(&mut self, entity_id: u64) -> Option<&mut Ragdoll> {
        self.ragdolls.get_mut(&entity_id)
    }

    /// Remove a ragdoll
    pub fn remove_ragdoll(&mut self, entity_id: u64) -> Option<Ragdoll> {
        self.ragdolls.remove(&entity_id)
    }

    /// Activate a ragdoll
    pub fn activate_ragdoll(&mut self, entity_id: u64, physics: &mut PhysicsWorld3D) {
        if let Some(ragdoll) = self.ragdolls.get_mut(&entity_id) {
            ragdoll.activate(physics);
        }
    }

    /// Deactivate a ragdoll
    pub fn deactivate_ragdoll(&mut self, entity_id: u64, physics: &mut PhysicsWorld3D) {
        if let Some(ragdoll) = self.ragdolls.get_mut(&entity_id) {
            ragdoll.deactivate(physics);
        }
    }

    /// Get the number of ragdolls
    pub fn ragdoll_count(&self) -> usize {
        self.ragdolls.len()
    }

    /// Get the number of active ragdolls
    pub fn active_ragdoll_count(&self) -> usize {
        self.ragdolls.values().filter(|r| r.is_active).count()
    }
}

impl Default for RagdollManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ragdoll_config_creation() {
        let config = RagdollConfig::new();
        assert_eq!(config.total_mass, 70.0);
        assert_eq!(config.density, 1000.0);
        assert_eq!(config.angular_damping, 0.5);
        assert_eq!(config.linear_damping, 0.1);
        assert!(config.use_ccd);
    }

    #[test]
    fn test_ragdoll_config_builder() {
        let config = RagdollConfig::new()
            .total_mass(80.0)
            .angular_damping(0.7)
            .linear_damping(0.2)
            .use_ccd(false);

        assert_eq!(config.total_mass, 80.0);
        assert_eq!(config.angular_damping, 0.7);
        assert_eq!(config.linear_damping, 0.2);
        assert!(!config.use_ccd);
    }

    #[test]
    fn test_bone_shape_capsule() {
        let shape = BoneShape::capsule(0.1, 0.5);
        match shape {
            BoneShape::Capsule {
                radius,
                half_height,
            } => {
                assert_eq!(radius, 0.1);
                assert_eq!(half_height, 0.5);
            }
            _ => panic!("Expected capsule shape"),
        }
    }

    #[test]
    fn test_bone_shape_volume_capsule() {
        let shape = BoneShape::capsule(0.1, 0.5);
        let volume = shape.volume();
        assert!(volume > 0.0);
    }

    #[test]
    fn test_bone_shape_volume_sphere() {
        let shape = BoneShape::sphere(0.1);
        let volume = shape.volume();
        let expected = (4.0 / 3.0) * std::f32::consts::PI * 0.1_f32.powi(3);
        assert!((volume - expected).abs() < 0.001);
    }

    #[test]
    fn test_bone_shape_volume_box() {
        let shape = BoneShape::box_shape(Vec3::new(0.5, 0.5, 0.5));
        let volume = shape.volume();
        assert_eq!(volume, 1.0); // 1x1x1 box
    }

    #[test]
    fn test_ragdoll_joint_spherical() {
        let joint = RagdollJoint::spherical(
            0,
            1,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        );

        assert_eq!(joint.parent_bone, 0);
        assert_eq!(joint.child_bone, 1);
        assert_eq!(joint.joint_type, JointType::Spherical);
    }

    #[test]
    fn test_ragdoll_joint_revolute() {
        let joint = RagdollJoint::revolute(
            0,
            1,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            -90.0,
            90.0,
        );

        assert_eq!(joint.parent_bone, 0);
        assert_eq!(joint.child_bone, 1);
        assert_eq!(joint.joint_type, JointType::Revolute);
        assert_eq!(joint.min_angle, -90.0);
        assert_eq!(joint.max_angle, 90.0);
    }

    #[test]
    fn test_ragdoll_bone_creation() {
        let bone = RagdollBone::new(
            "test_bone".to_string(),
            0,
            BoneShape::capsule(0.1, 0.5),
            5.0,
        );

        assert_eq!(bone.name, "test_bone");
        assert_eq!(bone.bone_index, 0);
        assert_eq!(bone.mass, 5.0);
        assert!(bone.body_handle.is_none());
        assert!(bone.collider_handle.is_none());
    }

    #[test]
    fn test_ragdoll_creation() {
        let config = RagdollConfig::default();
        let ragdoll = Ragdoll::new(1, config);

        assert_eq!(ragdoll.entity_id, 1);
        assert_eq!(ragdoll.bones.len(), 0);
        assert_eq!(ragdoll.joints.len(), 0);
        assert!(!ragdoll.is_active);
    }

    #[test]
    fn test_ragdoll_add_bone() {
        let config = RagdollConfig::default();
        let mut ragdoll = Ragdoll::new(1, config);

        let bone = RagdollBone::new(
            "test_bone".to_string(),
            0,
            BoneShape::capsule(0.1, 0.5),
            5.0,
        );

        ragdoll.add_bone(bone);
        assert_eq!(ragdoll.bones.len(), 1);
        assert_eq!(ragdoll.bones[0].name, "test_bone");
    }

    #[test]
    fn test_ragdoll_add_joint() {
        let config = RagdollConfig::default();
        let mut ragdoll = Ragdoll::new(1, config);

        let joint = RagdollJoint::spherical(
            0,
            1,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        );

        ragdoll.add_joint(joint);
        assert_eq!(ragdoll.joints.len(), 1);
    }

    #[test]
    fn test_ragdoll_builder() {
        let ragdoll = RagdollBuilder::new()
            .add_bone(RagdollBone::new(
                "bone1".to_string(),
                0,
                BoneShape::capsule(0.1, 0.5),
                5.0,
            ))
            .add_bone(RagdollBone::new(
                "bone2".to_string(),
                1,
                BoneShape::sphere(0.1),
                3.0,
            ))
            .add_joint(RagdollJoint::spherical(
                0,
                1,
                Vec3::new(0.0, 0.5, 0.0),
                Vec3::new(0.0, -0.1, 0.0),
            ))
            .build(1);

        assert_eq!(ragdoll.entity_id, 1);
        assert_eq!(ragdoll.bones.len(), 2);
        assert_eq!(ragdoll.joints.len(), 1);
    }

    #[test]
    fn test_ragdoll_humanoid() {
        let ragdoll = RagdollBuilder::humanoid(1);

        assert_eq!(ragdoll.entity_id, 1);
        assert_eq!(ragdoll.bone_count(), 10); // Spine, head, 2 arms (2 bones each), 2 legs (2 bones each)
        assert_eq!(ragdoll.joint_count(), 9); // Connections between bones
    }

    #[test]
    fn test_ragdoll_manager_creation() {
        let manager = RagdollManager::new();
        assert_eq!(manager.ragdoll_count(), 0);
        assert_eq!(manager.active_ragdoll_count(), 0);
    }

    #[test]
    fn test_ragdoll_manager_add() {
        let mut manager = RagdollManager::new();
        let ragdoll = Ragdoll::new(1, RagdollConfig::default());

        manager.add_ragdoll(ragdoll);
        assert_eq!(manager.ragdoll_count(), 1);
    }

    #[test]
    fn test_ragdoll_manager_get() {
        let mut manager = RagdollManager::new();
        let ragdoll = Ragdoll::new(1, RagdollConfig::default());

        manager.add_ragdoll(ragdoll);

        let retrieved = manager.get_ragdoll(1);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().entity_id, 1);

        let missing = manager.get_ragdoll(999);
        assert!(missing.is_none());
    }

    #[test]
    fn test_ragdoll_manager_remove() {
        let mut manager = RagdollManager::new();
        let ragdoll = Ragdoll::new(1, RagdollConfig::default());

        manager.add_ragdoll(ragdoll);
        assert_eq!(manager.ragdoll_count(), 1);

        let removed = manager.remove_ragdoll(1);
        assert!(removed.is_some());
        assert_eq!(manager.ragdoll_count(), 0);
    }

    #[test]
    fn test_ragdoll_bone_count() {
        let mut ragdoll = Ragdoll::new(1, RagdollConfig::default());
        assert_eq!(ragdoll.bone_count(), 0);

        ragdoll.add_bone(RagdollBone::new(
            "bone1".to_string(),
            0,
            BoneShape::capsule(0.1, 0.5),
            5.0,
        ));
        assert_eq!(ragdoll.bone_count(), 1);

        ragdoll.add_bone(RagdollBone::new(
            "bone2".to_string(),
            1,
            BoneShape::sphere(0.1),
            3.0,
        ));
        assert_eq!(ragdoll.bone_count(), 2);
    }

    #[test]
    fn test_ragdoll_joint_count() {
        let mut ragdoll = Ragdoll::new(1, RagdollConfig::default());
        assert_eq!(ragdoll.joint_count(), 0);

        ragdoll.add_joint(RagdollJoint::spherical(
            0,
            1,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        ));
        assert_eq!(ragdoll.joint_count(), 1);
    }
}

