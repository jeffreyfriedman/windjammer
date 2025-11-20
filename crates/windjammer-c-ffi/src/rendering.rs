//! Rendering FFI bindings
//!
//! This module provides C-compatible FFI bindings for the rendering system.

use crate::*;
use std::os::raw::{c_char, c_float, c_int, c_uint};

// ============================================================================
// Sprite Rendering
// ============================================================================

/// Create a sprite
#[no_mangle]
pub extern "C" fn wj_sprite_new(
    entity: *mut WjEntity,
    _texture: *mut WjTexture,
    _position: WjVec2,
    _size: WjVec2,
    _color: WjColor,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Create actual sprite component
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_sprite_new: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Set sprite texture
#[no_mangle]
pub extern "C" fn wj_sprite_set_texture(
    entity: *mut WjEntity,
    texture: *mut WjTexture,
) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    if texture.is_null() {
        set_last_error("Null texture pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Set sprite texture
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_sprite_set_texture: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Set sprite color
#[no_mangle]
pub extern "C" fn wj_sprite_set_color(entity: *mut WjEntity, color: WjColor) -> WjErrorCode {
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Set sprite color
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_sprite_set_color: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Mesh Rendering (3D)
// ============================================================================

/// Create a cube mesh
#[no_mangle]
pub extern "C" fn wj_mesh_cube(_size: c_float) -> *mut WjMesh {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual cube mesh
        Box::into_raw(Box::new(0u8)) as *mut WjMesh
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_mesh_cube: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Create a sphere mesh
#[no_mangle]
pub extern "C" fn wj_mesh_sphere(_radius: c_float, _subdivisions: c_uint) -> *mut WjMesh {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual sphere mesh
        Box::into_raw(Box::new(0u8)) as *mut WjMesh
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_mesh_sphere: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Create a plane mesh
#[no_mangle]
pub extern "C" fn wj_mesh_plane(_size: c_float) -> *mut WjMesh {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual plane mesh
        Box::into_raw(Box::new(0u8)) as *mut WjMesh
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_mesh_plane: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Free a mesh
#[no_mangle]
pub extern "C" fn wj_mesh_free(mesh: *mut WjMesh) {
    if mesh.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(mesh as *mut u8);
        }
    });
}

// ============================================================================
// Texture Management
// ============================================================================

/// Load a texture from file
#[no_mangle]
pub extern "C" fn wj_texture_load(path: *const c_char) -> *mut WjTexture {
    if path.is_null() {
        set_last_error("Null path pointer".to_string());
        return ptr::null_mut();
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Load actual texture
        Box::into_raw(Box::new(0u8)) as *mut WjTexture
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_texture_load: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Create a texture from raw data
#[no_mangle]
pub extern "C" fn wj_texture_from_data(
    _width: c_uint,
    _height: c_uint,
    data: *const u8,
    _data_len: usize,
) -> *mut WjTexture {
    if data.is_null() {
        set_last_error("Null data pointer".to_string());
        return ptr::null_mut();
    }
    
    let result = panic::catch_unwind(|| {
        // TODO: Create texture from data
        Box::into_raw(Box::new(0u8)) as *mut WjTexture
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(e) => {
            set_last_error(format!("Panic in wj_texture_from_data: {:?}", e));
            ptr::null_mut()
        }
    }
}

/// Free a texture
#[no_mangle]
pub extern "C" fn wj_texture_free(texture: *mut WjTexture) {
    if texture.is_null() {
        return;
    }
    
    let _ = panic::catch_unwind(|| {
        unsafe {
            let _ = Box::from_raw(texture as *mut u8);
        }
    });
}

// ============================================================================
// Camera
// ============================================================================

/// Create a 2D camera
#[no_mangle]
pub extern "C" fn wj_camera2d_new(_position: WjVec2, _zoom: c_float) -> WjErrorCode {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual 2D camera
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_camera2d_new: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Create a 3D camera
#[no_mangle]
pub extern "C" fn wj_camera3d_new(
    _position: WjVec3,
    _look_at: WjVec3,
    _fov: c_float,
) -> WjErrorCode {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual 3D camera
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_camera3d_new: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Lighting
// ============================================================================

/// Create a point light
#[no_mangle]
pub extern "C" fn wj_point_light_new(
    _position: WjVec3,
    _color: WjColor,
    _intensity: c_float,
) -> WjErrorCode {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual point light
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_point_light_new: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

/// Create a directional light
#[no_mangle]
pub extern "C" fn wj_directional_light_new(
    _direction: WjVec3,
    _color: WjColor,
    _intensity: c_float,
) -> WjErrorCode {
    let result = panic::catch_unwind(|| {
        // TODO: Create actual directional light
        WjErrorCode::Ok
    });
    
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_directional_light_new: {:?}", e));
            WjErrorCode::Panic
        }
    }
}

// ============================================================================
// Material
// ============================================================================

/// Material properties
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WjMaterial {
    pub albedo: WjColor,
    pub metallic: c_float,
    pub roughness: c_float,
    pub emissive: WjColor,
}

/// Create a material
#[no_mangle]
pub extern "C" fn wj_material_new(
    albedo: WjColor,
    metallic: c_float,
    roughness: c_float,
) -> WjMaterial {
    WjMaterial {
        albedo,
        metallic,
        roughness,
        emissive: WjColor { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
    }
}

/// Set material emissive color
#[no_mangle]
pub extern "C" fn wj_material_set_emissive(material: *mut WjMaterial, emissive: WjColor) {
    if material.is_null() {
        return;
    }
    
    unsafe {
        (*material).emissive = emissive;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_creation() {
        let mat = wj_material_new(
            WjColor { r: 1.0, g: 0.0, b: 0.0, a: 1.0 },
            0.5,
            0.5,
        );
        assert_eq!(mat.albedo.r, 1.0);
        assert_eq!(mat.metallic, 0.5);
        assert_eq!(mat.roughness, 0.5);
    }
}

