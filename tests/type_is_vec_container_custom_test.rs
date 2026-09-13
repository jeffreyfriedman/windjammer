//! Regression: WJ `Vec<u8>` AST types must classify as vec containers.

use windjammer::parser::{Parser, Type};
use windjammer::type_classification::type_is_vec_container;

fn parse_type_from_param(src: &str) -> Type {
    let src = format!("pub fn f(x: {src}) -> int {{ 0 }}");
    let mut lexer = windjammer::lexer::Lexer::new(&src);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse");
    match &program.items[0] {
        windjammer::parser::Item::Function { decl, .. } => decl.parameters[0].type_.clone(),
        _ => panic!("expected function"),
    }
}

#[test]
fn vec_u8_parameterized_is_vec_container() {
    let ty = parse_type_from_param("Vec<u8>");
    assert!(
        type_is_vec_container(&ty),
        "Vec<u8> must be vec container, got {ty:?}"
    );
}
