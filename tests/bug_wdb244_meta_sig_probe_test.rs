#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Probe: cross-crate metadata for demoted sql_exec must keep bare string lits.

use windjammer::analyzer::{Analyzer, FunctionSignature, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::CodeGenerator;
use windjammer::ir::coercion::{compute_coercion, CoercionKind};
use windjammer::ir::safety_type::{
    BaseType, ConstEval, EffectSet, OwnedType, Region, SafetyType, TaintStatus,
};
use windjammer::ir::signature_bridge::safety_type_from_signature_param;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Type};
use windjammer::CompilationTarget;

fn sql_exec_sig() -> FunctionSignature {
    FunctionSignature {
        name: "Batch::sql_exec".into(),
        param_types: vec![
            Type::Reference(Box::new(Type::Custom("Self".into()))),
            Type::Custom("Batch".into()),
            Type::Reference(Box::new(Type::Custom("str".into()))),
            Type::Reference(Box::new(Type::Custom("str".into()))),
            Type::Reference(Box::new(Type::Custom("str".into()))),
        ],
        formal_param_types: vec![
            Type::Custom("Self".into()),
            Type::Custom("Batch".into()),
            Type::String,
            Type::String,
            Type::String,
        ],
        param_ownership: vec![
            OwnershipMode::Borrowed,
            OwnershipMode::Owned,
            OwnershipMode::Borrowed,
            OwnershipMode::Borrowed,
            OwnershipMode::Borrowed,
        ],
        return_type: Some(Type::Bool),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
        emitted_rust_ref_params: Some(vec![false, false, true, true, true]),
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

#[test]
fn wdb244_meta_sig_coercion_oracle_string_lit_is_identity() {
    let sig = sql_exec_sig();
    let pidx = sig.arg_param_index(1);
    let expected = safety_type_from_signature_param(&sig, pidx);
    assert!(
        matches!(expected.ownership, OwnedType::Ref(_)),
        "expected Ref from demoted metadata, got {:?}",
        expected.ownership
    );
    let actual = SafetyType {
        base: BaseType::String,
        ownership: OwnedType::Ref(Region::fresh(0)),
        effects: EffectSet::pure(),
        taint: TaintStatus::Clean,
        const_eval: ConstEval::Runtime,
        exec_mode: None,
    };
    assert_eq!(
        compute_coercion(&actual, &expected),
        CoercionKind::Identity,
        "string lit → demoted &str must be Identity"
    );
}

#[test]
fn wdb244_meta_sig_codegen_must_not_to_string_lits() {
    let source = r#"
pub struct Batch { pub n: int }
impl Batch {
    pub fn sql_exec(self, right: Batch, left_table: string, right_table: string, sql: string) -> bool {
        true
    }
}
pub fn run(left: Batch, right: Batch, sql: string) -> bool {
    left.sql_exec(right, "props", "edges", sql)
}
"#;
    let mut external = SignatureRegistry::new();
    external.add_function("Batch::sql_exec".into(), sql_exec_sig());
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_fns, registry, _) = analyzer
        .analyze_program_with_global_signatures(&program, &external)
        .unwrap();
    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.set_global_signature_registry(std::sync::Arc::new(external));
    let code = codegen.generate_program(&program, &analyzed_fns);
    eprintln!("WDB-244 meta probe:\n{code}");
    assert!(
        !code.contains("\"props\".to_string()") && !code.contains("\"edges\".to_string()"),
        "WDB-244 RED: meta demoted sql_exec got .to_string() lits:\n{code}"
    );
}
