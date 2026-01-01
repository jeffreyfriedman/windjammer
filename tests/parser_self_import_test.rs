// TDD Test: Parser should support "use self::module" syntax
// THE WINDJAMMER WAY: Support all Rust import patterns

use windjammer::lexer::Lexer;
use windjammer::parser::{Item, Parser};

fn parse_code(code: &str) -> Result<Vec<Item<'_>>, String> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser
        .parse()
        .map(|prog| prog.items)
        .map_err(|e| e.to_string())
}

#[test]
fn test_parse_self_import() {
    let code = "use self::utils";

    let result = parse_code(code);

    assert!(
        result.is_ok(),
        "Should parse 'use self::utils', got error: {:?}",
        result.err()
    );

    let items = result.unwrap();
    assert_eq!(items.len(), 1, "Should have 1 item");

    // Verify it's a Use item with "self" in the path
    match &items[0] {
        Item::Use { path, .. } => {
            assert!(
                !path.is_empty() && path[0] == "self",
                "First path segment should be 'self', got: {:?}",
                path
            );
        }
        other => panic!("Expected Use item, got: {:?}", other),
    }
}

#[test]
fn test_parse_self_nested_import() {
    let code = "use self::utils::format_string";

    let result = parse_code(code);

    assert!(
        result.is_ok(),
        "Should parse nested self import, got error: {:?}",
        result.err()
    );

    let items = result.unwrap();
    match &items[0] {
        Item::Use { path, .. } => {
            assert_eq!(path.len(), 3, "Should have 3 path segments");
            assert_eq!(path[0], "self");
            assert_eq!(path[1], "utils");
            assert_eq!(path[2], "format_string");
        }
        other => panic!("Expected Use item, got: {:?}", other),
    }
}

#[test]
fn test_parse_crate_import() {
    let code = "use crate::config::Settings";

    let result = parse_code(code);

    assert!(result.is_ok(), "Should parse 'use crate::...'");

    let items = result.unwrap();
    match &items[0] {
        Item::Use { path, .. } => {
            assert_eq!(path[0], "crate");
            assert_eq!(path[1], "config");
            assert_eq!(path[2], "Settings");
        }
        other => panic!("Expected Use item, got: {:?}", other),
    }
}

#[test]
fn test_parse_super_import() {
    let code = "use super::parent::Module";

    let result = parse_code(code);

    assert!(result.is_ok(), "Should parse 'use super::...'");

    let items = result.unwrap();
    match &items[0] {
        Item::Use { path, .. } => {
            assert_eq!(path[0], "super");
            assert_eq!(path[1], "parent");
            assert_eq!(path[2], "Module");
        }
        other => panic!("Expected Use item, got: {:?}", other),
    }
}
