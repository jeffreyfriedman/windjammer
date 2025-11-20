//! AI FFI bindings
//!
//! This module provides C-compatible FFI bindings for AI systems (behavior trees, pathfinding, steering).

use crate::*;
use std::os::raw::{c_char, c_float, c_int};

// ============================================================================
// Behavior Trees
// ============================================================================

/// Opaque handle to a behavior tree
#[repr(C)]
pub struct WjBehaviorTree {
    _private: [u8; 0],
}

/// Behavior tree node types
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjBehaviorNodeType {
    /// Sequence node (runs children in order, fails on first failure)
    Sequence = 0,
    /// Selector node (runs children in order, succeeds on first success)
    Selector = 1,
    /// Parallel node (runs all children simultaneously)
    Parallel = 2,
    /// Decorator node (modifies child behavior)
    Decorator = 3,
    /// Action node (leaf node that performs an action)
    Action = 4,
    /// Condition node (leaf node that checks a condition)
    Condition = 5,
}

/// Create a new behavior tree
#[no_mangle]
pub extern "C" fn wj_behavior_tree_new() -> *mut WjBehaviorTree {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual behavior tree
        Box::into_raw(Box::new(0u8)) as *mut WjBehaviorTree
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_behavior_tree_new: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Free a behavior tree
#[no_mangle]
pub extern "C" fn wj_behavior_tree_free(tree: *mut WjBehaviorTree) {
    if tree.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(tree as *mut u8);
        }
    });
}

/// Add node to behavior tree
#[no_mangle]
pub extern "C" fn wj_behavior_tree_add_node(
    tree: *mut WjBehaviorTree,
    node_type: WjBehaviorNodeType,
    name: *const c_char,
) -> WjErrorCode {
    if tree.is_null() {
        set_last_error("Null tree pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual node
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_behavior_tree_add_node: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Tick behavior tree (update for one frame)
#[no_mangle]
pub extern "C" fn wj_behavior_tree_tick(
    tree: *mut WjBehaviorTree,
    entity: *mut WjEntity,
    delta_time: c_float,
) -> WjErrorCode {
    if tree.is_null() {
        set_last_error("Null tree pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Tick actual tree
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_behavior_tree_tick: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Pathfinding
// ============================================================================

/// Path result
#[repr(C)]
#[derive(Debug, Clone)]
pub struct WjPath {
    pub points: *mut WjVec3,
    pub point_count: usize,
}

/// Find path from start to end
#[no_mangle]
pub extern "C" fn wj_pathfinding_find_path(
    world: *mut WjWorld,
    start: WjVec3,
    end: WjVec3,
) -> WjPath {
    if world.is_null() {
        set_last_error("Null world pointer".to_string());
        return WjPath {
            points: ptr::null_mut(),
            point_count: 0,
        };
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Find actual path
        WjPath {
            points: ptr::null_mut(),
            point_count: 0,
        }
    });
    
    match result {
        Ok(path) => path,
        Err(e) => {
            set_last_error(format!("Panic in wj_pathfinding_find_path: {:?}", e));
            WjPath {
                points: ptr::null_mut(),
                point_count: 0,
            }
        }
    }
}

/// Free path
#[no_mangle]
pub extern "C" fn wj_path_free(path: WjPath) {
    if path.points.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Vec::from_raw_parts(path.points, path.point_count, path.point_count);
        }
    });
}

// ============================================================================
// Steering Behaviors
// ============================================================================

/// Steering behavior types
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjSteeringBehavior {
    Seek = 0,
    Flee = 1,
    Arrive = 2,
    Pursue = 3,
    Evade = 4,
    Wander = 5,
}

/// Calculate steering force
#[no_mangle]
pub extern "C" fn wj_steering_calculate(
    behavior: WjSteeringBehavior,
    position: WjVec3,
    velocity: WjVec3,
    target: WjVec3,
    max_speed: c_float,
) -> WjVec3 {
    let result = panic::catch_unwind(|| {
        // TODO: Calculate actual steering force
        WjVec3 { x: 0.0, y: 0.0, z: 0.0 }
    });
    
    match result {
        Ok(force) => force,
        Err(e) => {
            set_last_error(format!("Panic in wj_steering_calculate: {:?}", e));
            WjVec3 { x: 0.0, y: 0.0, z: 0.0 }
        }
    }
}

/// Add steering behavior to entity
#[no_mangle]
pub extern "C" fn wj_add_steering_behavior(
    entity: *mut WjEntity,
    behavior: WjSteeringBehavior,
    target: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual steering behavior
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_steering_behavior: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// State Machines
// ============================================================================

/// Opaque handle to a state machine
#[repr(C)]
pub struct WjStateMachine {
    _private: [u8; 0],
}

/// Create a new state machine
#[no_mangle]
pub extern "C" fn wj_state_machine_new() -> *mut WjStateMachine {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual state machine
        Box::into_raw(Box::new(0u8)) as *mut WjStateMachine
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_state_machine_new: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Free a state machine
#[no_mangle]
pub extern "C" fn wj_state_machine_free(sm: *mut WjStateMachine) {
    if sm.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(sm as *mut u8);
        }
    });
}

/// Add state to state machine
#[no_mangle]
pub extern "C" fn wj_state_machine_add_state(
    sm: *mut WjStateMachine,
    state_name: *const c_char,
) -> WjErrorCode {
    if sm.is_null() {
        set_last_error("Null state machine pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    if state_name.is_null() {
        set_last_error("Null state name pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual state
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_state_machine_add_state: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Add transition to state machine
#[no_mangle]
pub extern "C" fn wj_state_machine_add_transition(
    sm: *mut WjStateMachine,
    from_state: *const c_char,
    to_state: *const c_char,
    condition: *const c_char,
) -> WjErrorCode {
    if sm.is_null() {
        set_last_error("Null state machine pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual transition
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_state_machine_add_transition: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Update state machine
#[no_mangle]
pub extern "C" fn wj_state_machine_update(
    sm: *mut WjStateMachine,
    entity: *mut WjEntity,
    delta_time: c_float,
) -> WjErrorCode {
    if sm.is_null() {
        set_last_error("Null state machine pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Update actual state machine
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_state_machine_update: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Get current state
#[no_mangle]
pub extern "C" fn wj_state_machine_get_current_state(
    sm: *mut WjStateMachine,
) -> *const c_char {
    if sm.is_null() {
        set_last_error("Null state machine pointer".to_string());
        return ptr::null();
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Get actual current state
        ptr::null()
    });
    
    match result {
        Ok(state) => state,
        Err(e) => {
            set_last_error(format!("Panic in wj_state_machine_get_current_state: {:?}", e));
            ptr::null()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior_node_types() {
        assert_eq!(WjBehaviorNodeType::Sequence as i32, 0);
        assert_eq!(WjBehaviorNodeType::Selector as i32, 1);
    }

    #[test]
    fn test_steering_behaviors() {
        assert_eq!(WjSteeringBehavior::Seek as i32, 0);
        assert_eq!(WjSteeringBehavior::Flee as i32, 1);
    }
}

