//! # Windjammer C FFI Layer
//!
//! This crate provides C-compatible FFI bindings for the Windjammer Game Framework,
//! enabling multi-language SDK support (Python, JavaScript, C#, C++, Go, Java, etc.).
//!
//! ## Design Principles
//! 1. **C-Compatible**: All exported functions use C ABI
//! 2. **Opaque Pointers**: Internal types are hidden behind opaque pointers
//! 3. **Error Handling**: All functions return error codes
//! 4. **Memory Safety**: Proper ownership and lifetime management
//! 5. **Panic Safety**: All panics are caught at FFI boundary
//!
//! ## Usage
//! This library is not meant to be used directly. Instead, use one of the language SDKs:
//! - Python SDK (`windjammer-sdk`)
//! - JavaScript/TypeScript SDK (`@windjammer/sdk`)
//! - C# SDK (`Windjammer.SDK`)
//! - etc.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int, c_uint, c_void};
use std::panic;
use std::ptr;

// Re-export glam types for FFI
pub use glam::{Vec2, Vec3, Vec4, Quat, Mat4};

// Submodules
pub mod rendering;
pub mod components;
pub mod input;
pub mod physics;
pub mod audio;
pub mod world;
pub mod ai;
pub mod networking;
pub mod animation;
pub mod ui;

// ============================================================================
// Error Handling
// ============================================================================

/// Error codes returned by FFI functions
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WjErrorCode {
    /// Success
    Ok = 0,
    /// Null pointer passed where non-null expected
    NullPointer = 1,
    /// Invalid handle
    InvalidHandle = 2,
    /// Out of memory
    OutOfMemory = 3,
    /// Invalid argument
    InvalidArgument = 4,
    /// Operation failed
    OperationFailed = 5,
    /// Panic occurred
    Panic = 6,
}

/// Last error message (thread-local)
thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

/// Set the last error message
fn set_last_error(msg: String) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(msg));
}

/// Get the last error message
#[no_mangle]
pub extern "C" fn wj_get_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        if let Some(ref msg) = *e.borrow() {
            msg.as_ptr() as *const c_char
        } else {
            ptr::null()
        }
    })
}

/// Clear the last error
#[no_mangle]
pub extern "C" fn wj_clear_last_error() {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
}

// ============================================================================
// Opaque Handles
// ============================================================================

/// Opaque handle to the game engine
#[repr(C)]
pub struct WjEngine {
    _private: [u8; 0],
}

/// Opaque handle to a window
#[repr(C)]
pub struct WjWindow {
    _private: [u8; 0],
}

/// Opaque handle to an entity
#[repr(C)]
pub struct WjEntity {
    _private: [u8; 0],
}

/// Opaque handle to a world
#[repr(C)]
pub struct WjWorld {
    _private: [u8; 0],
}

/// Opaque handle to a texture
#[repr(C)]
pub struct WjTexture {
    _private: [u8; 0],
}

/// Opaque handle to a mesh
#[repr(C)]
pub struct WjMesh {
    _private: [u8; 0],
}

/// Opaque handle to an audio source
#[repr(C)]
pub struct WjAudioSource {
    _private: [u8; 0],
}

// ============================================================================
// Math Types (C-compatible)
// ============================================================================

/// 2D vector
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjVec2 {
    pub x: c_float,
    pub y: c_float,
}

impl From<Vec2> for WjVec2 {
    fn from(v: Vec2) -> Self {
        WjVec2 { x: v.x, y: v.y }
    }
}

impl From<WjVec2> for Vec2 {
    fn from(v: WjVec2) -> Self {
        Vec2::new(v.x, v.y)
    }
}

/// 3D vector
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjVec3 {
    pub x: c_float,
    pub y: c_float,
    pub z: c_float,
}

impl From<Vec3> for WjVec3 {
    fn from(v: Vec3) -> Self {
        WjVec3 { x: v.x, y: v.y, z: v.z }
    }
}

impl From<WjVec3> for Vec3 {
    fn from(v: WjVec3) -> Self {
        Vec3::new(v.x, v.y, v.z)
    }
}

/// 4D vector
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjVec4 {
    pub x: c_float,
    pub y: c_float,
    pub z: c_float,
    pub w: c_float,
}

impl From<Vec4> for WjVec4 {
    fn from(v: Vec4) -> Self {
        WjVec4 { x: v.x, y: v.y, z: v.z, w: v.w }
    }
}

impl From<WjVec4> for Vec4 {
    fn from(v: WjVec4) -> Self {
        Vec4::new(v.x, v.y, v.z, v.w)
    }
}

/// Quaternion
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjQuat {
    pub x: c_float,
    pub y: c_float,
    pub z: c_float,
    pub w: c_float,
}

impl From<Quat> for WjQuat {
    fn from(q: Quat) -> Self {
        WjQuat { x: q.x, y: q.y, z: q.z, w: q.w }
    }
}

impl From<WjQuat> for Quat {
    fn from(q: WjQuat) -> Self {
        Quat::from_xyzw(q.x, q.y, q.z, q.w)
    }
}

/// Color (RGBA)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjColor {
    pub r: c_float,
    pub g: c_float,
    pub b: c_float,
    pub a: c_float,
}

// ============================================================================
// Memory Management
// ============================================================================

/// Allocate memory
#[no_mangle]
pub extern "C" fn wj_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    
    let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
    unsafe { std::alloc::alloc(layout) as *mut c_void }
}

/// Free memory
#[no_mangle]
pub extern "C" fn wj_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    
    // Note: We don't know the original size, so we can't properly deallocate
    // In practice, language bindings should manage their own memory
    // This is here for completeness
}

// ============================================================================
// String Utilities
// ============================================================================

/// Create a new C string
#[no_mangle]
pub extern "C" fn wj_string_new(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    
    unsafe {
        let c_str = CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            if let Ok(c_string) = CString::new(rust_str) {
                return c_string.into_raw();
            }
        }
    }
    
    ptr::null_mut()
}

/// Free a C string
#[no_mangle]
pub extern "C" fn wj_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    
    unsafe {
        let _ = CString::from_raw(s);
    }
}

// ============================================================================
// Engine Management
// ============================================================================

/// Create a new engine instance
#[no_mangle]
pub extern "C" fn wj_engine_new() -> *mut WjEngine {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual engine instance
        // For now, return a dummy pointer
        Box::into_raw(Box::new(0u8)) as *mut WjEngine
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_engine_new: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Destroy an engine instance
#[no_mangle]
pub extern "C" fn wj_engine_free(engine: *mut WjEngine) {
    if engine.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            // TODO: Properly destroy engine
            let _ = Box::from_raw(engine as *mut u8);
        }
    });
}

/// Run the engine (blocking)
#[no_mangle]
pub extern "C" fn wj_engine_run(engine: *mut WjEngine) -> WjErrorCode {
    if engine.is_null() {
        set_last_error("Null engine pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Run actual game loop
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_engine_run: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Window Management
// ============================================================================

/// Create a new window
#[no_mangle]
pub extern "C" fn wj_window_new(
    title: *const c_char,
    _width: c_uint,
    _height: c_uint,
) -> *mut WjWindow {
    if title.is_null() {
        set_last_error("Null title pointer".to_string());
        return ptr::null_mut();
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Create actual window
        Box::into_raw(Box::new(0u8)) as *mut WjWindow
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_window_new: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Destroy a window
#[no_mangle]
pub extern "C" fn wj_window_free(window: *mut WjWindow) {
    if window.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(window as *mut u8);
        }
    });
}

// ============================================================================
// Entity Management
// ============================================================================

/// Create a new entity
#[no_mangle]
pub extern "C" fn wj_entity_new(world: *mut WjWorld) -> *mut WjEntity {
    if world.is_null() {
        set_last_error("Null world pointer".to_string());
        return ptr::null_mut();
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Create actual entity
        Box::into_raw(Box::new(0u64)) as *mut WjEntity
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_entity_new: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Destroy an entity
#[no_mangle]
pub extern "C" fn wj_entity_free(entity: *mut WjEntity) {
    if entity.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(entity as *mut u64);
        }
    });
}

// ============================================================================
// Math Functions
// ============================================================================

/// Create a Vec2
#[no_mangle]
pub extern "C" fn wj_vec2_new(x: c_float, y: c_float) -> WjVec2 {
    WjVec2 { x, y }
}

/// Create a Vec3
#[no_mangle]
pub extern "C" fn wj_vec3_new(x: c_float, y: c_float, z: c_float) -> WjVec3 {
    WjVec3 { x, y, z }
}

/// Create a Vec4
#[no_mangle]
pub extern "C" fn wj_vec4_new(x: c_float, y: c_float, z: c_float, w: c_float) -> WjVec4 {
    WjVec4 { x, y, z, w }
}

/// Create a Color
#[no_mangle]
pub extern "C" fn wj_color_new(r: c_float, g: c_float, b: c_float, a: c_float) -> WjColor {
    WjColor { r, g, b, a }
}

// ============================================================================
// Version Information
// ============================================================================

/// Get the library version
#[no_mangle]
pub extern "C" fn wj_version() -> *const c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const c_char
}

/// Get the library version as integers
#[no_mangle]
pub extern "C" fn wj_version_numbers(major: *mut c_int, minor: *mut c_int, patch: *mut c_int) {
    if !major.is_null() {
        unsafe { *major = 0 };
    }
    if !minor.is_null() {
        unsafe { *minor = 1 };
    }
    if !patch.is_null() {
        unsafe { *patch = 0 };
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_handling() {
        wj_clear_last_error();
        assert!(wj_get_last_error().is_null());
        
        set_last_error("Test error".to_string());
        let error_ptr = wj_get_last_error();
        assert!(!error_ptr.is_null());
        
        wj_clear_last_error();
        assert!(wj_get_last_error().is_null());
    }

    #[test]
    fn test_vec2() {
        let v = wj_vec2_new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_vec3() {
        let v = wj_vec3_new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_color() {
        let c = wj_color_new(1.0, 0.5, 0.25, 1.0);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.5);
        assert_eq!(c.b, 0.25);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn test_version() {
        let version_ptr = wj_version();
        assert!(!version_ptr.is_null());
        
        let mut major = 0;
        let mut minor = 0;
        let mut patch = 0;
        wj_version_numbers(&mut major, &mut minor, &mut patch);
        assert_eq!(major, 0);
        assert_eq!(minor, 1);
        assert_eq!(patch, 0);
    }
}

