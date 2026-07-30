//! String Utilities
//!
//! Helper functions for string type analysis and codegen decisions.
//! These are pure functions with no state dependencies.

use crate::analyzer::OwnershipMode;
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

/// True when `arg` is `expr.to_string()` / `expr.string()` where `expr` is **not** already
/// text — a genuine type conversion that must be preserved at the call site.
///
/// `"lit".to_string()` on a string literal is redundant when the callee takes `&str` and
/// must be stripped. Due to `||` vs `&&` precedence, this must be one predicate — do not
/// inline as `method == "to_string" || method == "string" && !literal`.
pub fn is_genuine_non_literal_to_string_conversion(arg: &Expression) -> bool {
    matches!(
        arg,
        Expression::MethodCall { object, method, .. }
            if (method.as_str() == "to_string" || method.as_str() == "string")
                && !matches!(
                    &**object,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                )
    )
}

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

/// Parameter type is `&String` — a reference to an owned String.
/// Distinct from `&str` (`param_is_rust_str_ref`). String literals passed to
/// `&String` params need `&"literal".to_string()` conversion.
pub fn param_is_rust_string_ref(param_type: &Type) -> bool {
    matches!(
        param_type,
        Type::Reference(inner) if param_is_owned_string_type(inner)
    )
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
                || current_function_params
                    .iter()
                    .any(|p| p.name == *name && param_is_owned_string_type(&p.type_))
        }
        Expression::FieldAccess { .. } => true,
        _ => false,
    }
}

/// Module-level `pub const …: string` identifiers that lower to `&'static str` in Rust.
pub fn is_windjammer_string_const_name(name: &str) -> bool {
    name.starts_with("SCOPE_") || name.starts_with("AUDIT_") || name.starts_with("PERIOD_STATUS_")
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
            sig,
            sig_param_idx,
        ),
        crate::analyzer::OwnershipMode::Borrowed
    )
}

/// Types whose read-only methods converge string keys to `&str`.
/// Prefer [`crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified`].
pub fn is_readonly_string_key_method(
    method: &str,
    arg_index: usize,
    sig: Option<&crate::analyzer::FunctionSignature>,
    receiver_type: Option<&str>,
    registry: Option<&crate::analyzer::SignatureRegistry>,
) -> bool {
    if let Some(sig) = sig {
        return crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
            sig, arg_index,
        );
    }
    if let Some(registry) = registry {
        return crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
            method,
            receiver_type,
            registry,
            arg_index,
        );
    }
    false
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
        if crate::codegen::rust::stdlib_method_traits::is_map_key_method(m) && arg_index == 0 {
            return false;
        }
    }

    let Some(sig) = sig else {
        if let (Some(m), Some(tn), Some(variants)) = (method, receiver_type, enum_variant_types) {
            if enum_factory_string_param_needs_owned(variants, tn, m, arg_index) {
                return true;
            }
        }
        // TEMPORARY: Cross-crate constructor fallback when no signature is available.
        // TODO: Remove once all constructors are reliably in the global registry.
        if matches!(method, Some("new" | "from")) && receiver_type.is_some() {
            return true;
        }
        return false;
    };

    let idx = sig.arg_param_index(arg_index);
    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, idx) {
        return false;
    }
    let Some(param_type) = sig.param_types.get(idx) else {
        // No type info available. Check ownership + method context:
        // Only convert if ownership says Owned AND we're confident this is a string param.
        let ownership = sig.param_ownership.get(idx);
        if matches!(ownership, Some(OwnershipMode::Owned)) {
            if let Some(ft) = sig.formal_param_types.get(idx) {
                if crate::codegen::rust::types::is_windjammer_text_type(ft) {
                    return true;
                }
            }
        }
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

    // Rust formal is owned `String` — allocate when the converged contract is owned, even if
    // stale borrow metadata lingers from multipass analysis.
    if param_is_owned_string_type(param_type) {
        if matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership(sig, idx),
            crate::analyzer::OwnershipMode::Owned,
        ) || sig
            .param_ownership
            .get(idx)
            .is_some_and(|o| matches!(o, OwnershipMode::Owned))
        {
            return true;
        }
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
    arg_already_rust_ref: bool,
) {
    use crate::analyzer::OwnershipMode;

    let Some(sig) = sig else {
        return;
    };

    let effective =
        if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
            &sig.name,
        ) {
            let receiver = receiver_type.or_else(|| sig.name.rsplit_once("::").map(|(rt, _)| rt));
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                sig, arg_index, receiver,
            )
        } else {
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                sig, arg_index,
            )
        };

    let param_idx = sig.arg_param_index(arg_index);
    let callee_emits_rust_ref = crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
        sig, param_idx,
    );
    let callee_expects_rust_borrow = callee_emits_rust_ref
        || (!crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            sig, param_idx,
        ) && (matches!(effective, OwnershipMode::Borrowed)
            || sig.param_types.get(param_idx).is_some_and(|t| {
                param_is_rust_str_ref(t) || matches!(t, Type::Reference(_))
            })));
    if !callee_expects_rust_borrow {
        return;
    }

    // `&str` formals, `&String` formals, plain WJ `string`, or codegen-emitted shared
    // text refs (`Reference(string)` from refresh — not only `Reference(str)`).
    let param_is_text = sig.param_type_for_arg(arg_index).is_some_and(|t| {
        param_is_rust_str_ref(t)
            || param_is_rust_string_ref(t)
            || crate::codegen::rust::types::is_windjammer_text_type(t)
            || matches!(
                t,
                Type::Reference(inner)
                    if crate::codegen::rust::types::is_windjammer_text_type(inner)
            )
    }) || (callee_emits_rust_ref
        && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            sig, param_idx,
        ));
    if !param_is_text {
        return;
    }

    // Plain owned `string` formals pass by value at call sites when the callee also
    // emits an owned Rust `String` param. When the callee converged to `&str`, still
    // add `&` for owned caller bindings (forward-ref / borrow-at-call-site).
    if sig.formal_param_type(param_idx).is_some_and(|t| {
        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            && crate::codegen::rust::types::is_windjammer_text_type(t)
    }) && !callee_emits_rust_ref
    {
        return;
    }

    // `&String` formals need `&"lit".to_string()` — never strip to a bare `&"lit"` (&str).
    // Exception: map-key methods (`contains_key`/`get`/…) accept `&Q where K: Borrow<Q>`,
    // so a bare `"lit"` (&str) is correct for `HashMap<String, _>` — do not allocate.
    let method_name = sig.name.rsplit("::").next().unwrap_or(sig.name.as_str());
    let is_map_key_lookup =
        crate::codegen::rust::stdlib_method_traits::is_map_key_method(method_name);
    let param_is_amp_string = !is_map_key_lookup
        && sig.param_types.get(param_idx).is_some_and(|t| {
            param_is_rust_string_ref(t)
                || (callee_emits_rust_ref
                    && crate::codegen::rust::types::is_windjammer_text_type(t)
                    && !param_is_rust_str_ref(t)
                    && !matches!(
                        t,
                        Type::Reference(inner) if matches!(&**inner, Type::Custom(n) if n == "str")
                    ))
        });
    if param_is_amp_string
        && matches!(
            arg,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        )
    {
        let base = arg_str.trim_start_matches('&');
        let owned = if base.ends_with(".to_string()") {
            base.to_string()
        } else {
            format!("{base}.to_string()")
        };
        *arg_str = format!("&{owned}");
        return;
    }
    // Map-key `&K` with K=String: keep bare string literals (Borrow<&str>).
    if is_map_key_lookup
        && matches!(
            arg,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        )
    {
        let base = arg_str.trim_start_matches('&');
        let bare = base.strip_suffix(".to_string()").unwrap_or(base);
        *arg_str = bare.to_string();
        return;
    }

    if !param_is_amp_string && !is_genuine_non_literal_to_string_conversion(arg) {
        if arg_str.ends_with(".to_string()") {
            *arg_str = arg_str[..arg_str.len() - 12].to_string();
        } else if arg_str.ends_with(".into()") {
            *arg_str = arg_str[..arg_str.len() - 7].to_string();
        }
    }

    if matches!(
        arg,
        Expression::Identifier { .. } | Expression::FieldAccess { .. }
    ) {
        crate::codegen::rust::expression_utilities::strip_trailing_clone(arg_str);
    }

    // String literals are already &str — adding & would create &&str.
    // After stripping .to_string(), a MethodCall like "lit".to_string() becomes "lit",
    // which is a bare string literal. Don't re-add & in that case.
    let is_bare_string_literal =
        arg_str.starts_with('"') || arg_str.starts_with("r\"") || arg_str.starts_with("r#\"");

    if matches!(
        arg,
        Expression::Identifier { .. }
            | Expression::FieldAccess { .. }
            | Expression::MethodCall { .. }
            | Expression::Index { .. }
    ) && !arg_str.starts_with('&')
        && !arg_str.starts_with("&mut ")
        && !is_bare_string_literal
        && !arg_already_rust_ref
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
        // &String param: string literal → &"lit".to_string()
        let is_string_ref_param = sig
            .and_then(|s| s.param_type_for_arg(arg_index))
            .is_some_and(param_is_rust_string_ref);
        if is_string_ref_param {
            let base = arg_str.trim_start_matches('&');
            let base = if base.ends_with(".to_string()") {
                base.to_string()
            } else {
                format!("{}.to_string()", base)
            };
            *arg_str = format!("&{}", base);
            return;
        }

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
/// Append `.as_str()` when matching an owned `String` against string-literal patterns.
/// Phase-2 / borrowed `&str` formals (tracked in `borrowed_params`) stay bare.
pub fn maybe_append_as_str_for_match(
    value_str: &str,
    borrowed_params: &std::collections::HashSet<String>,
    function_params: &[crate::parser::Parameter],
) -> String {
    if value_str.ends_with(".as_str()") {
        return value_str.to_string();
    }
    // Already a string slice binding — `match name { "x" => ... }` is valid.
    if borrowed_params.contains(value_str) {
        return value_str.to_string();
    }
    let param_is_str_slice = function_params.iter().any(|p| {
        p.name == value_str
            && (matches!(p.type_, Type::Custom(ref n) if n == "str" || n == "&str")
                || matches!(
                    p.type_,
                    Type::Reference(ref inner)
                        if matches!(&**inner, Type::Custom(n) if n == "str")
                ))
    });
    if param_is_str_slice {
        value_str.to_string()
    } else {
        // Owned `String` / WJ `string` formals need `.as_str()` for `&str` patterns.
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
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
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
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
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
        finalize_borrowed_text_call_site_arg(Some(&sig), 1, None, &arg, &mut arg_str, false);
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
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let arg = Expression::Identifier {
            name: "squad_id".into(),
            location: None,
        };
        let mut arg_str = "squad_id.to_string()".to_string();
        finalize_borrowed_text_call_site_arg(Some(&sig), 0, Some("Squad"), &arg, &mut arg_str, false);
        assert_eq!(arg_str, "&squad_id");
    }

    #[test]
    fn finalize_borrowed_text_preserves_to_string_on_method_call_ast() {
        use crate::analyzer::{FunctionSignature, OwnershipMode};
        use crate::parser::Expression;

        // push_str expects &str; param arrays include self at index 0
        let sig = FunctionSignature {
            name: "push_str".into(),
            param_types: vec![
                Type::Custom("String".into()),                         // self (index 0)
                Type::Reference(Box::new(Type::Custom("str".into()))), // &str param (index 1)
            ],
            formal_param_types: vec![Type::Custom("String".into()), Type::String],
            param_ownership: vec![OwnershipMode::MutBorrowed, OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };

        // AST: self.rows.to_string() — a MethodCall with method "to_string"
        let self_expr = Expression::Identifier {
            name: "self".into(),
            location: None,
        };
        let field_access = Expression::FieldAccess {
            object: &self_expr,
            field: "rows".into(),
            location: None,
        };
        let arg = Expression::MethodCall {
            object: &field_access,
            method: "to_string".into(),
            type_args: None,
            arguments: vec![],
            location: None,
        };

        let mut arg_str = "self.rows.to_string()".to_string();
        finalize_borrowed_text_call_site_arg(Some(&sig), 0, None, &arg, &mut arg_str, false);

        // .to_string() on a non-string type is a TYPE CONVERSION, not redundant.
        // Must be preserved as &self.rows.to_string(), NOT stripped to &self.rows.
        assert_eq!(
            arg_str, "&self.rows.to_string()",
            "must NOT strip .to_string() from MethodCall AST — it's a type conversion"
        );
    }

    #[test]
    fn finalize_borrowed_text_skips_double_borrow_on_str_ref_param() {
        use crate::analyzer::{FunctionSignature, OwnershipMode};
        use crate::parser::Expression;

        let sig = FunctionSignature {
            name: "HashMap::get".into(),
            param_types: vec![
                Type::Custom("HashMap".into()),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::Custom("HashMap".into()), Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let arg = Expression::Identifier {
            name: "key".into(),
            location: None,
        };
        let mut arg_str = "key".to_string();
        finalize_borrowed_text_call_site_arg(Some(&sig), 0, Some("HashMap"), &arg, &mut arg_str, true);
        assert_eq!(arg_str, "key", "&str formal must not get extra & at map lookup");
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
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
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
