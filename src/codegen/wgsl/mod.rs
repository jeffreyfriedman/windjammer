//! WGSL (WebGPU Shading Language) code generation backend
//!
//! This module generates WGSL shader code from Windjammer AST.
//! WGSL is a domain-specific shading language for GPU compute and graphics.
//!
//! ## Design Principles
//!
//! 1. **Type Safety**: CPU/GPU struct layouts verified at compile time
//! 2. **Automatic Alignment**: Insert padding to match WGSL alignment rules
//! 3. **GPU-Optimized**: Generate efficient GPU code
//! 4. **Developer-Friendly**: Clear error messages for GPU limitations
//!
//! ## Type Mapping
//!
//! | Windjammer | WGSL |
//! |------------|------|
//! | `u32` | `u32` |
//! | `i32` | `i32` |
//! | `f32` | `f32` |
//! | `bool` | `bool` |
//! | `vec2<f32>` | `vec2<f32>` |
//! | `vec3<f32>` | `vec3<f32>` |
//! | `vec4<f32>` | `vec4<f32>` |
//! | `mat4x4<f32>` | `mat4x4<f32>` |
//!
//! ## GPU-Specific Attributes
//!
//! - `@compute(workgroup_size = [x, y, z])` - Compute shader entry point
//! - `@vertex` - Vertex shader entry point
//! - `@fragment` - Fragment shader entry point
//! - `@binding(n)` - Bind group binding
//! - `@uniform` - Uniform buffer
//! - `@storage(access)` - Storage buffer (read/write)
//!
//! ## Limitations (GPU constraints)
//!
//! - No recursion (GPU limitation)
//! - No dynamic dispatch (GPU limitation)
//! - Fixed-size arrays only
//! - Bounded loops only

mod codegen;
mod types;
mod validation;
mod structs;
pub mod shader_metadata;

pub use codegen::WgslBackend;
pub use types::{WgslType, map_type_to_wgsl};
pub use validation::validate_for_gpu;
pub use structs::{StructLayout, LayoutField};
