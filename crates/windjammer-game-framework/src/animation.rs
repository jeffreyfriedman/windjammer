//! Animation System for Windjammer
//!
//! Provides skeletal animation, blending, and inverse kinematics (IK).
//!
//! **Philosophy**: Zero crate leakage - no glam or other Rust types exposed.

use crate::math::{Mat4, Quat, Vec3};
use std::collections::HashMap;

/// A single bone in a skeleton
#[derive(Clone, Debug)]
pub struct Bone {
    /// Bone name
    pub name: String,

    /// Parent bone index (None for root)
    pub parent: Option<usize>,

    /// Local transform (relative to parent)
    pub local_transform: Transform,

    /// Inverse bind pose matrix
    pub inverse_bind_pose: Mat4,
}

/// Transform component (position, rotation, scale)
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn identity() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// A skeleton (bone hierarchy)
#[derive(Clone, Debug)]
pub struct Skeleton {
    /// All bones in the skeleton
    pub bones: Vec<Bone>,

    /// Bone name to index mapping
    bone_indices: HashMap<String, usize>,
}

impl Skeleton {
    /// Create a new skeleton
    pub fn new() -> Self {
        Self {
            bones: Vec::new(),
            bone_indices: HashMap::new(),
        }
    }

    /// Add a bone to the skeleton
    pub fn add_bone(
        &mut self,
        name: String,
        parent: Option<usize>,
        local_transform: Transform,
        inverse_bind_pose: Mat4,
    ) -> usize {
        let index = self.bones.len();
        self.bone_indices.insert(name.clone(), index);
        self.bones.push(Bone {
            name,
            parent,
            local_transform,
            inverse_bind_pose,
        });
        index
    }

    /// Get bone index by name
    pub fn get_bone_index(&self, name: &str) -> Option<usize> {
        self.bone_indices.get(name).copied()
    }

    /// Calculate world-space bone matrices
    pub fn calculate_bone_matrices(&self) -> Vec<Mat4> {
        let mut matrices = vec![Mat4::IDENTITY; self.bones.len()];

        // Calculate world transforms
        for i in 0..self.bones.len() {
            let bone = &self.bones[i];
            let local_matrix = bone.local_transform.to_matrix();

            if let Some(parent_idx) = bone.parent {
                matrices[i] = matrices[parent_idx] * local_matrix;
            } else {
                matrices[i] = local_matrix;
            }
        }

        // Apply inverse bind pose
        for i in 0..self.bones.len() {
            matrices[i] = matrices[i] * self.bones[i].inverse_bind_pose;
        }

        matrices
    }
}

/// A keyframe in an animation
#[derive(Clone, Debug)]
pub struct Keyframe {
    /// Time in seconds
    pub time: f32,

    /// Bone transforms at this keyframe
    pub transforms: Vec<Transform>,
}

/// An animation clip
#[derive(Clone, Debug)]
pub struct Animation {
    /// Animation name
    pub name: String,

    /// Duration in seconds
    pub duration: f32,

    /// Keyframes
    pub keyframes: Vec<Keyframe>,

    /// Whether to loop
    pub looping: bool,
}

impl Animation {
    /// Create a new animation
    pub fn new(name: String, duration: f32, looping: bool) -> Self {
        Self {
            name,
            duration,
            keyframes: Vec::new(),
            looping,
        }
    }

    /// Add a keyframe
    pub fn add_keyframe(&mut self, time: f32, transforms: Vec<Transform>) {
        self.keyframes.push(Keyframe { time, transforms });
        // Keep keyframes sorted by time
        self.keyframes
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Sample the animation at a given time
    pub fn sample(&self, time: f32) -> Vec<Transform> {
        if self.keyframes.is_empty() {
            return Vec::new();
        }

        // Handle looping
        let t = if self.looping {
            time % self.duration
        } else {
            time.min(self.duration)
        };

        // Find surrounding keyframes
        let mut prev_idx = 0;
        let mut next_idx = 0;

        for (i, keyframe) in self.keyframes.iter().enumerate() {
            if keyframe.time <= t {
                prev_idx = i;
            }
            if keyframe.time >= t {
                next_idx = i;
                break;
            }
        }

        // If at exact keyframe, return it
        if prev_idx == next_idx {
            return self.keyframes[prev_idx].transforms.clone();
        }

        // Interpolate between keyframes
        let prev_keyframe = &self.keyframes[prev_idx];
        let next_keyframe = &self.keyframes[next_idx];

        let time_diff = next_keyframe.time - prev_keyframe.time;
        let alpha = if time_diff > 0.0 {
            (t - prev_keyframe.time) / time_diff
        } else {
            0.0
        };

        // Interpolate transforms
        let mut result = Vec::new();
        for i in 0..prev_keyframe.transforms.len() {
            let prev_transform = &prev_keyframe.transforms[i];
            let next_transform = &next_keyframe.transforms[i];

            result.push(Transform {
                position: prev_transform.position.lerp(next_transform.position, alpha),
                rotation: prev_transform
                    .rotation
                    .slerp(next_transform.rotation, alpha),
                scale: prev_transform.scale.lerp(next_transform.scale, alpha),
            });
        }

        result
    }
}

/// Animation player
#[derive(Clone, Debug)]
pub struct AnimationPlayer {
    /// Current animation
    pub current_animation: Option<Animation>,

    /// Current time
    pub current_time: f32,

    /// Playback speed
    pub speed: f32,

    /// Whether playing
    pub playing: bool,
}

impl AnimationPlayer {
    /// Create a new animation player
    pub fn new() -> Self {
        Self {
            current_animation: None,
            current_time: 0.0,
            speed: 1.0,
            playing: false,
        }
    }

    /// Play an animation
    pub fn play(&mut self, animation: Animation) {
        self.current_animation = Some(animation);
        self.current_time = 0.0;
        self.playing = true;
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.playing = false;
        self.current_time = 0.0;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resume playback
    pub fn resume(&mut self) {
        self.playing = true;
    }

    /// Update animation
    pub fn update(&mut self, delta: f32) {
        if !self.playing {
            return;
        }

        if let Some(animation) = &self.current_animation {
            self.current_time += delta * self.speed;

            // Handle looping
            if animation.looping {
                self.current_time = self.current_time % animation.duration;
            } else if self.current_time >= animation.duration {
                self.current_time = animation.duration;
                self.playing = false;
            }
        }
    }

    /// Get current pose
    pub fn get_pose(&self) -> Vec<Transform> {
        if let Some(animation) = &self.current_animation {
            animation.sample(self.current_time)
        } else {
            Vec::new()
        }
    }
    
    /// Get current pose (alias for get_pose)
    pub fn get_current_pose(&self) -> Vec<Transform> {
        self.get_pose()
    }
}

/// Animation blending
pub struct AnimationBlender {
    /// Animations to blend
    animations: Vec<(Animation, f32)>, // (animation, weight)
}

impl AnimationBlender {
    /// Create a new blender
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
        }
    }

    /// Add an animation with weight
    pub fn add_animation(&mut self, animation: Animation, weight: f32) {
        self.animations.push((animation, weight));
    }

    /// Clear all animations
    pub fn clear(&mut self) {
        self.animations.clear();
    }

    /// Blend animations at a given time
    pub fn blend(&self, time: f32) -> Vec<Transform> {
        if self.animations.is_empty() {
            return Vec::new();
        }

        // Sample all animations
        let mut samples: Vec<Vec<Transform>> = Vec::new();
        let mut total_weight = 0.0;

        for (animation, weight) in &self.animations {
            samples.push(animation.sample(time));
            total_weight += weight;
        }

        if samples.is_empty() || total_weight == 0.0 {
            return Vec::new();
        }

        // Normalize weights
        let weights: Vec<f32> = self
            .animations
            .iter()
            .map(|(_, w)| w / total_weight)
            .collect();

        // Blend transforms
        let bone_count = samples[0].len();
        let mut result = Vec::new();

        for bone_idx in 0..bone_count {
            let mut position = Vec3::new(0.0, 0.0, 0.0);
            let mut rotation = Quat::IDENTITY;
            let mut scale = Vec3::new(0.0, 0.0, 0.0);

            for (sample_idx, sample) in samples.iter().enumerate() {
                let weight = weights[sample_idx];
                let transform = &sample[bone_idx];

                position = position + transform.position * weight;
                scale = scale + transform.scale * weight;

                // Quaternion blending (simplified - should use slerp for multiple)
                if sample_idx == 0 {
                    rotation = transform.rotation;
                } else {
                    rotation = rotation.slerp(transform.rotation, weight);
                }
            }

            result.push(Transform {
                position,
                rotation,
                scale,
            });
        }

        result
    }
}

/// Simple IK (Inverse Kinematics) solver
pub struct IKChain {
    /// Bone indices in the chain
    pub bones: Vec<usize>,

    /// Target position
    pub target: Vec3,

    /// Maximum iterations
    pub max_iterations: usize,

    /// Tolerance
    pub tolerance: f32,
}

impl IKChain {
    /// Create a new IK chain
    pub fn new(bones: Vec<usize>, target: Vec3) -> Self {
        Self {
            bones,
            target,
            max_iterations: 10,
            tolerance: 0.01,
        }
    }

    /// Solve IK using FABRIK (Forward And Backward Reaching Inverse Kinematics)
    pub fn solve(&self, skeleton: &mut Skeleton) {
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
            if (end_effector - self.target).length() < self.tolerance {
                break;
            }

            // Backward pass (from end to root)
            let last_idx = positions.len() - 1;
            positions[last_idx] = self.target;
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

        // Apply solved positions back to skeleton
        for (i, &bone_idx) in self.bones.iter().enumerate() {
            skeleton.bones[bone_idx].local_transform.position = positions[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_creation() {
        let mut skeleton = Skeleton::new();

        // Add root bone
        let root = skeleton.add_bone(
            "root".to_string(),
            None,
            Transform::identity(),
            Mat4::IDENTITY,
        );

        // Add child bone
        let _child = skeleton.add_bone(
            "child".to_string(),
            Some(root),
            Transform::new(
                Vec3::new(0.0, 1.0, 0.0),
                Quat::IDENTITY,
                Vec3::new(1.0, 1.0, 1.0),
            ),
            Mat4::IDENTITY,
        );

        assert_eq!(skeleton.bones.len(), 2);
        assert_eq!(skeleton.get_bone_index("root"), Some(0));
        assert_eq!(skeleton.get_bone_index("child"), Some(1));
    }

    #[test]
    fn test_animation_sampling() {
        let mut animation = Animation::new("test".to_string(), 2.0, false);

        // Add keyframes
        animation.add_keyframe(0.0, vec![Transform::identity()]);
        animation.add_keyframe(
            1.0,
            vec![Transform::new(
                Vec3::new(1.0, 0.0, 0.0),
                Quat::IDENTITY,
                Vec3::new(1.0, 1.0, 1.0),
            )],
        );
        animation.add_keyframe(
            2.0,
            vec![Transform::new(
                Vec3::new(2.0, 0.0, 0.0),
                Quat::IDENTITY,
                Vec3::new(1.0, 1.0, 1.0),
            )],
        );

        // Sample at 0.5s (should be halfway between first two keyframes)
        let pose = animation.sample(0.5);
        assert_eq!(pose.len(), 1);
        assert!((pose[0].position.x - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_animation_player() {
        let mut player = AnimationPlayer::new();
        let animation = Animation::new("test".to_string(), 1.0, true);

        player.play(animation);
        assert!(player.playing);

        player.update(0.5);
        assert!((player.current_time - 0.5).abs() < 0.01);

        player.pause();
        assert!(!player.playing);
    }
}
