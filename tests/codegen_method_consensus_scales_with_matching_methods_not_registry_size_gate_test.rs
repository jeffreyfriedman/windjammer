#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Regression for finance-screens / large multipass builds.
//!
//! Consensus trait queries (`consensus_mutates_receiver`, type-preserving checks)
//! must use the method index — not scan every unrelated signature per call site.

use std::time::{Duration, Instant};

use windjammer::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use windjammer::analyzer::stdlib_method_traits::{
    method_is_type_preserving_qualified, method_mutates_receiver_qualified,
};
use windjammer::parser::Type;

const NOISE_SIGNATURES: usize = 80_000;
const QUERIES: usize = 1_000;
const MAX_QUERY_TIME: Duration = Duration::from_millis(750);

fn static_method_signature(name: String) -> FunctionSignature {
    FunctionSignature {
        name,
        param_types: vec![Type::Int32],
        formal_param_types: vec![Type::Int32],
        param_ownership: vec![OwnershipMode::MutBorrowed, OwnershipMode::Owned],
        return_type: Some(Type::Custom("Self".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

#[test]
fn method_consensus_queries_scale_with_matching_methods_not_registry_size() {
    let mut registry = SignatureRegistry::new();
    for index in 0..NOISE_SIGNATURES {
        let name = format!("noise::module_{index}::unrelated_{index}");
        registry.add_function(name.clone(), static_method_signature(name));
    }

    let target = "game::CombatController::tick".to_string();
    registry.add_function(target.clone(), static_method_signature(target));

    let started = Instant::now();
    for _ in 0..QUERIES {
        assert!(
            method_mutates_receiver_qualified("tick", None, &registry),
            "consensus mutates-receiver must find the indexed method"
        );
        assert!(
            method_is_type_preserving_qualified("tick", None, &registry),
            "consensus type-preserving must find the indexed method"
        );
    }

    let elapsed = started.elapsed();
    assert!(
        elapsed <= MAX_QUERY_TIME,
        "method consensus scanned unrelated signatures: {QUERIES} query pairs over \
         {NOISE_SIGNATURES} unrelated signatures took {elapsed:?}; expected indexed lookup \
         within {MAX_QUERY_TIME:?}"
    );
}
