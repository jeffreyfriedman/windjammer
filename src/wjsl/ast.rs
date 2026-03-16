//! WJSL AST - Abstract Syntax Tree for RFC-style .wjsl shader language
//!
//! WGSL-like syntax: @vertex, @fragment, @compute, @group, @binding, etc.

/// A complete WJSL shader module (one .wjsl file)
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderModule {
    /// Struct declarations (CameraUniforms, Material, etc.)
    pub structs: Vec<StructDecl>,
    /// Global bindings (uniforms, storage, textures, samplers)
    pub bindings: Vec<Binding>,
    /// Module-level private variables (var<private>)
    pub private_vars: Vec<PrivateVar>,
    /// Helper functions (non-entry-point)
    pub functions: Vec<Function>,
    /// Entry point functions (@vertex, @fragment, @compute)
    pub entry_points: Vec<EntryPoint>,
}

/// Private module-level variable (var<private>)
#[derive(Debug, Clone, PartialEq)]
pub struct PrivateVar {
    pub name: String,
    pub ty: Type,
}

/// Struct declaration with fields
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
}

/// Field in a struct
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    /// Optional @align(N) or @size(N) - for Phase 2
    pub align: Option<u32>,
    pub size: Option<u32>,
}

/// Global binding (uniform, storage, texture, sampler)
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub kind: BindingKind,
}

/// Kind of binding
#[derive(Debug, Clone, PartialEq)]
pub enum BindingKind {
    Uniform(Type),
    Storage {
        access_mode: StorageAccess,
        ty: Type,
    },
    Texture {
        texture_type: TextureType,
    },
    Sampler,
}

/// Storage buffer access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageAccess {
    Read,
    Write,
    ReadWrite,
}

/// Texture type (texture_2d<f32>, texture_cube<f32>, etc.)
#[derive(Debug, Clone, PartialEq)]
pub enum TextureType {
    Texture2D(ScalarType),
    TextureCube(ScalarType),
    Texture3D(ScalarType),
}

/// Shader execution stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

/// Entry point function (@vertex, @fragment, @compute)
#[derive(Debug, Clone, PartialEq)]
pub struct EntryPoint {
    pub stage: ShaderStage,
    pub workgroup_size: Option<(u32, u32, u32)>,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<ReturnType>,
    /// Raw function body (for Phase 1)
    pub body: String,
}

/// Helper function (non-entry-point)
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>, // None = void (no return type in WGSL)
    pub body: String,
}

/// Function parameter with optional attributes
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub location: Option<u32>,
    pub builtin: Option<String>,
}

/// Return type with optional @location or @builtin
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnType {
    pub ty: Type,
    pub location: Option<u32>,
    pub builtin: Option<String>,
}

/// WJSL type system (WGSL-compatible)
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Scalar(ScalarType),
    Vec2(Option<ScalarType>), // None = default f32
    Vec3(Option<ScalarType>),
    Vec4(Option<ScalarType>),
    Mat2x2(Option<ScalarType>),
    Mat3x3(Option<ScalarType>),
    Mat4x4(Option<ScalarType>),
    Array(Box<Type>, Option<u32>), // Optional fixed size: array<T, N>
    Atomic(ScalarType),
    Struct(String),
    Texture2D(ScalarType),
    TextureCube(ScalarType),
    Texture3D(ScalarType),
    Sampler,
    SamplerComparison,
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
