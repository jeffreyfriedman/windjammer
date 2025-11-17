//! Advanced Animation Blending System
//!
//! Provides smooth animation transitions, crossfades, and layered blending.

use crate::animation::{Animation, AnimationPlayer, Transform};
use crate::math::{Quat, Vec3};
use std::collections::HashMap;

/// Animation blend mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Override (replace previous animation)
    Override,
    
    /// Additive (add to previous animation)
    Additive,
    
    /// Layer (blend on top with mask)
    Layer,
}

/// Animation layer with masking
#[derive(Debug, Clone)]
pub struct AnimationLayer {
    /// Layer name
    pub name: String,
    
    /// Animation player for this layer
    pub player: AnimationPlayer,
    
    /// Blend weight (0.0 to 1.0)
    pub weight: f32,
    
    /// Blend mode
    pub blend_mode: BlendMode,
    
    /// Bone mask (None = all bones, Some = specific bones)
    pub bone_mask: Option<Vec<usize>>,
    
    /// Whether this layer is active
    pub active: bool,
}

impl AnimationLayer {
    /// Create a new animation layer
    pub fn new(name: String, blend_mode: BlendMode) -> Self {
        Self {
            name,
            player: AnimationPlayer::new(),
            weight: 1.0,
            blend_mode,
            bone_mask: None,
            active: true,
        }
    }
    
    /// Set bone mask (which bones this layer affects)
    pub fn set_bone_mask(&mut self, bones: Vec<usize>) {
        self.bone_mask = Some(bones);
    }
    
    /// Clear bone mask (affect all bones)
    pub fn clear_bone_mask(&mut self) {
        self.bone_mask = None;
    }
}

/// Crossfade transition between animations
#[derive(Debug, Clone)]
pub struct Crossfade {
    /// Source animation (fading out)
    pub from_animation: Animation,
    
    /// Source animation time
    pub from_time: f32,
    
    /// Target animation (fading in)
    pub to_animation: Animation,
    
    /// Target animation time
    pub to_time: f32,
    
    /// Crossfade duration
    pub duration: f32,
    
    /// Current crossfade time
    pub current_time: f32,
    
    /// Whether the crossfade is complete
    pub complete: bool,
}

impl Crossfade {
    /// Create a new crossfade
    pub fn new(
        from_animation: Animation,
        from_time: f32,
        to_animation: Animation,
        duration: f32,
    ) -> Self {
        Self {
            from_animation,
            from_time,
            to_animation,
            to_time: 0.0,
            duration,
            current_time: 0.0,
            complete: false,
        }
    }
    
    /// Update crossfade
    pub fn update(&mut self, delta: f32) {
        if self.complete {
            return;
        }
        
        self.current_time += delta;
        self.to_time += delta;
        
        if self.current_time >= self.duration {
            self.current_time = self.duration;
            self.complete = true;
        }
    }
    
    /// Get blend weight (0.0 = from, 1.0 = to)
    pub fn get_blend_weight(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        
        (self.current_time / self.duration).clamp(0.0, 1.0)
    }
    
    /// Sample the crossfade at current time
    pub fn sample(&self) -> Vec<Transform> {
        let from_pose = self.from_animation.sample(self.from_time);
        let to_pose = self.to_animation.sample(self.to_time);
        
        let blend_weight = self.get_blend_weight();
        
        blend_transforms(&from_pose, &to_pose, blend_weight)
    }
}

/// Advanced animation blending system
pub struct AnimationBlendingSystem {
    /// Animation layers (ordered by priority)
    layers: Vec<AnimationLayer>,
    
    /// Active crossfade (if any)
    crossfade: Option<Crossfade>,
    
    /// Blend tree nodes (for complex blending)
    blend_nodes: HashMap<String, BlendNode>,
}

impl AnimationBlendingSystem {
    /// Create a new blending system
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            crossfade: None,
            blend_nodes: HashMap::new(),
        }
    }
    
    /// Add an animation layer
    pub fn add_layer(&mut self, layer: AnimationLayer) {
        self.layers.push(layer);
    }
    
    /// Get layer by name
    pub fn get_layer_mut(&mut self, name: &str) -> Option<&mut AnimationLayer> {
        self.layers.iter_mut().find(|l| l.name == name)
    }
    
    /// Remove layer by name
    pub fn remove_layer(&mut self, name: &str) {
        self.layers.retain(|l| l.name != name);
    }
    
    /// Start a crossfade to a new animation
    pub fn crossfade_to(
        &mut self,
        animation: Animation,
        duration: f32,
        layer_name: Option<&str>,
    ) {
        // Get current animation and time from specified layer (or first layer)
        let (current_animation, current_time) = if let Some(name) = layer_name {
            if let Some(layer) = self.get_layer_mut(name) {
                if let Some(anim) = layer.player.current_animation.clone() {
                    (anim, layer.player.current_time)
                } else {
                    // No current animation, just start the new one
                    layer.player.play(animation);
                    return;
                }
            } else {
                return;
            }
        } else if let Some(layer) = self.layers.first_mut() {
            if let Some(anim) = layer.player.current_animation.clone() {
                (anim, layer.player.current_time)
            } else {
                layer.player.play(animation);
                return;
            }
        } else {
            return;
        };
        
        // Create crossfade
        self.crossfade = Some(Crossfade::new(
            current_animation,
            current_time,
            animation,
            duration,
        ));
    }
    
    /// Update all layers and crossfades
    pub fn update(&mut self, delta: f32) {
        // Update crossfade
        if let Some(crossfade) = &mut self.crossfade {
            crossfade.update(delta);
            
            // If crossfade is complete, apply the target animation
            if crossfade.complete {
                if let Some(layer) = self.layers.first_mut() {
                    layer.player.play(crossfade.to_animation.clone());
                    layer.player.current_time = crossfade.to_time;
                }
                self.crossfade = None;
            }
        }
        
        // Update all active layers
        for layer in &mut self.layers {
            if layer.active {
                layer.player.update(delta);
            }
        }
    }
    
    /// Get the final blended pose
    pub fn get_blended_pose(&self) -> Vec<Transform> {
        // If crossfade is active, return crossfade result
        if let Some(crossfade) = &self.crossfade {
            return crossfade.sample();
        }
        
        // Otherwise, blend all active layers
        let active_layers: Vec<&AnimationLayer> = self
            .layers
            .iter()
            .filter(|l| l.active && l.player.current_animation.is_some())
            .collect();
        
        if active_layers.is_empty() {
            return Vec::new();
        }
        
        // Start with the first layer
        let mut result = active_layers[0].player.get_current_pose();
        
        // Blend additional layers
        for layer in active_layers.iter().skip(1) {
            let layer_pose = layer.player.get_current_pose();
            
            match layer.blend_mode {
                BlendMode::Override => {
                    // Replace with layer pose (weighted)
                    result = blend_transforms(&result, &layer_pose, layer.weight);
                }
                BlendMode::Additive => {
                    // Add layer pose (weighted)
                    result = add_transforms(&result, &layer_pose, layer.weight);
                }
                BlendMode::Layer => {
                    // Blend with bone mask
                    if let Some(mask) = &layer.bone_mask {
                        result = blend_transforms_masked(&result, &layer_pose, layer.weight, mask);
                    } else {
                        result = blend_transforms(&result, &layer_pose, layer.weight);
                    }
                }
            }
        }
        
        result
    }
}

/// Blend node for blend trees
#[derive(Debug, Clone)]
pub enum BlendNode {
    /// Single animation clip
    Clip {
        animation: Animation,
        time: f32,
    },
    
    /// Linear blend between two nodes
    Blend {
        left: Box<BlendNode>,
        right: Box<BlendNode>,
        weight: f32, // 0.0 = left, 1.0 = right
    },
    
    /// 2D blend (blend space)
    Blend2D {
        nodes: Vec<(Vec3, BlendNode)>, // (position, node)
        position: Vec3,
    },
}

impl BlendNode {
    /// Evaluate the blend node
    pub fn evaluate(&self) -> Vec<Transform> {
        match self {
            BlendNode::Clip { animation, time } => animation.sample(*time),
            
            BlendNode::Blend { left, right, weight } => {
                let left_pose = left.evaluate();
                let right_pose = right.evaluate();
                blend_transforms(&left_pose, &right_pose, *weight)
            }
            
            BlendNode::Blend2D { nodes, position } => {
                // Find closest nodes and blend
                // Simplified: just use first node for now
                if let Some((_, node)) = nodes.first() {
                    node.evaluate()
                } else {
                    Vec::new()
                }
            }
        }
    }
}

// Helper functions

/// Blend two transform arrays
fn blend_transforms(from: &[Transform], to: &[Transform], weight: f32) -> Vec<Transform> {
    from.iter()
        .zip(to.iter())
        .map(|(a, b)| Transform {
            position: a.position.lerp(b.position, weight),
            rotation: a.rotation.slerp(b.rotation, weight),
            scale: a.scale.lerp(b.scale, weight),
        })
        .collect()
}

/// Add two transform arrays (for additive blending)
fn add_transforms(base: &[Transform], additive: &[Transform], weight: f32) -> Vec<Transform> {
    base.iter()
        .zip(additive.iter())
        .map(|(a, b)| Transform {
            position: a.position + b.position * weight,
            // For additive rotation, slerp from identity towards the additive rotation
            rotation: a.rotation * Quat::IDENTITY.slerp(b.rotation, weight),
            scale: a.scale + b.scale * weight,
        })
        .collect()
}

/// Blend transforms with bone mask
fn blend_transforms_masked(
    from: &[Transform],
    to: &[Transform],
    weight: f32,
    mask: &[usize],
) -> Vec<Transform> {
    from.iter()
        .enumerate()
        .map(|(i, a)| {
            if mask.contains(&i) {
                // Bone is in mask, blend
                let b = &to[i];
                Transform {
                    position: a.position.lerp(b.position, weight),
                    rotation: a.rotation.slerp(b.rotation, weight),
                    scale: a.scale.lerp(b.scale, weight),
                }
            } else {
                // Bone not in mask, keep original
                a.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_crossfade_creation() {
        let anim1 = Animation {
            name: "walk".to_string(),
            duration: 1.0,
            looping: true,
            keyframes: Vec::new(),
        };
        
        let anim2 = Animation {
            name: "run".to_string(),
            duration: 0.8,
            looping: true,
            keyframes: Vec::new(),
        };
        
        let crossfade = Crossfade::new(anim1, 0.5, anim2, 0.3);
        
        assert_eq!(crossfade.duration, 0.3);
        assert_eq!(crossfade.current_time, 0.0);
        assert!(!crossfade.complete);
    }
    
    #[test]
    fn test_crossfade_update() {
        let anim1 = Animation {
            name: "walk".to_string(),
            duration: 1.0,
            looping: true,
            keyframes: Vec::new(),
        };
        
        let anim2 = Animation {
            name: "run".to_string(),
            duration: 0.8,
            looping: true,
            keyframes: Vec::new(),
        };
        
        let mut crossfade = Crossfade::new(anim1, 0.5, anim2, 0.3);
        
        // Update halfway
        crossfade.update(0.15);
        assert_eq!(crossfade.get_blend_weight(), 0.5);
        assert!(!crossfade.complete);
        
        // Update to completion
        crossfade.update(0.15);
        assert_eq!(crossfade.get_blend_weight(), 1.0);
        assert!(crossfade.complete);
    }
    
    #[test]
    fn test_animation_layer() {
        let layer = AnimationLayer::new("base".to_string(), BlendMode::Override);
        
        assert_eq!(layer.name, "base");
        assert_eq!(layer.weight, 1.0);
        assert_eq!(layer.blend_mode, BlendMode::Override);
        assert!(layer.active);
        assert!(layer.bone_mask.is_none());
    }
    
    #[test]
    fn test_blending_system() {
        let mut system = AnimationBlendingSystem::new();
        
        let layer = AnimationLayer::new("base".to_string(), BlendMode::Override);
        system.add_layer(layer);
        
        assert_eq!(system.layers.len(), 1);
        
        // Get layer
        assert!(system.get_layer_mut("base").is_some());
        assert!(system.get_layer_mut("nonexistent").is_none());
    }
}

