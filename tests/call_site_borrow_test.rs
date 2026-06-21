#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use windjammer::analyzer::{FunctionSignature, OwnershipMode};
use windjammer::codegen::rust::call_site_borrow::{
    apply_call_site_borrow, effective_ownership_for_call_arg, should_borrow_at_call_site,
    CallSiteBorrowDecision,
};
use windjammer::parser::{Expression, Literal, Type};

fn sig_with_formal(
    name: &str,
    param_types: Vec<Type>,
    formal_param_types: Vec<Type>,
    ownership: Vec<OwnershipMode>,
    has_self: bool,
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
    }
}

#[test]
fn test_owned_formal_copy_struct_no_ampersand() {
    let sig = sig_with_formal(
        "MannequinMesh::generate",
        vec![Type::Custom("MannequinConfig".into())],
        vec![Type::Custom("MannequinConfig".into())],
        vec![OwnershipMode::Owned],
        false,
    );
    let arg = Expression::Identifier {
        name: "config".into(),
        location: Default::default(),
    };
    let decision = should_borrow_at_call_site(&sig, 0, &arg, "config", "generate", false);
    assert!(
        !decision.add_ref,
        "owned Copy formal must not add & at call site"
    );
    assert_eq!(
        effective_ownership_for_call_arg(&sig, 0),
        OwnershipMode::Owned,
        "effective ownership must be Owned for bare Copy formal"
    );
}

#[test]
fn test_borrowed_reference_param_adds_ampersand() {
    let sig = sig_with_formal(
        "QuestManager::is_quest_active",
        vec![
            Type::Custom("Self".into()),
            Type::Reference(Box::new(Type::Custom("QuestId".into()))),
        ],
        vec![Type::Custom("Self".into()), Type::Custom("QuestId".into())],
        vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        true,
    );
    let arg = Expression::Identifier {
        name: "quest_id".into(),
        location: Default::default(),
    };
    let decision = should_borrow_at_call_site(&sig, 0, &arg, "quest_id", "is_quest_active", false);
    assert!(decision.add_ref, "converged borrow must add & at call site");

    let mut arg_str = "quest_id".to_string();
    apply_call_site_borrow(&decision, &mut arg_str);
    assert_eq!(arg_str, "&quest_id");
}

#[test]
fn test_copy_scalar_i32_no_ampersand() {
    let sig = sig_with_formal(
        "example::push",
        vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
        vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
        vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
        true,
    );
    let arg = Expression::Literal {
        value: Literal::Int(64),
        location: Default::default(),
    };
    let decision = should_borrow_at_call_site(&sig, 0, &arg, "64", "push", false);
    assert!(!decision.add_ref, "Copy i32 literal must not add &");
}

#[test]
fn test_string_literal_to_str_param_no_extra_ampersand() {
    let sig = sig_with_formal(
        "example::log",
        vec![Type::Reference(Box::new(Type::Custom("str".into())))],
        vec![Type::Custom("string".into())],
        vec![OwnershipMode::Borrowed],
        false,
    );
    let arg = Expression::Literal {
        value: Literal::String("hello".into()),
        location: Default::default(),
    };
    let decision = should_borrow_at_call_site(&sig, 0, &arg, "\"hello\"", "log", false);
    assert!(
        !decision.add_ref,
        "string literal to &str param must not add extra &"
    );
}

#[test]
fn test_apply_strips_clone_before_borrow() {
    let mut arg_str = "item_id.clone()".to_string();
    let decision = CallSiteBorrowDecision {
        add_ref: true,
        strip_clone: true,
        ..Default::default()
    };
    apply_call_site_borrow(&decision, &mut arg_str);
    assert_eq!(arg_str, "&item_id");
}
