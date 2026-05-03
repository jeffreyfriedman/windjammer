//! Windjammer Shader Language (.wjsl)
//!
//! Compile-time type-checked shader format that transpiles to WGSL.
//! Catches host/shader type mismatches at compile time (not runtime).

mod ast;
mod parser;
mod type_checker;
mod wgsl_codegen;

pub use ast::{AccessMode, ScalarType, ShaderModule, StorageDecl, Type, UniformDecl};
pub use parser::parse_shader;
pub use type_checker::{TypeChecker, TypeError};
pub use wgsl_codegen::generate_wgsl;

/// Compile a .wjsl source string to WGSL with type checking.
pub fn compile_shader(source: &str) -> Result<String, anyhow::Error> {
    let module = parse_shader(source)?;
    let checker = TypeChecker::new();
    checker.check(&module)?;
    Ok(generate_wgsl(&module))
}
