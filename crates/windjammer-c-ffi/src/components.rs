//! Component FFI bindings
//!
//! This module provides C-compatible FFI bindings for ECS components.

use crate::*;
use std::os::raw::{c_char, c_float};

// ============================================================================
// Transform Components
// ============================================================================

/// Add Transform2D component to entity
#[no_mangle]
pub extern "C" fn wj_add_transform2d(
    entity: *mut WjEntity,
    _position: WjVec2,
    _rotation: c_float,
    _scale: WjVec2,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual Transform2D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_transform2d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Get Transform2D position
#[no_mangle]
pub extern "C" fn wj_get_transform2d_position(entity: *mut WjEntity) -> WjVec2 {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjVec2 { x: 0.0, y: 0.0 };
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Get actual position
        WjVec2 { x: 0.0, y: 0.0 }
    });
    
    match result {
        Ok(pos) => pos,
        Err(e) => {
            set_last_error(format!("Panic in wj_get_transform2d_position: {:?}", e));
            WjVec2 { x: 0.0, y: 0.0 }
        }
    }
}

/// Set Transform2D position
#[no_mangle]
pub extern "C" fn wj_set_transform2d_position(
    entity: *mut WjEntity,
    position: WjVec2,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Set actual position
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_set_transform2d_position: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Add Transform3D component to entity
#[no_mangle]
pub extern "C" fn wj_add_transform3d(
    entity: *mut WjEntity,
    _position: WjVec3,
    _rotation: WjQuat,
    _scale: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual Transform3D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_transform3d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Get Transform3D position
#[no_mangle]
pub extern "C" fn wj_get_transform3d_position(entity: *mut WjEntity) -> WjVec3 {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjVec3 { x: 0.0, y: 0.0, z: 0.0 };
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Get actual position
        WjVec3 { x: 0.0, y: 0.0, z: 0.0 }
    });
    
    match result {
        Ok(pos) => pos,
        Err(e) => {
            set_last_error(format!("Panic in wj_get_transform3d_position: {:?}", e));
            WjVec3 { x: 0.0, y: 0.0, z: 0.0 }
        }
    }
}

/// Set Transform3D position
#[no_mangle]
pub extern "C" fn wj_set_transform3d_position(
    entity: *mut WjEntity,
    position: WjVec3,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Set actual position
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_set_transform3d_position: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Velocity Component
// ============================================================================

/// Add Velocity2D component to entity
#[no_mangle]
pub extern "C" fn wj_add_velocity2d(entity: *mut WjEntity, _velocity: WjVec2) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual Velocity2D component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_velocity2d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Get Velocity2D
#[no_mangle]
pub extern "C" fn wj_get_velocity2d(entity: *mut WjEntity) -> WjVec2 {
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
            set_last_error(format!("Panic in wj_get_velocity2d: {:?}", e));
            WjVec2 { x: 0.0, y: 0.0 }
        }
    }
}

/// Set Velocity2D
#[no_mangle]
pub extern "C" fn wj_set_velocity2d(entity: *mut WjEntity, velocity: WjVec2) -> WjErrorCode {
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
            set_last_error(format!("Panic in wj_set_velocity2d: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Name Component
// ============================================================================

/// Add Name component to entity
#[no_mangle]
pub extern "C" fn wj_add_name(entity: *mut WjEntity, name: *const c_char) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    if name.is_null() {
        set_last_error("Null name pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Add actual Name component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_add_name: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Get Name component
#[no_mangle]
pub extern "C" fn wj_get_name(entity: *mut WjEntity) -> *const c_char {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return ptr::null();
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Get actual name
        ptr::null()
    });
    
    match result {
        Ok(name) => name,
        Err(e) => {
            set_last_error(format!("Panic in wj_get_name: {:?}", e));
            ptr::null()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform2d_operations() {
        // These are placeholder tests that will work once we implement the actual ECS
        let pos = WjVec2 { x: 10.0, y: 20.0 };
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }
}

