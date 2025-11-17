//! Animation Controller
//!
//! High-level controller that integrates state machines, blending, and animation playback.
//! This is the main API for controlling character animations in AAA games.

use crate::animation::{Animation, AnimationPlayer, Transform};
use crate::animation_blending::{AnimationBlendingSystem, AnimationLayer, BlendMode, Crossfade};
use crate::animation_state_machine::{AnimationState, AnimationStateMachine, AnimationTransition};
use std::collections::HashMap;

/// Animation controller
///
/// Integrates state machines, blending, and playback for complete character animation control.
pub struct AnimationController {
    /// State machine for managing animation states
    pub state_machine: AnimationStateMachine,
    
    /// Blending system for smooth transitions
    pub blending_system: AnimationBlendingSystem,
    
    /// Animation library (name -> animation)
    animations: HashMap<String, Animation>,
    
    /// Current animation time
    current_time: f32,
    
    /// Playback speed
    speed: f32,
}

impl AnimationController {
    /// Create a new animation controller
    pub fn new(initial_state: &str) -> Self {
        let mut blending_system = AnimationBlendingSystem::new();
        
        // Add a base layer for the state machine animations
        blending_system.add_layer(AnimationLayer::new(
            "base".to_string(),
            BlendMode::Override,
        ));
        
        Self {
            state_machine: AnimationStateMachine::new(initial_state),
            blending_system,
            animations: HashMap::new(),
            current_time: 0.0,
            speed: 1.0,
        }
    }
    
    /// Register an animation
    pub fn add_animation(&mut self, name: String, animation: Animation) {
        self.animations.insert(name, animation);
    }
    
    /// Add a state to the state machine
    pub fn add_state(&mut self, state: AnimationState) {
        self.state_machine.add_state(state);
    }
    
    /// Add a transition between states
    pub fn add_transition(&mut self, transition: AnimationTransition) {
        self.state_machine.add_transition(transition);
    }
    
    /// Set a bool parameter
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.state_machine.set_bool(name, value);
    }
    
    /// Set a float parameter
    pub fn set_float(&mut self, name: &str, value: f32) {
        self.state_machine.set_float(name, value);
    }
    
    /// Set an int parameter
    pub fn set_int(&mut self, name: &str, value: i32) {
        self.state_machine.set_int(name, value);
    }
    
    /// Activate a trigger
    pub fn set_trigger(&mut self, name: &str) {
        self.state_machine.set_trigger(name);
    }
    
    /// Get a bool parameter
    pub fn get_bool(&self, name: &str) -> bool {
        self.state_machine.get_bool(name)
    }
    
    /// Get a float parameter
    pub fn get_float(&self, name: &str) -> f32 {
        self.state_machine.get_float(name)
    }
    
    /// Get an int parameter
    pub fn get_int(&self, name: &str) -> i32 {
        self.state_machine.get_int(name)
    }
    
    /// Set playback speed
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
    
    /// Update the controller
    pub fn update(&mut self, delta: f32) {
        let scaled_delta = delta * self.speed;
        
        // Update state machine
        let previous_state = self.state_machine.current_state.clone();
        self.state_machine.update(scaled_delta);
        let current_state = self.state_machine.current_state.clone();
        
        // Check if state changed
        if previous_state != current_state && !self.state_machine.is_transitioning() {
            // State changed without transition, start crossfade
            if let Some(state) = self.state_machine.get_current_state() {
                if let Some(animation) = self.animations.get(&state.animation_name) {
                    self.blending_system.crossfade_to(
                        animation.clone(),
                        0.2, // Default crossfade time
                        Some("base"),
                    );
                }
            }
        } else if self.state_machine.is_transitioning() {
            // Handle transition with crossfade
            if let Some(target_name) = self.state_machine.get_transition_target() {
                if let Some(target_state) = self.state_machine.states.get(target_name) {
                    if let Some(animation) = self.animations.get(&target_state.animation_name) {
                        // Get transition blend time from state machine
                        let blend_time = self.state_machine.transition_blend_time;
                        
                        // Only start crossfade if not already crossfading to this animation
                        if !self.blending_system.crossfade.as_ref()
                            .map(|cf| cf.to_animation.name == animation.name)
                            .unwrap_or(false)
                        {
                            self.blending_system.crossfade_to(
                                animation.clone(),
                                blend_time,
                                Some("base"),
                            );
                        }
                    }
                }
            }
        }
        
        // Update blending system
        self.blending_system.update(scaled_delta);
        
        self.current_time += scaled_delta;
    }
    
    /// Get the final blended pose
    pub fn get_pose(&self) -> Vec<Transform> {
        self.blending_system.get_blended_pose()
    }
    
    /// Get current state name
    pub fn get_current_state_name(&self) -> &str {
        &self.state_machine.current_state
    }
    
    /// Check if transitioning
    pub fn is_transitioning(&self) -> bool {
        self.state_machine.is_transitioning()
    }
    
    /// Get transition progress (0.0 to 1.0)
    pub fn get_transition_progress(&self) -> f32 {
        self.state_machine.get_transition_progress()
    }
    
    /// Add an animation layer (for additive or layered animations)
    pub fn add_layer(&mut self, layer: AnimationLayer) {
        self.blending_system.add_layer(layer);
    }
    
    /// Get layer by name
    pub fn get_layer_mut(&mut self, name: &str) -> Option<&mut AnimationLayer> {
        self.blending_system.get_layer_mut(name)
    }
}

/// Builder for creating animation controllers
pub struct AnimationControllerBuilder {
    initial_state: String,
    animations: HashMap<String, Animation>,
    states: Vec<AnimationState>,
    transitions: Vec<AnimationTransition>,
}

impl AnimationControllerBuilder {
    /// Create a new builder
    pub fn new(initial_state: &str) -> Self {
        Self {
            initial_state: initial_state.to_string(),
            animations: HashMap::new(),
            states: Vec::new(),
            transitions: Vec::new(),
        }
    }
    
    /// Add an animation
    pub fn with_animation(mut self, name: String, animation: Animation) -> Self {
        self.animations.insert(name, animation);
        self
    }
    
    /// Add a state
    pub fn with_state(mut self, state: AnimationState) -> Self {
        self.states.push(state);
        self
    }
    
    /// Add a transition
    pub fn with_transition(mut self, transition: AnimationTransition) -> Self {
        self.transitions.push(transition);
        self
    }
    
    /// Build the controller
    pub fn build(self) -> AnimationController {
        let mut controller = AnimationController::new(&self.initial_state);
        
        // Add animations
        for (name, animation) in self.animations {
            controller.add_animation(name, animation);
        }
        
        // Add states
        for state in self.states {
            controller.add_state(state);
        }
        
        // Add transitions
        for transition in self.transitions {
            controller.add_transition(transition);
        }
        
        controller
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation_state_machine::TransitionCondition;
    
    #[test]
    fn test_controller_creation() {
        let controller = AnimationController::new("Idle");
        assert_eq!(controller.get_current_state_name(), "Idle");
        assert!(!controller.is_transitioning());
    }
    
    #[test]
    fn test_controller_parameters() {
        let mut controller = AnimationController::new("Idle");
        
        controller.set_bool("grounded", true);
        controller.set_float("speed", 5.0);
        controller.set_int("health", 100);
        
        assert_eq!(controller.get_bool("grounded"), true);
        assert_eq!(controller.get_float("speed"), 5.0);
        assert_eq!(controller.get_int("health"), 100);
    }
    
    #[test]
    fn test_controller_builder() {
        let idle_anim = Animation {
            name: "idle".to_string(),
            duration: 1.0,
            looping: true,
            keyframes: Vec::new(),
        };
        
        let walk_anim = Animation {
            name: "walk".to_string(),
            duration: 0.8,
            looping: true,
            keyframes: Vec::new(),
        };
        
        let controller = AnimationControllerBuilder::new("Idle")
            .with_animation("idle".to_string(), idle_anim)
            .with_animation("walk".to_string(), walk_anim)
            .with_state(AnimationState::new("Idle", "idle"))
            .with_state(AnimationState::new("Walk", "walk"))
            .with_transition(
                AnimationTransition::new("Idle", "Walk", 0.2)
                    .with_condition(TransitionCondition::greater_than("speed", 0.1))
            )
            .build();
        
        assert_eq!(controller.get_current_state_name(), "Idle");
    }
}

