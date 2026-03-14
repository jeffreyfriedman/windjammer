//! Type checker for .wjsl shader modules
//!
//! Validates host/shader type consistency at compile time.

use crate::shader::ast::{ScalarType, ShaderModule, Type};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TypeError {
    #[error("Uniform '{0}' not found in shader")]
    UniformNotFound(String),

    #[error("Storage '{0}' not found in shader")]
    StorageNotFound(String),

    #[error("Type mismatch at {location}: expected {expected}, found {found}")]
    TypeMismatch {
        expected: Type,
        found: Type,
        location: String,
    },

    #[error("WGSL does not support f64 - use f32 for uniforms")]
    F64NotSupported(String),

    #[error("Struct '{0}' not registered - cannot validate")]
    UnknownStruct(String),
}

/// Type checker for shader/host interface validation
pub struct TypeChecker {
    host_types: HashMap<String, Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            host_types: HashMap::new(),
        }
    }

    /// Register a host-side type for validation
    pub fn register_host_type(&mut self, name: &str, ty: Type) {
        self.host_types.insert(name.to_string(), ty);
    }

    /// Check the entire shader module
    pub fn check(&self, shader: &ShaderModule) -> Result<(), TypeError> {
        for uniform in &shader.uniforms {
            if uniform.ty == Type::Vec2(ScalarType::F64)
                || uniform.ty == Type::Vec3(ScalarType::F64)
                || uniform.ty == Type::Vec4(ScalarType::F64)
            {
                return Err(TypeError::F64NotSupported(format!(
                    "uniform {}",
                    uniform.name
                )));
            }
        }
        Ok(())
    }

    /// Check that a host variable's type matches the shader's uniform declaration
    pub fn check_uniform_match(
        &self,
        shader: &ShaderModule,
        host_var: &str,
        host_type: &Type,
    ) -> Result<(), TypeError> {
        let uniform = shader
            .uniforms
            .iter()
            .find(|u| u.name == host_var)
            .ok_or_else(|| TypeError::UniformNotFound(host_var.to_string()))?;

        if uniform.ty != *host_type {
            return Err(TypeError::TypeMismatch {
                expected: uniform.ty.clone(),
                found: host_type.clone(),
                location: format!("uniform {}", host_var),
            });
        }

        // WGSL doesn't support f64
        if matches!(host_type, Type::Vec2(ScalarType::F64) | Type::Vec3(ScalarType::F64) | Type::Vec4(ScalarType::F64)) {
            return Err(TypeError::F64NotSupported(format!("uniform {}", host_var)));
        }

        Ok(())
    }

    /// Check that a host variable's type matches the shader's storage declaration
    pub fn check_storage_match(
        &self,
        shader: &ShaderModule,
        host_var: &str,
        host_type: &Type,
    ) -> Result<(), TypeError> {
        let storage = shader
            .storage
            .iter()
            .find(|s| s.name == host_var)
            .ok_or_else(|| TypeError::StorageNotFound(host_var.to_string()))?;

        if storage.ty != *host_type {
            return Err(TypeError::TypeMismatch {
                expected: storage.ty.clone(),
                found: host_type.clone(),
                location: format!("storage {}", host_var),
            });
        }

        Ok(())
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader::parse_shader;

    #[test]
    fn test_type_checker_detects_mismatch() {
        let source = "shader S { uniform x: Vec2<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        // Should fail: Vec2<f64> != Vec2<f32>
        let result =
            checker.check_uniform_match(&shader, "x", &Type::Vec2(ScalarType::F64));
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_accepts_match() {
        let source = "shader S { uniform x: Vec2<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result =
            checker.check_uniform_match(&shader, "x", &Type::Vec2(ScalarType::F32));
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_checker_uniform_not_found() {
        let source = "shader S { uniform x: Vec2<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result =
            checker.check_uniform_match(&shader, "y", &Type::Vec2(ScalarType::F32));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TypeError::UniformNotFound(_)));
    }

    #[test]
    fn test_type_checker_vec3_mismatch() {
        let source = "shader S { uniform pos: Vec3<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result =
            checker.check_uniform_match(&shader, "pos", &Type::Vec3(ScalarType::U32));
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_vec4_mismatch() {
        let source = "shader S { uniform color: Vec4<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result =
            checker.check_uniform_match(&shader, "color", &Type::Vec2(ScalarType::F32));
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_mat4_match() {
        let source = "shader S { uniform m: Mat4 }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check_uniform_match(&shader, "m", &Type::Mat4);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_checker_f64_rejected() {
        let source = "shader S { uniform x: Vec2<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result =
            checker.check_uniform_match(&shader, "x", &Type::Vec2(ScalarType::F64));
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_storage_match() {
        let source = "shader S { storage out: array<Vec4<f32>> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check_storage_match(
            &shader,
            "out",
            &Type::Array(Box::new(Type::Vec4(ScalarType::F32))),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_checker_storage_mismatch() {
        let source = "shader S { storage out: array<Vec4<f32>> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check_storage_match(
            &shader,
            "out",
            &Type::Array(Box::new(Type::Vec2(ScalarType::F32))),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_storage_not_found() {
        let source = "shader S { storage out: array<Vec4<f32>> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check_storage_match(
            &shader,
            "other",
            &Type::Array(Box::new(Type::Vec4(ScalarType::F32))),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TypeError::StorageNotFound(_)));
    }

    #[test]
    fn test_type_checker_u32_match() {
        let source = "shader S { uniform count: Vec2<u32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result =
            checker.check_uniform_match(&shader, "count", &Type::Vec2(ScalarType::U32));
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_checker_scalar_mismatch() {
        let source = "shader S { uniform x: Vec2<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result =
            checker.check_uniform_match(&shader, "x", &Type::Scalar(ScalarType::F32));
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_array_element_mismatch() {
        let source = "shader S { storage out: array<Vec4<f32>> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check_storage_match(
            &shader,
            "out",
            &Type::Array(Box::new(Type::Vec4(ScalarType::U32))),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_check_rejects_f64_uniform() {
        let source = "shader S { uniform x: Vec2<f64> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check(&shader);
        assert!(result.is_err());
    }

    #[test]
    fn test_type_checker_check_accepts_f32_uniform() {
        let source = "shader S { uniform x: Vec2<f32> }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check(&shader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_checker_struct_match() {
        let source = "shader S { uniform cam: CameraData }";
        let shader = parse_shader(source).unwrap();
        let checker = TypeChecker::new();

        let result = checker.check_uniform_match(
            &shader,
            "cam",
            &Type::Struct("CameraData".to_string()),
        );
        assert!(result.is_ok());
    }
}
