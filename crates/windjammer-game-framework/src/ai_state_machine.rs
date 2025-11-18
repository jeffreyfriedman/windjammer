// AI State Machine System
// Provides state-based AI behavior for NPCs, enemies, and agents.

use std::collections::HashMap;

/// AI state identifier
pub type StateId = String;

/// AI state machine state
#[derive(Debug, Clone)]
pub struct AIState {
    /// Unique identifier for this state
    pub id: StateId,
    /// Human-readable name
    pub name: String,
    /// Whether this is an entry state
    pub is_entry: bool,
    /// Custom data for this state
    pub data: HashMap<String, f32>,
}

impl AIState {
    pub fn new(id: StateId, name: String) -> Self {
        Self {
            id,
            name,
            is_entry: false,
            data: HashMap::new(),
        }
    }

    pub fn entry(mut self) -> Self {
        self.is_entry = true;
        self
    }

    pub fn with_data(mut self, key: String, value: f32) -> Self {
        self.data.insert(key, value);
        self
    }

    pub fn get_data(&self, key: &str) -> Option<f32> {
        self.data.get(key).copied()
    }

    pub fn set_data(&mut self, key: String, value: f32) {
        self.data.insert(key, value);
    }
}

/// Transition condition type
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionCondition {
    /// Always transition
    Always,
    /// Transition after a duration (seconds)
    Timer(f32),
    /// Transition when a parameter equals a value
    ParameterEquals { param: String, value: f32 },
    /// Transition when a parameter is greater than a value
    ParameterGreater { param: String, value: f32 },
    /// Transition when a parameter is less than a value
    ParameterLess { param: String, value: f32 },
    /// Transition when a boolean parameter is true
    ParameterTrue { param: String },
    /// Transition when a boolean parameter is false
    ParameterFalse { param: String },
    /// Custom condition (evaluated externally)
    Custom { id: String },
}

impl TransitionCondition {
    pub fn timer(duration: f32) -> Self {
        Self::Timer(duration)
    }

    pub fn parameter_equals(param: String, value: f32) -> Self {
        Self::ParameterEquals { param, value }
    }

    pub fn parameter_greater(param: String, value: f32) -> Self {
        Self::ParameterGreater { param, value }
    }

    pub fn parameter_less(param: String, value: f32) -> Self {
        Self::ParameterLess { param, value }
    }

    pub fn parameter_true(param: String) -> Self {
        Self::ParameterTrue { param }
    }

    pub fn parameter_false(param: String) -> Self {
        Self::ParameterFalse { param }
    }

    pub fn custom(id: String) -> Self {
        Self::Custom { id }
    }
}

/// AI state transition
#[derive(Debug, Clone)]
pub struct AITransition {
    /// Source state ID
    pub from: StateId,
    /// Target state ID
    pub to: StateId,
    /// Transition condition
    pub condition: TransitionCondition,
    /// Priority (higher priority transitions are checked first)
    pub priority: i32,
    /// Transition duration (for blending)
    pub duration: f32,
    /// Internal timer for Timer conditions
    pub(crate) timer: f32,
}

impl AITransition {
    pub fn new(from: StateId, to: StateId, condition: TransitionCondition) -> Self {
        Self {
            from,
            to,
            condition,
            priority: 0,
            duration: 0.0,
            timer: 0.0,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// Check if this transition should fire
    pub fn should_transition(&self, parameters: &HashMap<String, f32>) -> bool {
        match &self.condition {
            TransitionCondition::Always => true,
            TransitionCondition::Timer(duration) => self.timer >= *duration,
            TransitionCondition::ParameterEquals { param, value } => {
                parameters.get(param).map(|v| (v - value).abs() < 0.001).unwrap_or(false)
            }
            TransitionCondition::ParameterGreater { param, value } => {
                parameters.get(param).map(|v| v > value).unwrap_or(false)
            }
            TransitionCondition::ParameterLess { param, value } => {
                parameters.get(param).map(|v| v < value).unwrap_or(false)
            }
            TransitionCondition::ParameterTrue { param } => {
                parameters.get(param).map(|v| *v > 0.5).unwrap_or(false)
            }
            TransitionCondition::ParameterFalse { param } => {
                parameters.get(param).map(|v| *v <= 0.5).unwrap_or(false)
            }
            TransitionCondition::Custom { .. } => false, // Evaluated externally
        }
    }

    /// Update the transition timer
    pub fn update(&mut self, delta_time: f32) {
        if matches!(self.condition, TransitionCondition::Timer(_)) {
            self.timer += delta_time;
        }
    }

    /// Reset the transition timer
    pub fn reset(&mut self) {
        self.timer = 0.0;
    }
}

/// AI state machine
pub struct AIStateMachine {
    /// All states in the machine
    states: HashMap<StateId, AIState>,
    /// All transitions
    transitions: Vec<AITransition>,
    /// Current state ID
    current_state: Option<StateId>,
    /// Parameters for transition conditions
    parameters: HashMap<String, f32>,
    /// Custom condition evaluators
    custom_conditions: HashMap<String, Box<dyn Fn(&HashMap<String, f32>) -> bool>>,
    /// Time spent in current state
    state_time: f32,
}

impl AIStateMachine {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            transitions: Vec::new(),
            current_state: None,
            parameters: HashMap::new(),
            custom_conditions: HashMap::new(),
            state_time: 0.0,
        }
    }

    /// Add a state to the machine
    pub fn add_state(&mut self, state: AIState) {
        let is_entry = state.is_entry;
        let id = state.id.clone();
        self.states.insert(state.id.clone(), state);

        // Set as current state if it's an entry state and we don't have one
        if is_entry && self.current_state.is_none() {
            self.current_state = Some(id);
        }
    }

    /// Add a transition to the machine
    pub fn add_transition(&mut self, transition: AITransition) {
        self.transitions.push(transition);
        // Sort by priority (higher priority first)
        self.transitions.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Set a parameter value
    pub fn set_parameter(&mut self, name: String, value: f32) {
        self.parameters.insert(name, value);
    }

    /// Get a parameter value
    pub fn get_parameter(&self, name: &str) -> Option<f32> {
        self.parameters.get(name).copied()
    }

    /// Set a boolean parameter
    pub fn set_bool(&mut self, name: String, value: bool) {
        self.parameters.insert(name, if value { 1.0 } else { 0.0 });
    }

    /// Get a boolean parameter
    pub fn get_bool(&self, name: &str) -> bool {
        self.parameters.get(name).map(|v| *v > 0.5).unwrap_or(false)
    }

    /// Register a custom condition evaluator
    pub fn register_custom_condition<F>(&mut self, id: String, evaluator: F)
    where
        F: Fn(&HashMap<String, f32>) -> bool + 'static,
    {
        self.custom_conditions.insert(id, Box::new(evaluator));
    }

    /// Get the current state
    pub fn current_state(&self) -> Option<&AIState> {
        self.current_state.as_ref().and_then(|id| self.states.get(id))
    }

    /// Get the current state ID
    pub fn current_state_id(&self) -> Option<&StateId> {
        self.current_state.as_ref()
    }

    /// Get time spent in current state
    pub fn state_time(&self) -> f32 {
        self.state_time
    }

    /// Force a transition to a specific state
    pub fn transition_to(&mut self, state_id: StateId) -> Result<(), String> {
        if !self.states.contains_key(&state_id) {
            return Err(format!("State '{}' does not exist", state_id));
        }

        self.current_state = Some(state_id);
        self.state_time = 0.0;

        // Reset all transition timers
        for transition in &mut self.transitions {
            transition.reset();
        }

        Ok(())
    }

    /// Update the state machine
    pub fn update(&mut self, delta_time: f32) {
        self.state_time += delta_time;

        // Update all transition timers
        for transition in &mut self.transitions {
            transition.update(delta_time);
        }

        // Check for transitions from current state
        if let Some(current_id) = &self.current_state {
            for transition in &self.transitions {
                if &transition.from == current_id {
                    // Check standard conditions
                    let should_transition = transition.should_transition(&self.parameters);

                    // Check custom conditions
                    let custom_result = if let TransitionCondition::Custom { id } = &transition.condition {
                        self.custom_conditions
                            .get(id)
                            .map(|f| f(&self.parameters))
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if should_transition || custom_result {
                        // Perform transition
                        let _ = self.transition_to(transition.to.clone());
                        break; // Only one transition per update
                    }
                }
            }
        }
    }

    /// Get all states
    pub fn states(&self) -> impl Iterator<Item = &AIState> {
        self.states.values()
    }

    /// Get all transitions
    pub fn transitions(&self) -> &[AITransition] {
        &self.transitions
    }

    /// Get the number of states
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Get the number of transitions
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }
}

impl Default for AIStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for AI state machines
pub struct AIStateMachineBuilder {
    machine: AIStateMachine,
}

impl AIStateMachineBuilder {
    pub fn new() -> Self {
        Self {
            machine: AIStateMachine::new(),
        }
    }

    pub fn add_state(mut self, state: AIState) -> Self {
        self.machine.add_state(state);
        self
    }

    pub fn add_transition(mut self, transition: AITransition) -> Self {
        self.machine.add_transition(transition);
        self
    }

    pub fn with_parameter(mut self, name: String, value: f32) -> Self {
        self.machine.set_parameter(name, value);
        self
    }

    pub fn build(self) -> AIStateMachine {
        self.machine
    }

    /// Create a simple patrol state machine
    pub fn patrol() -> AIStateMachine {
        let mut builder = Self::new();

        // States
        builder = builder
            .add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry())
            .add_state(AIState::new("patrol".to_string(), "Patrol".to_string()))
            .add_state(AIState::new("investigate".to_string(), "Investigate".to_string()));

        // Transitions
        builder = builder
            .add_transition(AITransition::new(
                "idle".to_string(),
                "patrol".to_string(),
                TransitionCondition::timer(2.0),
            ))
            .add_transition(AITransition::new(
                "patrol".to_string(),
                "investigate".to_string(),
                TransitionCondition::parameter_true("heard_noise".to_string()),
            ))
            .add_transition(AITransition::new(
                "investigate".to_string(),
                "patrol".to_string(),
                TransitionCondition::timer(5.0),
            ));

        builder.build()
    }

    /// Create a simple combat state machine
    pub fn combat() -> AIStateMachine {
        let mut builder = Self::new();

        // States
        builder = builder
            .add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry())
            .add_state(AIState::new("chase".to_string(), "Chase".to_string()))
            .add_state(AIState::new("attack".to_string(), "Attack".to_string()))
            .add_state(AIState::new("retreat".to_string(), "Retreat".to_string()));

        // Transitions
        builder = builder
            .add_transition(AITransition::new(
                "idle".to_string(),
                "chase".to_string(),
                TransitionCondition::parameter_true("enemy_spotted".to_string()),
            ))
            .add_transition(AITransition::new(
                "chase".to_string(),
                "attack".to_string(),
                TransitionCondition::parameter_less("distance_to_enemy".to_string(), 5.0),
            ))
            .add_transition(AITransition::new(
                "attack".to_string(),
                "chase".to_string(),
                TransitionCondition::parameter_greater("distance_to_enemy".to_string(), 7.0),
            ))
            .add_transition(AITransition::new(
                "attack".to_string(),
                "retreat".to_string(),
                TransitionCondition::parameter_less("health".to_string(), 0.3),
            ))
            .add_transition(AITransition::new(
                "retreat".to_string(),
                "idle".to_string(),
                TransitionCondition::parameter_false("enemy_spotted".to_string()),
            ));

        builder.build()
    }
}

impl Default for AIStateMachineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let state = AIState::new("idle".to_string(), "Idle".to_string());
        assert_eq!(state.id, "idle");
        assert_eq!(state.name, "Idle");
        assert!(!state.is_entry);
    }

    #[test]
    fn test_state_entry() {
        let state = AIState::new("idle".to_string(), "Idle".to_string()).entry();
        assert!(state.is_entry);
    }

    #[test]
    fn test_state_data() {
        let mut state = AIState::new("idle".to_string(), "Idle".to_string())
            .with_data("speed".to_string(), 5.0);

        assert_eq!(state.get_data("speed"), Some(5.0));
        assert_eq!(state.get_data("missing"), None);

        state.set_data("speed".to_string(), 10.0);
        assert_eq!(state.get_data("speed"), Some(10.0));
    }

    #[test]
    fn test_transition_condition_timer() {
        let condition = TransitionCondition::timer(2.0);
        assert_eq!(condition, TransitionCondition::Timer(2.0));
    }

    #[test]
    fn test_transition_should_transition_timer() {
        let mut transition = AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::timer(2.0),
        );

        let params = HashMap::new();
        assert!(!transition.should_transition(&params));

        transition.timer = 2.5;
        assert!(transition.should_transition(&params));
    }

    #[test]
    fn test_transition_should_transition_parameter_equals() {
        let transition = AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::parameter_equals("health".to_string(), 100.0),
        );

        let mut params = HashMap::new();
        assert!(!transition.should_transition(&params));

        params.insert("health".to_string(), 100.0);
        assert!(transition.should_transition(&params));

        params.insert("health".to_string(), 50.0);
        assert!(!transition.should_transition(&params));
    }

    #[test]
    fn test_transition_should_transition_parameter_greater() {
        let transition = AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::parameter_greater("speed".to_string(), 5.0),
        );

        let mut params = HashMap::new();
        params.insert("speed".to_string(), 10.0);
        assert!(transition.should_transition(&params));

        params.insert("speed".to_string(), 3.0);
        assert!(!transition.should_transition(&params));
    }

    #[test]
    fn test_transition_should_transition_parameter_less() {
        let transition = AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::parameter_less("health".to_string(), 50.0),
        );

        let mut params = HashMap::new();
        params.insert("health".to_string(), 30.0);
        assert!(transition.should_transition(&params));

        params.insert("health".to_string(), 70.0);
        assert!(!transition.should_transition(&params));
    }

    #[test]
    fn test_transition_should_transition_parameter_true() {
        let transition = AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::parameter_true("is_active".to_string()),
        );

        let mut params = HashMap::new();
        params.insert("is_active".to_string(), 1.0);
        assert!(transition.should_transition(&params));

        params.insert("is_active".to_string(), 0.0);
        assert!(!transition.should_transition(&params));
    }

    #[test]
    fn test_transition_update() {
        let mut transition = AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::timer(2.0),
        );

        assert_eq!(transition.timer, 0.0);

        transition.update(1.0);
        assert_eq!(transition.timer, 1.0);

        transition.update(0.5);
        assert_eq!(transition.timer, 1.5);
    }

    #[test]
    fn test_transition_reset() {
        let mut transition = AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::timer(2.0),
        );

        transition.timer = 5.0;
        transition.reset();
        assert_eq!(transition.timer, 0.0);
    }

    #[test]
    fn test_state_machine_creation() {
        let machine = AIStateMachine::new();
        assert_eq!(machine.state_count(), 0);
        assert_eq!(machine.transition_count(), 0);
        assert!(machine.current_state().is_none());
    }

    #[test]
    fn test_state_machine_add_state() {
        let mut machine = AIStateMachine::new();
        machine.add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry());

        assert_eq!(machine.state_count(), 1);
        assert_eq!(machine.current_state_id(), Some(&"idle".to_string()));
    }

    #[test]
    fn test_state_machine_add_transition() {
        let mut machine = AIStateMachine::new();
        machine.add_transition(AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::timer(2.0),
        ));

        assert_eq!(machine.transition_count(), 1);
    }

    #[test]
    fn test_state_machine_parameters() {
        let mut machine = AIStateMachine::new();
        machine.set_parameter("health".to_string(), 100.0);

        assert_eq!(machine.get_parameter("health"), Some(100.0));
        assert_eq!(machine.get_parameter("missing"), None);
    }

    #[test]
    fn test_state_machine_bool_parameters() {
        let mut machine = AIStateMachine::new();
        machine.set_bool("is_active".to_string(), true);

        assert!(machine.get_bool("is_active"));
        assert!(!machine.get_bool("missing"));

        machine.set_bool("is_active".to_string(), false);
        assert!(!machine.get_bool("is_active"));
    }

    #[test]
    fn test_state_machine_transition_to() {
        let mut machine = AIStateMachine::new();
        machine.add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry());
        machine.add_state(AIState::new("patrol".to_string(), "Patrol".to_string()));

        let result = machine.transition_to("patrol".to_string());
        assert!(result.is_ok());
        assert_eq!(machine.current_state_id(), Some(&"patrol".to_string()));

        let result = machine.transition_to("missing".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_state_machine_update() {
        let mut machine = AIStateMachine::new();
        machine.add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry());
        machine.add_state(AIState::new("patrol".to_string(), "Patrol".to_string()));
        machine.add_transition(AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::timer(2.0),
        ));

        assert_eq!(machine.current_state_id(), Some(&"idle".to_string()));

        machine.update(1.0);
        assert_eq!(machine.current_state_id(), Some(&"idle".to_string()));

        machine.update(1.5);
        assert_eq!(machine.current_state_id(), Some(&"patrol".to_string()));
    }

    #[test]
    fn test_state_machine_state_time() {
        let mut machine = AIStateMachine::new();
        machine.add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry());

        assert_eq!(machine.state_time(), 0.0);

        machine.update(1.0);
        assert_eq!(machine.state_time(), 1.0);

        machine.update(0.5);
        assert_eq!(machine.state_time(), 1.5);
    }

    #[test]
    fn test_builder_basic() {
        let machine = AIStateMachineBuilder::new()
            .add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry())
            .add_state(AIState::new("patrol".to_string(), "Patrol".to_string()))
            .add_transition(AITransition::new(
                "idle".to_string(),
                "patrol".to_string(),
                TransitionCondition::timer(2.0),
            ))
            .with_parameter("health".to_string(), 100.0)
            .build();

        assert_eq!(machine.state_count(), 2);
        assert_eq!(machine.transition_count(), 1);
        assert_eq!(machine.get_parameter("health"), Some(100.0));
    }

    #[test]
    fn test_builder_patrol_preset() {
        let machine = AIStateMachineBuilder::patrol();

        assert_eq!(machine.state_count(), 3);
        assert!(machine.transition_count() > 0);
        assert_eq!(machine.current_state_id(), Some(&"idle".to_string()));
    }

    #[test]
    fn test_builder_combat_preset() {
        let machine = AIStateMachineBuilder::combat();

        assert_eq!(machine.state_count(), 4);
        assert!(machine.transition_count() > 0);
        assert_eq!(machine.current_state_id(), Some(&"idle".to_string()));
    }

    #[test]
    fn test_custom_condition() {
        let mut machine = AIStateMachine::new();
        machine.add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry());
        machine.add_state(AIState::new("patrol".to_string(), "Patrol".to_string()));

        // Register custom condition
        machine.register_custom_condition("always_true".to_string(), |_| true);

        machine.add_transition(AITransition::new(
            "idle".to_string(),
            "patrol".to_string(),
            TransitionCondition::custom("always_true".to_string()),
        ));

        machine.update(0.1);
        assert_eq!(machine.current_state_id(), Some(&"patrol".to_string()));
    }

    #[test]
    fn test_transition_priority() {
        let mut machine = AIStateMachine::new();
        machine.add_state(AIState::new("idle".to_string(), "Idle".to_string()).entry());
        machine.add_state(AIState::new("patrol".to_string(), "Patrol".to_string()));
        machine.add_state(AIState::new("alert".to_string(), "Alert".to_string()));

        // Add two transitions with different priorities
        machine.add_transition(
            AITransition::new(
                "idle".to_string(),
                "patrol".to_string(),
                TransitionCondition::Always,
            )
            .with_priority(1),
        );

        machine.add_transition(
            AITransition::new(
                "idle".to_string(),
                "alert".to_string(),
                TransitionCondition::Always,
            )
            .with_priority(10),
        );

        machine.update(0.1);
        // Should transition to alert (higher priority)
        assert_eq!(machine.current_state_id(), Some(&"alert".to_string()));
    }
}


