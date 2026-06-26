//! String Utilities
//!
//! Helper functions for string type analysis and codegen decisions.
//! These are pure functions with no state dependencies.

use crate::parser::{Expression, Literal, Statement, Type};

/// Untyped `let`/`let mut` with string literal or string-producing `match` RHS needs `: String`
/// so `"x".into()` resolves (Rust cannot infer from `&str`-accepting call sites alone).
pub fn untyped_let_rhs_needs_string_ascription(value: &Expression) -> bool {
    match value {
        Expression::Literal {
            value: Literal::String(s),
            ..
        } => !s.is_empty(),
        Expression::Block { statements, .. } => statements.iter().any(|stmt| match stmt {
            Statement::Match { arms, .. } => arms
                .iter()
                .any(|arm| match_arm_needs_string_ascription(arm.body)),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                block_tail_is_string_producing(then_block)
                    && else_block
                        .as_ref()
                        .is_some_and(|b| block_tail_is_string_producing(b))
            }
            Statement::Expression { expr, .. } => match_arm_needs_string_ascription(expr),
            _ => false,
        }),
        _ => false,
    }
}

fn block_tail_is_string_producing(stmts: &[&Statement]) -> bool {
    stmts
        .last()
        .is_some_and(|s| statement_tail_is_string_producing(s))
}

fn statement_tail_is_string_producing(stmt: &Statement) -> bool {
    match stmt {
        Statement::Expression { expr, .. } => match_arm_needs_string_ascription(expr),
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            block_tail_is_string_producing(then_block)
                && else_block
                    .as_ref()
                    .is_some_and(|b| block_tail_is_string_producing(b))
        }
        _ => false,
    }
}

pub fn match_arm_needs_string_ascription(body: &Expression) -> bool {
    matches!(
        body,
        Expression::Literal {
            value: Literal::String(s),
            ..
        } if !s.is_empty()
    ) || crate::codegen::rust::string_analysis::expression_produces_string(body)
        || crate::codegen::rust::arm_string_analysis::arm_returns_converted_string(body)
}

/// Check if return type expects owned String in Rust.
/// Enclosing function/slot expects owned `String` in Rust (`string` / `String` in Windjammer).
pub fn return_type_expects_owned_string(ret: &Option<Type>) -> bool {
    match ret {
        Some(Type::String) => true,
        Some(Type::Custom(n)) if n == "String" || n == "string" => true,
        _ => false,
    }
}

/// Generated Rust already produces an owned `String` (no second conversion pass).
pub fn already_owned_string_expr(expr_str: &str) -> bool {
    expr_str.ends_with(".to_string()")
        || expr_str.ends_with(".into()")
        || expr_str.ends_with(".clone()")
        || expr_str.starts_with("String::from(")
        || expr_str == "String::new()"
}

/// Idempotent: coerce a generated expression to owned `String` without `.to_string()` leakage.
pub fn coerce_expr_to_owned_string(expr_str: &str) -> String {
    if already_owned_string_expr(expr_str) {
        return expr_str.to_string();
    }
    if expr_str.starts_with('"') {
        return crate::codegen::rust::literals::string_literal_to_owned_rust(expr_str);
    }
    format!("{}.to_string()", expr_str)
}

// =============================================================================
// Shared call-site string coercion predicates
//
// These are used by the three argument-lowering pipelines:
//   regular_call_arguments.rs, method_call_expression_generation/arguments.rs,
//   field_access_method_args.rs
// =============================================================================

/// Parameter type is explicitly `&str` (not `&String`).
/// This indicates the callee wants a string slice — string literals can be passed directly.
pub fn param_is_rust_str_ref(param_type: &Type) -> bool {
    matches!(
        param_type,
        Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
    )
}

/// Parameter type is an owned Windjammer `string` / Rust `String`.
pub fn param_is_owned_string_type(param_type: &Type) -> bool {
    matches!(param_type, Type::String)
        || matches!(param_type, Type::Custom(n) if n == "string" || n == "String")
}

/// Whether a call-site expression should be borrowed for runtime std `AsRef<str>` APIs.
pub fn expression_is_owned_string_for_asref_borrow<'ast>(
    expr: &Expression<'ast>,
    inferred_type: Option<&Type>,
    local_var_types: &std::collections::HashMap<String, Type>,
    current_function_params: &[crate::parser::Parameter<'ast>],
) -> bool {
    if inferred_type.is_some_and(param_is_owned_string_type) {
        return true;
    }
    match expr {
        Expression::Identifier { name, .. } => {
            local_var_types
                .get(name)
                .is_some_and(param_is_owned_string_type)
                || current_function_params.iter().any(|p| {
                    p.name == *name && param_is_owned_string_type(&p.type_)
                })
        }
        Expression::FieldAccess { .. } => true,
        _ => false,
    }
}

/// Module-level `pub const …: string` identifiers that lower to `&'static str` in Rust.
pub fn is_windjammer_string_const_name(name: &str) -> bool {
    name.starts_with("SCOPE_")
        || name.starts_with("AUDIT_")
        || name.starts_with("PERIOD_STATUS_")
}

/// Identifier is a string constant (`SCOPE_*` or a variable bound to a string literal).
pub fn is_string_const_identifier(
    name: &str,
    auto_clone: Option<&crate::auto_clone::AutoCloneAnalysis>,
) -> bool {
    is_windjammer_string_const_name(name)
        || auto_clone.is_some_and(|a| a.string_literal_vars.contains(name))
}

/// Callee borrows a string parameter: Rust will receive `&str` or `&String`.
/// True when the signature explicitly marks the param as `Borrowed`, or when
/// no ownership metadata exists but the param type is a Windjammer text type
/// (default borrow for `string` params in non-extern functions).
pub fn callee_borrows_string_param(
    sig: &crate::analyzer::FunctionSignature,
    sig_param_idx: usize,
) -> bool {
    if sig.is_extern {
        return false;
    }
    matches!(
        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
            sig, sig_param_idx,
        ),
        crate::analyzer::OwnershipMode::Borrowed
    )
}

/// Engine `Blackboard` stores keys as borrowed `&str` at the API boundary (see game-core gen).
fn is_blackboard_borrowed_key_method(
    receiver_type: &str,
    method: &str,
    arg_index: usize,
) -> bool {
    receiver_type == "Blackboard"
        && arg_index == 0
        && matches!(
            method,
            "set_bool" | "set_f32" | "set_i32" | "set_string" | "get_bool" | "get_f32"
                | "get_i32" | "get_string" | "find_index"
        )
}

/// Read-only lookup APIs (`get_*`, map `get`, BT conditions, etc.) lower string keys to
/// `&str` in Rust even when stale metadata still lists owned `String`.
pub fn is_readonly_string_key_method(method: &str, arg_index: usize) -> bool {
    if arg_index != 0 {
        return false;
    }
    method.starts_with("get_")
        || matches!(
            method,
            "get" | "contains" | "contains_key" | "has" | "has_key" | "has_value" | "find_index"
                | "add_condition" | "add_action" | "remove"
        )
}

/// Enum variant constructor arg (e.g. `QuestReward::relationship` → `Relationship(string, i32)`).
pub fn enum_factory_string_param_needs_owned(
    enum_variant_types: &std::collections::HashMap<String, Vec<Type>>,
    receiver_type: &str,
    method: &str,
    arg_index: usize,
) -> bool {
    let mut variant = String::new();
    if let Some(first) = method.chars().next() {
        variant.push(first.to_ascii_uppercase());
        variant.push_str(&method[first.len_utf8()..]);
    }
    let key = format!("{receiver_type}::{variant}");
    let method_key = format!("{receiver_type}::{method}");
    for lookup in [&key, &method_key] {
        if enum_variant_types
            .get(lookup)
            .and_then(|ts| ts.get(arg_index))
            .is_some_and(param_is_owned_string_type)
        {
            return true;
        }
    }
    false
}

/// Whether a string literal at this call site should become owned (`".to_string()"` / `into()`).
pub fn string_literal_needs_owned_coercion(
    sig: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
    method: Option<&str>,
) -> bool {
    string_literal_needs_owned_coercion_with_enum(sig, arg_index, method, None, None, None)
}

/// Whether a string literal at this call site should become owned (`".to_string()"` / `into()`).
pub fn string_literal_needs_owned_coercion_with_enum(
    sig: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
    method: Option<&str>,
    receiver_type: Option<&str>,
    enum_variant_types: Option<&std::collections::HashMap<String, Vec<Type>>>,
    runtime_module: Option<&str>,
) -> bool {
    if runtime_module
        .is_some_and(crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str)
    {
        return false;
    }

    // Runtime `strings::*` / String search APIs: pattern args are `&str` in Rust (arg 1+).
    if let Some(m) = method {
        if arg_index >= 1
            && matches!(
                m,
                "starts_with" | "ends_with" | "contains" | "replace" | "replacen" | "split"
            )
        {
            return false;
        }
    }

    if let Some(m) = method {
        if let Some(tn) = receiver_type {
            if is_blackboard_borrowed_key_method(tn, m, arg_index) {
                return false;
            }
        }
        if crate::codegen::rust::stdlib_method_traits::is_map_key_method(m) && arg_index == 0 {
            return false;
        }
        if is_readonly_string_key_method(m, arg_index) {
            return false;
        }
        if matches!(
            m,
            "push" | "insert" | "extend" | "append" | "push_front" | "push_back" | "add"
                | "fill"
        ) && arg_index == 0
        {
            return true;
        }
    }

    let Some(sig) = sig else {
        if let (Some(m), Some(tn), Some(variants)) = (method, receiver_type, enum_variant_types) {
            return enum_factory_string_param_needs_owned(variants, tn, m, arg_index);
        }
        // Without a signature, do not coerce identifiers to `.to_string()` for `Type::new`.
        // String literals are handled by dedicated literal lowering once a signature is resolved.
        return false;
    };

    let idx = sig.arg_param_index(arg_index);
    let Some(param_type) = sig.param_types.get(idx) else {
        return false;
    };

    if !crate::codegen::rust::types::is_windjammer_text_type(param_type) {
        return false;
    }

    if let (Some(m), Some(tn), Some(variants)) = (method, receiver_type, enum_variant_types) {
        if enum_factory_string_param_needs_owned(variants, tn, m, arg_index) {
            return true;
        }
    }

    if param_is_rust_str_ref(param_type) {
        return false;
    }
    if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
        &Some(sig.clone()),
        idx,
    ) {
        return false;
    }

    if crate::codegen::rust::string_utilities::callee_borrows_string_param(sig, idx) {
        // Stale `Borrowed` on plain `string` must not suppress `.to_string()` when the
        // converged Rust formal is owned `String` (only `&str` / `&String` stay bare).
        if param_is_rust_str_ref(param_type)
            || crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                &Some(sig.clone()),
                idx,
            )
        {
            return false;
        }
    }

    // Rust formal is owned `String` — allocate even when stale metadata still says Borrowed.
    // Exception: static associated methods (`Squad::new`) with body-inferred borrow pass `&owned`.
    if param_is_owned_string_type(param_type) {
        if !sig.has_self_receiver
            && matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership(sig, idx),
                crate::analyzer::OwnershipMode::Borrowed | crate::analyzer::OwnershipMode::MutBorrowed
            )
        {
            return false;
        }
        return true;
    }

    if matches!(
        crate::codegen::rust::call_signature_resolution::effective_param_ownership(sig, idx),
        crate::analyzer::OwnershipMode::Owned
    ) {
        return true;
    }

    false
}

/// Final pass: callee expects `&str` — pass owned locals/fields as `&expr`, never `.to_string()`.
pub fn finalize_borrowed_text_call_site_arg<'ast>(
    sig: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
    receiver_type: Option<&str>,
    arg: &Expression<'ast>,
    arg_str: &mut String,
) {
    use crate::analyzer::OwnershipMode;

    let Some(sig) = sig else {
        return;
    };

    let effective =
        if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(&sig.name)
        {
            let receiver = receiver_type.or_else(|| sig.name.rsplit_once("::").map(|(rt, _)| rt));
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                sig, arg_index, receiver,
            )
        } else {
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                sig, arg_index,
            )
        };

    if !matches!(effective, OwnershipMode::Borrowed) {
        return;
    }

    let param_is_text = sig.param_type_for_arg(arg_index).is_some_and(|t| {
        param_is_rust_str_ref(t) || crate::codegen::rust::types::is_windjammer_text_type(t)
    });
    if !param_is_text {
        return;
    }

    if arg_str.ends_with(".to_string()") {
        *arg_str = arg_str[..arg_str.len() - 12].to_string();
    } else if arg_str.ends_with(".into()") {
        *arg_str = arg_str[..arg_str.len() - 7].to_string();
    }

    if matches!(
        arg,
        Expression::Identifier { .. } | Expression::FieldAccess { .. }
    ) {
        crate::codegen::rust::expression_utilities::strip_trailing_clone(arg_str);
    }

    if matches!(
        arg,
        Expression::Identifier { .. }
            | Expression::FieldAccess { .. }
            | Expression::MethodCall { .. }
            | Expression::Index { .. }
    ) && !arg_str.starts_with('&')
        && !arg_str.starts_with("&mut ")
    {
        *arg_str = format!("&{arg_str}");
    }
}

/// Final pass: align string literal emission with [`string_literal_needs_owned_coercion`].
pub fn finalize_string_literal_call_site_arg<'ast>(
    sig: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
    method: Option<&str>,
    arg: &Expression<'ast>,
    arg_str: &mut String,
    receiver_type: Option<&str>,
    enum_variant_types: Option<&std::collections::HashMap<String, Vec<Type>>>,
    runtime_module: Option<&str>,
) {
    let is_string_literal = matches!(
        arg,
        Expression::Literal {
            value: Literal::String(_),
            ..
        }
    );
    if !is_string_literal {
        return;
    }

    let needs_owned = string_literal_needs_owned_coercion_with_enum(
        sig,
        arg_index,
        method,
        receiver_type,
        enum_variant_types,
        runtime_module,
    );
    if needs_owned {
        if !already_owned_string_expr(arg_str) {
            *arg_str = coerce_expr_to_owned_string(arg_str);
        }
    } else {
        if arg_str.ends_with(".to_string()") {
            *arg_str = arg_str[..arg_str.len() - 12].to_string();
        } else if arg_str.ends_with(".into()") {
            *arg_str = arg_str[..arg_str.len() - 7].to_string();
        }
        if arg_str.starts_with('&') {
            *arg_str = arg_str.trim_start_matches('&').to_string();
        }
    }
}

/// When `expr_str` ends with `.clone()` and the cloned identifier is a borrowed
/// string parameter, rewrite `.clone()` to `.to_string()`. Cloning a `&str`
/// produces another `&str`; `.to_string()` produces an owned `String`.
///
/// Returns `true` if a rewrite happened.
pub fn rewrite_borrowed_str_clone_to_to_string<'ast>(
    expr_str: &mut String,
    expr: &Expression<'ast>,
    borrowed_params: &std::collections::HashSet<String>,
    function_params: &[crate::parser::Parameter<'ast>],
) -> bool {
    if !expr_str.ends_with(".clone()") {
        return false;
    }
    let ident_name: Option<&str> = match expr {
        Expression::MethodCall { method, object, .. } if method == "clone" => match &**object {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            _ => None,
        },
        _ => None,
    };
    if let Some(name) = ident_name {
        let is_string_type = function_params.iter().any(|p| {
            p.name == name
                && (matches!(p.type_, Type::String)
                    || matches!(p.type_, Type::Custom(ref n) if n == "string"))
        });
        let is_borrowed = borrowed_params.contains(name);
        if is_borrowed && is_string_type {
            *expr_str = expr_str.replace(".clone()", ".to_string()");
            return true;
        }
    }
    false
}

/// Append `.as_str()` to a match scrutinee when the match contains string literal
/// patterns. Skips if the expression is already `&str` (a borrowed param or a
/// param typed as `string`/`str`/`&str`).
pub fn maybe_append_as_str_for_match(
    value_str: &str,
    borrowed_params: &std::collections::HashSet<String>,
    function_params: &[crate::parser::Parameter],
) -> String {
    if value_str.ends_with(".as_str()") {
        return value_str.to_string();
    }
    let is_already_str_ref = borrowed_params.contains(value_str)
        || function_params.iter().any(|p| {
            p.name == value_str
                && (matches!(p.type_, Type::String)
                    || matches!(p.type_, Type::Custom(ref n) if n == "str" || n == "string" || n == "&str"))
        });
    if is_already_str_ref {
        value_str.to_string()
    } else {
        format!("{}.as_str()", value_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_std_module_skips_literal_owned_coercion() {
        assert!(!string_literal_needs_owned_coercion_with_enum(
            None,
            1,
            Some("starts_with"),
            None,
            None,
            Some("strings"),
        ));
    }

    #[test]
    fn string_search_second_arg_literal_stays_bare() {
        assert!(!string_literal_needs_owned_coercion_with_enum(
            None,
            1,
            Some("starts_with"),
            None,
            None,
            None,
        ));
    }

    #[test]
    fn static_impl_borrowed_string_formal_does_not_coerce_identifiers_to_owned() {
        use crate::analyzer::{FunctionSignature, OwnershipMode};
        let sig = FunctionSignature {
            name: "new".into(),
            param_types: vec![Type::String, Type::String],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Squad".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        };
        assert!(
            !string_literal_needs_owned_coercion_with_enum(
                Some(&sig),
                0,
                Some("new"),
                Some("Squad"),
                None,
                None,
            ),
            "bare String + Borrowed static impl must not force owned coercion"
        );
    }

    #[test]
    fn finalize_borrowed_text_strips_clone_from_field_access_without_trailing_dot() {
        use crate::analyzer::{FunctionSignature, OwnershipMode};
        use crate::parser::Expression;

        let sig = FunctionSignature {
            name: "audit_canonical_payload".into(),
            param_types: vec![
                Type::String,
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Borrowed],
            return_type: Some(Type::String),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        };
        let event = Expression::Identifier {
            name: "event".into(),
            location: None,
        };
        let arg = Expression::FieldAccess {
            object: &event,
            field: "occurred_at".into(),
            location: None,
        };
        let mut arg_str = "event.occurred_at.clone()".to_string();
        finalize_borrowed_text_call_site_arg(Some(&sig), 1, None, &arg, &mut arg_str);
        assert_eq!(
            arg_str, "&event.occurred_at",
            "must strip .clone() fully (8 chars) before borrowing field access"
        );
    }

    #[test]
    fn finalize_borrowed_text_strips_to_string_and_borrows_owned_local() {
        use crate::analyzer::{FunctionSignature, OwnershipMode};
        use crate::parser::Expression;

        let sig = FunctionSignature {
            name: "Squad::new".into(),
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("str".into()))),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Squad".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        };
        let arg = Expression::Identifier {
            name: "squad_id".into(),
            location: None,
        };
        let mut arg_str = "squad_id.to_string()".to_string();
        finalize_borrowed_text_call_site_arg(Some(&sig), 0, Some("Squad"), &arg, &mut arg_str);
        assert_eq!(arg_str, "&squad_id");
    }

    #[test]
    fn impl_new_with_str_ref_sig_does_not_coerce_literals_to_owned() {
        use crate::analyzer::{FunctionSignature, OwnershipMode};
        let sig = FunctionSignature {
            name: "Squad::new".into(),
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("str".into()))),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Squad".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        };
        assert!(
            !string_literal_needs_owned_coercion_with_enum(
                Some(&sig),
                0,
                Some("new"),
                Some("Squad"),
                None,
                None,
            ),
            "static impl new(&str) must not use blind new→owned heuristic"
        );
    }
}
