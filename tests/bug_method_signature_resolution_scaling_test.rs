#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Regression for Breach Protocol / windjammer-game dogfooding.
//!
//! The engine build resolves thousands of `Type::method` call sites.  Resolution
//! must use the registry's method index; scanning every unrelated signature for
//! every call caused the engine compiler phase to reach 13.4 GiB peak RSS without
//! emitting generated Rust.

use std::time::{Duration, Instant};

use windjammer::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::call_signature_resolution::resolve_method_for_call_site;
use windjammer::parser::Type;

const NOISE_SIGNATURES: usize = 80_000;
const RESOLUTIONS: usize = 1_000;
const MAX_RESOLUTION_TIME: Duration = Duration::from_millis(750);

fn static_method_signature(name: String) -> FunctionSignature {
    FunctionSignature {
        name,
        param_types: vec![Type::Int32],
        formal_param_types: vec![Type::Int32],
        param_ownership: vec![OwnershipMode::Owned],
        return_type: Some(Type::Int32),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

#[test]
fn receiver_method_resolution_scales_with_matching_methods_not_registry_size() {
    let mut registry = SignatureRegistry::new();
    for index in 0..NOISE_SIGNATURES {
        let name = format!("noise::module_{index}::unrelated_{index}");
        registry.add_function(name.clone(), static_method_signature(name));
    }

    let target = "game::CombatController::tick".to_string();
    registry.add_function(target.clone(), static_method_signature(target));

    let started = Instant::now();
    for _ in 0..RESOLUTIONS {
        let resolved = resolve_method_for_call_site(
            &registry,
            None,
            "CombatController",
            "tick",
            1,
        );
        assert!(resolved.is_some(), "receiver-qualified method must resolve");
    }

    let elapsed = started.elapsed();
    assert!(
        elapsed <= MAX_RESOLUTION_TIME,
        "receiver method resolution scanned unrelated signatures: {RESOLUTIONS} resolutions over \
         {NOISE_SIGNATURES} unrelated signatures took {elapsed:?}; expected indexed lookup within \
         {MAX_RESOLUTION_TIME:?}"
    );
}
