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
