#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! IR coercion gates replacing legacy `should_borrow_at_call_site*` oracles.

use windjammer::analyzer::{FunctionSignature, OwnershipMode};
use windjammer::codegen::rust::call_site_borrow::{
    apply_call_site_borrow, effective_ownership_for_call_arg, CallSiteBorrowDecision,
};
use windjammer::ir::coercion::{compute_coercion, CoercionKind};
use windjammer::ir::emission_contract::callee_emits_shared_rust_ref_param;
use windjammer::ir::safety_type::{
    BaseType, ConstEval, EffectSet, OwnedType, Region, SafetyType, TaintStatus,
};
use windjammer::ir::signature_bridge::safety_type_from_signature_param;
use windjammer::parser::Type;

fn sig_with_formal(
    name: &str,
    param_types: Vec<Type>,
    formal_param_types: Vec<Type>,
    ownership: Vec<OwnershipMode>,
    has_self: bool,
    emitted: Option<Vec<bool>>,
) -> FunctionSignature {
    FunctionSignature {
        name: name.into(),
        param_types,
        formal_param_types,
        param_ownership: ownership,
        return_type: None,
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: has_self,
        is_extern: false,
        emitted_rust_ref_params: emitted,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

fn pure(actual_base: BaseType, own: OwnedType) -> SafetyType {
    SafetyType {
        base: actual_base,
        ownership: own,
        effects: EffectSet::pure(),
        taint: TaintStatus::Clean,
        const_eval: ConstEval::Runtime,
        exec_mode: None,
    }
}

fn coercion_at(sig: &FunctionSignature, arg_index: usize, actual: SafetyType) -> CoercionKind {
    let pidx = sig.arg_param_index(arg_index);
    let expected = safety_type_from_signature_param(sig, pidx);
    compute_coercion(&actual, &expected)
}

#[test]
fn owned_formal_copy_struct_no_borrow() {
    let sig = sig_with_formal(
        "MannequinMesh::generate",
        vec![Type::Custom("MannequinConfig".into())],
        vec![Type::Custom("MannequinConfig".into())],
        vec![OwnershipMode::Owned],
        false,
        None,
    );
    let actual = pure(
        BaseType::Custom("MannequinConfig".into()),
        OwnedType::Owned,
    );
    assert_eq!(
        coercion_at(&sig, 0, actual),
        CoercionKind::Identity,
        "owned Copy formal must not borrow at call site"
    );
    assert_eq!(
        effective_ownership_for_call_arg(&sig, 0),
        OwnershipMode::Owned,
    );
}

#[test]
fn confirmed_shared_ref_param_borrows_owned_actual() {
    // Codegen-confirmed shared-ref emission (`emitted_rust_ref_params`) is required —
    // stale Reference(T) alone must not force Borrow (emission_contract / WDB-099).
    let sig = sig_with_formal(
        "QuestManager::is_quest_active",
        vec![
            Type::Custom("Self".into()),
            Type::Reference(Box::new(Type::Custom("QuestId".into()))),
        ],
        vec![
            Type::Custom("Self".into()),
            Type::Reference(Box::new(Type::Custom("QuestId".into()))),
        ],
        vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        true,
        Some(vec![false, true]),
    );
    let pidx = sig.arg_param_index(0);
    assert!(
        callee_emits_shared_rust_ref_param(&sig, pidx),
        "emission flag must confirm shared-ref contract"
    );
    let actual = pure(BaseType::Custom("QuestId".into()), OwnedType::Owned);
    assert_eq!(
        coercion_at(&sig, 0, actual),
        CoercionKind::Borrow,
        "confirmed shared-ref formal must coerce owned identifier to Borrow"
    );
}

#[test]
fn copy_scalar_i32_no_borrow() {
    let sig = sig_with_formal(
        "example::push",
        vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
        vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
        vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
        true,
        None,
    );
    let actual = pure(BaseType::I32, OwnedType::Copy);
    assert_eq!(
        coercion_at(&sig, 0, actual),
        CoercionKind::Identity,
        "Copy i32 literal must not borrow"
    );
}

#[test]
fn confirmed_str_ref_keeps_string_literal_identity() {
    let sig = sig_with_formal(
        "example::log",
        vec![Type::Reference(Box::new(Type::Custom("str".into())))],
        vec![Type::Custom("string".into())],
        vec![OwnershipMode::Borrowed],
        false,
        Some(vec![true]),
    );
    assert!(callee_emits_shared_rust_ref_param(&sig, 0));
    // String literals are &str-shaped in IR (Ref base String).
    let actual = pure(BaseType::String, OwnedType::Ref(Region::fresh(1)));
    assert_eq!(
        coercion_at(&sig, 0, actual),
        CoercionKind::Identity,
        "string literal to confirmed &str formal must not add extra coercion"
    );
}

#[test]
fn apply_strips_clone_before_borrow() {
    let mut arg_str = "item_id.clone()".to_string();
    let decision = CallSiteBorrowDecision {
        add_ref: true,
        strip_clone: true,
        ..Default::default()
    };
    apply_call_site_borrow(&decision, &mut arg_str);
    assert_eq!(arg_str, "&item_id");
}

#[test]
fn confirmed_shared_ref_apply_borrow_prefix() {
    let sig = sig_with_formal(
        "QuestManager::is_quest_active",
        vec![
            Type::Custom("Self".into()),
            Type::Reference(Box::new(Type::Custom("QuestId".into()))),
        ],
        vec![
            Type::Custom("Self".into()),
            Type::Reference(Box::new(Type::Custom("QuestId".into()))),
        ],
        vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        true,
        Some(vec![false, true]),
    );
    let actual = pure(BaseType::Custom("QuestId".into()), OwnedType::Owned);
    assert_eq!(coercion_at(&sig, 0, actual), CoercionKind::Borrow);
    let mut arg_str = "quest_id".to_string();
    let decision = CallSiteBorrowDecision {
        add_ref: true,
        ..Default::default()
    };
    apply_call_site_borrow(&decision, &mut arg_str);
    assert_eq!(arg_str, "&quest_id");
}
