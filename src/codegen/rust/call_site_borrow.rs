//! Unified call-site borrow lowering.
//!
//! Centralizes decisions about when to emit `&`, `&mut`, strip `&`, or strip `.clone()` at
//! method/function call sites based on effective parameter ownership and formal types.

use crate::analyzer::{FunctionSignature, OwnershipMode};
use crate::codegen::rust::call_signature_resolution::{
    effective_param_ownership_for_arg, param_type_is_owned_non_text,
};
use crate::codegen::rust::expression_utilities;
use crate::codegen::rust::rust_coercion_rules::Coercion;
use crate::codegen::rust::stdlib_method_traits;
use crate::codegen::rust::string_utilities;
use crate::codegen::rust::type_analysis_pure;
use crate::codegen::rust::types;
use crate::parser::{Expression, Literal, Type};

/// Plain WJ `string` formals that emit owned `String` at call sites until codegen confirms `&str`.
pub(crate) fn plain_string_formal_passes_owned_at_call_site(
    sig: &FunctionSignature,
    param_idx: usize,
) -> bool {
    if !crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
        sig, param_idx,
    ) {
        return false;
    }
    if callee_emits_shared_rust_ref_param(sig, param_idx) {
        return false;
    }
    // Body/codegen converged to `&str` + Borrowed (cross-file `.wj.meta` often lacks
    // `emitted_rust_ref_params`) — pass as borrow, never force owned `.to_string()`.
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Borrowed)
    ) && sig.param_types.get(param_idx).is_some_and(|t| {
        string_utilities::param_is_rust_str_ref(t) || matches!(t, Type::Reference(_))
    }) {
        return false;
    }
    // Body-inferred borrow without converged param_types yet (readonly string callees).
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Borrowed)
    ) && !sig.param_types.get(param_idx).is_some_and(|t| {
        matches!(t, Type::Reference(_) | Type::MutableReference(_))
    }) {
        return false;
    }
    // Free-function plain `string`: Owned analyzer contract → pass by value.
    // (Readonly `&str` emission is gated on `emitted_rust_ref_params` / Borrowed ownership.)
    if !sig.has_self_receiver {
        return matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Owned) | None
        ) && !callee_emits_shared_rust_ref_param(sig, param_idx);
    }
    true
}

/// Whether codegen recorded (or unambiguously converged) a shared-ref Rust formal for `param_idx`.
///
/// Plain WJ `string` formals require `emitted_rust_ref_params` — stale analyzer `Reference(str)`
/// metadata alone must not force call-site `&` (circular-dep/multipass owned formals).
pub(crate) fn callee_emits_shared_rust_ref_param(
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
    // Plain WJ `string` formals: trust body/codegen convergence to `&str` + Borrowed
    // (metadata often omits `emitted_rust_ref_params`). Stale `Reference(str)` with
    // *Owned* ownership still falls through to false below.
    if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
        sig, param_idx,
    ) {
        if sig.param_types.get(param_idx).is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
        }) && matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        ) {
            return true;
        }
        if sig.is_extern
            && sig.param_types.get(param_idx).is_some_and(|t| {
                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
            })
            && matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            )
        {
            return true;
        }
        // Registry/stdlib &str on plain `string` method formals (e.g. TextBuffer::append_slice).
        if sig.has_self_receiver
            && sig.param_types.get(param_idx).is_some_and(|t| {
                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
            })
            && matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            )
        {
            return true;
        }
        return false;
    }
    // Registry/stdlib &str contracts (e.g. String::push_str) — param_types Reference(str)
    // with Borrowed ownership.
    if sig.param_types.get(param_idx).is_some_and(|t| {
        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
    }) && matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
    ) {
        return true;
    }
    // Bare WJ Custom formals (`other: Lsn`, `key: Key`) emit owned Rust unless codegen
    // recorded shared-ref (`emitted_rust_ref_params[idx] == true`). Stale `Reference(T)`
    // in formal_param_types / param_types must not force call-site `&` (WDB-060).
    if let Some(formal) = sig.formal_param_type(param_idx) {
        let bare = match formal {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        if matches!(bare, Type::Custom(_))
            && !crate::codegen::rust::types::is_windjammer_text_type(bare)
            && sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied()
                != Some(true)
        {
            return false;
        }
        // Copy aggregates (`other: Lsn`) always emit owned formals; stale Reference wrap
        // in formal_param_types must not resurrect `&through` at call sites (WDB-060).
        if crate::codegen::rust::type_analysis::is_copy_type(bare)
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
            if types::is_windjammer_text_type(inner) {
                return false;
            }
            // Codegen-converged bare formal beats stale `Reference(T)` in `param_types`
            // (`other: Lsn` emission vs analyzer `Reference(Lsn)` — WDB-060).
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
            // Copy aggregates emit owned Rust formals — stale Reference(T) must not borrow (WDB-060).
            if (crate::codegen::rust::type_analysis::is_copy_type(inner)
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
            // Stale `Reference(Custom)` is not proof of emitted `&T` unless codegen
            // confirmed shared-ref emission (WDB-060 `Lsn::is_at_or_before`).
            if matches!(inner, Type::Custom(_))
                && sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    != Some(true)
            {
                return false;
            }
            true
        }
        _ => false,
    })
}

fn type_is_vec_container(t: &Type) -> bool {
    matches!(t, Type::Vec(_)) || matches!(t, Type::Parameterized(name, _) if name == "Vec")
}

pub(crate) fn expression_is_vec_macro_literal(expr: &Expression) -> bool {
    matches!(expr, Expression::MacroInvocation { name, .. } if name == "vec")
}

/// `Vec::new()` / `Vec::<T>::new()` at a call site — owned constructor, safe to prefix `&`.
pub(crate) fn expression_is_vec_new_constructor(expr: &Expression) -> bool {
    if expression_is_vec_macro_literal(expr) {
        return true;
    }
    let Expression::Call { function, arguments, .. } = expr else {
        return false;
    };
    if !arguments.is_empty() {
        return false;
    }
    match &**function {
        Expression::FieldAccess { object, field, .. } if field == "new" => {
            matches!(&**object, Expression::Identifier { name, .. } if name == "Vec")
        }
        _ => false,
    }
}

fn coerced_is_owned_vec_constructor(coerced: &str) -> bool {
    coerced.starts_with("Vec::new()")
        || (coerced.starts_with("Vec::<") && coerced.contains(">::new()"))
}

fn expression_is_owned_vec_at_call_site<'ast>(
    gen: &crate::codegen::rust::generator::CodeGenerator<'ast>,
    arg_expr: &Expression<'ast>,
) -> bool {
    if expression_is_vec_new_constructor(arg_expr) {
        return true;
    }
    let Expression::Identifier { name, .. } = arg_expr else {
        return false;
    };
    gen.local_var_types.get(name).is_some_and(type_is_vec_container)
        || gen
            .infer_expression_type(arg_expr)
            .is_some_and(|t| type_is_vec_container(&t))
}

fn callee_arg_expects_shared_vec_ref(sig: &FunctionSignature, arg_index: usize) -> bool {
    let pidx = sig.arg_param_index(arg_index);
    if callee_emits_shared_rust_ref_param(sig, pidx) {
        return true;
    }
    sig.param_types.get(pidx).is_some_and(|t| {
        matches!(t, Type::Reference(inner) if type_is_vec_container(inner))
    })
}

/// Borrow owned local `Vec` bindings when the converged callee formal is `&Vec<T>`.
pub(crate) fn maybe_borrow_owned_vec_local_for_ref_formal<'ast>(
    gen: &crate::codegen::rust::generator::CodeGenerator<'ast>,
    sig: &FunctionSignature,
    arg_index: usize,
    arg_expr: &Expression<'ast>,
    coerced: String,
    receiver_type: Option<&str>,
    method: Option<&str>,
    arg_count: Option<usize>,
) -> String {
    if coerced.starts_with('&') {
        return coerced;
    }
    if !expression_is_owned_vec_at_call_site(gen, arg_expr)
        && !coerced_is_owned_vec_constructor(&coerced)
    {
        return coerced;
    }

    let count = arg_count.unwrap_or(arg_index + 1);
    if let (Some(rt), Some(method)) = (receiver_type, method) {
        if let Some(resolved) = gen.resolve_method_function_signature(rt, method, count) {
            if callee_arg_expects_shared_vec_ref(&resolved, arg_index) {
                return format!("&{coerced}");
            }
        }
        let qualified = format!("{rt}::{method}");
        if let Some(global) = gen.get_signature_with_global(&qualified) {
            if callee_arg_expects_shared_vec_ref(global, arg_index) {
                return format!("&{coerced}");
            }
        }
    }

    if callee_arg_expects_shared_vec_ref(sig, arg_index) {
        format!("&{coerced}")
    } else {
        coerced
    }
}

/// Whether a user free-function call must not add `&` because callee formals emit owned
/// `String` (or registry refresh recorded owned contract) despite stale call-site borrow metadata.
pub(crate) fn skip_stale_borrow_on_owned_user_free_fn(
    registry: &crate::analyzer::SignatureRegistry,
    callee_name: &str,
    call_sig: &FunctionSignature,
    param_idx: usize,
    arg_index: usize,
) -> bool {
    skip_stale_borrow_on_owned_user_free_fn_with_global(
        registry, None, callee_name, call_sig, param_idx, arg_index,
    )
}

/// Same as [`skip_stale_borrow_on_owned_user_free_fn`] but also consults a converged
/// global registry so cross-module `&str` emission is not treated as a stale borrow.
pub(crate) fn skip_stale_borrow_on_owned_user_free_fn_with_global(
    registry: &crate::analyzer::SignatureRegistry,
    global: Option<&crate::analyzer::SignatureRegistry>,
    callee_name: &str,
    call_sig: &FunctionSignature,
    param_idx: usize,
    arg_index: usize,
) -> bool {
    if callee_name.contains("::") {
        return false;
    }
    let check = |sig: &FunctionSignature, pidx: usize| -> bool {
        // Codegen confirmed shared-ref (`&str`) — never skip the borrow prefix.
        if callee_emits_shared_rust_ref_param(sig, pidx) {
            return false;
        }
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return true;
        }
        if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            sig, pidx,
        ) && matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership(sig, pidx),
            OwnershipMode::Borrowed
        ) {
            return false;
        }
        plain_string_formal_passes_owned_at_call_site(sig, pidx)
    };
    if check(call_sig, param_idx) {
        // Still allow global codegen refresh to veto the skip.
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let global_emits_ref = global.is_some_and(|g| {
            g.get_signature(callee_name)
                .or_else(|| g.get_signature(simple))
                .is_some_and(|rs| {
                    let pidx = rs.arg_param_index(arg_index);
                    callee_emits_shared_rust_ref_param(rs, pidx)
                })
        });
        if global_emits_ref {
            return false;
        }
        return true;
    }
    let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
    registry
        .get_signature(callee_name)
        .or_else(|| registry.get_signature(simple))
        .or_else(|| {
            global.and_then(|g| {
                g.get_signature(callee_name)
                    .or_else(|| g.get_signature(simple))
            })
        })
        .is_some_and(|rs| {
            let pidx = rs.arg_param_index(arg_index);
            check(rs, pidx)
        })
}

/// Lowering actions to apply to a generated argument expression string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CallSiteBorrowDecision {
    pub add_ref: bool,
    pub add_mut_ref: bool,
    pub strip_ref: bool,
    pub strip_clone: bool,
}

/// Effective ownership for call argument `arg_index`, honoring formal owned types.
pub fn effective_ownership_for_call_arg(
    sig: &FunctionSignature,
    arg_index: usize,
) -> OwnershipMode {
    effective_param_ownership_for_arg(sig, arg_index)
}

/// Stale metadata: body/converged `Borrowed` on a bare owned **Copy** formal (e.g. MannequinConfig).
/// Non-copy formals (Vec<AABB>) with converged borrow are real borrows — not stale.
pub fn is_stale_borrow_on_owned_copy_formal(sig: &FunctionSignature, arg_index: usize) -> bool {
    let param_idx = sig.arg_param_index(arg_index);
    let ownership = effective_ownership_for_call_arg(sig, arg_index);
    if ownership != OwnershipMode::Borrowed {
        return false;
    }
    if !param_type_is_owned_non_text(sig, param_idx) {
        return false;
    }
    sig.formal_param_type(param_idx)
        .is_some_and(type_analysis_pure::is_copy_type)
}

fn is_collection_key_arg(
    sig: &FunctionSignature,
    arg_index: usize,
    receiver_type: Option<&str>,
) -> bool {
    stdlib_method_traits::is_collection_key_lookup(sig, arg_index, receiver_type)
}

pub fn expression_supports_shared_borrow_at_call_site(
    arg_expr: &Expression,
    arg_str: &str,
) -> bool {
    if expression_is_vec_macro_literal(arg_expr) {
        return true;
    }
    if matches!(
        arg_expr,
        Expression::Identifier { .. }
            | Expression::FieldAccess { .. }
            | Expression::Index { .. }
    ) {
        return true;
    }
    // Constructor / factory rvalues (`QuestId::from_u32(n)`) borrow to `&T` formals.
    if matches!(
        arg_expr,
        Expression::Call { .. } | Expression::MethodCall { .. }
    ) && !expression_is_copy_literal(arg_expr)
    {
        return true;
    }
    // Owned String producers (`i32.to_string()`, etc.) passed to `&str` formals.
    arg_str.ends_with(".to_string()") || arg_str.ends_with(".to_owned()")
}

pub fn expression_is_copy_literal(arg_expr: &Expression) -> bool {
    matches!(
        arg_expr,
        Expression::Literal {
            value: Literal::Int(_)
                | Literal::IntSuffixed(_, _)
                | Literal::Float(_)
                | Literal::Bool(_),
            ..
        }
    )
}

pub fn expression_is_string_literal(arg_expr: &Expression) -> bool {
    matches!(
        arg_expr,
        Expression::Literal {
            value: Literal::String(_),
            ..
        }
    ) || matches!(
        arg_expr,
        Expression::MethodCall { method, object, .. }
        if method.as_str() == "to_string" || method.as_str() == "string"
            && matches!(
            &**object,
            Expression::Literal { value: Literal::String(_), .. }
        )
    )
}

/// Identifier (or `&ident`) at the root of a call argument expression.
pub fn borrow_target_identifier_name(arg_expr: &Expression) -> Option<String> {
    match arg_expr {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::Unary {
            op: crate::parser::UnaryOp::Ref,
            operand,
            ..
        } => match &**operand {
            Expression::Identifier { name, .. } => Some(name.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Drop a user-written `&` when the binding already emits as a shared Rust reference.
pub fn strip_redundant_borrow_on_ref_binding(arg_expr: &Expression, arg_str: &mut String) {
    let Some(name) = borrow_target_identifier_name(arg_expr) else {
        return;
    };
    if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
        let base = crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str);
        if base == name.as_str() {
            *arg_str = name;
        }
    }
}

/// Strip a leading `&ident` only when the binding already lowers as a Rust shared ref
/// (`key: &str` → `map.get(key)`, never `map.get(&key)` / `&&str`).
pub fn strip_double_ref_on_shared_binding(
    arg_expr: &Expression,
    arg_str: &mut String,
    binding_already_shared_ref: bool,
) {
    if !binding_already_shared_ref {
        return;
    }
    strip_redundant_borrow_on_ref_binding(arg_expr, arg_str);
}

/// Decide call-site borrow lowering from effective ownership, formal types, and the argument.
pub fn should_borrow_at_call_site(
    sig: &FunctionSignature,
    arg_index: usize,
    arg_expr: &Expression,
    arg_str: &str,
    method_name: &str,
    arg_already_rust_ref: bool,
    receiver_type: Option<&str>,
) -> CallSiteBorrowDecision {
    should_borrow_at_call_site_with_copy_check(
        sig,
        arg_index,
        arg_expr,
        arg_str,
        method_name,
        arg_already_rust_ref,
        receiver_type,
        false,
    )
}

/// Same as `should_borrow_at_call_site` but with an explicit `formal_type_is_copy` flag.
///
/// When the formal parameter type is a known Copy type (from the copy types registry),
/// the call site should NOT add `&` — Rust passes Copy types by value automatically.
/// Collection key lookups (`HashMap::get`) are an exception: they always need `&K`.
pub fn should_borrow_at_call_site_with_copy_check(
    sig: &FunctionSignature,
    arg_index: usize,
    arg_expr: &Expression,
    arg_str: &str,
    method_name: &str,
    arg_already_rust_ref: bool,
    receiver_type: Option<&str>,
    formal_type_is_copy: bool,
) -> CallSiteBorrowDecision {
    let param_idx = sig.arg_param_index(arg_index);
    let effective = effective_ownership_for_call_arg(sig, arg_index);
    let is_collection_key = is_collection_key_arg(sig, arg_index, receiver_type);

    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx)
    {
        return CallSiteBorrowDecision::default();
    }

    // Already allocated an owned String from a literal — never wrap in `&`
    // (`&"lit".to_string()`). Owned formals want the allocation bare; `&str`
    // formals should have kept the bare literal instead.
    if expression_is_string_literal(arg_expr)
        && (arg_str.ends_with(".to_string()") || arg_str.ends_with(".to_owned()"))
    {
        return CallSiteBorrowDecision::default();
    }

    // Phase-3 registry wrap: `param_types` Reference/MutableReference is the emitted Rust contract.
    if let Some(param_ty) = sig.param_types.get(param_idx) {
        let registry_ref_is_emitted = matches!(param_ty, Type::MutableReference(_))
            || callee_emits_shared_rust_ref_param(sig, param_idx);
        if registry_ref_is_emitted {
            if arg_str.starts_with("&") || arg_already_rust_ref {
                return CallSiteBorrowDecision::default();
            }
            match param_ty {
                Type::Reference(inner) => {
                    if expression_is_string_literal(arg_expr)
                        && string_utilities::param_is_rust_str_ref(param_ty)
                    {
                        return CallSiteBorrowDecision::default();
                    }
                    if expression_is_copy_literal(arg_expr) {
                        return CallSiteBorrowDecision::default();
                    }
                    if expression_supports_shared_borrow_at_call_site(arg_expr, arg_str) {
                        return CallSiteBorrowDecision {
                            add_ref: true,
                            ..Default::default()
                        };
                    }
                }
                Type::MutableReference(_) => {
                    if expression_is_copy_literal(arg_expr)
                        || expression_is_string_literal(arg_expr)
                    {
                        return CallSiteBorrowDecision::default();
                    }
                    if expression_supports_shared_borrow_at_call_site(arg_expr, arg_str) {
                        return CallSiteBorrowDecision {
                            add_mut_ref: true,
                            strip_clone: arg_str.ends_with(".clone()"),
                            ..Default::default()
                        };
                    }
                }
                _ => {}
            }
        }
    }

    let param_expects_mut_borrowed = matches!(effective, OwnershipMode::MutBorrowed);

    // &mut parameters: insert `&mut` even for Copy formals (e.g. `increment(&mut counter)`).
    // Trust `effective_param_ownership` — raw `param_ownership` MutBorrowed can be stale
    // (self-slot leakage / multipass) while effective correctly reports Owned.
    if param_expects_mut_borrowed {
        if arg_str.starts_with("&mut ") || arg_str.starts_with("&") {
            return CallSiteBorrowDecision::default();
        }
        // String literals are never `&mut` lvalues; owned String formals use `.to_string()`.
        if expression_is_string_literal(arg_expr)
            || expression_is_copy_literal(arg_expr)
            || arg_already_rust_ref
        {
            return CallSiteBorrowDecision::default();
        }
        return CallSiteBorrowDecision {
            add_mut_ref: true,
            strip_clone: arg_str.ends_with(".clone()"),
            ..Default::default()
        };
    }

    // Copy formal types: pass by value, don't borrow (unless collection key lookup).
    if formal_type_is_copy && !is_collection_key {
        return CallSiteBorrowDecision::default();
    }

    // Plain owned `string` formals pass by value until codegen confirms an emitted `&str` formal.
    // Stale multipass `Reference(str)` / Borrowed metadata must not add `&` (circular deps).
    if plain_string_formal_passes_owned_at_call_site(sig, param_idx)
        && !is_collection_key
        && !matches!(arg_expr, Expression::FieldAccess { .. })
    {
        return CallSiteBorrowDecision::default();
    }

    let callee_emits_rust_ref = callee_emits_shared_rust_ref_param(sig, param_idx);
        // Stale Owned metadata must not suppress map/set key auto-borrow (`HashMap::get(&k)`).
    // Also must not suppress when param_types already encodes Reference(T) (registry wrap)
    // or codegen refresh recorded an emitted `&str`/`&T` formal (`emitted_rust_ref_params`).
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Owned)
    ) && effective == OwnershipMode::Owned
        && !is_collection_key
        && !callee_emits_rust_ref
        && !sig
            .param_types
            .get(param_idx)
            .is_some_and(string_utilities::param_is_rust_str_ref)
        && sig
            .param_types
            .get(param_idx)
            .is_none_or(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)))
        && !matches!(arg_expr, Expression::FieldAccess { .. })
    {
        return CallSiteBorrowDecision::default();
    }

    let bridge_expects_shared =
        crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, param_idx);
    let bridge_expects_owned =
        crate::ir::signature_bridge::call_site_expects_owned_pass(sig, param_idx);

    if bridge_expects_owned {
        return CallSiteBorrowDecision::default();
    }

    let formal_plain_string = sig
        .formal_param_type(param_idx)
        .is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                && types::is_windjammer_text_type(t)
        });
    let param_expects_borrowed = if formal_plain_string {
        bridge_expects_shared
    } else {
        bridge_expects_shared
            || matches!(effective, OwnershipMode::Borrowed)
            || matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed)
            )
            || (sig.param_types.get(param_idx).is_some_and(|t| {
                matches!(t, Type::Reference(_) | Type::MutableReference(_))
            }) && callee_emits_rust_ref
                && sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    != Some(false))
            || callee_emits_rust_ref
    };

    let mut decision = CallSiteBorrowDecision::default();

    let user_facing_param = |idx: usize| {
        if sig.has_self_receiver_slot() {
            idx > 0
        } else {
            true
        }
    };
    let callee_has_ref_user_param = sig.param_types.iter().enumerate().any(|(idx, t)| {
        user_facing_param(idx) && matches!(t, Type::Reference(_) | Type::MutableReference(_))
    });

    if arg_str.ends_with(".clone()")
        && (param_expects_borrowed || param_expects_mut_borrowed || callee_has_ref_user_param)
    {
        decision.strip_clone = true;
    }
    if is_collection_key && arg_str.ends_with(".clone()") {
        decision.strip_clone = true;
    }

    if arg_str.starts_with('&') {
        return decision;
    }

    if expression_is_copy_literal(arg_expr) {
        return decision;
    }

    if matches!(arg_expr, Expression::StructLiteral { .. }) {
        return decision;
    }

    let arg_is_copy = expression_is_copy_literal(arg_expr);

    if arg_already_rust_ref {
        return decision;
    }

    if !(param_expects_borrowed || is_collection_key) {
        return decision;
    }

    // Copy keys on map/set lookups still need `&K` (HashMap::get(&u32)).
    if arg_is_copy && !is_collection_key {
        return decision;
    }

    if param_expects_borrowed {
        let param_idx = sig.arg_param_index(arg_index);
        let param_is_str_ref = sig
            .param_types
            .get(param_idx)
            .is_some_and(string_utilities::param_is_rust_str_ref);
        let arg_is_string_literal = expression_is_string_literal(arg_expr);
        if param_is_str_ref {
            // Literals coerce to `&str`; owned locals/fields still need `&`.
            if !arg_is_string_literal {
                decision.add_ref = true;
            }
            return decision;
        }
        if !arg_is_string_literal && !(is_collection_key && arg_already_rust_ref) {
            decision.add_ref = true;
        }
    } else if is_collection_key {
        if sig
            .param_type_for_arg(arg_index)
            .is_some_and(types::is_windjammer_text_type)
        {
            if expression_is_string_literal(arg_expr) || arg_already_rust_ref {
                return decision;
            }
            decision.add_ref = true;
        } else if sig
            .param_type_for_arg(arg_index)
            .is_some_and(|t| !types::is_windjammer_text_type(t))
        {
            decision.add_ref = true;
        }
    }

    decision
}

/// Final pass: map/set lookup keys need `&K` at the Rust call site.
pub fn finalize_collection_key_call_site_arg(
    sig: Option<&FunctionSignature>,
    arg_index: usize,
    arg_expr: &Expression,
    arg_str: &mut String,
    arg_already_rust_ref: bool,
    receiver_type: Option<&str>,
    arg_binding_already_shared_ref: bool,
) {
    let Some(sig) = sig else {
        return;
    };
    if arg_binding_already_shared_ref
        || !is_collection_key_arg(sig, arg_index, receiver_type)
        || arg_str.starts_with('&')
    {
        return;
    }
    if expression_is_string_literal(arg_expr) || expression_is_copy_literal(arg_expr) {
        return;
    }
    if arg_already_rust_ref {
        return;
    }
    if matches!(arg_expr, Expression::Cast { .. }) {
        *arg_str = format!("&({arg_str})");
    } else {
        crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(arg_str);
    }
}

/// Apply [`CallSiteBorrowDecision`] to generated argument Rust source.
pub fn apply_call_site_borrow(decision: &CallSiteBorrowDecision, arg_str: &mut String) {
    if decision.strip_ref {
        if arg_str.starts_with("&mut ") {
            *arg_str = arg_str["&mut ".len()..].to_string();
        } else if arg_str.starts_with('&') {
            *arg_str = arg_str[1..].to_string();
        }
    }

    if decision.strip_clone {
        expression_utilities::strip_trailing_clone(arg_str);
    }

    if decision.add_mut_ref {
        if !arg_str.starts_with("&mut ") {
            Coercion::BorrowMut.apply(arg_str);
        }
    }

    if decision.add_ref && !arg_str.starts_with('&') {
        crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(arg_str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::OwnershipMode;
    use crate::parser::Type;

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
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    #[test]
    fn owned_formal_copy_struct_no_borrow() {
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
        let decision = should_borrow_at_call_site(&sig, 0, &arg, "config", "generate", false, None);
        assert!(!decision.add_ref, "owned Copy formal must not add &");
        assert_eq!(
            effective_ownership_for_call_arg(&sig, 0),
            OwnershipMode::Owned
        );
    }

    #[test]
    fn owned_string_formal_no_borrow_despite_stale_borrowed_param_type() {
        let sig = sig_with_formal(
            "TrialBalanceReader::trial_balance_lines",
            vec![Type::Reference(Box::new(Type::String))],
            vec![Type::String],
            vec![OwnershipMode::Borrowed],
            true,
        );
        let arg = Expression::Identifier {
            name: "tenant_slug".into(),
            location: Default::default(),
        };
        let decision = should_borrow_at_call_site(
            &sig,
            0,
            &arg,
            "tenant_slug.clone()",
            "trial_balance_lines",
            false,
            None,
        );
        assert!(
            !decision.add_ref,
            "owned string formal must not borrow at call site"
        );
    }

    #[test]
    fn vec_formal_honors_converged_borrow() {
        let elem = Type::Custom("AABB".into());
        let vec_ty = Type::Parameterized("Vec".into(), vec![elem]);
        let sig = sig_with_formal(
            "check_collisions",
            vec![vec_ty.clone()],
            vec![vec_ty],
            vec![OwnershipMode::Borrowed],
            false,
        );
        let arg = Expression::Identifier {
            name: "walls".into(),
            location: Default::default(),
        };
        let decision =
            should_borrow_at_call_site(&sig, 0, &arg, "walls", "check_collisions", false, None);
        assert!(
            decision.add_ref,
            "Vec formal with converged borrow must add &"
        );
        assert_eq!(
            effective_ownership_for_call_arg(&sig, 0),
            OwnershipMode::Borrowed
        );
    }

    #[test]
    fn borrowed_reference_param_adds_borrow() {
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
        let decision =
            should_borrow_at_call_site(&sig, 0, &arg, "quest_id", "is_quest_active", false, None);
        assert!(decision.add_ref, "converged borrow must add &");
    }

    #[test]
    fn registry_str_ref_param_emits_shared_borrow() {
        let sig = sig_with_formal(
            "TextBuffer::append_slice",
            vec![
                Type::Custom("TextBuffer".into()),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            vec![Type::Custom("TextBuffer".into()), Type::String],
            vec![OwnershipMode::MutBorrowed, OwnershipMode::Borrowed],
            true,
        );
        let pidx = sig.arg_param_index(0);
        assert!(
            callee_emits_shared_rust_ref_param(&sig, pidx),
            "Reference(str) + Borrowed ownership must emit shared borrow at call sites"
        );
    }

    #[test]
    fn borrowed_string_formal_without_codegen_confirmation_passes_owned() {
        let sig = sig_with_formal(
            "accept_label",
            vec![Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![Type::String],
            vec![OwnershipMode::Borrowed],
            false,
        );
        assert!(
            !callee_emits_shared_rust_ref_param(&sig, 0),
            "stale borrow metadata alone must not force shared ref"
        );
        let arg = Expression::Identifier {
            name: "label".into(),
            location: Default::default(),
        };
        let decision =
            should_borrow_at_call_site(&sig, 0, &arg, "label", "accept_label", false, None);
        assert!(
            !decision.add_ref,
            "plain string formal without emitted_rust_ref_params must pass owned"
        );
    }

    #[test]
    fn analyzer_bar_signature_skips_call_site_borrow() {
        use crate::analyzer::Analyzer;
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let source = r#"
fn foo(x: string) -> bool {
    if x == "stop" { true } else { bar(x) }
}
fn bar(y: string) -> bool {
    if y == "stop" { false } else { foo(y) }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (_, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let bar = registry.get_signature("bar").expect("bar");
        let arg = Expression::Identifier {
            name: "x".into(),
            location: Default::default(),
        };
        assert!(
            plain_string_formal_passes_owned_at_call_site(bar, 0),
            "formal={:?} param_types={:?}",
            bar.formal_param_types,
            bar.param_types
        );
        let decision = should_borrow_at_call_site(&bar, 0, &arg, "x", "bar", false, None);
        assert!(!decision.add_ref);
    }

    #[test]
    fn circular_dep_plain_string_no_call_site_borrow() {
        let sig = sig_with_formal(
            "bar",
            vec![Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![Type::String],
            vec![OwnershipMode::Borrowed],
            false,
        );
        let arg = Expression::Identifier {
            name: "x".into(),
            location: Default::default(),
        };
        let decision = should_borrow_at_call_site(&sig, 0, &arg, "x", "bar", false, None);
        assert!(
            !decision.add_ref,
            "mutual-recursion stale borrow must not emit bar(&x)"
        );
    }

    #[test]
    fn copy_scalar_owned_param_no_borrow() {
        let sig = sig_with_formal(
            "IntList::append",
            vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
            vec![Type::Custom("Self".into()), Type::Custom("i32".into())],
            vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            true,
        );
        let arg = Expression::Literal {
            value: Literal::Int(42),
            location: Default::default(),
        };
        let decision =
            should_borrow_at_call_site(&sig, 0, &arg, "42", "append", false, Some("IntList"));
        assert!(!decision.add_ref, "Copy scalar literal to owned param must not add &");
    }

    #[test]
    fn string_literal_to_str_param_no_extra_borrow() {
        let sig = sig_with_formal(
            "write_line",
            vec![Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![Type::Custom("string".into())],
            vec![OwnershipMode::Borrowed],
            false,
        );
        let arg = Expression::Literal {
            value: Literal::String("hello".into()),
            location: Default::default(),
        };
        let decision =
            should_borrow_at_call_site(&sig, 0, &arg, "\"hello\"", "write_line", false, None);
        assert!(
            !decision.add_ref,
            "string literal to &str must not add extra &"
        );
    }

    #[test]
    fn copy_key_reference_formal_still_borrows() {
        let sig = sig_with_formal(
            "SymbolTable::resolve",
            vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("u32".into()))),
            ],
            vec![Type::Custom("Self".into()), Type::Custom("u32".into())],
            vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            true,
        );
        let arg = Expression::Identifier {
            name: "node".into(),
            location: Default::default(),
        };
        let decision = should_borrow_at_call_site(
            &sig,
            0,
            &arg,
            "node",
            "resolve",
            false,
            Some("SymbolTable"),
        );
        assert!(
            decision.add_ref,
            "Copy key with Reference formal must still get & at call site"
        );
    }

    #[test]
    fn registry_reference_param_borrows_owned_local() {
        let map_ty = Type::Parameterized(
            "HashMap".into(),
            vec![Type::Custom("i64".into()), Type::String],
        );
        let sig = sig_with_formal(
            "get_user_name",
            vec![Type::Reference(Box::new(map_ty)), Type::Custom("i64".into())],
            vec![Type::Parameterized(
                "HashMap".into(),
                vec![Type::Custom("i64".into()), Type::String],
            ), Type::Custom("i64".into())],
            vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            false,
        );
        let arg = Expression::Identifier {
            name: "users".into(),
            location: Default::default(),
        };
        let decision = should_borrow_at_call_site(&sig, 0, &arg, "users", "get_user_name", false, None);
        assert!(decision.add_ref, "Reference(HashMap) formal must borrow owned local");
    }

    #[test]
    fn apply_strip_clone_then_borrow() {
        let mut arg = "value.clone()".to_string();
        let decision = CallSiteBorrowDecision {
            add_ref: true,
            strip_clone: true,
            ..Default::default()
        };
        apply_call_site_borrow(&decision, &mut arg);
        assert_eq!(arg, "&value");
    }

    #[test]
    fn stale_mut_borrowed_ownership_does_not_mut_ref_owned_string_literal() {
        let sig = FunctionSignature {
            name: "Catalog::register_table".into(),
            param_types: vec![
                Type::MutableReference(Box::new(Type::Custom("Self".into()))),
                Type::Custom("Catalog".into()),
                Type::Int,
            ],
            formal_param_types: vec![
                Type::Custom("Self".into()),
                Type::String,
                Type::Int,
            ],
            param_ownership: vec![
                OwnershipMode::MutBorrowed,
                OwnershipMode::MutBorrowed,
                OwnershipMode::Owned,
            ],
            return_type: Some(Type::Int),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let arg = Expression::Literal {
            value: Literal::String("users".into()),
            location: Default::default(),
        };
        let decision = should_borrow_at_call_site(
            &sig,
            0,
            &arg,
            r#""users""#,
            "register_table",
            false,
            Some("Catalog"),
        );
        assert!(
            !decision.add_mut_ref,
            "stale MutBorrowed on string formal must not emit &mut on string literal; effective={:?}",
            effective_ownership_for_call_arg(&sig, 0)
        );
    }
}
