//! Input FFI bindings
//!
//! This module provides C-compatible FFI bindings for the input system.

use crate::*;
use std::os::raw::{c_int};

// ============================================================================
// Key Codes
// ============================================================================

/// Key codes (subset of common keys)
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjKeyCode {
    // Letters
    A = 0, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers
    Key0 = 100, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    
    // Function keys
    F1 = 200, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Special keys
    Space = 300,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    
    // Arrow keys
    Left = 400,
    Right,
    Up,
    Down,
    
    // Modifiers
    LeftShift = 500,
    RightShift,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
}

/// Mouse buttons
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjMouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

// ============================================================================
// Keyboard Input
// ============================================================================

/// Check if a key is currently pressed
#[no_mangle]
pub extern "C" fn wj_input_is_key_down(key: WjKeyCode) -> bool {
    let result = panic::catch_unwind(|| {
        // TODO: Check actual key state
        false
    });
    
    match result {
        Ok(pressed) => pressed,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_is_key_down: {:?}", e));
            false
        }
    }
}

/// Check if a key was just pressed this frame
#[no_mangle]
pub extern "C" fn wj_input_is_key_pressed(key: WjKeyCode) -> bool {
    let result = panic::catch_unwind(|| {
        // TODO: Check actual key state
        false
    });
    
    match result {
        Ok(pressed) => pressed,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_is_key_pressed: {:?}", e));
            false
        }
    }
}

/// Check if a key was just released this frame
#[no_mangle]
pub extern "C" fn wj_input_is_key_released(key: WjKeyCode) -> bool {
    let result = panic::catch_unwind(|| {
        // TODO: Check actual key state
        false
    });
    
    match result {
        Ok(released) => released,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_is_key_released: {:?}", e));
            false
        }
    }
}

// ============================================================================
// Mouse Input
// ============================================================================

/// Check if a mouse button is currently pressed
#[no_mangle]
pub extern "C" fn wj_input_is_mouse_button_down(button: WjMouseButton) -> bool {
    let result = panic::catch_unwind(|| {
        // TODO: Check actual mouse button state
        false
    });
    
    match result {
        Ok(pressed) => pressed,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_is_mouse_button_down: {:?}", e));
            false
        }
    }
}

/// Check if a mouse button was just pressed this frame
#[no_mangle]
pub extern "C" fn wj_input_is_mouse_button_pressed(button: WjMouseButton) -> bool {
    let result = panic::catch_unwind(|| {
        // TODO: Check actual mouse button state
        false
    });
    
    match result {
        Ok(pressed) => pressed,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_is_mouse_button_pressed: {:?}", e));
            false
        }
    }
}

/// Get mouse position
#[no_mangle]
pub extern "C" fn wj_input_get_mouse_position() -> WjVec2 {
    let result = panic::catch_unwind(|| {
        // TODO: Get actual mouse position
        WjVec2 { x: 0.0, y: 0.0 }
    });
    
    match result {
        Ok(pos) => pos,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_get_mouse_position: {:?}", e));
            WjVec2 { x: 0.0, y: 0.0 }
        }
    }
}

/// Get mouse delta (movement since last frame)
#[no_mangle]
pub extern "C" fn wj_input_get_mouse_delta() -> WjVec2 {
    let result = panic::catch_unwind(|| {
        // TODO: Get actual mouse delta
        WjVec2 { x: 0.0, y: 0.0 }
    });
    
    match result {
        Ok(delta) => delta,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_get_mouse_delta: {:?}", e));
            WjVec2 { x: 0.0, y: 0.0 }
        }
    }
}

/// Get mouse scroll delta
#[no_mangle]
pub extern "C" fn wj_input_get_mouse_scroll() -> WjVec2 {
    let result = panic::catch_unwind(|| {
        // TODO: Get actual mouse scroll
        WjVec2 { x: 0.0, y: 0.0 }
    });
    
    match result {
        Ok(scroll) => scroll,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_get_mouse_scroll: {:?}", e));
            WjVec2 { x: 0.0, y: 0.0 }
        }
    }
}

// ============================================================================
// Gamepad Input
// ============================================================================

/// Gamepad buttons
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjGamepadButton {
    A = 0,
    B,
    X,
    Y,
    LeftBumper,
    RightBumper,
    Back,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Gamepad axes
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjGamepadAxis {
    LeftStickX = 0,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

/// Check if a gamepad button is pressed
#[no_mangle]
pub extern "C" fn wj_input_is_gamepad_button_down(
    _gamepad_id: c_int,
    _button: WjGamepadButton,
) -> bool {
    let result = panic::catch_unwind(|| {
        // TODO: Check actual gamepad button state
        false
    });
    
    match result {
        Ok(pressed) => pressed,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_is_gamepad_button_down: {:?}", e));
            false
        }
    }
}

/// Get gamepad axis value
#[no_mangle]
pub extern "C" fn wj_input_get_gamepad_axis(
    _gamepad_id: c_int,
    _axis: WjGamepadAxis,
) -> f32 {
    let result = panic::catch_unwind(|| {
        // TODO: Get actual gamepad axis value
        0.0
    });
    
    match result {
        Ok(value) => value,
        Err(e) => {
            set_last_error(format!("Panic in wj_input_get_gamepad_axis: {:?}", e));
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_code_values() {
        assert_eq!(WjKeyCode::A as i32, 0);
        assert_eq!(WjKeyCode::Space as i32, 300);
    }

    #[test]
    fn test_mouse_button_values() {
        assert_eq!(WjMouseButton::Left as i32, 0);
        assert_eq!(WjMouseButton::Right as i32, 1);
    }
}

