#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use windjammer::analyzer::{FunctionSignature, OwnershipMode};
use windjammer::codegen::rust::call_signature_resolution::{ResolutionMethod, ResolvedSignature};
use windjammer::codegen::rust::signature_promotion::{
    body_borrow_must_not_replace_owned_formal_stub, pick_best_resolved_signature,
    prefer_converged_over_stub,
};
use windjammer::parser::Type;

fn mannequin_engine_stub() -> FunctionSignature {
    FunctionSignature {
        name: "MannequinMesh::generate".into(),
        param_types: vec![Type::Custom("MannequinConfig".into())],
            formal_param_types: vec![],
        param_ownership: vec![],
        return_type: Some(Type::Custom("MannequinMesh".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
    }
}

fn mannequin_body_borrow() -> FunctionSignature {
    FunctionSignature {
        name: "MannequinMesh::generate".into(),
        param_types: vec![Type::Reference(Box::new(Type::Custom("MannequinConfig".into())))],
            formal_param_types: vec![],
        param_ownership: vec![OwnershipMode::Borrowed],
        return_type: Some(Type::Custom("MannequinMesh".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
    }
}

fn mannequin_owned_formal() -> FunctionSignature {
    FunctionSignature {
        name: "MannequinMesh::generate".into(),
        param_types: vec![Type::Custom("MannequinConfig".into())],
            formal_param_types: vec![],
        param_ownership: vec![OwnershipMode::Owned],
        return_type: Some(Type::Custom("MannequinMesh".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
    }
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
        return_type: Some(Type::Custom("Bool".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
    }
}

fn quest_converged_borrow() -> FunctionSignature {
    FunctionSignature {
        name: "QuestManager::is_quest_active".into(),
        param_types: vec![
            Type::Custom("Self".into()),
            Type::Reference(Box::new(Type::Custom("QuestId".into()))),
        ],
        formal_param_types: vec![
            Type::Custom("Self".into()),
            Type::Custom("QuestId".into()),
        ],
        param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        return_type: Some(Type::Custom("Bool".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
    forwarding_borrow_params: None,
    }
}

struct PreferCase {
    name: &'static str,
    local: fn() -> FunctionSignature,
    global: fn() -> FunctionSignature,
    expect_prefer_global: bool,
}

const PREFER_CASES: &[PreferCase] = &[
    PreferCase {
        name: "mannequin_stub_vs_body_borrow_pattern6",
        local: mannequin_body_borrow,
        global: mannequin_engine_stub,
        expect_prefer_global: true,
    },
    PreferCase {
        name: "mannequin_body_borrow_vs_owned_formal_pattern4",
        local: mannequin_body_borrow,
        global: mannequin_owned_formal,
        expect_prefer_global: true,
    },
    PreferCase {
        name: "mannequin_engine_stub_vs_body_borrow_no_promotion",
        local: mannequin_engine_stub,
        global: mannequin_body_borrow,
        expect_prefer_global: false,
    },
    PreferCase {
        name: "quest_stale_engine_vs_converged_pattern3",
        local: quest_engine_stub,
        global: quest_converged_borrow,
        expect_prefer_global: true,
    },
    PreferCase {
        name: "mannequin_empty_ownership_vs_owned_pattern5",
        local: mannequin_engine_stub,
        global: mannequin_owned_formal,
        expect_prefer_global: true,
    },
];

#[test]
fn signature_promotion_prefer_converged_table() {
    for case in PREFER_CASES {
        let local = (case.local)();
        let global = (case.global)();
        let got = prefer_converged_over_stub(&local, &global);
        assert_eq!(
            got, case.expect_prefer_global,
            "case {}: prefer_converged_over_stub(local, global)",
            case.name
        );
    }
}

#[test]
fn signature_promotion_mannequin_stub_blocks_body_borrow_promotion() {
    let stub = mannequin_engine_stub();
    let body = mannequin_body_borrow();
    assert!(body_borrow_must_not_replace_owned_formal_stub(&stub, &body));
    assert!(!prefer_converged_over_stub(&stub, &body));
}

#[test]
fn signature_promotion_pick_best_quest_id_prefers_global() {
    let local = quest_engine_stub();
    let global = quest_converged_borrow();
    let picked = pick_best_resolved_signature(
        Some(ResolvedSignature {
            sig: local,
            qualified_key: "QuestManager::is_quest_active".into(),
            resolution_method: ResolutionMethod::ReceiverQualified,
            has_collision: false,
        }),
        Some(ResolvedSignature {
            sig: global,
            qualified_key: "QuestManager::is_quest_active".into(),
            resolution_method: ResolutionMethod::ReceiverQualified,
            has_collision: false,
        }),
    )
    .expect("pick");
    assert!(matches!(picked.sig.param_types[1], Type::Reference(_)));
    assert_eq!(picked.sig.param_ownership[1], OwnershipMode::Borrowed);
}

#[test]
fn signature_promotion_pick_best_codegen_ref_vec_formal_beats_caller_stub() {
    let vec_u8 = Type::Parameterized("Vec".into(), vec![Type::Custom("u8".into())]);
    let caller_stub = FunctionSignature {
        name: "WalSegment::from_bytes".into(),
        param_types: vec![vec_u8.clone()],
        formal_param_types: vec![vec_u8.clone()],
        param_ownership: vec![OwnershipMode::Owned],
        return_type: Some(Type::Custom("WalSegment".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    };
    let mut defining_refresh = caller_stub.clone();
    defining_refresh.param_types = vec![Type::Reference(Box::new(vec_u8.clone()))];
    defining_refresh.param_ownership = vec![OwnershipMode::Borrowed];
    defining_refresh.emitted_rust_ref_params = Some(vec![true]);

    let picked = pick_best_resolved_signature(
        Some(ResolvedSignature {
            sig: caller_stub,
            qualified_key: "WalSegment::from_bytes".into(),
            resolution_method: ResolutionMethod::ReceiverQualified,
            has_collision: false,
        }),
        Some(ResolvedSignature {
            sig: defining_refresh,
            qualified_key: "WalSegment::from_bytes".into(),
            resolution_method: ResolutionMethod::ReceiverQualified,
            has_collision: false,
        }),
    )
    .expect("pick");
    assert!(matches!(picked.sig.param_types[0], Type::Reference(_)));
    assert_eq!(picked.sig.param_ownership[0], OwnershipMode::Borrowed);
    assert_eq!(picked.sig.emitted_rust_ref_params, Some(vec![true]));
}

#[test]
fn signature_promotion_pick_best_mannequin_owned_formal_wins() {
    let body = mannequin_body_borrow();
    let formal = mannequin_owned_formal();
    let picked = pick_best_resolved_signature(
        Some(ResolvedSignature {
            sig: body,
            qualified_key: "MannequinMesh::generate".into(),
            resolution_method: ResolutionMethod::MethodRegistry,
            has_collision: false,
        }),
        Some(ResolvedSignature {
            sig: formal,
            qualified_key: "MannequinMesh::generate".into(),
            resolution_method: ResolutionMethod::ExactQualified,
            has_collision: false,
        }),
    )
    .expect("pick");
    assert_eq!(picked.sig.param_ownership[0], OwnershipMode::Owned);
}
