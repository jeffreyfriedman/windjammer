#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Parity gates: document where IR bridge and emission oracle diverge.
//!
//! `call_site_needs_shared_ref_at_emit` unions both sources; these tests lock
//! known intentional divergences so refactors do not silently collapse them.

use windjammer::analyzer::{FunctionSignature, OwnershipMode};
use windjammer::ir::emission_contract::callee_emits_shared_rust_ref_param;
use windjammer::ir::signature_bridge::{
    call_site_expects_shared_borrow, call_site_needs_shared_ref_at_emit,
};
use windjammer::parser::Type;

fn sig(
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

#[test]
fn union_helper_covers_both_oracle_and_bridge() {
    let bridge_only = sig(
        "example::log",
        vec![Type::Reference(Box::new(Type::Custom("str".into())))],
        vec![Type::String],
        vec![OwnershipMode::Borrowed],
        false,
        Some(vec![true]),
    );
    assert!(
        call_site_needs_shared_ref_at_emit(&bridge_only, 0),
        "union must be true when either source says shared ref"
    );
}

#[test]
fn plain_string_free_fn_stale_borrow_metadata_stays_owned() {
    let s = sig(
        "accept_label",
        vec![Type::Reference(Box::new(Type::Custom("str".into())))],
        vec![Type::String],
        vec![OwnershipMode::Borrowed],
        false,
        None,
    );
    assert!(
        !callee_emits_shared_rust_ref_param(&s, 0),
        "oracle: stale Reference(str) without emission flags → owned"
    );
    assert!(
        !call_site_expects_shared_borrow(&s, 0),
        "bridge: plain WJ string without codegen confirmation → owned"
    );
    assert!(
        !call_site_needs_shared_ref_at_emit(&s, 0),
        "union agrees: no shared ref at call site"
    );
}

#[test]
fn copy_aggregate_owned_formal_beats_stale_reference_wrap() {
    let s = sig(
        "BatchHandle::merge",
        vec![Type::Reference(Box::new(Type::Custom("Lsn".into())))],
        vec![Type::Custom("Lsn".into())],
        vec![OwnershipMode::Borrowed],
        true,
        Some(vec![false, false]),
    );
    let pidx = s.arg_param_index(0);
    assert!(
        !callee_emits_shared_rust_ref_param(&s, pidx),
        "oracle: emitted owned Copy aggregate must not borrow"
    );
}

#[test]
fn registry_str_ref_with_emission_flag_both_agree_shared() {
    let s = sig(
        "TextBuffer::append_slice",
        vec![
            Type::Custom("TextBuffer".into()),
            Type::Reference(Box::new(Type::Custom("str".into()))),
        ],
        vec![Type::Custom("TextBuffer".into()), Type::String],
        vec![OwnershipMode::MutBorrowed, OwnershipMode::Borrowed],
        true,
        Some(vec![false, true]),
    );
    let pidx = s.arg_param_index(0);
    assert!(callee_emits_shared_rust_ref_param(&s, pidx));
    assert!(call_site_expects_shared_borrow(&s, pidx));
    assert!(call_site_needs_shared_ref_at_emit(&s, pidx));
}
