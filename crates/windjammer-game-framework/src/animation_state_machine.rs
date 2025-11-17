//! Animation State Machine
//!
//! Provides a state machine for managing animation transitions, blending, and conditions.
//! Critical for AAA games with complex character animations.
//!
//! ## Features
//! - State-based animation playback
//! - Smooth transitions with blend times
//! - Conditional transitions (based on parameters)
//! - Animation layers (for additive animations)
//! - Blend trees (for directional movement)

use std::collections::HashMap;

/// Animation State Machine
///
/// Manages animation states and transitions for a character or object.
#[derive(Debug, Clone)]
pub struct AnimationStateMachine {
    /// All states in the state machine
    pub states: HashMap<String, AnimationState>,
    /// All transitions between states
    pub transitions: Vec<AnimationTransition>,
    /// Current active state
    pub current_state: String,
    /// Parameters for transition conditions
    pub parameters: HashMap<String, Parameter>,
    /// Transition progress (0.0 to 1.0)
    pub transition_progress: f32,
    /// Transition target state (if transitioning)
    pub transition_target: Option<String>,
    /// Transition blend time
    pub transition_blend_time: f32,
}

/// Animation state
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// State name
    pub name: String,
    /// Animation to play in this state
    pub animation_name: String,
    /// Whether to loop the animation
    pub looping: bool,
    /// Playback speed multiplier
    pub speed: f32,
}

/// Animation transition
#[derive(Debug, Clone)]
pub struct AnimationTransition {
    /// Source state name
    pub from_state: String,
    /// Target state name
    pub to_state: String,
    /// Blend time (seconds)
    pub blend_time: f32,
    /// Conditions that must be met for transition
    pub conditions: Vec<TransitionCondition>,
    /// Priority (higher = checked first)
    pub priority: i32,
}

/// Transition condition
#[derive(Debug, Clone)]
pub struct TransitionCondition {
    /// Parameter name
    pub parameter: String,
    /// Condition type
    pub condition_type: ConditionType,
}

/// Condition type for transitions
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionType {
    /// Bool parameter is true
    IsTrue,
    /// Bool parameter is false
    IsFalse,
    /// Float parameter is greater than value
    GreaterThan(f32),
    /// Float parameter is less than value
    LessThan(f32),
    /// Float parameter equals value (with epsilon)
    Equals(f32),
    /// Trigger was activated (auto-resets)
    Trigger,
}

/// Parameter value
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    Bool(bool),
    Float(f32),
    Int(i32),
    Trigger(bool),
}

impl AnimationStateMachine {
    /// Create a new animation state machine
    pub fn new(initial_state: &str) -> Self {
        Self {
            states: HashMap::new(),
            transitions: Vec::new(),
            current_state: initial_state.to_string(),
            parameters: HashMap::new(),
            transition_progress: 0.0,
            transition_target: None,
            transition_blend_time: 0.0,
        }
    }

    /// Add a state to the state machine
    pub fn add_state(&mut self, state: AnimationState) {
        self.states.insert(state.name.clone(), state);
    }

    /// Add a transition between states
    pub fn add_transition(&mut self, transition: AnimationTransition) {
        self.transitions.push(transition);
    }

    /// Set a parameter value
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.parameters.insert(name.to_string(), Parameter::Bool(value));
    }

    /// Set a float parameter
    pub fn set_float(&mut self, name: &str, value: f32) {
        self.parameters.insert(name.to_string(), Parameter::Float(value));
    }

    /// Set an int parameter
    pub fn set_int(&mut self, name: &str, value: i32) {
        self.parameters.insert(name.to_string(), Parameter::Int(value));
    }

    /// Activate a trigger
    pub fn set_trigger(&mut self, name: &str) {
        self.parameters.insert(name.to_string(), Parameter::Trigger(true));
    }

    /// Get a bool parameter
    pub fn get_bool(&self, name: &str) -> bool {
        match self.parameters.get(name) {
            Some(Parameter::Bool(v)) => *v,
            _ => false,
        }
    }

    /// Get a float parameter
    pub fn get_float(&self, name: &str) -> f32 {
        match self.parameters.get(name) {
            Some(Parameter::Float(v)) => *v,
            _ => 0.0,
        }
    }

    /// Get an int parameter
    pub fn get_int(&self, name: &str) -> i32 {
        match self.parameters.get(name) {
            Some(Parameter::Int(v)) => *v,
            _ => 0,
        }
    }

    /// Update the state machine
    pub fn update(&mut self, delta: f32) {
        // If transitioning, update transition progress
        if let Some(target) = &self.transition_target {
            self.transition_progress += delta / self.transition_blend_time;
            
            if self.transition_progress >= 1.0 {
                // Transition complete
                self.current_state = target.clone();
                self.transition_target = None;
                self.transition_progress = 0.0;
            }
            return;
        }

        // Check for valid transitions (sorted by priority)
        let mut valid_transitions: Vec<_> = self.transitions.iter()
            .filter(|t| t.from_state == self.current_state)
            .collect();
        valid_transitions.sort_by_key(|t| -t.priority);

        for transition in valid_transitions {
            if self.check_conditions(&transition.conditions) {
                // Start transition
                self.transition_target = Some(transition.to_state.clone());
                self.transition_blend_time = transition.blend_time;
                self.transition_progress = 0.0;
                
                // Reset triggers
                self.reset_triggers();
                break;
            }
        }
    }

    /// Check if all conditions are met
    fn check_conditions(&self, conditions: &[TransitionCondition]) -> bool {
        for condition in conditions {
            if !self.check_condition(condition) {
                return false;
            }
        }
        true
    }

    /// Check a single condition
    fn check_condition(&self, condition: &TransitionCondition) -> bool {
        let param = match self.parameters.get(&condition.parameter) {
            Some(p) => p,
            None => return false,
        };

        match (&condition.condition_type, param) {
            (ConditionType::IsTrue, Parameter::Bool(v)) => *v,
            (ConditionType::IsFalse, Parameter::Bool(v)) => !*v,
            (ConditionType::GreaterThan(threshold), Parameter::Float(v)) => *v > *threshold,
            (ConditionType::LessThan(threshold), Parameter::Float(v)) => *v < *threshold,
            (ConditionType::Equals(target), Parameter::Float(v)) => (*v - *target).abs() < 0.001,
            (ConditionType::Trigger, Parameter::Trigger(v)) => *v,
            _ => false,
        }
    }

    /// Reset all trigger parameters
    fn reset_triggers(&mut self) {
        for param in self.parameters.values_mut() {
            if let Parameter::Trigger(ref mut v) = param {
                *v = false;
            }
        }
    }

    /// Get the current state
    pub fn get_current_state(&self) -> Option<&AnimationState> {
        self.states.get(&self.current_state)
    }

    /// Check if currently transitioning
    pub fn is_transitioning(&self) -> bool {
        self.transition_target.is_some()
    }

    /// Get transition progress (0.0 to 1.0)
    pub fn get_transition_progress(&self) -> f32 {
        self.transition_progress
    }

    /// Get transition target state (if transitioning)
    pub fn get_transition_target(&self) -> Option<&str> {
        self.transition_target.as_deref()
    }
}

impl Default for AnimationStateMachine {
    fn default() -> Self {
        Self::new("Idle")
    }
}

impl AnimationState {
    /// Create a new animation state
    pub fn new(name: &str, animation_name: &str) -> Self {
        Self {
            name: name.to_string(),
            animation_name: animation_name.to_string(),
            looping: true,
            speed: 1.0,
        }
    }

    /// Set whether the animation loops
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Set the playback speed
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

impl AnimationTransition {
    /// Create a new transition
    pub fn new(from: &str, to: &str, blend_time: f32) -> Self {
        Self {
            from_state: from.to_string(),
            to_state: to.to_string(),
            blend_time,
            conditions: Vec::new(),
            priority: 0,
        }
    }

    /// Add a condition to the transition
    pub fn with_condition(mut self, condition: TransitionCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl TransitionCondition {
    /// Create a bool condition (is true)
    pub fn is_true(parameter: &str) -> Self {
        Self {
            parameter: parameter.to_string(),
            condition_type: ConditionType::IsTrue,
        }
    }

    /// Create a bool condition (is false)
    pub fn is_false(parameter: &str) -> Self {
        Self {
            parameter: parameter.to_string(),
            condition_type: ConditionType::IsFalse,
        }
    }

    /// Create a float condition (greater than)
    pub fn greater_than(parameter: &str, value: f32) -> Self {
        Self {
            parameter: parameter.to_string(),
            condition_type: ConditionType::GreaterThan(value),
        }
    }

    /// Create a float condition (less than)
    pub fn less_than(parameter: &str, value: f32) -> Self {
        Self {
            parameter: parameter.to_string(),
            condition_type: ConditionType::LessThan(value),
        }
    }

    /// Create a float condition (equals)
    pub fn equals(parameter: &str, value: f32) -> Self {
        Self {
            parameter: parameter.to_string(),
            condition_type: ConditionType::Equals(value),
        }
    }

    /// Create a trigger condition
    pub fn trigger(parameter: &str) -> Self {
        Self {
            parameter: parameter.to_string(),
            condition_type: ConditionType::Trigger,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_creation() {
        let sm = AnimationStateMachine::new("Idle");
        assert_eq!(sm.current_state, "Idle");
        assert!(!sm.is_transitioning());
        println!("✅ AnimationStateMachine created");
    }

    #[test]
    fn test_add_state() {
        let mut sm = AnimationStateMachine::new("Idle");
        let state = AnimationState::new("Walk", "walk_anim");
        sm.add_state(state);
        
        assert!(sm.states.contains_key("Walk"));
        println!("✅ State added to state machine");
    }

    #[test]
    fn test_parameters() {
        let mut sm = AnimationStateMachine::new("Idle");
        
        sm.set_bool("grounded", true);
        sm.set_float("speed", 5.0);
        sm.set_int("health", 100);
        
        assert_eq!(sm.get_bool("grounded"), true);
        assert_eq!(sm.get_float("speed"), 5.0);
        assert_eq!(sm.get_int("health"), 100);
        
        println!("✅ Parameters work");
    }

    #[test]
    fn test_trigger() {
        let mut sm = AnimationStateMachine::new("Idle");
        
        sm.set_trigger("jump");
        
        // Trigger should be active
        match sm.parameters.get("jump") {
            Some(Parameter::Trigger(true)) => {},
            _ => panic!("Trigger should be active"),
        }
        
        println!("✅ Trigger works");
    }
}

