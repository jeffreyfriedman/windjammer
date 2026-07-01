#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use windjammer::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::call_signature_resolution::{
    effective_param_ownership_for_arg, resolve_method_for_call_site,
};
use windjammer::parser::Type;

fn mannequin_engine_stub() -> FunctionSignature {
    FunctionSignature {
        name: "MannequinMesh::generate".into(),
        param_types: vec![Type::Custom("MannequinConfig".into())],
        formal_param_types: vec![Type::Custom("MannequinConfig".into())],
        param_ownership: vec![],
        return_type: Some(Type::Custom("MannequinMesh".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
    }
}

fn mannequin_body_borrow() -> FunctionSignature {
    FunctionSignature {
        name: "MannequinMesh::generate".into(),
        param_types: vec![Type::Reference(Box::new(Type::Custom("MannequinConfig".into())))],
        formal_param_types: vec![Type::Custom("MannequinConfig".into())],
        param_ownership: vec![OwnershipMode::Borrowed],
        return_type: Some(Type::Custom("MannequinMesh".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
    }
}

fn mannequin_owned_formal() -> FunctionSignature {
    FunctionSignature {
        name: "MannequinMesh::generate".into(),
        param_types: vec![Type::Custom("MannequinConfig".into())],
        formal_param_types: vec![Type::Custom("MannequinConfig".into())],
        param_ownership: vec![OwnershipMode::Owned],
        return_type: Some(Type::Custom("MannequinMesh".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
    }
}

#[test]
fn test_mannequin_generate_owned_formal_wins_over_body_borrow() {
    let mut local = SignatureRegistry::new();
    local.add_function("MannequinMesh::generate".into(), mannequin_body_borrow());

    let mut global = SignatureRegistry::new();
    global.add_function("MannequinMesh::generate".into(), mannequin_owned_formal());

    let resolved = resolve_method_for_call_site(
        &local,
        Some(&global),
        "MannequinMesh",
        "generate",
        1,
    )
    .expect("must resolve MannequinMesh::generate");

    assert_eq!(
        resolved.sig.param_ownership[0],
        OwnershipMode::Owned,
        "owned formal must win over body-inferred borrow"
    );
    assert_eq!(
        effective_param_ownership_for_arg(&resolved.sig, 0),
        OwnershipMode::Owned,
    );
}

#[test]
fn test_mannequin_generate_engine_stub_blocks_body_borrow_promotion() {
    let mut local = SignatureRegistry::new();
    local.add_function("MannequinMesh::generate".into(), mannequin_body_borrow());

    let mut global = SignatureRegistry::new();
    global.add_function("MannequinMesh::generate".into(), mannequin_engine_stub());

    let resolved = resolve_method_for_call_site(
        &local,
        Some(&global),
        "MannequinMesh",
        "generate",
        1,
    )
    .expect("must resolve MannequinMesh::generate");

    assert!(
        resolved.sig.param_ownership.is_empty()
            || resolved.sig.param_ownership[0] == OwnershipMode::Owned,
        "engine stub with bare owned formal must not lose to body borrow; ownership={:?}",
        resolved.sig.param_ownership
    );
    assert_eq!(
        effective_param_ownership_for_arg(&resolved.sig, 0),
        OwnershipMode::Owned,
    );
}

#[test]
fn test_mannequin_generate_local_only_owned_formal() {
    let mut local = SignatureRegistry::new();
    local.add_function("MannequinMesh::generate".into(), mannequin_owned_formal());

    let resolved = resolve_method_for_call_site(&local, None, "MannequinMesh", "generate", 1)
        .expect("local-only resolve");

    assert_eq!(resolved.sig.param_ownership[0], OwnershipMode::Owned);
}
