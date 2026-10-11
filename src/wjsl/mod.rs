//! WJSL - Windjammer Shader Language (RFC syntax)
//!
//! WGSL-like shader DSL: @vertex, @fragment, @compute, @group, @binding, etc.
//! Transpiles .wjsl source to WGSL.
//!
//! Supports `#include "path.wjsl"` directives for sharing structs and functions
//! across shaders. Includes are resolved before parsing, with circular dependency
//! detection and deduplication.

mod ast;
mod block_syntax;
mod codegen;
mod diagnostics;
mod expression_type_checking;
mod includes;
mod lexer;
pub mod parser;
mod shader_type_rules;
mod statement_type_checking;
mod type_checker;
pub mod wgsl_printer;

use std::path::Path;

pub use ast::*;
pub use parser::parse_wjsl;
pub use parser::parse_wjsl_with_filename;
pub use diagnostics::ShaderDiagnostics;
pub use includes::resolve_includes;
pub use type_checker::type_check_wjsl;
pub use block_syntax::{
    check_block_shader, check_uniform_match, compile_legacy_shader, emit_block_shader,
    parse_block_shader, BlockShader,
};

/// Parse WJSL source into an AST (stage 1 of the pipeline).
pub fn parse(source: &str) -> anyhow::Result<ShaderModule> {
    parse_wjsl(source).map_err(|e| ShaderDiagnostics::new(source).enrich(e))
}

/// Type-check a parsed module (stage 2). `source` is used for diagnostics only.
pub fn check(module: &ShaderModule, source: &str) -> anyhow::Result<()> {
    type_checker::check(module, source).map_err(|e| ShaderDiagnostics::new(source).enrich(e))
}

/// Print a checked module as WGSL (stage 3, the single WGSL emitter).
pub fn emit_wgsl(module: ShaderModule, source: &str) -> anyhow::Result<String> {
    codegen::WjslCodegen::new(module)
        .generate()
        .map_err(|e| ShaderDiagnostics::new(source).enrich(e))
}

/// Transpile WJSL source to WGSL: `parse` -> `check` -> `emit_wgsl`.
pub fn transpile_wjsl(source: &str) -> Result<String, anyhow::Error> {
    let ast = parse(source)?;
    check(&ast, source)?;
    emit_wgsl(ast, source)
}

/// Transpile WJSL source to WGSL, resolving `#include` directives relative to `base_dir`.
pub fn transpile_wjsl_with_includes(
    source: &str,
    base_dir: &Path,
) -> Result<String, anyhow::Error> {
    let resolved = resolve_includes(source, base_dir, &mut Vec::new())
        .map_err(|e| ShaderDiagnostics::new(source).enrich(e))?;
    transpile_wjsl(&resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_array_with_size() {
        let source = "struct Data { values: array<f32, 16> }";
        let ast = parse_wjsl(source).unwrap();
        let field = &ast.structs[0].fields[0];
        match &field.ty {
            Type::Array(elem, size) => {
                assert_eq!(*size, Some(16));
                assert!(matches!(**elem, Type::Scalar(ScalarType::F32)));
            }
            _ => panic!("Expected array<f32, 16>"),
        }
    }

    #[test]
    fn test_array_indexing_in_body() {
        let source = r#"
@group(0) @binding(0) storage read clusters: array<vec4>;
@group(0) @binding(1) storage read_write instances: array<u32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_id = id.x;
    let cluster = clusters[cluster_id];
    instances[cluster_id] = 1u;
}
"#;
        let wgsl = transpile_wjsl(source).unwrap();
        assert!(wgsl.contains("clusters[cluster_id]"));
        assert!(wgsl.contains("instances[cluster_id]"));
    }
}
