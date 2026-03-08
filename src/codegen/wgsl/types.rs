//! WGSL type mapping and alignment calculation
//!
//! WGSL has strict alignment rules that differ from Rust:
//! - vec2: 8-byte alignment
//! - vec3: 16-byte alignment (!)
//! - vec4: 16-byte alignment
//! - mat4x4: 16-byte alignment
//!
//! The compiler must automatically insert padding to match GPU requirements.

use crate::parser::Type;
use anyhow::{Result, bail};

/// WGSL type representation
#[derive(Debug, Clone, PartialEq)]
pub enum WgslType {
    // Scalar types
    U32,
    I32,
    F32,
    Bool,
    
    // Vector types
    Vec2F32,
    Vec3F32,
    Vec4F32,
    Vec2U32,
    Vec3U32,
    Vec4U32,
    Vec2I32,
    Vec3I32,
    Vec4I32,
    
    // Matrix types
    Mat2x2F32,
    Mat3x3F32,
    Mat4x4F32,
    
    // Complex types
    Array(Box<WgslType>, Option<usize>), // element type, optional size
    Struct(String), // struct name
    
    // Texture types
    Texture2D(Box<WgslType>), // element type (f32, u32, i32)
    Texture3D(Box<WgslType>),
    TextureCube(Box<WgslType>),
    Sampler,
}

impl WgslType {
    /// Convert type for uniform buffer (u32 → f32)
    /// 
    /// CRITICAL: WebGPU uniform buffers prefer f32 over u32.
    /// This prevents the black screen bug where host sends f32,
    /// but shader expects u32, causing garbage values!
    pub fn to_uniform_safe_type(&self) -> WgslType {
        match self {
            // Convert u32 → f32 in uniforms
            WgslType::U32 => WgslType::F32,
            WgslType::Vec2U32 => WgslType::Vec2F32,
            WgslType::Vec3U32 => WgslType::Vec3F32,
            WgslType::Vec4U32 => WgslType::Vec4F32,
            
            // i32 is OK in uniforms (but rare)
            // f32 types are already correct
            // Everything else stays the same
            _ => self.clone(),
        }
    }
    
    /// Get the WGSL type string
    pub fn to_wgsl_string(&self) -> String {
        match self {
            WgslType::U32 => "u32".to_string(),
            WgslType::I32 => "i32".to_string(),
            WgslType::F32 => "f32".to_string(),
            WgslType::Bool => "bool".to_string(),
            
            WgslType::Vec2F32 => "vec2<f32>".to_string(),
            WgslType::Vec3F32 => "vec3<f32>".to_string(),
            WgslType::Vec4F32 => "vec4<f32>".to_string(),
            WgslType::Vec2U32 => "vec2<u32>".to_string(),
            WgslType::Vec3U32 => "vec3<u32>".to_string(),
            WgslType::Vec4U32 => "vec4<u32>".to_string(),
            WgslType::Vec2I32 => "vec2<i32>".to_string(),
            WgslType::Vec3I32 => "vec3<i32>".to_string(),
            WgslType::Vec4I32 => "vec4<i32>".to_string(),
            
            WgslType::Mat2x2F32 => "mat2x2<f32>".to_string(),
            WgslType::Mat3x3F32 => "mat3x3<f32>".to_string(),
            WgslType::Mat4x4F32 => "mat4x4<f32>".to_string(),
            
            WgslType::Array(elem_type, Some(size)) => {
                format!("array<{}, {}>", elem_type.to_wgsl_string(), size)
            }
            WgslType::Array(elem_type, None) => {
                format!("array<{}>", elem_type.to_wgsl_string())
            }
            WgslType::Struct(name) => name.clone(),
            
            WgslType::Texture2D(elem) => format!("texture_2d<{}>", elem.to_wgsl_string()),
            WgslType::Texture3D(elem) => format!("texture_3d<{}>", elem.to_wgsl_string()),
            WgslType::TextureCube(elem) => format!("texture_cube<{}>", elem.to_wgsl_string()),
            WgslType::Sampler => "sampler".to_string(),
        }
    }
    
    /// Get the size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            WgslType::U32 | WgslType::I32 | WgslType::F32 => 4,
            WgslType::Bool => 4, // WGSL bools are 4 bytes
            
            WgslType::Vec2F32 | WgslType::Vec2U32 | WgslType::Vec2I32 => 8,
            WgslType::Vec3F32 | WgslType::Vec3U32 | WgslType::Vec3I32 => 12,
            WgslType::Vec4F32 | WgslType::Vec4U32 | WgslType::Vec4I32 => 16,
            
            WgslType::Mat2x2F32 => 16,
            WgslType::Mat3x3F32 => 48,
            WgslType::Mat4x4F32 => 64,
            
            WgslType::Array(elem_type, Some(size)) => {
                elem_type.size_bytes() * size
            }
            WgslType::Array(_, None) => 0, // Runtime-sized arrays
            WgslType::Struct(_) => 0, // Need struct registry to compute
            
            WgslType::Texture2D(_) | WgslType::Texture3D(_) | WgslType::TextureCube(_) | WgslType::Sampler => {
                0 // Textures/samplers are opaque handles
            }
        }
    }
    
    /// Get the alignment in bytes
    /// CRITICAL: WGSL has different alignment rules than Rust!
    pub fn alignment_bytes(&self) -> usize {
        match self {
            WgslType::U32 | WgslType::I32 | WgslType::F32 | WgslType::Bool => 4,
            
            WgslType::Vec2F32 | WgslType::Vec2U32 | WgslType::Vec2I32 => 8,
            
            // CRITICAL: vec3 has 16-byte alignment in WGSL!
            WgslType::Vec3F32 | WgslType::Vec3U32 | WgslType::Vec3I32 => 16,
            
            WgslType::Vec4F32 | WgslType::Vec4U32 | WgslType::Vec4I32 => 16,
            
            WgslType::Mat2x2F32 => 8,
            WgslType::Mat3x3F32 => 16,
            WgslType::Mat4x4F32 => 16,
            
            WgslType::Array(elem_type, _) => elem_type.alignment_bytes(),
            WgslType::Struct(_) => 16, // Struct alignment is 16 bytes (largest alignment)
            
            WgslType::Texture2D(_) | WgslType::Texture3D(_) | WgslType::TextureCube(_) | WgslType::Sampler => {
                1 // Textures/samplers don't have alignment requirements (opaque handles)
            }
        }
    }
}

/// Map Windjammer type to WGSL type
pub fn map_type_to_wgsl(wj_type: &Type) -> Result<WgslType> {
    match wj_type {
        Type::Uint => Ok(WgslType::U32),
        Type::Int => Ok(WgslType::I32),   // int -> i32
        Type::Int32 => Ok(WgslType::I32),
        Type::Float => Ok(WgslType::F32),
        Type::Bool => Ok(WgslType::Bool),
        
        Type::Parameterized(name, type_params) if name == "vec2" => {
            if let Some(Type::Float) = type_params.get(0) {
                Ok(WgslType::Vec2F32)
            } else if let Some(Type::Uint) = type_params.get(0) {
                Ok(WgslType::Vec2U32)
            } else if let Some(Type::Int32) = type_params.get(0) {
                Ok(WgslType::Vec2I32)
            } else {
                bail!("Unsupported vec2 element type: {:?}", type_params)
            }
        }
        
        Type::Parameterized(name, type_params) if name == "vec3" => {
            if let Some(Type::Float) = type_params.get(0) {
                Ok(WgslType::Vec3F32)
            } else if let Some(Type::Uint) = type_params.get(0) {
                Ok(WgslType::Vec3U32)
            } else if let Some(Type::Int32) = type_params.get(0) {
                Ok(WgslType::Vec3I32)
            } else {
                bail!("Unsupported vec3 element type: {:?}", type_params)
            }
        }
        
        Type::Parameterized(name, type_params) if name == "vec4" => {
            if let Some(Type::Float) = type_params.get(0) {
                Ok(WgslType::Vec4F32)
            } else if let Some(Type::Uint) = type_params.get(0) {
                Ok(WgslType::Vec4U32)
            } else if let Some(Type::Int32) = type_params.get(0) {
                Ok(WgslType::Vec4I32)
            } else {
                bail!("Unsupported vec4 element type: {:?}", type_params)
            }
        }
        
        Type::Parameterized(name, type_params) if name == "mat4x4" => {
            if let Some(Type::Float) = type_params.get(0) {
                Ok(WgslType::Mat4x4F32)
            } else {
                bail!("Unsupported mat4x4 element type: {:?}", type_params)
            }
        }
        
        Type::Parameterized(name, args) => {
            match name.as_str() {
                "array" => {
                    // Unbounded array: array<T>
                    let elem_type = args.first()
                        .ok_or_else(|| anyhow::anyhow!("array requires element type"))?;
                    let wgsl_elem_type = map_type_to_wgsl(elem_type)?;
                    Ok(WgslType::Array(Box::new(wgsl_elem_type), None))
                }
                "texture_2d" => {
                    let elem_type = args.first()
                        .ok_or_else(|| anyhow::anyhow!("texture_2d requires element type"))?;
                    let wgsl_elem_type = map_type_to_wgsl(elem_type)?;
                    Ok(WgslType::Texture2D(Box::new(wgsl_elem_type)))
                }
                "texture_3d" => {
                    let elem_type = args.first()
                        .ok_or_else(|| anyhow::anyhow!("texture_3d requires element type"))?;
                    let wgsl_elem_type = map_type_to_wgsl(elem_type)?;
                    Ok(WgslType::Texture3D(Box::new(wgsl_elem_type)))
                }
                "texture_cube" => {
                    let elem_type = args.first()
                        .ok_or_else(|| anyhow::anyhow!("texture_cube requires element type"))?;
                    let wgsl_elem_type = map_type_to_wgsl(elem_type)?;
                    Ok(WgslType::TextureCube(Box::new(wgsl_elem_type)))
                }
                _ => bail!("Type {} not supported in WGSL backend", name),
            }
        }
        
        Type::Array(elem_type, size) => {
            let wgsl_elem_type = map_type_to_wgsl(elem_type)?;
            Ok(WgslType::Array(Box::new(wgsl_elem_type), Some(*size)))
        }
        
        Type::Custom(name) if name == "sampler" => Ok(WgslType::Sampler),
        Type::Custom(name) => Ok(WgslType::Struct(name.clone())),

        _ => bail!("Type {:?} not supported in WGSL backend", wj_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgsl_type_strings() {
        assert_eq!(WgslType::U32.to_wgsl_string(), "u32");
        assert_eq!(WgslType::Vec3F32.to_wgsl_string(), "vec3<f32>");
        assert_eq!(WgslType::Mat4x4F32.to_wgsl_string(), "mat4x4<f32>");
    }

    #[test]
    fn test_wgsl_type_sizes() {
        assert_eq!(WgslType::U32.size_bytes(), 4);
        assert_eq!(WgslType::Vec2F32.size_bytes(), 8);
        assert_eq!(WgslType::Vec3F32.size_bytes(), 12);
        assert_eq!(WgslType::Vec4F32.size_bytes(), 16);
        assert_eq!(WgslType::Mat4x4F32.size_bytes(), 64);
    }

    #[test]
    fn test_wgsl_alignment() {
        assert_eq!(WgslType::U32.alignment_bytes(), 4);
        assert_eq!(WgslType::Vec2F32.alignment_bytes(), 8);
        
        // CRITICAL: vec3 has 16-byte alignment!
        assert_eq!(WgslType::Vec3F32.alignment_bytes(), 16);
        
        assert_eq!(WgslType::Vec4F32.alignment_bytes(), 16);
        assert_eq!(WgslType::Mat4x4F32.alignment_bytes(), 16);
    }
}
