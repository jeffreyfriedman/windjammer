#![cfg(not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
)))]

// BUG: Parser reports "Unexpected Type token" in quest/objective.wj
//
// DISCOVERED DURING: Dogfooding windjammer-game
//
// PROBLEM:
// Windjammer source: `pub type QuestObjective = Objective`
// Parser fails: "Unexpected token: Type (at token position N)"
//
// ROOT CAUSE:
// Parser's parse_item() doesn't handle top-level `pub type Name = Type` type alias.
// The `type` keyword is only recognized inside impl blocks (associated types).
//
// FIX:
// Add Token::Type handling in parse_item for module-level type aliases.
// Syntax: pub type Name = Type;

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_pub_type_alias_minimal() {
    // Minimal: pub type Alias = Struct (exact pattern from quest/objective.wj)
    let source = r#"
pub struct Objective {
    id: u32,
}

pub type QuestObjective = Objective
"#;

    let result = test_utils::compile_single_result(source);
    assert!(
        result.is_ok(),
        "pub type alias should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("pub type QuestObjective = Objective"),
        "Generated Rust should contain type alias: {}",
        rust_code
    );
}

#[test]
fn test_type_alias_without_pub() {
    // type alias without pub
    let source = r#"
pub struct Foo {
    x: i32,
}

type Bar = Foo
"#;

    let result = test_utils::compile_single_result(source);
    assert!(
        result.is_ok(),
        "type alias without pub should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("type Bar = Foo"),
        "Generated Rust should contain type alias"
    );
}

#[test]
fn test_type_alias_quest_objective_exact() {
    // Exact pattern from quest/objective.wj (simplified)
    let source = r#"
pub enum ObjectiveType {
    Kill,
    Collect,
}

pub struct Objective {
    id: u32,
    objective_type: ObjectiveType,
}

impl Objective {
    fn new(id: u32, objective_type: ObjectiveType) -> Objective {
        Objective { id, objective_type }
    }
}

pub type QuestObjective = Objective
"#;

    let result = test_utils::compile_single_result(source);
    assert!(
        result.is_ok(),
        "quest/objective.wj pattern should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("pub type QuestObjective = Objective"),
        "Generated Rust should contain pub type QuestObjective = Objective"
    );
}
