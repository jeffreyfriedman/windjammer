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

fn is_collection_key_arg(method_name: &str, arg_index: usize, receiver_type: Option<&str>) -> bool {
    if arg_index != 0 {
        return false;
    }
    let is_key_method = stdlib_method_traits::is_map_key_method(method_name)
        || stdlib_method_traits::is_set_lookup_method(method_name);
    if !is_key_method {
        return false;
    }
    if let Some(rt) = receiver_type {
        let base = rt.split('<').next().unwrap_or(rt);
        return stdlib_method_traits::is_map_type_name(base)
            || stdlib_method_traits::is_set_type_name(base);
    }
    // When receiver type is unknown, don't assume collection key —
    // the method signature's ownership mode handles borrowing correctly.
    false
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
        if method.as_str() == "to_string" && matches!(
            &**object,
            Expression::Literal { value: Literal::String(_), .. }
        )
    )
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
    let param_idx = sig.arg_param_index(arg_index);
    let effective = effective_ownership_for_call_arg(sig, arg_index);
    let is_collection_key = is_collection_key_arg(method_name, arg_index, receiver_type);

    // Plain owned `string` formals pass by value unless body analysis converged to borrow.
    if sig.formal_param_type(param_idx).is_some_and(|t| {
        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            && types::is_windjammer_text_type(t)
    }) && effective == OwnershipMode::Owned
    {
        return CallSiteBorrowDecision::default();
    }
    // Stale Owned metadata must not suppress map/set key auto-borrow (`HashMap::get(&k)`).
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Owned)
    ) && effective == OwnershipMode::Owned
        && !is_collection_key
    {
        return CallSiteBorrowDecision::default();
    }

    let param_expects_borrowed = matches!(effective, OwnershipMode::Borrowed);
    let param_expects_mut_borrowed = matches!(effective, OwnershipMode::MutBorrowed);

    let mut decision = CallSiteBorrowDecision::default();

    // &mut parameters: insert `&mut` at call site (skip literals and already-ref args).
    if param_expects_mut_borrowed {
        if arg_str.starts_with("&mut ") || arg_str.starts_with("&") {
            return decision;
        }
        if expression_is_copy_literal(arg_expr) || arg_already_rust_ref {
            return decision;
        }
        decision.add_mut_ref = true;
        return decision;
    }

    if param_expects_borrowed && arg_str.ends_with(".clone()") {
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
            return decision;
        }
        if sig
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
    method_name: &str,
    arg_index: usize,
    arg_expr: &Expression,
    arg_str: &mut String,
    arg_already_rust_ref: bool,
    receiver_type: Option<&str>,
) {
    if !is_collection_key_arg(method_name, arg_index, receiver_type) || arg_str.starts_with('&') {
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
        Coercion::BorrowMut.apply(arg_str);
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
    fn copy_scalar_i32_no_borrow() {
        let sig = sig_with_formal(
            "Vec::push",
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
            should_borrow_at_call_site(&sig, 0, &arg, "42", "push", false, Some("Vec"));
        assert!(!decision.add_ref, "Copy scalar literal must not add &");
    }

    #[test]
    fn string_literal_to_str_param_no_extra_borrow() {
        let sig = sig_with_formal(
            "println",
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
            should_borrow_at_call_site(&sig, 0, &arg, "\"hello\"", "println", false, None);
        assert!(
            !decision.add_ref,
            "string literal to &str must not add extra &"
        );
    }

    #[test]
    fn copy_map_key_still_borrows() {
        let sig = sig_with_formal(
            "HashMap::get",
            vec![Type::Custom("Self".into()), Type::Custom("u32".into())],
            vec![Type::Custom("Self".into()), Type::Custom("u32".into())],
            vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            true,
        );
        let arg = Expression::Identifier {
            name: "node".into(),
            location: Default::default(),
        };
        let decision =
            should_borrow_at_call_site(&sig, 0, &arg, "node", "get", false, Some("HashMap"));
        assert!(
            decision.add_ref,
            "Copy map keys must still get & at call site"
        );
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
}
