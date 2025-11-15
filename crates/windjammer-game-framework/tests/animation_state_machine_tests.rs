//! Unit tests for Animation State Machine
//!
//! Tests state management, transitions, conditions, and parameters.

use windjammer_game_framework::animation_state_machine::*;

// ============================================================================
// AnimationStateMachine Tests
// ============================================================================

#[test]
fn test_state_machine_creation() {
    let sm = AnimationStateMachine::new("Idle");
    assert_eq!(sm.current_state, "Idle");
    assert!(!sm.is_transitioning());
    assert_eq!(sm.get_transition_progress(), 0.0);
    println!("✅ AnimationStateMachine created with initial state");
}

#[test]
fn test_state_machine_default() {
    let sm = AnimationStateMachine::default();
    assert_eq!(sm.current_state, "Idle");
    println!("✅ AnimationStateMachine default");
}

// ============================================================================
// AnimationState Tests
// ============================================================================

#[test]
fn test_animation_state_creation() {
    let state = AnimationState::new("Walk", "walk_anim");
    assert_eq!(state.name, "Walk");
    assert_eq!(state.animation_name, "walk_anim");
    assert_eq!(state.looping, true);
    assert_eq!(state.speed, 1.0);
    println!("✅ AnimationState created");
}

#[test]
fn test_animation_state_builder() {
    let state = AnimationState::new("Run", "run_anim")
        .with_looping(false)
        .with_speed(1.5);
    
    assert_eq!(state.looping, false);
    assert_eq!(state.speed, 1.5);
    println!("✅ AnimationState builder pattern works");
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
fn test_get_current_state() {
    let mut sm = AnimationStateMachine::new("Idle");
    let idle_state = AnimationState::new("Idle", "idle_anim");
    sm.add_state(idle_state);
    
    let current = sm.get_current_state();
    assert!(current.is_some());
    assert_eq!(current.unwrap().name, "Idle");
    println!("✅ Get current state works");
}

// ============================================================================
// Parameter Tests
// ============================================================================

#[test]
fn test_bool_parameter() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    sm.set_bool("grounded", true);
    assert_eq!(sm.get_bool("grounded"), true);
    
    sm.set_bool("grounded", false);
    assert_eq!(sm.get_bool("grounded"), false);
    
    println!("✅ Bool parameter works");
}

#[test]
fn test_float_parameter() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    sm.set_float("speed", 5.5);
    assert_eq!(sm.get_float("speed"), 5.5);
    
    sm.set_float("speed", 0.0);
    assert_eq!(sm.get_float("speed"), 0.0);
    
    println!("✅ Float parameter works");
}

#[test]
fn test_int_parameter() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    sm.set_int("health", 100);
    assert_eq!(sm.get_int("health"), 100);
    
    sm.set_int("health", 50);
    assert_eq!(sm.get_int("health"), 50);
    
    println!("✅ Int parameter works");
}

#[test]
fn test_trigger_parameter() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    sm.set_trigger("jump");
    
    // Check that trigger is active
    match sm.parameters.get("jump") {
        Some(Parameter::Trigger(true)) => {},
        _ => panic!("Trigger should be active"),
    }
    
    println!("✅ Trigger parameter works");
}

#[test]
fn test_nonexistent_parameter() {
    let sm = AnimationStateMachine::new("Idle");
    
    // Should return default values for nonexistent parameters
    assert_eq!(sm.get_bool("nonexistent"), false);
    assert_eq!(sm.get_float("nonexistent"), 0.0);
    assert_eq!(sm.get_int("nonexistent"), 0);
    
    println!("✅ Nonexistent parameters return defaults");
}

// ============================================================================
// AnimationTransition Tests
// ============================================================================

#[test]
fn test_transition_creation() {
    let transition = AnimationTransition::new("Idle", "Walk", 0.2);
    assert_eq!(transition.from_state, "Idle");
    assert_eq!(transition.to_state, "Walk");
    assert_eq!(transition.blend_time, 0.2);
    assert_eq!(transition.priority, 0);
    println!("✅ AnimationTransition created");
}

#[test]
fn test_transition_builder() {
    let condition = TransitionCondition::is_true("walking");
    let transition = AnimationTransition::new("Idle", "Walk", 0.2)
        .with_condition(condition)
        .with_priority(10);
    
    assert_eq!(transition.conditions.len(), 1);
    assert_eq!(transition.priority, 10);
    println!("✅ AnimationTransition builder pattern works");
}

#[test]
fn test_add_transition() {
    let mut sm = AnimationStateMachine::new("Idle");
    let transition = AnimationTransition::new("Idle", "Walk", 0.2);
    sm.add_transition(transition);
    
    assert_eq!(sm.transitions.len(), 1);
    println!("✅ Transition added to state machine");
}

// ============================================================================
// TransitionCondition Tests
// ============================================================================

#[test]
fn test_condition_is_true() {
    let condition = TransitionCondition::is_true("walking");
    assert_eq!(condition.parameter, "walking");
    assert_eq!(condition.condition_type, ConditionType::IsTrue);
    println!("✅ IsTrue condition created");
}

#[test]
fn test_condition_is_false() {
    let condition = TransitionCondition::is_false("grounded");
    assert_eq!(condition.condition_type, ConditionType::IsFalse);
    println!("✅ IsFalse condition created");
}

#[test]
fn test_condition_greater_than() {
    let condition = TransitionCondition::greater_than("speed", 1.0);
    assert_eq!(condition.condition_type, ConditionType::GreaterThan(1.0));
    println!("✅ GreaterThan condition created");
}

#[test]
fn test_condition_less_than() {
    let condition = TransitionCondition::less_than("speed", 0.5);
    assert_eq!(condition.condition_type, ConditionType::LessThan(0.5));
    println!("✅ LessThan condition created");
}

#[test]
fn test_condition_equals() {
    let condition = TransitionCondition::equals("speed", 0.0);
    assert_eq!(condition.condition_type, ConditionType::Equals(0.0));
    println!("✅ Equals condition created");
}

#[test]
fn test_condition_trigger() {
    let condition = TransitionCondition::trigger("jump");
    assert_eq!(condition.condition_type, ConditionType::Trigger);
    println!("✅ Trigger condition created");
}

// ============================================================================
// Transition Logic Tests
// ============================================================================

#[test]
fn test_simple_transition() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    // Add states
    sm.add_state(AnimationState::new("Idle", "idle_anim"));
    sm.add_state(AnimationState::new("Walk", "walk_anim"));
    
    // Add transition with condition
    let transition = AnimationTransition::new("Idle", "Walk", 0.2)
        .with_condition(TransitionCondition::is_true("walking"));
    sm.add_transition(transition);
    
    // Set parameter to trigger transition
    sm.set_bool("walking", true);
    
    // Update state machine
    sm.update(0.016); // One frame
    
    // Should be transitioning
    assert!(sm.is_transitioning());
    assert_eq!(sm.get_transition_target(), Some("Walk"));
    
    println!("✅ Simple transition works");
}

#[test]
fn test_transition_completion() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    // Add states
    sm.add_state(AnimationState::new("Idle", "idle_anim"));
    sm.add_state(AnimationState::new("Walk", "walk_anim"));
    
    // Add transition
    let transition = AnimationTransition::new("Idle", "Walk", 0.1)
        .with_condition(TransitionCondition::is_true("walking"));
    sm.add_transition(transition);
    
    // Trigger transition
    sm.set_bool("walking", true);
    sm.update(0.016);
    
    // Complete transition
    sm.update(0.2); // Enough time to complete
    
    // Should be in Walk state now
    assert!(!sm.is_transitioning());
    assert_eq!(sm.current_state, "Walk");
    
    println!("✅ Transition completion works");
}

#[test]
fn test_float_condition() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    // Add states
    sm.add_state(AnimationState::new("Idle", "idle_anim"));
    sm.add_state(AnimationState::new("Run", "run_anim"));
    
    // Add transition with float condition
    let transition = AnimationTransition::new("Idle", "Run", 0.2)
        .with_condition(TransitionCondition::greater_than("speed", 5.0));
    sm.add_transition(transition);
    
    // Set speed below threshold
    sm.set_float("speed", 3.0);
    sm.update(0.016);
    assert!(!sm.is_transitioning(), "Should not transition with speed < 5.0");
    
    // Set speed above threshold
    sm.set_float("speed", 6.0);
    sm.update(0.016);
    assert!(sm.is_transitioning(), "Should transition with speed > 5.0");
    
    println!("✅ Float condition works");
}

#[test]
fn test_trigger_condition() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    // Add states
    sm.add_state(AnimationState::new("Idle", "idle_anim"));
    sm.add_state(AnimationState::new("Jump", "jump_anim"));
    
    // Add transition with trigger
    let transition = AnimationTransition::new("Idle", "Jump", 0.1)
        .with_condition(TransitionCondition::trigger("jump"));
    sm.add_transition(transition);
    
    // Activate trigger
    sm.set_trigger("jump");
    sm.update(0.016);
    
    // Should be transitioning
    assert!(sm.is_transitioning());
    
    println!("✅ Trigger condition works");
}

#[test]
fn test_multiple_conditions() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    // Add states
    sm.add_state(AnimationState::new("Idle", "idle_anim"));
    sm.add_state(AnimationState::new("Sprint", "sprint_anim"));
    
    // Add transition with multiple conditions
    let transition = AnimationTransition::new("Idle", "Sprint", 0.2)
        .with_condition(TransitionCondition::is_true("grounded"))
        .with_condition(TransitionCondition::greater_than("speed", 8.0));
    sm.add_transition(transition);
    
    // Set only one condition
    sm.set_bool("grounded", true);
    sm.set_float("speed", 5.0);
    sm.update(0.016);
    assert!(!sm.is_transitioning(), "Should not transition with only one condition met");
    
    // Set both conditions
    sm.set_float("speed", 10.0);
    sm.update(0.016);
    assert!(sm.is_transitioning(), "Should transition with both conditions met");
    
    println!("✅ Multiple conditions work");
}

#[test]
fn test_transition_priority() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    // Add states
    sm.add_state(AnimationState::new("Idle", "idle_anim"));
    sm.add_state(AnimationState::new("Walk", "walk_anim"));
    sm.add_state(AnimationState::new("Run", "run_anim"));
    
    // Add two transitions with different priorities
    let low_priority = AnimationTransition::new("Idle", "Walk", 0.2)
        .with_condition(TransitionCondition::is_true("moving"))
        .with_priority(1);
    
    let high_priority = AnimationTransition::new("Idle", "Run", 0.2)
        .with_condition(TransitionCondition::is_true("moving"))
        .with_priority(10);
    
    sm.add_transition(low_priority);
    sm.add_transition(high_priority);
    
    // Trigger both transitions
    sm.set_bool("moving", true);
    sm.update(0.016);
    
    // Should transition to Run (higher priority)
    assert_eq!(sm.get_transition_target(), Some("Run"));
    
    println!("✅ Transition priority works");
}

#[test]
fn test_transition_progress() {
    let mut sm = AnimationStateMachine::new("Idle");
    
    // Add states
    sm.add_state(AnimationState::new("Idle", "idle_anim"));
    sm.add_state(AnimationState::new("Walk", "walk_anim"));
    
    // Add transition with 0.2s blend time
    let transition = AnimationTransition::new("Idle", "Walk", 0.2)
        .with_condition(TransitionCondition::is_true("walking"));
    sm.add_transition(transition);
    
    // Trigger transition
    sm.set_bool("walking", true);
    sm.update(0.016); // Start transition
    
    // Should be transitioning
    assert!(sm.is_transitioning(), "Should be transitioning");
    
    // Progress starts at 0.0 when transition begins
    let progress0 = sm.get_transition_progress();
    assert_eq!(progress0, 0.0, "Progress should start at 0.0");
    
    // Update to make progress
    sm.update(0.05);
    let progress1 = sm.get_transition_progress();
    assert!(progress1 > 0.0, "Progress should have increased: {}", progress1);
    assert!(progress1 <= 1.0, "Progress should be <= 1.0: {}", progress1);
    
    // Update more
    sm.update(0.05);
    let progress2 = sm.get_transition_progress();
    assert!(progress2 > progress1, "Progress should increase: {} -> {}", progress1, progress2);
    
    println!("✅ Transition progress tracking works: 0.0 -> {} -> {}", progress1, progress2);
}

// ============================================================================
// Parameter Type Tests
// ============================================================================

#[test]
fn test_parameter_equality() {
    assert_eq!(Parameter::Bool(true), Parameter::Bool(true));
    assert_eq!(Parameter::Float(1.0), Parameter::Float(1.0));
    assert_eq!(Parameter::Int(5), Parameter::Int(5));
    assert_eq!(Parameter::Trigger(false), Parameter::Trigger(false));
    
    assert_ne!(Parameter::Bool(true), Parameter::Bool(false));
    assert_ne!(Parameter::Float(1.0), Parameter::Float(2.0));
    
    println!("✅ Parameter equality works");
}

#[test]
fn test_condition_type_equality() {
    assert_eq!(ConditionType::IsTrue, ConditionType::IsTrue);
    assert_eq!(ConditionType::IsFalse, ConditionType::IsFalse);
    assert_eq!(ConditionType::GreaterThan(1.0), ConditionType::GreaterThan(1.0));
    
    assert_ne!(ConditionType::IsTrue, ConditionType::IsFalse);
    assert_ne!(ConditionType::GreaterThan(1.0), ConditionType::GreaterThan(2.0));
    
    println!("✅ ConditionType equality works");
}

