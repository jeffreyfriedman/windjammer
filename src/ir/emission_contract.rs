//! Emission-contract oracle for call-site shared-ref / owned formals.
//!
//! Lives in IR so `signature_bridge` does not depend on `call_site_borrow`.
//! Type predicates live in [`crate::ir::formal_predicates`]; `emitted_owned_arg_contract`
//! remains a temporary codegen import until formal emission metadata moves to IR.

use crate::analyzer::{FunctionSignature, OwnershipMode};
use crate::ir::formal_predicates::{
    formal_is_plain_windjammer_string, is_windjammer_text_type, param_is_rust_str_ref,
    param_is_rust_string_ref,
};
use crate::parser::Type;

/// Plain WJ `string` formals that emit owned `String` at call sites until codegen confirms `&str`.
pub fn plain_string_formal_passes_owned_at_call_site(
    sig: &FunctionSignature,
    param_idx: usize,
) -> bool {
    if !formal_is_plain_windjammer_string(
        sig, param_idx,
    ) {
        return false;
    }
    // Shared-ref emission confirmed → call sites borrow (`&str`), not own.
    if callee_emits_shared_rust_ref_param(sig, param_idx) {
        return false;
    }
    // Plain WJ `string` without codegen-confirmed `&str` → pass owned.
    // Stale analyzer `Borrowed` / `Reference(str)` multipass metadata must not force
    // call-site `&` (circular-dep owned formals, WDB-099).
    true
}

/// Whether codegen recorded (or unambiguously converged) a shared-ref Rust formal for `param_idx`.
///
/// Plain WJ `string` formals require `emitted_rust_ref_params` — stale analyzer `Reference(str)`
/// metadata alone must not force call-site `&` (circular-dep/multipass owned formals).
pub fn callee_emits_shared_rust_ref_param(
    sig: &FunctionSignature,
    param_idx: usize,
) -> bool {
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx) {
        return false;
    }
    if let Some(ref flags) = sig.emitted_rust_ref_params {
        if flags.get(param_idx).copied().unwrap_or(false) {
            return true;
        }
        if flags.get(param_idx).copied() == Some(false) {
            // Codegen recorded an owned Rust formal; stale analyzer Reference(T) must not force `&`.
            return false;
        }
    }
    // Plain WJ `string` formals require `emitted_rust_ref_params` for free functions —
    // stale analyzer `Reference(str)` + Borrowed alone must not force call-site `&`
    // (circular-dep / multipass owned formals). Methods and externs may trust
    // registry `&str` contracts without emission flags.
    if formal_is_plain_windjammer_string(
        sig, param_idx,
    ) {
        if sig.is_extern
            && sig
                .param_types
                .get(param_idx)
                .is_some_and(param_is_rust_str_ref)
            && matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            )
        {
            return true;
        }
        // User methods with bare WJ `string` formals stay owned unless codegen recorded
        // shared-ref emission (`emitted_rust_ref_params[idx] == true`). Analyzer Borrowed +
        // stale `Reference(str)` must not force `&field` into owned trait formals
        // (`authenticate(email: string, password: string)`).
        // Stdlib/`&str` contracts are represented without a bare WJ formal in
        // `formal_param_types`, or with an explicit emission flag — handled above/below.
        return false;
    }
    // Registry/stdlib &str contracts (e.g. String::push_str) — param_types Reference(str)
    // with Borrowed ownership. Plain WJ `string` user formals are handled above (return
    // false): stale multipass `Reference(str)` + Borrowed must not force call-site `&`
    // on owned movers like `Objective::talk_to(npc_name: string)`.
    if !formal_is_plain_windjammer_string(
        sig, param_idx,
    ) && sig
        .param_types
        .get(param_idx)
        .is_some_and(param_is_rust_str_ref)
        && matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        )
    {
        return true;
    }
    // Bare WJ Custom formals (`other: Lsn`, `key: Key`) emit owned Rust unless codegen
    // recorded shared-ref (`emitted_rust_ref_params[idx] == true`) OR analyzer converged
    // Borrowed + `Reference(T)` for a non-Copy type (`MemoryEngine::put(key: &Key)` /
    // `QuestManager::is_quest_active(quest_id: &QuestId)`).
    // Stale `Reference(T)` on Copy aggregates must not force call-site `&` (regression-060).
    //
    // When codegen recorded owned emission (`emitted_rust_ref_params[idx] == false`) or
    // ownership is Owned, bare Custom is owned even if `param_types` still has a stale
    // `Reference(T)` (ReBAC `policy: Policy` after keep-owned refresh).
    if let Some(formal) = sig.formal_param_type(param_idx) {
        let emits_shared_flag = sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied();
        // Source-level `&T` / `&mut T` formals (QuestId, Key, …) — shared-ref contract.
        // Distinct from body-converged `Reference(T)` on bare owned aggregates (TableColumn).
        if matches!(formal, Type::Reference(_) | Type::MutableReference(_)) {
            if emits_shared_flag == Some(false) {
                return false;
            }
            let bare = match formal {
                Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                _ => formal,
            };
            let is_copy_aggregate = (crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                || matches!(
                    bare,
                    Type::Custom(name)
                        if crate::type_classification::is_known_copy_aggregate(name)
                ))
                && !crate::type_classification::is_copy_pass_by_value_formal(bare);
            if !is_copy_aggregate
                && matches!(
                    sig.param_ownership.get(param_idx),
                    Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                )
            {
                return true;
            }
        }
        if matches!(formal, Type::Custom(_))
            && !is_windjammer_text_type(formal)
            && (emits_shared_flag == Some(false)
                || matches!(
                    sig.param_ownership.get(param_idx),
                    Some(OwnershipMode::Owned)
                ))
            && emits_shared_flag != Some(true)
        {
            return false;
        }
        let bare = match formal {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        if matches!(bare, Type::Custom(_))
            && !is_windjammer_text_type(bare)
            && emits_shared_flag != Some(true)
        {
            let is_copy_aggregate = (crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                || matches!(
                    bare,
                    Type::Custom(name)
                        if crate::type_classification::is_known_copy_aggregate(name)
                ))
                && !crate::type_classification::is_copy_pass_by_value_formal(bare);
            let analyzer_converged_borrow = matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            ) && sig
                .param_types
                .get(param_idx)
                .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
            // Non-Copy Custom with body-converged `&T` (Key, Value, QuestId, …) — trust borrow
            // only after codegen confirms shared-ref emission. Stale analyzer Reference+Borrowed
            // without refresh must not claim `&T` (Table::column builder forwards).
            if analyzer_converged_borrow && !is_copy_aggregate {
                return emits_shared_flag == Some(true);
            }
            return false;
        }
        // Copy aggregates (`other: Lsn`) always emit owned formals; stale Reference wrap
        // in formal_param_types must not resurrect `&through` at call sites (regression-060).
        if crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
            && sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied()
                != Some(true)
        {
            return false;
        }
    }
    sig.param_types.get(param_idx).is_some_and(|t| match t {
        Type::Reference(inner) | Type::MutableReference(inner) => {
            let inner = inner.as_ref();
            if is_windjammer_text_type(inner) {
                return false;
            }
            // Codegen-converged bare formal beats stale `Reference(T)` in `param_types`
            // (`other: Lsn` emission vs analyzer `Reference(Lsn)` — regression-060).
            if !sig.formal_param_types.is_empty() {
                if let Some(formal) = sig.formal_param_types.get(param_idx) {
                    if !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                        && formal == inner
                        && sig
                            .emitted_rust_ref_params
                            .as_ref()
                            .and_then(|flags| flags.get(param_idx))
                            .copied()
                            != Some(true)
                    {
                        return false;
                    }
                }
            }
            // Copy aggregates emit owned Rust formals — stale Reference(T) must not borrow (regression-060).
            if (crate::codegen::rust::type_analysis_pure::is_copy_type(inner)
                || matches!(
                    inner,
                    Type::Custom(name)
                        if crate::type_classification::is_known_copy_aggregate(name)
                ))
                && !crate::type_classification::is_copy_pass_by_value_formal(inner)
                && sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    != Some(true)
            {
                return false;
            }
            // Stale `Reference(Custom)` is not proof of emitted `&T` for Copy aggregates
            // (regression-060 `Lsn::is_at_or_before`). Non-Copy Custom with Borrowed ownership
            // (`MemoryEngine::put`) does emit `&T` even without emitted_rust_ref_params.
            if matches!(inner, Type::Custom(_))
                && sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    != Some(true)
            {
                let is_copy_aggregate = (crate::codegen::rust::type_analysis_pure::is_copy_type(inner)
                    || matches!(
                        inner,
                        Type::Custom(name)
                            if crate::type_classification::is_known_copy_aggregate(name)
                    ))
                    && !crate::type_classification::is_copy_pass_by_value_formal(inner);
                let analyzer_borrows = matches!(
                    sig.param_ownership.get(param_idx),
                    Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                );
                if is_copy_aggregate || !analyzer_borrows {
                    return false;
                }
            }
            if param_is_rust_str_ref(t) || param_is_rust_string_ref(t) {
                if formal_is_plain_windjammer_string(
                    sig, param_idx,
                ) {
                    return false;
                }
                return sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    == Some(true);
            }
            true
        }
        _ => false,
    })
}
