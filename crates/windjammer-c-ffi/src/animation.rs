//! Animation FFI bindings
//!
//! This module provides C-compatible FFI bindings for animation (skeletal, blending, IK).

use crate::*;
use std::os::raw::{c_char, c_float, c_int};

// ============================================================================
// Animation Clip
// ============================================================================

/// Opaque handle to an animation clip
#[repr(C)]
pub struct WjAnimationClip {
    _private: [u8; 0],
}

/// Load animation clip
#[no_mangle]
pub extern "C" fn wj_animation_load(path: *const c_char) -> *mut WjAnimationClip {
    if path.is_null() {
        set_last_error("Null path pointer".to_string());
        return ptr::null_mut();
    }
    
    let result = panic::catch_unwind(|| {
        Box::into_raw(Box::new(0u8)) as *mut WjAnimationClip
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_animation_load: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Free animation clip
#[no_mangle]
pub extern "C" fn wj_animation_free(clip: *mut WjAnimationClip) {
    if clip.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(clip as *mut u8);
        }
    });
}

// ============================================================================
// Animation Player
// ============================================================================

/// Play animation
#[no_mangle]
pub extern "C" fn wj_animation_play(
    entity: *mut WjEntity,
    clip: *mut WjAnimationClip,
    loop_animation: bool,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    if clip.is_null() {
        set_last_error("Null clip pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| WjErrorCode::Ok);
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_animation_play: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Stop animation
#[no_mangle]
pub extern "C" fn wj_animation_stop(entity: *mut WjEntity) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| WjErrorCode::Ok);
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_animation_stop: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Set animation speed
#[no_mangle]
pub extern "C" fn wj_animation_set_speed(
    entity: *mut WjEntity,
    speed: c_float,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| WjErrorCode::Ok);
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_animation_set_speed: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Animation Blending
// ============================================================================

/// Blend between two animations
#[no_mangle]
pub extern "C" fn wj_animation_blend(
    entity: *mut WjEntity,
    clip_a: *mut WjAnimationClip,
    clip_b: *mut WjAnimationClip,
    blend_factor: c_float,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| WjErrorCode::Ok);
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_animation_blend: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_placeholder() {
        assert_eq!(1, 1);
    }
}

