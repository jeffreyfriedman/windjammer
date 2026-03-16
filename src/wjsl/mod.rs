//! WJSL - Windjammer Shader Language (RFC syntax)
//!
//! WGSL-like shader DSL: @vertex, @fragment, @compute, @group, @binding, etc.
//! Transpiles .wjsl source to WGSL.

mod ast;
mod codegen;
mod lexer;
mod parser;
mod type_checker;

pub use ast::*;
pub use parser::parse_wjsl;
pub use type_checker::type_check_wjsl;

/// Transpile WJSL source to WGSL
pub fn transpile_wjsl(source: &str) -> Result<String, anyhow::Error> {
    let ast = parse_wjsl(source)?;
    type_checker::check(&ast, source)?;
    let wgsl = codegen::WjslCodegen::new(ast).generate()?;
    Ok(wgsl)
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
