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
            emitted_rust_ref_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
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
            emitted_rust_ref_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
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
            emitted_rust_ref_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
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

fn quest_engine_stub() -> FunctionSignature {
    FunctionSignature {
        name: "QuestManager::is_quest_active".into(),
        param_types: vec![
            Type::Custom("Self".into()),
            Type::Custom("QuestId".into()),
        ],
        formal_param_types: vec![
            Type::Custom("Self".into()),
            Type::Custom("QuestId".into()),
        ],
        param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
        return_type: Some(Type::Bool),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
        emitted_rust_ref_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

fn quest_converged_borrow() -> FunctionSignature {
    FunctionSignature {
        name: "QuestManager::is_quest_active".into(),
        param_types: vec![
            Type::Custom("Self".into()),
            Type::Custom("QuestId".into()),
        ],
        formal_param_types: vec![
            Type::Custom("Self".into()),
            Type::Custom("QuestId".into()),
        ],
        param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        return_type: Some(Type::Bool),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
        emitted_rust_ref_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

#[test]
fn quest_engine_local_stub_global_converged_borrow() {
    let mut local = SignatureRegistry::new();
    local.add_function(
        "QuestManager::is_quest_active".into(),
        quest_engine_stub(),
    );

    let mut global = SignatureRegistry::new();
    global.add_function(
        "QuestManager::is_quest_active".into(),
        quest_converged_borrow(),
    );

    let resolved = resolve_method_for_call_site(
        &local,
        Some(&global),
        "QuestManager",
        "is_quest_active",
        1,
    )
    .expect("resolve QuestManager::is_quest_active");

    assert_eq!(
        resolved.sig.param_ownership[1],
        OwnershipMode::Borrowed,
        "global converged borrow must beat stale engine local stub"
    );
    assert_eq!(
        effective_param_ownership_for_arg(&resolved.sig, 0),
        OwnershipMode::Borrowed,
    );
}

#[test]
fn quest_local_engine_global_converged_reference_types() {
    let mut local = SignatureRegistry::new();
    local.add_function(
        "QuestManager::is_quest_active".into(),
        quest_engine_stub(),
    );

    let mut global = SignatureRegistry::new();
    global.add_function(
        "QuestManager::is_quest_active".into(),
        FunctionSignature {
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("Self".into()))),
                Type::Reference(Box::new(Type::Custom("QuestId".into()))),
            ],
            ..quest_converged_borrow()
        },
    );

    let resolved = resolve_method_for_call_site(
        &local,
        Some(&global),
        "QuestManager",
        "is_quest_active",
        1,
    )
    .expect("resolve");

    assert_eq!(
        effective_param_ownership_for_arg(&resolved.sig, 0),
        OwnershipMode::Borrowed,
    );
    assert!(matches!(
        resolved.sig.param_types[1],
        Type::Reference(_)
    ));
}

#[test]
fn quest_local_borrowed_bare_global_borrowed_reference_types() {
    let mut local = SignatureRegistry::new();
    local.add_function(
        "QuestManager::is_quest_active".into(),
        FunctionSignature {
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("Self".into()))),
                Type::Custom("QuestId".into()),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            ..quest_engine_stub()
        },
    );

    let mut global = SignatureRegistry::new();
    global.add_function(
        "QuestManager::is_quest_active".into(),
        FunctionSignature {
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("Self".into()))),
                Type::Reference(Box::new(Type::Custom("QuestId".into()))),
            ],
            ..quest_converged_borrow()
        },
    );

    let resolved = resolve_method_for_call_site(
        &local,
        Some(&global),
        "QuestManager",
        "is_quest_active",
        1,
    )
    .expect("resolve");

    assert!(matches!(
        resolved.sig.param_types[1],
        Type::Reference(_)
    ));
}
