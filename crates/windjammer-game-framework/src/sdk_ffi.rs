//! # C FFI Layer for Multi-Language Bindings
//!
//! Provides a C-compatible FFI (Foreign Function Interface) for the Windjammer Game Framework.
//! This is the foundation for all language bindings (Python, JavaScript, C#, etc.).
//!
//! ## Features
//! - C-compatible API surface
//! - Opaque pointer types for safety
//! - Error handling via return codes
//! - Memory management helpers
//! - String conversion utilities
//! - Vector/array helpers
//!
//! ## Safety
//! All FFI functions perform null pointer checks and handle panics gracefully.
//!
//! ## Example
//! ```c
//! // C code using the FFI
//! WjEngine* engine = wj_engine_new();
//! WjWindow* window = wj_window_create(engine, "My Game", 800, 600);
//! wj_engine_run(engine);
//! wj_engine_free(engine);
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

/// FFI error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WjErrorCode {
    /// Success
    Success = 0,
    /// Null pointer error
    NullPointer = 1,
    /// Invalid argument
    InvalidArgument = 2,
    /// Out of memory
    OutOfMemory = 3,
    /// Not found
    NotFound = 4,
    /// Already exists
    AlreadyExists = 5,
    /// IO error
    IoError = 6,
    /// Internal error
    InternalError = 7,
    /// Panic caught
    PanicCaught = 8,
}

/// FFI result type
pub type WjResult = WjErrorCode;

/// Opaque engine handle
#[repr(C)]
pub struct WjEngine {
    _private: [u8; 0],
}

/// Opaque window handle
#[repr(C)]
pub struct WjWindow {
    _private: [u8; 0],
}

/// Opaque entity handle
#[repr(C)]
pub struct WjEntity {
    _private: [u8; 0],
}

/// Opaque component handle
#[repr(C)]
pub struct WjComponent {
    _private: [u8; 0],
}

/// 2D vector (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WjVec2 {
    pub x: f32,
    pub y: f32,
}

/// 3D vector (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WjVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// 4D vector (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WjVec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// Color (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WjColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Rectangle (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WjRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Helper macro for FFI functions that catch panics and return error codes
macro_rules! ffi_catch_result {
    ($body:expr) => {
        match catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => WjErrorCode::PanicCaught,
        }
    };
}

/// Helper macro for FFI functions that catch panics and return pointers
macro_rules! ffi_catch_ptr {
    ($body:expr) => {
        match catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => ptr::null_mut(),
        }
    };
}

/// Helper macro for null pointer checks
macro_rules! check_null {
    ($ptr:expr) => {
        if $ptr.is_null() {
            return WjErrorCode::NullPointer;
        }
    };
}

// ============================================================================
// Engine Functions
// ============================================================================

/// Create a new engine instance
#[no_mangle]
pub extern "C" fn wj_engine_new() -> *mut WjEngine {
    ffi_catch_ptr!({
        let engine = Box::new(EngineImpl::new());
        Box::into_raw(engine) as *mut WjEngine
    })
}

/// Free an engine instance
#[no_mangle]
pub extern "C" fn wj_engine_free(engine: *mut WjEngine) -> WjResult {
    ffi_catch_result!({
        check_null!(engine);
        unsafe {
            let _ = Box::from_raw(engine as *mut EngineImpl);
        }
        WjErrorCode::Success
    })
}

/// Run the engine (blocking)
#[no_mangle]
pub extern "C" fn wj_engine_run(engine: *mut WjEngine) -> WjResult {
    ffi_catch_result!({
        check_null!(engine);
        unsafe {
            let engine = &mut *(engine as *mut EngineImpl);
            engine.run();
        }
        WjErrorCode::Success
    })
}

/// Update the engine (single frame)
#[no_mangle]
pub extern "C" fn wj_engine_update(engine: *mut WjEngine, delta_time: f32) -> WjResult {
    ffi_catch_result!({
        check_null!(engine);
        unsafe {
            let engine = &mut *(engine as *mut EngineImpl);
            engine.update(delta_time);
        }
        WjErrorCode::Success
    })
}

// ============================================================================
// Window Functions
// ============================================================================

/// Create a new window
#[no_mangle]
pub extern "C" fn wj_window_create(
    engine: *mut WjEngine,
    title: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut WjWindow {
    ffi_catch_ptr!({
        if engine.is_null() || title.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let title_str = match CStr::from_ptr(title).to_str() {
                Ok(s) => s,
                Err(_) => return ptr::null_mut(),
            };

            let engine = &mut *(engine as *mut EngineImpl);
            let window = Box::new(WindowImpl::new(engine, title_str, width as u32, height as u32));
            Box::into_raw(window) as *mut WjWindow
        }
    })
}

/// Free a window
#[no_mangle]
pub extern "C" fn wj_window_free(window: *mut WjWindow) -> WjResult {
    ffi_catch_result!({
        check_null!(window);
        unsafe {
            let _ = Box::from_raw(window as *mut WindowImpl);
        }
        WjErrorCode::Success
    })
}

/// Set window title
#[no_mangle]
pub extern "C" fn wj_window_set_title(window: *mut WjWindow, title: *const c_char) -> WjResult {
    ffi_catch_result!({
        check_null!(window);
        check_null!(title);

        unsafe {
            let title_str = match CStr::from_ptr(title).to_str() {
                Ok(s) => s,
                Err(_) => return WjErrorCode::InvalidArgument,
            };

            let window = &mut *(window as *mut WindowImpl);
            window.set_title(title_str);
        }
        WjErrorCode::Success
    })
}

/// Get window width
#[no_mangle]
pub extern "C" fn wj_window_get_width(window: *const WjWindow) -> c_int {
    match catch_unwind(AssertUnwindSafe(|| {
        if window.is_null() {
            return 0;
        }

        unsafe {
            let window = &*(window as *const WindowImpl);
            window.width() as c_int
        }
    })) {
        Ok(result) => result,
        Err(_) => 0,
    }
}

/// Get window height
#[no_mangle]
pub extern "C" fn wj_window_get_height(window: *const WjWindow) -> c_int {
    match catch_unwind(AssertUnwindSafe(|| {
        if window.is_null() {
            return 0;
        }

        unsafe {
            let window = &*(window as *const WindowImpl);
            window.height() as c_int
        }
    })) {
        Ok(result) => result,
        Err(_) => 0,
    }
}

// ============================================================================
// Entity Functions
// ============================================================================

/// Create a new entity
#[no_mangle]
pub extern "C" fn wj_entity_create(engine: *mut WjEngine) -> *mut WjEntity {
    ffi_catch_ptr!({
        if engine.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let engine = &mut *(engine as *mut EngineImpl);
            let entity = Box::new(EntityImpl::new(engine));
            Box::into_raw(entity) as *mut WjEntity
        }
    })
}

/// Free an entity
#[no_mangle]
pub extern "C" fn wj_entity_free(entity: *mut WjEntity) -> WjResult {
    ffi_catch_result!({
        check_null!(entity);
        unsafe {
            let _ = Box::from_raw(entity as *mut EntityImpl);
        }
        WjErrorCode::Success
    })
}

/// Get entity ID
#[no_mangle]
pub extern "C" fn wj_entity_get_id(entity: *const WjEntity) -> u64 {
    match catch_unwind(AssertUnwindSafe(|| {
        if entity.is_null() {
            return 0;
        }

        unsafe {
            let entity = &*(entity as *const EntityImpl);
            entity.id()
        }
    })) {
        Ok(result) => result,
        Err(_) => 0,
    }
}

// ============================================================================
// String Utilities
// ============================================================================

/// Allocate a C string (caller must free with wj_string_free)
#[no_mangle]
pub extern "C" fn wj_string_new(s: *const c_char) -> *mut c_char {
    ffi_catch_ptr!({
        if s.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let rust_str = match CStr::from_ptr(s).to_str() {
                Ok(s) => s,
                Err(_) => return ptr::null_mut(),
            };

            match CString::new(rust_str) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        }
    })
}

/// Free a C string allocated by wj_string_new
#[no_mangle]
pub extern "C" fn wj_string_free(s: *mut c_char) -> WjResult {
    ffi_catch_result!({
        check_null!(s);
        unsafe {
            let _ = CString::from_raw(s);
        }
        WjErrorCode::Success
    })
}

// ============================================================================
// Memory Utilities
// ============================================================================

/// Allocate memory (caller must free with wj_free)
#[no_mangle]
pub extern "C" fn wj_malloc(size: usize) -> *mut c_void {
    ffi_catch_ptr!({
        if size == 0 {
            return ptr::null_mut();
        }

        let layout = match std::alloc::Layout::from_size_align(size, std::mem::align_of::<u8>()) {
            Ok(layout) => layout,
            Err(_) => return ptr::null_mut(),
        };

        unsafe { std::alloc::alloc(layout) as *mut c_void }
    })
}

/// Free memory allocated by wj_malloc
#[no_mangle]
pub extern "C" fn wj_free(ptr: *mut c_void, size: usize) -> WjResult {
    ffi_catch_result!({
        check_null!(ptr);

        if size == 0 {
            return WjErrorCode::InvalidArgument;
        }

        let layout = match std::alloc::Layout::from_size_align(size, std::mem::align_of::<u8>()) {
            Ok(layout) => layout,
            Err(_) => return WjErrorCode::InvalidArgument,
        };

        unsafe {
            std::alloc::dealloc(ptr as *mut u8, layout);
        }

        WjErrorCode::Success
    })
}

// ============================================================================
// Vector Math Functions
// ============================================================================

/// Create a 2D vector
#[no_mangle]
pub extern "C" fn wj_vec2_new(x: f32, y: f32) -> WjVec2 {
    WjVec2 { x, y }
}

/// Add two 2D vectors
#[no_mangle]
pub extern "C" fn wj_vec2_add(a: WjVec2, b: WjVec2) -> WjVec2 {
    WjVec2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

/// Subtract two 2D vectors
#[no_mangle]
pub extern "C" fn wj_vec2_sub(a: WjVec2, b: WjVec2) -> WjVec2 {
    WjVec2 {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

/// Multiply a 2D vector by a scalar
#[no_mangle]
pub extern "C" fn wj_vec2_mul(v: WjVec2, scalar: f32) -> WjVec2 {
    WjVec2 {
        x: v.x * scalar,
        y: v.y * scalar,
    }
}

/// Get the length of a 2D vector
#[no_mangle]
pub extern "C" fn wj_vec2_length(v: WjVec2) -> f32 {
    (v.x * v.x + v.y * v.y).sqrt()
}

/// Normalize a 2D vector
#[no_mangle]
pub extern "C" fn wj_vec2_normalize(v: WjVec2) -> WjVec2 {
    let len = wj_vec2_length(v);
    if len > 0.0 {
        WjVec2 {
            x: v.x / len,
            y: v.y / len,
        }
    } else {
        v
    }
}

/// Create a 3D vector
#[no_mangle]
pub extern "C" fn wj_vec3_new(x: f32, y: f32, z: f32) -> WjVec3 {
    WjVec3 { x, y, z }
}

/// Add two 3D vectors
#[no_mangle]
pub extern "C" fn wj_vec3_add(a: WjVec3, b: WjVec3) -> WjVec3 {
    WjVec3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

/// Get the length of a 3D vector
#[no_mangle]
pub extern "C" fn wj_vec3_length(v: WjVec3) -> f32 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

// ============================================================================
// Internal Implementation Types (not exposed to FFI)
// ============================================================================

struct EngineImpl {
    running: bool,
}

impl EngineImpl {
    fn new() -> Self {
        Self { running: false }
    }

    fn run(&mut self) {
        self.running = true;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic here
    }
}

struct WindowImpl {
    title: String,
    width: u32,
    height: u32,
}

impl WindowImpl {
    fn new(_engine: &mut EngineImpl, title: &str, width: u32, height: u32) -> Self {
        Self {
            title: title.to_string(),
            width,
            height,
        }
    }

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }
}

struct EntityImpl {
    id: u64,
}

impl EntityImpl {
    fn new(_engine: &mut EngineImpl) -> Self {
        static mut NEXT_ID: u64 = 1;
        unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            Self { id }
        }
    }

    fn id(&self) -> u64 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(WjErrorCode::Success as i32, 0);
        assert_eq!(WjErrorCode::NullPointer as i32, 1);
    }

    #[test]
    fn test_vec2_new() {
        let v = wj_vec2_new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_vec2_add() {
        let a = wj_vec2_new(1.0, 2.0);
        let b = wj_vec2_new(3.0, 4.0);
        let c = wj_vec2_add(a, b);
        assert_eq!(c.x, 4.0);
        assert_eq!(c.y, 6.0);
    }

    #[test]
    fn test_vec2_sub() {
        let a = wj_vec2_new(5.0, 7.0);
        let b = wj_vec2_new(2.0, 3.0);
        let c = wj_vec2_sub(a, b);
        assert_eq!(c.x, 3.0);
        assert_eq!(c.y, 4.0);
    }

    #[test]
    fn test_vec2_mul() {
        let v = wj_vec2_new(2.0, 3.0);
        let result = wj_vec2_mul(v, 2.0);
        assert_eq!(result.x, 4.0);
        assert_eq!(result.y, 6.0);
    }

    #[test]
    fn test_vec2_length() {
        let v = wj_vec2_new(3.0, 4.0);
        let len = wj_vec2_length(v);
        assert!((len - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_vec2_normalize() {
        let v = wj_vec2_new(3.0, 4.0);
        let normalized = wj_vec2_normalize(v);
        let len = wj_vec2_length(normalized);
        assert!((len - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_vec3_new() {
        let v = wj_vec3_new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_vec3_add() {
        let a = wj_vec3_new(1.0, 2.0, 3.0);
        let b = wj_vec3_new(4.0, 5.0, 6.0);
        let c = wj_vec3_add(a, b);
        assert_eq!(c.x, 5.0);
        assert_eq!(c.y, 7.0);
        assert_eq!(c.z, 9.0);
    }

    #[test]
    fn test_vec3_length() {
        let v = wj_vec3_new(2.0, 3.0, 6.0);
        let len = wj_vec3_length(v);
        assert!((len - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_engine_lifecycle() {
        let engine = wj_engine_new();
        assert!(!engine.is_null());
        let result = wj_engine_free(engine);
        assert_eq!(result, WjErrorCode::Success);
    }

    #[test]
    fn test_engine_null_check() {
        let result = wj_engine_free(ptr::null_mut());
        assert_eq!(result, WjErrorCode::NullPointer);
    }

    #[test]
    fn test_engine_update() {
        let engine = wj_engine_new();
        let result = wj_engine_update(engine, 0.016);
        assert_eq!(result, WjErrorCode::Success);
        wj_engine_free(engine);
    }

    #[test]
    fn test_window_lifecycle() {
        let engine = wj_engine_new();
        let title = CString::new("Test Window").unwrap();
        let window = wj_window_create(engine, title.as_ptr(), 800, 600);
        assert!(!window.is_null());
        
        let width = wj_window_get_width(window);
        let height = wj_window_get_height(window);
        assert_eq!(width, 800);
        assert_eq!(height, 600);
        
        wj_window_free(window);
        wj_engine_free(engine);
    }

    #[test]
    fn test_window_set_title() {
        let engine = wj_engine_new();
        let title = CString::new("Test").unwrap();
        let window = wj_window_create(engine, title.as_ptr(), 800, 600);
        
        let new_title = CString::new("New Title").unwrap();
        let result = wj_window_set_title(window, new_title.as_ptr());
        assert_eq!(result, WjErrorCode::Success);
        
        wj_window_free(window);
        wj_engine_free(engine);
    }

    #[test]
    fn test_entity_lifecycle() {
        let engine = wj_engine_new();
        let entity = wj_entity_create(engine);
        assert!(!entity.is_null());
        
        let id = wj_entity_get_id(entity);
        assert!(id > 0);
        
        wj_entity_free(entity);
        wj_engine_free(engine);
    }

    #[test]
    fn test_entity_unique_ids() {
        let engine = wj_engine_new();
        let entity1 = wj_entity_create(engine);
        let entity2 = wj_entity_create(engine);
        
        let id1 = wj_entity_get_id(entity1);
        let id2 = wj_entity_get_id(entity2);
        assert_ne!(id1, id2);
        
        wj_entity_free(entity1);
        wj_entity_free(entity2);
        wj_engine_free(engine);
    }

    #[test]
    fn test_string_new_free() {
        let original = CString::new("Hello, World!").unwrap();
        let copied = wj_string_new(original.as_ptr());
        assert!(!copied.is_null());
        
        let result = wj_string_free(copied);
        assert_eq!(result, WjErrorCode::Success);
    }

    #[test]
    fn test_string_null_check() {
        let result = wj_string_free(ptr::null_mut());
        assert_eq!(result, WjErrorCode::NullPointer);
    }

    #[test]
    fn test_malloc_free() {
        let ptr = wj_malloc(1024);
        assert!(!ptr.is_null());
        
        let result = wj_free(ptr, 1024);
        assert_eq!(result, WjErrorCode::Success);
    }

    #[test]
    fn test_malloc_zero_size() {
        let ptr = wj_malloc(0);
        assert!(ptr.is_null());
    }

    #[test]
    fn test_free_null_check() {
        let result = wj_free(ptr::null_mut(), 1024);
        assert_eq!(result, WjErrorCode::NullPointer);
    }

    #[test]
    fn test_color_struct() {
        let color = WjColor {
            r: 1.0,
            g: 0.5,
            b: 0.0,
            a: 1.0,
        };
        assert_eq!(color.r, 1.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_rect_struct() {
        let rect = WjRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 50.0);
    }
}

