//! Shader AST - Abstract Syntax Tree for .wjsl shader modules

use std::fmt;

/// A complete shader module (one .wjsl file)
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderModule {
    pub name: String,
    pub uniforms: Vec<UniformDecl>,
    pub storage: Vec<StorageDecl>,
    pub functions: Vec<Function>,
}

/// Uniform buffer declaration
#[derive(Debug, Clone, PartialEq)]
pub struct UniformDecl {
    pub name: String,
    pub ty: Type,
    pub binding: u32,
}

/// Storage buffer declaration
#[derive(Debug, Clone, PartialEq)]
pub struct StorageDecl {
    pub name: String,
    pub ty: Type,
    pub binding: u32,
    pub access: AccessMode,
}

/// Storage buffer access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
}

impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::ReadWrite
    }
}

/// Shader function (compute, vertex, fragment)
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub stage: ShaderStage,
    pub workgroup_size: Option<(u32, u32, u32)>,
    pub body: String, // Raw body for now; can be expanded to full AST later
}

/// Shader execution stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Compute,
    Vertex,
    Fragment,
}

/// Shader type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Scalar(ScalarType),
    Vec2(ScalarType),
    Vec3(ScalarType),
    Vec4(ScalarType),
    Mat4,
    Array(Box<Type>),
    Struct(String),
}

/// Scalar element type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarType {
    F32,
    F64,
    U32,
    I32,
    Bool,
}

impl fmt::Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScalarType::F32 => write!(f, "f32"),
            ScalarType::F64 => write!(f, "f64"),
            ScalarType::U32 => write!(f, "u32"),
            ScalarType::I32 => write!(f, "i32"),
            ScalarType::Bool => write!(f, "bool"),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Scalar(s) => write!(f, "{}", s),
            Type::Vec2(s) => write!(f, "Vec2<{}>", s),
            Type::Vec3(s) => write!(f, "Vec3<{}>", s),
            Type::Vec4(s) => write!(f, "Vec4<{}>", s),
            Type::Mat4 => write!(f, "Mat4"),
            Type::Array(inner) => write!(f, "array<{}>", inner),
            Type::Struct(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_type_display() {
        assert_eq!(format!("{}", ScalarType::F32), "f32");
        assert_eq!(format!("{}", ScalarType::U32), "u32");
    }

    #[test]
    fn test_type_display() {
        assert_eq!(format!("{}", Type::Vec2(ScalarType::F32)), "Vec2<f32>");
        assert_eq!(
            format!("{}", Type::Array(Box::new(Type::Vec4(ScalarType::F32)))),
            "array<Vec4<f32>>"
        );
    }

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::Vec2(ScalarType::F32), Type::Vec2(ScalarType::F32));
        assert_ne!(Type::Vec2(ScalarType::F32), Type::Vec2(ScalarType::F64));
    }
}
