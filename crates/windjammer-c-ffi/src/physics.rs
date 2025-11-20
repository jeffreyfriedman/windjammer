//! Physics FFI bindings
//!
//! This module provides C-compatible FFI bindings for the physics system.

use crate::*;
use std::os::raw::{c_float, c_int};

// ============================================================================
// Physics Body Types
// ============================================================================

/// Physics body type
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjBodyType {
    /// Dynamic body (affected by forces)
    Dynamic = 0,
    /// Static body (never moves)
    Static = 1,
    /// Kinematic body (moves but not affected by forces)
    Kinematic = 2,
}

// ============================================================================
// 2D Physics
// ============================================================================

/// Add RigidBody2D component to entity
#[no_mangle]
pub extern "C" fn wj_add_rigidbody2d(
    entity: *mut WjEntity,
    body_type: WjBodyType,
    mass: c_float,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual RigidBody2D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_rigidbody2d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Add BoxCollider2D component to entity
#[no_mangle]
pub extern "C" fn wj_add_box_collider2d(
    entity: *mut WjEntity,
    size: WjVec2,
    offset: WjVec2,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual BoxCollider2D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_box_collider2d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Add CircleCollider2D component to entity
#[no_mangle]
pub extern "C" fn wj_add_circle_collider2d(
    entity: *mut WjEntity,
    radius: c_float,
    offset: WjVec2,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual CircleCollider2D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_circle_collider2d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Apply force to 2D rigid body
#[no_mangle]
pub extern "C" fn wj_rigidbody2d_apply_force(
    entity: *mut WjEntity,
    force: WjVec2,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Apply actual force
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_rigidbody2d_apply_force: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Apply impulse to 2D rigid body
#[no_mangle]
pub extern "C" fn wj_rigidbody2d_apply_impulse(
    entity: *mut WjEntity,
    impulse: WjVec2,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Apply actual impulse
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_rigidbody2d_apply_impulse: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Set 2D rigid body velocity
#[no_mangle]
pub extern "C" fn wj_rigidbody2d_set_velocity(
    entity: *mut WjEntity,
    velocity: WjVec2,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Set actual velocity
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_rigidbody2d_set_velocity: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Get 2D rigid body velocity
#[no_mangle]
pub extern "C" fn wj_rigidbody2d_get_velocity(entity: *mut WjEntity) -> WjVec2 {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjVec2 { x: 0.0, y: 0.0 };
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Get actual velocity
        WjVec2 { x: 0.0, y: 0.0 }
    });
    
    match result {
        Ok(vel) => vel,
        Err(e) => {
            set_last_error(format!("Panic in wj_rigidbody2d_get_velocity: {:?}", e));
            WjVec2 { x: 0.0, y: 0.0 }
        }
    }
}

// ============================================================================
// 3D Physics
// ============================================================================

/// Add RigidBody3D component to entity
#[no_mangle]
pub extern "C" fn wj_add_rigidbody3d(
    entity: *mut WjEntity,
    body_type: WjBodyType,
    mass: c_float,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual RigidBody3D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_rigidbody3d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Add BoxCollider3D component to entity
#[no_mangle]
pub extern "C" fn wj_add_box_collider3d(
    entity: *mut WjEntity,
    size: WjVec3,
    offset: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual BoxCollider3D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_box_collider3d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Add SphereCollider3D component to entity
#[no_mangle]
pub extern "C" fn wj_add_sphere_collider3d(
    entity: *mut WjEntity,
    radius: c_float,
    offset: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual SphereCollider3D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_sphere_collider3d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Add CapsuleCollider3D component to entity
#[no_mangle]
pub extern "C" fn wj_add_capsule_collider3d(
    entity: *mut WjEntity,
    radius: c_float,
    height: c_float,
    offset: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual CapsuleCollider3D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_capsule_collider3d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Apply force to 3D rigid body
#[no_mangle]
pub extern "C" fn wj_rigidbody3d_apply_force(
    entity: *mut WjEntity,
    force: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Apply actual force
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_rigidbody3d_apply_force: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Apply torque to 3D rigid body
#[no_mangle]
pub extern "C" fn wj_rigidbody3d_apply_torque(
    entity: *mut WjEntity,
    torque: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Apply actual torque
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_rigidbody3d_apply_torque: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Raycasting
// ============================================================================

/// Raycast result
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjRaycastHit2D {
    pub hit: bool,
    pub point: WjVec2,
    pub normal: WjVec2,
    pub distance: c_float,
    pub entity: *mut WjEntity,
}

/// Raycast result (3D)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjRaycastHit3D {
    pub hit: bool,
    pub point: WjVec3,
    pub normal: WjVec3,
    pub distance: c_float,
    pub entity: *mut WjEntity,
}

/// Perform 2D raycast
#[no_mangle]
pub extern "C" fn wj_raycast2d(
    world: *mut WjWorld,
    origin: WjVec2,
    direction: WjVec2,
    max_distance: c_float,
) -> WjRaycastHit2D {
    if world.is_null() {
        set_last_error("Null world pointer".to_string());
        return WjRaycastHit2D {
            hit: false,
            point: WjVec2 { x: 0.0, y: 0.0 },
            normal: WjVec2 { x: 0.0, y: 0.0 },
            distance: 0.0,
            entity: ptr::null_mut(),
        };
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Perform actual raycast
        WjRaycastHit2D {
            hit: false,
            point: WjVec2 { x: 0.0, y: 0.0 },
            normal: WjVec2 { x: 0.0, y: 0.0 },
            distance: 0.0,
            entity: ptr::null_mut(),
        }
    });
    
    match result {
        Ok(hit) => hit,
        Err(e) => {
            set_last_error(format!("Panic in wj_raycast2d: {:?}", e));
            WjRaycastHit2D {
                hit: false,
                point: WjVec2 { x: 0.0, y: 0.0 },
                normal: WjVec2 { x: 0.0, y: 0.0 },
                distance: 0.0,
                entity: ptr::null_mut(),
            }
        }
    }
}

/// Perform 3D raycast
#[no_mangle]
pub extern "C" fn wj_raycast3d(
    world: *mut WjWorld,
    origin: WjVec3,
    direction: WjVec3,
    max_distance: c_float,
) -> WjRaycastHit3D {
    if world.is_null() {
        set_last_error("Null world pointer".to_string());
        return WjRaycastHit3D {
            hit: false,
            point: WjVec3 { x: 0.0, y: 0.0, z: 0.0 },
            normal: WjVec3 { x: 0.0, y: 0.0, z: 0.0 },
            distance: 0.0,
            entity: ptr::null_mut(),
        };
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Perform actual raycast
        WjRaycastHit3D {
            hit: false,
            point: WjVec3 { x: 0.0, y: 0.0, z: 0.0 },
            normal: WjVec3 { x: 0.0, y: 0.0, z: 0.0 },
            distance: 0.0,
            entity: ptr::null_mut(),
        }
    });
    
    match result {
        Ok(hit) => hit,
        Err(e) => {
            set_last_error(format!("Panic in wj_raycast3d: {:?}", e));
            WjRaycastHit3D {
                hit: false,
                point: WjVec3 { x: 0.0, y: 0.0, z: 0.0 },
                normal: WjVec3 { x: 0.0, y: 0.0, z: 0.0 },
                distance: 0.0,
                entity: ptr::null_mut(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_type_values() {
        assert_eq!(WjBodyType::Dynamic as i32, 0);
        assert_eq!(WjBodyType::Static as i32, 1);
        assert_eq!(WjBodyType::Kinematic as i32, 2);
    }

    #[test]
    fn test_raycast_hit_creation() {
        let hit = WjRaycastHit2D {
            hit: true,
            point: WjVec2 { x: 1.0, y: 2.0 },
            normal: WjVec2 { x: 0.0, y: 1.0 },
            distance: 5.0,
            entity: ptr::null_mut(),
        };
        assert!(hit.hit);
        assert_eq!(hit.distance, 5.0);
    }
}

