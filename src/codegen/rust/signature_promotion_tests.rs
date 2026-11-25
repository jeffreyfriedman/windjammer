//! Tests of `signature_promotion` (kept out of the module file so it stays under the file-size gate).

use super::*;
use crate::analyzer::OwnershipMode;
use crate::parser::Type;
use std::time::Instant;

#[test]
fn promote_overlapping_must_not_copy_absent_global_keys() {
    let mut local = SignatureRegistry::empty();
    local.signatures.insert(
        "LocalOnly::foo".into(),
        FunctionSignature {
            name: "foo".into(),
            param_types: vec![Type::Custom("T".into())],
            formal_param_types: vec![Type::Custom("T".into())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Custom("T".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    let mut global = SignatureRegistry::empty();
    for i in 0..5_000 {
        global.signatures.insert(
            format!("GlobalOnly::m{i}"),
            FunctionSignature {
                name: format!("m{i}"),
                param_types: vec![],
                formal_param_types: vec![],
                param_ownership: vec![],
                return_type: Some(Type::Custom("Unit".into())),
                return_ownership: OwnershipMode::Owned,
                has_self_receiver: false,
                is_extern: false,
                emitted_rust_ref_params: None,
                string_ref_string_formal_params: None,
                field_extract_params: None,
                forwarding_borrow_params: None,
            },
        );
    }

    let start = Instant::now();
    promote_overlapping_global_signatures_into_local(&mut local, &global);
    let elapsed = start.elapsed();

    assert_eq!(
        local.signatures.len(),
        1,
        "must not import absent global keys into local"
    );
    assert!(
        !local.signatures.contains_key("GlobalOnly::m0"),
        "global-only keys must stay out of local"
    );
    assert!(
        elapsed.as_millis() < 200,
        "promote of 5k globals over 1 local key must stay cheap, took {:?}",
        elapsed
    );
}

#[test]
fn promote_overlapping_prefers_mixed_owned_string_over_all_ref_importer_stub() {
    let mixed = FunctionSignature {
        name: "require_nonempty".into(),
        param_types: vec![
            Type::Reference(Box::new(Type::Custom("str".into()))),
            Type::String,
        ],
        formal_param_types: vec![
            Type::Reference(Box::new(Type::Custom("str".into()))),
            Type::String,
        ],
        param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
        return_type: Some(Type::Result(Box::new(Type::String), Box::new(Type::String))),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: Some(vec![true, false]),
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    };
    let all_ref_stub = FunctionSignature {
        name: "require_nonempty".into(),
        param_types: vec![Type::String, Type::String],
        formal_param_types: vec![Type::String, Type::String],
        param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        return_type: Some(Type::Result(Box::new(Type::String), Box::new(Type::String))),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: Some(vec![true, true]),
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    };

    let mut local = SignatureRegistry::empty();
    local
        .signatures
        .insert("require_nonempty".into(), all_ref_stub);
    let mut global = SignatureRegistry::empty();
    global.signatures.insert("require_nonempty".into(), mixed);

    promote_overlapping_global_signatures_into_local(&mut local, &global);

    let promoted = local
        .signatures
        .get("require_nonempty")
        .expect("key stays local");
    assert_eq!(
        promoted.param_ownership,
        vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
        "path-dep mixed formals must replace importer all-ref stub at Step 4B-b"
    );
    assert_eq!(
        promoted.emitted_rust_ref_params.as_deref(),
        Some(&[true, false][..])
    );
}

#[test]
fn exact_qualified_get_does_not_or_bare_mut_borrow_get() {
    let mut reg = SignatureRegistry::empty();
    reg.add_function(
        "wj_querystring::get".into(),
        FunctionSignature {
            name: "wj_querystring::get".into(),
            param_types: vec![Type::String, Type::Reference(Box::new(Type::String))],
            formal_param_types: vec![Type::String, Type::Reference(Box::new(Type::String))],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Borrowed],
            return_type: Some(Type::Option(Box::new(Type::String))),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![false, true]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );
    reg.add_function(
        "get".into(),
        FunctionSignature {
            name: "get".into(),
            param_types: vec![Type::String],
            formal_param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::MutBorrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![true]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );
    let cands = callee_signature_lookup_candidates(&reg, "wj_querystring::get");
    assert_eq!(
        cands.len(),
        1,
        "exact crate::get must not OR bare get: {cands:#?}"
    );
    assert_eq!(
        cands[0].emitted_rust_ref_params.as_deref(),
        Some(&[false, true][..])
    );
    assert!(
        !cands
            .iter()
            .any(|s| matches!(s.param_ownership.first(), Some(OwnershipMode::MutBorrowed))),
        "MutBorrowed HashMap-style get must not steal qs key"
    );
}

fn sig(name: &str, has_self: bool, types: Vec<Type>, own: Vec<OwnershipMode>, flags: Vec<bool>) -> FunctionSignature {
    FunctionSignature {
        name: name.into(),
        formal_param_types: types.clone(),
        param_types: types,
        param_ownership: own,
        return_type: None,
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: has_self,
        is_extern: false,
        emitted_rust_ref_params: Some(flags),
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

/// A codegen refresh merges only views of one function. A static fn `Effect::set_variable(key, value)` (owned) is not
/// the method `Realm::set_variable(self, key, value)` (borrowed): its flags must not land on the method's slots
/// (that made the receiver and `key` Owned: `realm.set_variable(key, &value)`, E0308). Nor is `Column::contains` a
/// view of `Vec::contains`.
#[test]
fn refresh_merge_skips_a_homonym_of_another_receiver() {
    let text = || Type::Reference(Box::new(Type::String));
    let method = sig(
        "set_variable",
        true,
        vec![Type::Reference(Box::new(Type::Custom("Self".into()))), text(), text()],
        vec![OwnershipMode::Borrowed; 3],
        vec![true, true, true],
    );
    let static_fn =
        sig("set_variable", false, vec![Type::String, Type::String], vec![OwnershipMode::Owned; 2], vec![false, false]);
    let mut merged = method.clone();
    merge_codegen_refresh_metadata(&mut merged, &static_fn);
    assert_eq!(merged.param_ownership, method.param_ownership);
    assert_eq!(merged.param_types, method.param_types);

    let int = || Type::Custom("i64".into());
    let vec_contains = sig(
        "Vec::contains",
        true,
        vec![Type::Reference(Box::new(Type::Custom("Vec".into()))), Type::Reference(Box::new(int()))],
        vec![OwnershipMode::Borrowed; 2],
        vec![true, true],
    );
    let user_contains = sig(
        "Column::contains",
        true,
        vec![Type::Reference(Box::new(Type::Custom("Column".into()))), int()],
        vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
        vec![true, false],
    );
    let mut merged = vec_contains.clone();
    merge_codegen_refresh_metadata(&mut merged, &user_contains);
    assert_eq!(merged.param_ownership, vec_contains.param_ownership);
    assert_eq!(merged.param_types, vec_contains.param_types);
}
