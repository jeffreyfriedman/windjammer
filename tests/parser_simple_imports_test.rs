// TDD Test: Simple imports without self/crate/super should work
// THE WINDJAMMER WAY: Compiler finds modules automatically!

use windjammer::parser::{Item, Parser};
use windjammer::lexer::Lexer;

#[test]
fn test_simple_import() {
    let source = "use Vec2;";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Should parse simple import");
    
    assert_eq!(program.items.len(), 1);
    if let Item::Use { path, .. } = &program.items[0] {
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], "Vec2");
    } else {
        panic!("Expected Use item");
    }
}

#[test]
fn test_namespaced_import() {
    let source = "use math::Vec2;";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Should parse namespaced import");
    
    assert_eq!(program.items.len(), 1);
    if let Item::Use { path, .. } = &program.items[0] {
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], "math");
        assert_eq!(path[1], "Vec2");
    } else {
        panic!("Expected Use item");
    }
}

#[test]
fn test_rust_style_imports_still_work() {
    // We support these but don't encourage them
    let test_cases = vec![
        ("use self::utils;", vec!["self", "utils"]),
        ("use crate::math::Vec2;", vec!["crate", "math", "Vec2"]),
        ("use super::Camera2D;", vec!["super", "Camera2D"]),
    ];
    
    for (source, expected_path) in test_cases {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect(&format!("Should parse: {}", source));
        
        if let Item::Use { path, .. } = &program.items[0] {
            assert_eq!(path, &expected_path, "Failed for: {}", source);
        } else {
            panic!("Expected Use item for: {}", source);
        }
    }
}
