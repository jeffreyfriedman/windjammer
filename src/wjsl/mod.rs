//! WJSL - Windjammer Shader Language (RFC syntax)
//!
//! WGSL-like shader DSL: @vertex, @fragment, @compute, @group, @binding, etc.
//! Transpiles .wjsl source to WGSL.

mod ast;
mod codegen;
mod lexer;
mod parser;

pub use ast::*;
pub use parser::parse_wjsl;

/// Transpile WJSL source to WGSL
pub fn transpile_wjsl(source: &str) -> Result<String, anyhow::Error> {
    let ast = parse_wjsl(source)?;
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
}
