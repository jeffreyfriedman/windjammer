#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! WDB-094: qualified external-crate free calls must resolve via registry aliases.
//!
//! Cross-crate metadata registers bare `fn` keys. Call sites may use
//! `dep_crate::fn(...)`. IR fail-closed must not emit `compile_error!` when the
//! bare signature exists and is aliased under the crate prefix — without
//! hardcoding API names.

use windjammer::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Type};
use windjammer::CompilationTarget;

fn parse_program(source: &str) -> windjammer::parser::Program<'static> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().expect("parse")
}

fn owned_string_sig(name: &str) -> FunctionSignature {
    FunctionSignature {
        name: name.to_string(),
        param_types: vec![Type::String],
        formal_param_types: vec![Type::String],
        param_ownership: vec![OwnershipMode::Owned],
        return_type: None,
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: Some(vec![false]),
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

#[test]
fn qualified_external_crate_call_resolves_via_crate_prefix_alias() {
    let program = parse_program(
        r#"
fn main() {
    dep_crate::circuit_delta_from_edge_inserts("x")
}
"#,
    );

    let mut registry = SignatureRegistry::new();
    let bare = owned_string_sig("circuit_delta_from_edge_inserts");
    registry.add_function("circuit_delta_from_edge_inserts".into(), bare.clone());
    // Simulates metadata load registering crate-qualified aliases (WDB-094).
    registry.register_crate_prefix_aliases("dep_crate");

    let mut analyzer = windjammer::analyzer::Analyzer::new();
    let (analyzed, local_reg, _) = analyzer.analyze_program(&program).expect("analyze");
    registry.merge(&local_reg);

    let mut gen = CodeGenerator::new(registry, CompilationTarget::Rust);
    let output = gen.generate_program(&program, &analyzed);

    assert!(
        !output.contains("compile_error!") && !output.contains("missing boundary signature"),
        "qualified dep_crate::fn must resolve via crate-prefix alias. Got:\n{output}"
    );
    assert!(
        output.contains("circuit_delta_from_edge_inserts"),
        "expected call to survive codegen. Got:\n{output}"
    );
}

#[test]
fn qualified_external_without_crate_prefix_alias_fails_closed() {
    let program = parse_program(
        r#"
fn main() {
    dep_crate::circuit_delta_from_edge_inserts("x")
}
"#,
    );

    let mut registry = SignatureRegistry::new();
    // Bare metadata key only — no `dep_crate::` alias (pre-WDB-094 behavior).
    registry.add_function(
        "circuit_delta_from_edge_inserts".into(),
        owned_string_sig("circuit_delta_from_edge_inserts"),
    );

    let mut analyzer = windjammer::analyzer::Analyzer::new();
    let (analyzed, local_reg, _) = analyzer.analyze_program(&program).expect("analyze");
    registry.merge(&local_reg);

    let mut gen = CodeGenerator::new(registry, CompilationTarget::Rust);
    let output = gen.generate_program(&program, &analyzed);
    assert!(
        output.contains("compile_error!") && output.contains("missing boundary signature"),
        "bare-only registry must fail closed for qualified calls. Got:\n{output}"
    );
}

#[test]
fn unknown_crate_qualified_call_still_fails_closed() {
    let program = parse_program(
        r#"
fn main() {
    unknown_crate::missing_api("x")
}
"#,
    );
    let mut analyzer = windjammer::analyzer::Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
    let mut gen = CodeGenerator::new(registry, CompilationTarget::Rust);
    let output = gen.generate_program(&program, &analyzed);
    assert!(
        output.contains("compile_error!") && output.contains("missing boundary signature"),
        "unknown crate must stay fail-closed. Got:\n{output}"
    );
}
