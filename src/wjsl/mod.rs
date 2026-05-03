//! WJSL - Windjammer Shader Language (RFC syntax)
//!
//! WGSL-like shader DSL: @vertex, @fragment, @compute, @group, @binding, etc.
//! Transpiles .wjsl source to WGSL.
//!
//! Supports `#include "path.wjsl"` directives for sharing structs and functions
//! across shaders. Includes are resolved before parsing, with circular dependency
//! detection and deduplication.

mod ast;
mod codegen;
mod lexer;
pub mod parser;
mod type_checker;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub use ast::*;
pub use parser::parse_wjsl;
pub use parser::parse_wjsl_with_filename;
pub use type_checker::type_check_wjsl;

/// Transpile WJSL source to WGSL
pub fn transpile_wjsl(source: &str) -> Result<String, anyhow::Error> {
    let ast = parse_wjsl(source)?;
    type_checker::check(&ast, source)?;
    let wgsl = codegen::WjslCodegen::new(ast).generate()?;
    Ok(wgsl)
}

/// Transpile WJSL source to WGSL, resolving `#include` directives relative to `base_dir`.
pub fn transpile_wjsl_with_includes(
    source: &str,
    base_dir: &Path,
) -> Result<String, anyhow::Error> {
    let resolved = resolve_includes(source, base_dir, &mut Vec::new())?;
    transpile_wjsl(&resolved)
}

/// Resolve all `#include "path"` directives by inlining file contents.
///
/// - `include_stack` tracks the chain of files being processed for circular dependency detection.
/// - Files already included are skipped (deduplication via canonical path tracking).
pub fn resolve_includes(
    source: &str,
    base_dir: &Path,
    include_stack: &mut Vec<PathBuf>,
) -> Result<String, anyhow::Error> {
    let mut seen = HashSet::new();
    resolve_includes_inner(source, base_dir, include_stack, &mut seen)
}

fn resolve_includes_inner(
    source: &str,
    base_dir: &Path,
    include_stack: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) -> Result<String, anyhow::Error> {
    let mut output = String::with_capacity(source.len());

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(path_str) = parse_include_directive(trimmed) {
            let include_path = base_dir.join(path_str);
            let canonical = include_path
                .canonicalize()
                .unwrap_or_else(|_| include_path.clone());

            // Circular dependency detection
            if include_stack.contains(&canonical) {
                let chain: Vec<String> = include_stack
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect();
                return Err(anyhow::anyhow!(
                    "Circular #include detected: {} -> {} (chain: {})",
                    include_stack
                        .last()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    canonical.display(),
                    chain.join(" -> ")
                ));
            }

            // Deduplication: skip if already included
            if seen.contains(&canonical) {
                continue;
            }

            // Read the included file
            let content = std::fs::read_to_string(&include_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read #include \"{}\": {} (resolved to: {})",
                    path_str,
                    e,
                    include_path.display()
                )
            })?;

            seen.insert(canonical.clone());
            include_stack.push(canonical.clone());

            // Determine base directory for nested includes
            let nested_base = include_path.parent().unwrap_or(base_dir);
            let resolved = resolve_includes_inner(&content, nested_base, include_stack, seen)?;

            include_stack.pop();

            output.push_str(&resolved);
            output.push('\n');
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    Ok(output)
}

/// Parse a `use "path"` or `#include "path"` directive, returning the path if matched.
/// `use` is the preferred Windjammer-native syntax; `#include` is supported for compatibility.
fn parse_include_directive(line: &str) -> Option<&str> {
    let rest = if let Some(r) = line.strip_prefix("use ") {
        r
    } else {
        line.strip_prefix("#include")?
    };
    let rest = rest.trim();
    if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
        Some(&rest[1..rest.len() - 1])
    } else {
        None
    }
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
