// BUG: Parser reports "Unexpected Type token" for `pub type` alias
//
// DISCOVERED DURING: Dogfooding quest/quest_state.wj
//
// PROBLEM:
// Windjammer source: `pub type QuestStatus = QuestState`
// Parser fails: "Unexpected token: Type (at token position N)"
//
// ROOT CAUSE:
// parse_item() has no branch for Token::Type at top level.
// `pub type Name = Type` is a type alias (like Rust's type alias).
//
// FIX:
// Add Token::Type branch in parse_item() to parse type alias:
//   pub type AliasName = ConcreteType
// Semicolon optional (ASI).

use windjammer::lexer::Lexer;
use windjammer::parser::ast::*;
use windjammer::parser_impl::Parser;

fn parse_program(input: &str) -> Result<Program<'_>, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[test]
fn test_pub_type_alias_quest_status() {
    // Minimal: pub type QuestStatus = QuestState (from quest_state.wj)
    let source = r#"
pub enum QuestState {
    NotStarted,
    Active,
    Completed,
    Failed,
}

pub type QuestStatus = QuestState
"#;

    let result = parse_program(source);
    assert!(
        result.is_ok(),
        "pub type alias should parse. Error: {:?}",
        result.err()
    );

    let program = result.unwrap();
    // Should have 2 items: enum + type alias
    assert_eq!(program.items.len(), 2, "Expected enum and type alias");
    let type_alias = &program.items[1];
    assert!(
        matches!(type_alias, Item::TypeAlias { .. }),
        "Second item should be TypeAlias, got {:?}",
        type_alias
    );
}

#[test]
fn test_pub_type_alias_objective() {
    // Same pattern from objective.wj: pub type QuestObjective = Objective
    let source = r#"
pub struct Objective {
    id: u32,
}

pub type QuestObjective = Objective
"#;

    let result = parse_program(source);
    assert!(
        result.is_ok(),
        "pub type alias should parse. Error: {:?}",
        result.err()
    );
}
