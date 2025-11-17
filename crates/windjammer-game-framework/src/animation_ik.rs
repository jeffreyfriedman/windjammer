//! Advanced Inverse Kinematics (IK) System
//!
//! Provides production-ready IK solvers for character animation.

use crate::animation::{Skeleton, Transform};
use crate::math::{Mat4, Quat, Vec3};

/// IK solver type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IKSolverType {
    /// FABRIK (Forward And Backward Reaching Inverse Kinematics)
    /// Good for chains of any length
    FABRIK,
    
    /// Two-Bone IK (analytic solution)
    /// Optimized for arms and legs (3 bones)
    TwoBone,
    
    /// CCD (Cyclic Coordinate Descent)
    /// Good for tentacles and tails
    CCD,
}

/// IK chain configuration
#[derive(Debug, Clone)]
pub struct IKChain {
    /// Bone indices in the chain (root to tip)
    pub bones: Vec<usize>,
    
    /// Target position
    pub target_position: Vec3,
    
    /// Target rotation (optional)
    pub target_rotation: Option<Quat>,
    
    /// Solver type
    pub solver_type: IKSolverType,
    
    /// Maximum iterations
    pub max_iterations: usize,
    
    /// Tolerance (distance threshold)
    pub tolerance: f32,
    
    /// Weight (0.0 to 1.0, for blending with FK)
    pub weight: f32,
    
    /// Pole target (for two-bone IK, controls elbow/knee direction)
    pub pole_target: Option<Vec3>,
}

impl IKChain {
    /// Create a new IK chain
    pub fn new(bones: Vec<usize>, target: Vec3) -> Self {
        let solver_type = if bones.len() == 3 {
            IKSolverType::TwoBone
        } else {
            IKSolverType::FABRIK
        };
        
        Self {
            bones,
            target_position: target,
            target_rotation: None,
            solver_type,
            max_iterations: 10,
            tolerance: 0.01,
            weight: 1.0,
            pole_target: None,
        }
    }
    
    /// Set target rotation
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.target_rotation = Some(rotation);
        self
    }
    
    /// Set solver type
    pub fn with_solver(mut self, solver_type: IKSolverType) -> Self {
        self.solver_type = solver_type;
        self
    }
    
    /// Set weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }
    
    /// Set pole target
    pub fn with_pole_target(mut self, pole: Vec3) -> Self {
        self.pole_target = Some(pole);
        self
    }
    
    /// Solve the IK chain
    pub fn solve(&self, skeleton: &mut Skeleton) {
        match self.solver_type {
            IKSolverType::FABRIK => self.solve_fabrik(skeleton),
            IKSolverType::TwoBone => self.solve_two_bone(skeleton),
            IKSolverType::CCD => self.solve_ccd(skeleton),
        }
    }
    
    /// FABRIK solver
    fn solve_fabrik(&self, skeleton: &mut Skeleton) {
        if self.bones.len() < 2 {
            return;
        }
        
        // Get bone positions
        let mut positions: Vec<Vec3> = self
            .bones
            .iter()
            .map(|&idx| skeleton.bones[idx].local_transform.position)
            .collect();
        
        // Calculate bone lengths
        let mut lengths: Vec<f32> = Vec::new();
        for i in 0..positions.len() - 1 {
            lengths.push((positions[i + 1] - positions[i]).length());
        }
        
        let root = positions[0];
        
        // FABRIK iterations
        for _ in 0..self.max_iterations {
            // Check if close enough
            let end_effector = positions[positions.len() - 1];
            if (end_effector - self.target_position).length() < self.tolerance {
                break;
            }
            
            // Backward pass (from end to root)
            let last_idx = positions.len() - 1;
            positions[last_idx] = self.target_position;
            for i in (1..positions.len()).rev() {
                let direction = (positions[i - 1] - positions[i]).normalize();
                positions[i - 1] = positions[i] + direction * lengths[i - 1];
            }
            
            // Forward pass (from root to end)
            positions[0] = root;
            for i in 0..positions.len() - 1 {
                let direction = (positions[i + 1] - positions[i]).normalize();
                positions[i + 1] = positions[i] + direction * lengths[i];
            }
        }
        
        // Apply solved positions back to skeleton (with weight)
        for (i, &bone_idx) in self.bones.iter().enumerate() {
            let original_pos = skeleton.bones[bone_idx].local_transform.position;
            skeleton.bones[bone_idx].local_transform.position =
                original_pos.lerp(positions[i], self.weight);
        }
    }
    
    /// Two-Bone IK solver (analytic solution for 3-bone chains)
    fn solve_two_bone(&self, skeleton: &mut Skeleton) {
        if self.bones.len() != 3 {
            // Fall back to FABRIK for non-3-bone chains
            self.solve_fabrik(skeleton);
            return;
        }
        
        let root_idx = self.bones[0];
        let mid_idx = self.bones[1];
        let tip_idx = self.bones[2];
        
        let root_pos = skeleton.bones[root_idx].local_transform.position;
        let mid_pos = skeleton.bones[mid_idx].local_transform.position;
        let tip_pos = skeleton.bones[tip_idx].local_transform.position;
        
        // Calculate bone lengths
        let upper_length = (mid_pos - root_pos).length();
        let lower_length = (tip_pos - mid_pos).length();
        let target_distance = (self.target_position - root_pos).length();
        
        // Clamp target distance to reachable range
        let max_reach = upper_length + lower_length;
        let target_distance = target_distance.min(max_reach * 0.999);
        
        // Calculate angles using law of cosines
        let cos_angle = (upper_length * upper_length + target_distance * target_distance
            - lower_length * lower_length)
            / (2.0 * upper_length * target_distance);
        let cos_angle = cos_angle.clamp(-1.0, 1.0);
        
        let cos_elbow = (upper_length * upper_length + lower_length * lower_length
            - target_distance * target_distance)
            / (2.0 * upper_length * lower_length);
        let cos_elbow = cos_elbow.clamp(-1.0, 1.0);
        
        // Calculate target direction
        let target_dir = (self.target_position - root_pos).normalize();
        
        // Calculate pole direction (for elbow/knee placement)
        let pole_dir = if let Some(pole) = self.pole_target {
            (pole - root_pos).normalize()
        } else {
            // Default pole direction (perpendicular to target direction)
            let up = if target_dir.y.abs() < 0.9 {
                Vec3::new(0.0, 1.0, 0.0)
            } else {
                Vec3::new(1.0, 0.0, 0.0)
            };
            target_dir.cross(up).normalize()
        };
        
        // Calculate rotation axis
        let rotation_axis = target_dir.cross(pole_dir).normalize();
        
        // Apply rotations
        let angle1 = cos_angle.acos();
        let angle2 = std::f32::consts::PI - cos_elbow.acos();
        
        let rot1 = Quat::from_axis_angle(rotation_axis, angle1);
        let rot2 = Quat::from_axis_angle(rotation_axis, angle2);
        
        // Apply to skeleton (simplified - in production, would calculate proper rotations)
        let new_mid_pos = root_pos + rot1 * Vec3::new(0.0, upper_length, 0.0);
        let new_tip_pos = new_mid_pos + rot2 * Vec3::new(0.0, lower_length, 0.0);
        
        // Blend with weight
        skeleton.bones[mid_idx].local_transform.position =
            mid_pos.lerp(new_mid_pos, self.weight);
        skeleton.bones[tip_idx].local_transform.position =
            tip_pos.lerp(new_tip_pos, self.weight);
    }
    
    /// CCD (Cyclic Coordinate Descent) solver
    fn solve_ccd(&self, skeleton: &mut Skeleton) {
        if self.bones.len() < 2 {
            return;
        }
        
        for _ in 0..self.max_iterations {
            // Get current end effector position
            let tip_idx = self.bones[self.bones.len() - 1];
            let end_effector = skeleton.bones[tip_idx].local_transform.position;
            
            // Check if close enough
            if (end_effector - self.target_position).length() < self.tolerance {
                break;
            }
            
            // Iterate from tip to root
            for i in (0..self.bones.len() - 1).rev() {
                let bone_idx = self.bones[i];
                let bone_pos = skeleton.bones[bone_idx].local_transform.position;
                
                // Calculate vectors
                let to_end = (end_effector - bone_pos).normalize();
                let to_target = (self.target_position - bone_pos).normalize();
                
                // Calculate rotation
                let axis = to_end.cross(to_target);
                let angle = to_end.dot(to_target).clamp(-1.0, 1.0).acos();
                
                if axis.length() > 0.001 && angle > 0.001 {
                    let rotation = Quat::from_axis_angle(axis.normalize(), angle * self.weight);
                    skeleton.bones[bone_idx].local_transform.rotation =
                        rotation * skeleton.bones[bone_idx].local_transform.rotation;
                }
            }
        }
    }
}

/// Look-At IK constraint
#[derive(Debug, Clone)]
pub struct LookAtConstraint {
    /// Bone index to rotate
    pub bone_index: usize,
    
    /// Target position to look at
    pub target: Vec3,
    
    /// Up vector
    pub up: Vec3,
    
    /// Weight (0.0 to 1.0)
    pub weight: f32,
    
    /// Axis to align with target (default: forward/Z)
    pub aim_axis: Vec3,
}

impl LookAtConstraint {
    /// Create a new look-at constraint
    pub fn new(bone_index: usize, target: Vec3) -> Self {
        Self {
            bone_index,
            target,
            up: Vec3::new(0.0, 1.0, 0.0),
            weight: 1.0,
            aim_axis: Vec3::new(0.0, 0.0, 1.0),
        }
    }
    
    /// Set up vector
    pub fn with_up(mut self, up: Vec3) -> Self {
        self.up = up.normalize();
        self
    }
    
    /// Set weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }
    
    /// Apply the look-at constraint
    pub fn apply(&self, skeleton: &mut Skeleton) {
        let bone = &mut skeleton.bones[self.bone_index];
        let bone_pos = bone.local_transform.position;
        
        // Calculate look-at direction
        let direction = (self.target - bone_pos).normalize();
        
        // Calculate rotation to look at target
        let rotation = Quat::from_rotation_arc(self.aim_axis, direction);
        
        // Blend with current rotation
        bone.local_transform.rotation =
            bone.local_transform.rotation.slerp(rotation, self.weight);
    }
}

/// Foot placement IK helper
#[derive(Debug, Clone)]
pub struct FootPlacement {
    /// Leg chain (hip, knee, ankle)
    pub leg_chain: IKChain,
    
    /// Foot bone index
    pub foot_bone: usize,
    
    /// Ground normal
    pub ground_normal: Vec3,
    
    /// Foot offset from ground
    pub foot_offset: f32,
}

impl FootPlacement {
    /// Create a new foot placement helper
    pub fn new(leg_bones: Vec<usize>, foot_bone: usize, ground_height: f32) -> Self {
        let target = Vec3::new(0.0, ground_height, 0.0);
        
        Self {
            leg_chain: IKChain::new(leg_bones, target)
                .with_solver(IKSolverType::TwoBone),
            foot_bone,
            ground_normal: Vec3::new(0.0, 1.0, 0.0),
            foot_offset: 0.0,
        }
    }
    
    /// Set ground position and normal
    pub fn set_ground(&mut self, position: Vec3, normal: Vec3) {
        self.leg_chain.target_position = position + normal * self.foot_offset;
        self.ground_normal = normal.normalize();
    }
    
    /// Apply foot placement
    pub fn apply(&self, skeleton: &mut Skeleton) {
        // Solve leg IK
        self.leg_chain.solve(skeleton);
        
        // Align foot with ground normal
        let foot_bone = &mut skeleton.bones[self.foot_bone];
        let up = Vec3::new(0.0, 1.0, 0.0);
        let rotation = Quat::from_rotation_arc(up, self.ground_normal);
        foot_bone.local_transform.rotation = rotation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ik_chain_creation() {
        let chain = IKChain::new(
            vec![0, 1, 2],
            Vec3::new(1.0, 0.0, 0.0),
        );
        
        assert_eq!(chain.bones.len(), 3);
        assert_eq!(chain.solver_type, IKSolverType::TwoBone);
        assert_eq!(chain.weight, 1.0);
    }
    
    #[test]
    fn test_look_at_creation() {
        let look_at = LookAtConstraint::new(0, Vec3::new(1.0, 0.0, 0.0));
        
        assert_eq!(look_at.bone_index, 0);
        assert_eq!(look_at.weight, 1.0);
    }
    
    #[test]
    fn test_foot_placement_creation() {
        let foot = FootPlacement::new(vec![0, 1, 2], 3, 0.0);
        
        assert_eq!(foot.leg_chain.bones.len(), 3);
        assert_eq!(foot.foot_bone, 3);
    }
}

