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

/// True when `ty` is Windjammer/Rust owned string (`string` / `String`).
pub fn type_is_owned_string(ty: &Type) -> bool {
    matches!(ty, Type::String)
        || matches!(ty, Type::Custom(n) if n == "String" || n == "string")
}

/// True when a type slot ultimately needs an owned `String` payload.
/// Peels `Option` / `Result` wrappers so `Option<string>` / `Result<string, _>`
/// drive the same substring / match-arm ownership as bare `string`.
pub fn type_expects_owned_string_payload(ty: &Type) -> bool {
    match ty {
        Type::Option(inner) => type_expects_owned_string_payload(inner),
        Type::Result(ok, _) => type_expects_owned_string_payload(ok),
        other => type_is_owned_string(other),
    }
}

/// Check if return type expects owned String in Rust.
/// Enclosing function/slot expects owned `String` in Rust (`string` / `String` in Windjammer),
/// including when that payload is wrapped in `Option` / `Result`.
pub fn return_type_expects_owned_string(ret: &Option<Type>) -> bool {
    match ret {
        Some(ty) => type_expects_owned_string_payload(ty),
        None => false,
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
    type_is_owned_string(param_type)
}

/// True when the resolved signature expects an owned string at this argument index.
pub fn call_site_param_expects_owned_string(
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
) -> bool {
    let idx = sig.arg_param_index(arg_index);
    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, idx) {
        return false;
    }
    if sig.param_types.get(idx).is_some_and(|t| {
        param_is_rust_str_ref(t) || param_is_rust_string_ref(t)
    }) {
        return false;
    }
    if sig.param_types.get(idx).is_some_and(param_is_owned_string_type) {
        return true;
    }
    if matches!(
        crate::codegen::rust::call_signature_resolution::effective_param_ownership(sig, idx),
        crate::analyzer::OwnershipMode::Borrowed
    ) {
        return false;
    }
    if let Some(flags) = &sig.emitted_rust_ref_params {
        if flags.get(idx) == Some(&true) {
            return false;
        }
        if flags.get(idx) == Some(&false) {
            return sig
                .param_types
                .get(idx)
                .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
        }
    }
    // Plain WJ `string` formals may emit `&str` — do not trust formal_param_types alone.
    sig.formal_param_types
        .get(idx)
        .is_some_and(param_is_owned_string_type)
        && sig.param_types.get(idx).is_some_and(param_is_owned_string_type)
}

/// Bare string literals on type-qualified associated calls (`Column::new("lit")`) must
/// auto-own at the Rust boundary when there is no defining-module codegen refresh
/// (external path deps, stale import stubs). Signature-driven — not method-name lists.
pub fn type_qualified_associated_string_literal_needs_rust_owned_string(
    qualified_name: &str,
    arg_index: usize,
    sig: Option<&crate::analyzer::FunctionSignature>,
    registry: &crate::analyzer::SignatureRegistry,
    global_registry: Option<&crate::analyzer::SignatureRegistry>,
) -> bool {
    if !crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
        qualified_name,
    ) || arg_index != 0
    {
        return false;
    }
    let registry_has_codegen_refresh = |reg: &crate::analyzer::SignatureRegistry| {
        reg.get_signature(qualified_name)
            .is_some_and(|s| s.emitted_rust_ref_params.is_some())
    };
    if sig.is_some_and(|s| s.emitted_rust_ref_params.is_some()) {
        return sig.is_some_and(|s| call_site_param_expects_owned_string(s, arg_index));
    }
    if registry_has_codegen_refresh(registry)
        || global_registry.is_some_and(registry_has_codegen_refresh)
    {
        return sig.is_some_and(|s| call_site_param_expects_owned_string(s, arg_index));
    }
    // Signature-driven fallback: stdlib / global metadata (`HashMap::get` → Borrowed `&Q`).
    let resolved = sig
        .or_else(|| registry.get_signature(qualified_name))
        .or_else(|| global_registry.and_then(|g| g.get_signature(qualified_name)));
    if let Some(s) = resolved {
        return call_site_param_expects_owned_string(s, arg_index);
    }
    // Unknown extern associated call with no metadata — conservative owned literal.
    true
}

/// Bare string literals on instance method calls (`table.empty_message("lit")`) must
/// auto-own at the Rust boundary when there is no WJ signature / codegen refresh
/// (external path-dep builders). Parallel to
/// [`type_qualified_associated_string_literal_needs_rust_owned_string`].
///
/// Stdlib Pattern / `&str` consensus keeps literals bare (`find`, `starts_with`, …).
pub fn unresolved_instance_method_string_literal_needs_rust_owned_string(
    method: &str,
    arg_index: usize,
    sig: Option<&crate::analyzer::FunctionSignature>,
    registry: &crate::analyzer::SignatureRegistry,
    global_registry: Option<&crate::analyzer::SignatureRegistry>,
    receiver_type: Option<&str>,
) -> bool {
    if let Some(s) = sig {
        if s.emitted_rust_ref_params.is_some()
            || !s.param_types.is_empty()
            || !s.formal_param_types.is_empty()
        {
            return call_site_param_expects_owned_string(s, arg_index);
        }
    }

    let pattern_or_str_ref = |reg: &crate::analyzer::SignatureRegistry, recv: Option<&str>| {
        crate::codegen::rust::stdlib_method_traits::method_arg_is_string_pattern_qualified(
            method, recv, reg, arg_index,
        ) || crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
            method, recv, reg, arg_index,
        ) || crate::codegen::rust::stdlib_method_traits::method_is_string_search_qualified(
            method, recv, reg,
        )
    };

    for reg in std::iter::once(registry).chain(global_registry) {
        if pattern_or_str_ref(reg, receiver_type) || pattern_or_str_ref(reg, None) {
            return false;
        }
    }

    let stdlib = crate::analyzer::SignatureRegistry::stdlib();
    if pattern_or_str_ref(&stdlib, receiver_type) || pattern_or_str_ref(&stdlib, None) {
        return false;
    }

    // Unknown extern instance method with no metadata — conservative owned literal.
    true
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

/// At `&str` / `Pattern` formals, normalize owned `String` producers for rustc.
///
/// - `"lit".to_string()` → `"lit"` (literal is already `&str`)
/// - other `….to_string()` / `….to_owned()` → `&….to_string()` (borrow for `Pattern`)
pub fn normalize_owned_string_producer_for_str_ref_param(
    arg_expr: &crate::parser::Expression,
    arg_str: &mut String,
) {
    if arg_str.starts_with('&') {
        return;
    }
    if crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr) {
        if let Some(stripped) = arg_str.strip_suffix(".to_string()") {
            *arg_str = stripped.to_string();
        } else if let Some(stripped) = arg_str.strip_suffix(".to_owned()") {
            *arg_str = stripped.to_string();
        }
        while arg_str.starts_with('&') {
            *arg_str = arg_str[1..].to_string();
        }
        return;
    }
    if arg_str.ends_with(".to_string()") || arg_str.ends_with(".to_owned()") {
        *arg_str = format!("&{arg_str}");
    }
}

/// Whether a method call argument expects `&str` / Rust `Pattern` (resolved sig + registry).
pub fn method_call_arg_expects_pattern_str(
    method: &str,
    arg_index: usize,
    resolved_sig: Option<&crate::analyzer::FunctionSignature>,
    receiver_type_name: Option<&str>,
    receiver_is_text: bool,
    registry: &crate::analyzer::SignatureRegistry,
) -> bool {
    if let Some(sig) = resolved_sig {
        if crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
            sig, arg_index,
        ) {
            return true;
        }
    }
    // stdlib_meta registers text search methods on `String` even when the receiver
    // lowers to `&str` (`string`/`str` formals). Always consult those registry keys
    // — not only when `receiver_is_text` — so stale local signatures cannot block
    // Pattern/`&str` normalization (e.g. `s.find(":".to_string())` in match scrutinees).
    let mut receivers: Vec<String> = Vec::new();
    let mut push = |rt: &str| {
        if !rt.is_empty() && !receivers.iter().any(|existing| existing == rt) {
            receivers.push(rt.to_string());
        }
    };
    if let Some(rt) = receiver_type_name {
        push(rt);
        for candidate in
            crate::codegen::rust::stdlib_method_traits::stdlib_receiver_lookup_candidates(rt)
        {
            push(&candidate);
        }
    }
    if receiver_is_text || receiver_type_name.is_none() {
        push("String");
        push("str");
        push("string");
    }
    receivers.iter().any(|rt| {
        crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
            method,
            Some(rt.as_str()),
            registry,
            arg_index,
        )
    })
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

/// Signature-driven: a string literal at `arg_index` needs `.to_string()`.
///
/// Uses resolved ownership + param type only — never method-name heuristics.
/// Returns false when the param is borrowed, `&str`, or the signature is absent.
pub fn string_literal_needs_to_string(
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
    runtime_module: Option<&str>,
) -> bool {
    string_literal_needs_owned_coercion_with_enum(
        Some(sig),
        arg_index,
        None,
        None,
        None,
        runtime_module,
    )
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
    // Signature-driven: AsRef<&str> / `&str` runtime formals keep bare string literals.
    if crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
        sig, arg_index,
    ) {
        return false;
    }
    let _ = runtime_module;

    // Prefer signature: Pattern/`&str` formals stay bare (no method-name lists).
    if let Some(s) = sig {
        if crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
            s, arg_index,
        ) {
            return false;
        }
    }

    let Some(sig) = sig else {
        if let (Some(m), Some(tn), Some(variants)) = (method, receiver_type, enum_variant_types) {
            if enum_factory_string_param_needs_owned(variants, tn, m, arg_index) {
                return true;
            }
        }
        // Cross-crate builders: instance method + known receiver, no WJ signature.
        // Free-function / unknown-receiver sites stay bare (Pattern unit tests; no guess).
        if let Some(m) = method {
            if receiver_type.is_some() {
                return unresolved_instance_method_string_literal_needs_rust_owned_string(
                    m,
                    arg_index,
                    None,
                    &crate::analyzer::SignatureRegistry::stdlib(),
                    None,
                    receiver_type,
                );
            }
        }
        // No signature and no enum-factory evidence — emit as-is (registry must supply
        // constructor / search formals; do not guess from method names like `new`/`from`).
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

    // Owned `String` Rust formals always allocate for literals — even with stale Borrowed
    // analyzer ownership (store-forced Owned emission).
    if param_is_owned_string_type(param_type)
        && !param_is_rust_str_ref(param_type)
        && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, idx)
    {
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
    // Exception: map/set key lookups accept `&Q where K: Borrow<Q>`, so a bare `"lit"` (&str)
    // is correct for `HashMap<String, _>` — do not allocate.
    let receiver = receiver_type.or_else(|| sig.name.rsplit_once("::").map(|(rt, _)| rt));
    let is_collection_key_lookup =
        crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(sig, arg_index, receiver);
    let param_is_amp_string = !is_collection_key_lookup
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
    // Map/set key `&K` with K=String: keep bare string literals (Borrow<&str>).
    if is_collection_key_lookup
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
    } else if sig
        .is_some_and(|s| call_site_param_expects_owned_string(s, arg_index))
    {
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

        let strip_to_string = sig
            .and_then(|s| {
                let idx = s.arg_param_index(arg_index);
                Some(
                    crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        s, idx,
                    ) || s.param_types.get(idx).is_some_and(param_is_rust_str_ref),
                )
            })
            .unwrap_or(false);

        if strip_to_string {
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

/// Append `.as_str()` when matching an owned `String` against string-literal patterns.
/// Phase-2 / borrowed `&str` formals and locals whose inferred type is already `&str`
/// stay bare (stable Rust: `&str::as_str` is unstable).
pub fn maybe_append_as_str_for_match(
    value_str: &str,
    borrowed_params: &std::collections::HashSet<String>,
    function_params: &[crate::parser::Parameter],
    scrutinee_type: Option<&Type>,
) -> String {
    if value_str.ends_with(".as_str()") {
        return value_str.to_string();
    }
    // Already a string slice binding — `match name { "x" => ... }` is valid.
    if borrowed_params.contains(value_str) {
        return value_str.to_string();
    }
    if scrutinee_type.is_some_and(param_is_rust_str_ref) {
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
        let reg = crate::analyzer::SignatureRegistry::stdlib();
        let sig = reg
            .get_signature("strings::starts_with")
            .expect("strings::starts_with scanned from runtime");
        assert!(!string_literal_needs_owned_coercion_with_enum(
            Some(sig),
            1,
            Some("starts_with"),
            None,
            None,
            None,
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
    fn unresolved_builder_method_literal_needs_owned_without_sig() {
        assert!(
            unresolved_instance_method_string_literal_needs_rust_owned_string(
                "empty_message",
                0,
                None,
                &crate::analyzer::SignatureRegistry::stdlib(),
                None,
                Some("Table"),
            ),
            "cross-crate builder with no WJ sig must auto-own bare lit"
        );
    }

    #[test]
    fn unresolved_string_search_literal_stays_bare() {
        assert!(
            !unresolved_instance_method_string_literal_needs_rust_owned_string(
                "starts_with",
                0,
                None,
                &crate::analyzer::SignatureRegistry::stdlib(),
                None,
                Some("String"),
            ),
            "stdlib Pattern/&str consensus must keep search lits bare"
        );
        assert!(
            !unresolved_instance_method_string_literal_needs_rust_owned_string(
                "trim_end_matches",
                0,
                None,
                &crate::analyzer::SignatureRegistry::stdlib(),
                None,
                Some("String"),
            ),
            "trim_end_matches Pattern slot stays bare with String receiver"
        );
    }

    #[test]
    fn receiver_known_unresolved_builder_coerces_via_string_literal_predicate() {
        assert!(
            string_literal_needs_owned_coercion_with_enum(
                None,
                0,
                Some("empty_message"),
                Some("Table"),
                None,
                None,
            ),
            "receiver+method without sig must coerce for cross-crate builders"
        );
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

    #[test]
    fn normalize_string_literal_to_string_for_str_ref_param_strips_suffix() {
        use crate::parser::{Expression, Literal};
        use crate::test_utils::test_alloc_expr;
        let arg = test_alloc_expr(Expression::MethodCall {
            object: test_alloc_expr(Expression::Literal {
                value: Literal::String(":".into()),
                location: None,
            }),
            method: "to_string".into(),
            type_args: None,
            arguments: vec![],
            location: None,
        });
        let mut arg_str = "\":\".to_string()".to_string();
        normalize_owned_string_producer_for_str_ref_param(arg, &mut arg_str);
        assert_eq!(arg_str, "\":\"");
    }

    #[test]
    fn normalize_non_literal_to_string_for_str_ref_param_borrows() {
        use crate::parser::Expression;
        use crate::test_utils::test_alloc_expr;
        let arg = test_alloc_expr(Expression::MethodCall {
            object: test_alloc_expr(Expression::Identifier {
                name: "needle".into(),
                location: None,
            }),
            method: "to_string".into(),
            type_args: None,
            arguments: vec![],
            location: None,
        });
        let mut arg_str = "needle.to_string()".to_string();
        normalize_owned_string_producer_for_str_ref_param(arg, &mut arg_str);
        assert_eq!(arg_str, "&needle.to_string()");
    }
}
