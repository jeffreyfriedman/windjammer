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

//! TDD: Pass-through ownership inference for method calls on self.field.
//! When parameter `attack` is passed to `self.stats.apply_damage(attack)` and
//! `apply_damage` expects owned `Attack`, `take_damage` should infer `attack` as Owned.

use windjammer::analyzer::{Analyzer, FunctionSignature, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Type};
use windjammer::CompilationTarget;

fn compile_with_external_sigs(source: &str, external_sigs: &SignatureRegistry) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_fns, registry, _) = analyzer
        .analyze_program_with_global_signatures(&program, external_sigs)
        .unwrap();
    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed_fns)
}

#[test]
fn test_passthrough_to_owned_callee_keeps_param_owned() {
    // Simulate CombatStats::apply_damage(self, attack: Attack) where attack is Owned
    let mut external_sigs = SignatureRegistry::new();
    external_sigs.add_function(
        "CombatStats::apply_damage".to_string(),
        FunctionSignature {
            name: "CombatStats::apply_damage".to_string(),
            param_types: vec![Type::Custom("Self".into()), Type::Custom("Attack".into())],
            formal_param_types: vec![],
            param_ownership: vec![OwnershipMode::MutBorrowed, OwnershipMode::Owned],
            return_type: Some(Type::Custom("CombatResult".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        },
    );

    let source = r#"
struct Attack {
    damage: f32,
    damage_type: i32,
}

struct CombatResult {
    was_lethal: bool,
    damage_dealt: f32,
}

struct CombatStats {
    health: f32,
}

struct Enemy {
    stats: CombatStats,
    alive: bool,
}

impl Enemy {
    pub fn take_damage(self, attack: Attack) -> CombatResult {
        let result = self.stats.apply_damage(attack)
        if result.was_lethal {
            self.alive = false
        }
        result
    }
}
"#;

    let code = compile_with_external_sigs(source, &external_sigs);

    // attack should be Owned (not &Attack) because apply_damage expects owned
    assert!(
        code.contains("attack: Attack"),
        "attack should be Owned since apply_damage expects Owned. Got:\n{}",
        code
    );
    assert!(
        !code.contains("attack: &Attack"),
        "attack should NOT be &Attack. Got:\n{}",
        code
    );
}

#[test]
fn test_string_passthrough_to_borrowed_callee_stays_borrowed() {
    // Simulate deserialize_save_data(data: &str) where data is Borrowed string
    let mut external_sigs = SignatureRegistry::new();
    external_sigs.add_function(
        "deserialize_save_data".to_string(),
        FunctionSignature {
            name: "deserialize_save_data".to_string(),
            param_types: vec![Type::String],
            formal_param_types: vec![],
            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Result".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    let source = r#"
fn load_data(data: string) -> bool {
    let result = deserialize_save_data(data)
    true
}
"#;

    let code = compile_with_external_sigs(source, &external_sigs);

    // data should be a borrowed str-like type since callee expects Borrowed
    assert!(
        code.contains("data: &str") || code.contains("data: &String"),
        "data should be &str or &String since callee expects Borrowed. Got:\n{}",
        code
    );
}
