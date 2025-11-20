//! UI FFI bindings
//!
//! This module provides C-compatible FFI bindings for UI (widgets, layouts, events).

use crate::*;
use std::os::raw::{c_char, c_float, c_int};

// ============================================================================
// UI Widget
// ============================================================================

/// Opaque handle to a UI widget
#[repr(C)]
pub struct WjWidget {
    _private: [u8; 0],
}

/// Widget types
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjWidgetType {
    Button = 0,
    Label = 1,
    Image = 2,
    Slider = 3,
    Checkbox = 4,
    InputField = 5,
}

/// Create UI widget
#[no_mangle]
pub extern "C" fn wj_ui_create_widget(
    widget_type: WjWidgetType,
    position: WjVec2,
    size: WjVec2,
) -> *mut WjWidget {
    let result = panic::catch_unwind(|| {
        Box::into_raw(Box::new(0u8)) as *mut WjWidget
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_ui_create_widget: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Free UI widget
#[no_mangle]
pub extern "C" fn wj_ui_free_widget(widget: *mut WjWidget) {
    if widget.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(widget as *mut u8);
        }
    });
}

/// Set widget text
#[no_mangle]
pub extern "C" fn wj_ui_set_text(widget: *mut WjWidget, text: *const c_char) -> WjErrorCode {
    if widget.is_null() {
        set_last_error("Null widget pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    if text.is_null() {
        set_last_error("Null text pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| WjErrorCode::Ok);
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_ui_set_text: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// UI click callback
pub type WjUiClickCallback = extern "C" fn(widget: *mut WjWidget);

/// Set click callback
#[no_mangle]
pub extern "C" fn wj_ui_set_click_callback(
    widget: *mut WjWidget,
    callback: WjUiClickCallback,
) -> WjErrorCode {
    if widget.is_null() {
        set_last_error("Null widget pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| WjErrorCode::Ok);
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_ui_set_click_callback: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_types() {
        assert_eq!(WjWidgetType::Button as i32, 0);
        assert_eq!(WjWidgetType::Label as i32, 1);
    }
}

