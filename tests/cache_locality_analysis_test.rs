#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! Cache locality analyzer: ECS-style `Vec<Struct>` loop → AoSoA candidate.

use windjammer::analyzer::Analyzer;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

#[test]
fn test_cache_locality_detects_hot_fields_in_vec_loop() {
    let source = r#"
pub struct Entity {
    pub x: float,
    pub y: float,
    pub z: float,
    pub tag: int,
}

pub fn sum_positions(entities: Vec<Entity>) -> float {
    let mut acc = 0.0
    for e in entities {
        acc = acc + e.x + e.y + e.z
    }
    acc
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse");

    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer
        .analyze_program(&program)
        .expect("analyze");

    let f = analyzed
        .iter()
        .find(|a| a.decl.name == "sum_positions")
        .expect("sum_positions analyzed");

    assert_eq!(f.cache_locality.aosoa_candidates.len(), 1);
    let c = &f.cache_locality.aosoa_candidates[0];
    assert_eq!(c.element_struct, "Entity");
    assert_eq!(c.iterable_var, "entities");
    assert_eq!(c.loop_var, "e");
    assert!(c.simd_friendly_layout);
    let counts: std::collections::HashMap<String, u64> =
        c.field_access_counts.iter().cloned().collect();
    assert_eq!(counts.get("x"), Some(&1));
    assert_eq!(counts.get("y"), Some(&1));
    assert_eq!(counts.get("z"), Some(&1));
    assert_eq!(counts.get("tag"), None);
    assert!(c.hot_fields.contains(&"x".to_string()));
    assert!(c.cold_fields.contains(&"tag".to_string()));
}

#[test]
fn test_cache_locality_detects_same_vec_indexed_in_loop_body() {
    let source = r#"
pub struct Entity {
    pub x: float,
    pub y: float,
}

pub fn fiddle(entities: Vec<Entity>, j: usize) -> float {
    let mut acc = 0.0
    for e in entities {
        acc = acc + e.x
        acc = acc + entities[j].y
    }
    acc
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse");

    let mut analyzer = Analyzer::new();
    let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");

    let f = analyzed
        .iter()
        .find(|a| a.decl.name == "fiddle")
        .expect("fiddle analyzed");

    let c = &f.cache_locality.aosoa_candidates[0];
    assert_eq!(
        c.pattern_kind,
        windjammer::analyzer::AccessPatternKind::IterableAlsoIndexedInBody
    );
}
