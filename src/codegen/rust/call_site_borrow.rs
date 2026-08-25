//! Unified call-site borrow lowering.
//!
//! Centralizes decisions about when to emit `&`, `&mut`, strip `&`, or strip `.clone()` at
//! method/function call sites based on effective parameter ownership and formal types.

use crate::analyzer::{FunctionSignature, OwnershipMode};
use crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg;
use crate::codegen::rust::expression_utilities;
use crate::codegen::rust::rust_coercion_rules::Coercion;
use crate::codegen::rust::stdlib_method_traits;
use crate::codegen::rust::string_utilities;
use crate::codegen::rust::types;
use crate::parser::{Expression, Literal, Type};

/// Plain WJ `string` formals that emit owned `String` at call sites until codegen confirms `&str`.
pub(crate) use crate::ir::emission_contract::plain_string_formal_passes_owned_at_call_site;

/// Whether codegen recorded (or unambiguously converged) a shared-ref Rust formal for `param_idx`.
pub(crate) use crate::ir::emission_contract::callee_emits_shared_rust_ref_param;

fn type_is_vec_container(t: &Type) -> bool {
    crate::type_classification::type_is_vec_container(t)
}

pub(crate) fn expression_is_vec_macro_literal(expr: &Expression) -> bool {
    matches!(expr, Expression::MacroInvocation { name, .. } if name == "vec")
}

/// Zero-arg stdlib empty constructor (`Vec::new`, `HashMap::new`, `vec![]`, …).
///
/// Type-driven: zero-arg *associated* call (`Type::…()`) on a
/// [`is_stdlib_default_empty_type`] type. Method spelling is not consulted —
/// associated zero-arg formals on these types are empty factories (`new`/`default`).
pub(crate) fn expression_is_stdlib_empty_default_constructor(expr: &Expression) -> bool {
    if expression_is_vec_macro_literal(expr) {
        return true;
    }
    let Expression::Call {
        function,
        arguments,
        ..
    } = expr
    else {
        return false;
    };
    if !arguments.is_empty() {
        return false;
    }
    let type_name = match &**function {
        Expression::FieldAccess { object, .. } => match &**object {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            _ => None,
        },
        Expression::Identifier { name, .. } => name.rsplit_once("::").map(|(ty, _)| ty),
        _ => None,
    };
    type_name.is_some_and(|ty| {
        crate::codegen::rust::stdlib_method_traits::is_stdlib_default_empty_type(ty)
    })
}

/// Back-compat alias — prefer [`expression_is_stdlib_empty_default_constructor`].
pub(crate) fn expression_is_vec_new_constructor(expr: &Expression) -> bool {
    expression_is_stdlib_empty_default_constructor(expr)
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
    gen.local_var_types
        .get(name)
        .is_some_and(type_is_vec_container)
        || gen
            .infer_expression_type(arg_expr)
            .is_some_and(|t| type_is_vec_container(&t))
}

fn callee_arg_expects_shared_vec_ref(sig: &FunctionSignature, arg_index: usize) -> bool {
    let pidx = sig.arg_param_index(arg_index);
    // Owned emission wins over stale analyzer `Reference(Vec)` / Borrowed stubs.
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
        return false;
    }
    // Bare `Vec<T>` formals are owned — never auto-borrow at the call site.
    if sig
        .formal_param_type(pidx)
        .or_else(|| sig.param_types.get(pidx))
        .is_some_and(type_is_vec_container)
    {
        return false;
    }
    if callee_emits_shared_rust_ref_param(sig, pidx) {
        return true;
    }
    sig.param_types
        .get(pidx)
        .is_some_and(|t| matches!(t, Type::Reference(inner) if type_is_vec_container(inner)))
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
    // AST/type-driven only — never string-prefix `Vec::new()` heuristics.
    if !expression_is_owned_vec_at_call_site(gen, arg_expr) {
        return coerced;
    }

    let count = arg_count.unwrap_or(arg_index + 1);
    if let (Some(rt), Some(method)) = (receiver_type, method) {
        let qualified = format!("{rt}::{method}");
        if let Some(resolved) = gen.resolve_method_function_signature(rt, method, count) {
            if let Some(refreshed) =
                crate::codegen::rust::signature_promotion::refresh_call_site_signature_for_arg(
                    Some(resolved),
                    &qualified,
                    arg_index,
                    gen.global_signature_registry.as_deref(),
                    &gen.signature_registry,
                )
            {
                if callee_arg_expects_shared_vec_ref(&refreshed, arg_index) {
                    return format!("&{coerced}");
                }
            }
        }
        if let Some(global) = gen.get_signature_with_global(&qualified) {
            if callee_arg_expects_shared_vec_ref(global, arg_index) {
                return format!("&{coerced}");
            }
        }
    }

    if let Some(refreshed) =
        crate::codegen::rust::signature_promotion::refresh_call_site_signature_for_arg(
            Some(sig.clone()),
            sig.name.as_str(),
            arg_index,
            gen.global_signature_registry.as_deref(),
            &gen.signature_registry,
        )
    {
        if callee_arg_expects_shared_vec_ref(&refreshed, arg_index) {
            return format!("&{coerced}");
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
        registry,
        None,
        callee_name,
        call_sig,
        param_idx,
        arg_index,
    )
}

/// Resolve a bare free-fn name against direct keys and `{module}::{name}` suffixes.
fn lookup_free_fn_signature<'a>(
    registry: &'a crate::analyzer::SignatureRegistry,
    callee_name: &str,
) -> Option<&'a FunctionSignature> {
    let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
    registry
        .get_signature(callee_name)
        .or_else(|| registry.get_signature(simple))
        .or_else(|| registry.find_unique_signature_ending_with(simple))
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
        // Codegen recorded owned `String` emission — peel even when analyzer ownership
        // is still Borrowed (`build_html(name: String)`).
        if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            sig, pidx,
        ) && sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(pidx))
            .copied()
            == Some(false)
        {
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
    // Any codegen-confirmed shared-ref formal must keep the borrow prefix.
    let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
    let any_emits_shared_ref = |sig: &FunctionSignature| {
        let pidx = sig.arg_param_index(arg_index);
        callee_emits_shared_rust_ref_param(sig, pidx)
    };
    if any_emits_shared_ref(call_sig)
        || global
            .and_then(|g| lookup_free_fn_signature(g, callee_name))
            .is_some_and(any_emits_shared_ref)
        || lookup_free_fn_signature(registry, callee_name).is_some_and(any_emits_shared_ref)
    {
        return false;
    }
    if check(call_sig, param_idx) {
        return true;
    }
    // call_sig inconclusive (no emission flags): only then consult other registries for
    // owned-string skip. Never let a stale all-false local refresh override a Borrowed
    // call_sig when no registry confirmed shared-ref above.
    if call_sig.emitted_rust_ref_params.is_some() {
        return false;
    }
    lookup_free_fn_signature(registry, callee_name)
        .or_else(|| global.and_then(|g| lookup_free_fn_signature(g, callee_name)))
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
        Expression::Identifier { .. } | Expression::FieldAccess { .. } | Expression::Index { .. }
    ) {
        return true;
    }
    // Constructor / factory rvalues (`QuestId::from_u32(n)`) and unary temps borrow to `&T`.
    if matches!(
        arg_expr,
        Expression::Call { .. } | Expression::MethodCall { .. } | Expression::Unary { .. }
    ) && !expression_is_copy_literal(arg_expr)
    {
        return true;
    }
    // String concat / format temps: AST may still be Binary while emit text is
    // `_tempN` or `format!(...)` — both produce owned String borrowable as `&str`.
    if arg_str.starts_with("_temp") && arg_str.chars().skip(5).all(|c| c.is_ascii_digit()) {
        return true;
    }
    if arg_str.contains("format!(") || matches!(arg_expr, Expression::Binary { .. }) {
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
        if crate::type_classification::is_language_level_owned_string_convert(method.as_str())
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

/// `param_types` encodes body-converged shared borrow (`Reference(T)` + Borrowed) for a
/// non-Copy aggregate — trust the registry wrap even without `emitted_rust_ref_params`.
fn registry_param_types_indicate_shared_borrow(
    sig: &FunctionSignature,
    param_idx: usize,
    param_ty: &Type,
) -> bool {
    let Type::Reference(inner) = param_ty else {
        return false;
    };
    if sig
        .emitted_rust_ref_params
        .as_ref()
        .and_then(|flags| flags.get(param_idx))
        .copied()
        == Some(false)
    {
        return false;
    }
    // Plain WJ `string` formals need `emitted_rust_ref_params` — stale `Reference(str)`
    // alone must not claim shared-ref emission (circular-dep / multipass).
    if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
        sig, param_idx,
    ) {
        return false;
    }
    if !matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
    ) {
        return false;
    }
    let bare = inner.as_ref();
    let is_copy_aggregate = (crate::codegen::rust::type_analysis::is_copy_type(bare)
        || matches!(
            bare,
            Type::Custom(name) if crate::type_classification::is_known_copy_aggregate(name)
        ))
        && !crate::type_classification::is_copy_pass_by_value_formal(bare);
    !is_copy_aggregate
}

/// Bare formal type is a Copy aggregate that emits by value (not `&T`).
///
/// Used to detect stale `Reference(Lsn)`-style metadata that must not force
/// call-site `&` when the Rust formal is owned Copy (regression-060).
pub fn bare_type_is_copy_aggregate_owned_formal(
    bare: &Type,
    is_type_copy: impl FnOnce(&Type) -> bool,
) -> bool {
    (is_type_copy(bare)
        || matches!(
            bare,
            Type::Custom(name) if crate::type_classification::is_known_copy_aggregate(name)
        ))
        && !crate::type_classification::is_copy_pass_by_value_formal(bare)
}

/// Signature formal slot is a Copy-aggregate owned contract (stale `&T` metadata ignored
/// unless codegen recorded shared-ref emission).
pub fn sig_formal_is_copy_aggregate_owned(
    sig: &FunctionSignature,
    param_idx: usize,
    is_type_copy: impl FnOnce(&Type) -> bool,
) -> bool {
    sig.formal_param_type(param_idx).is_some_and(|t| {
        let bare = match t {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        bare_type_is_copy_aggregate_owned_formal(bare, is_type_copy)
            && !callee_emits_shared_rust_ref_param(sig, param_idx)
    })
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
        // Match arms that bind `HashMap<string, _>` may blanket-own every string
        // literal (`.to_string()`). Collection key lookups still want `&str`.
        if expression_is_string_literal(arg_expr) {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr, arg_str,
            );
        }
        return;
    }
    if let Expression::Cast { type_, .. } = arg_expr {
        if crate::codegen::rust::type_analysis_pure::is_copy_type(type_) {
            return;
        }
        *arg_str = format!("&({arg_str})");
        return;
    }
    if arg_already_rust_ref {
        return;
    }
    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(arg_str);
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

/// How to pass a hoisted `format!` / write!-block temp at a call site.
///
/// Signature-driven: shared-ref formals get `&_tempN`; owned string formals get `_tempN`.
/// Used by both free-function and method format-temp hoisting (DRY).
pub(crate) fn format_temp_arg_pass_expr(
    sig: Option<&FunctionSignature>,
    arg_index: usize,
    temp_name: &str,
    had_borrow_prefix: bool,
) -> String {
    if had_borrow_prefix {
        return format!("&{temp_name}");
    }
    let Some(sig) = sig else {
        return temp_name.to_string();
    };
    let param_idx = sig.arg_param_index(arg_index);
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx) {
        return temp_name.to_string();
    }
    if callee_emits_shared_rust_ref_param(sig, param_idx) {
        return format!("&{temp_name}");
    }
    if sig
        .param_type_for_arg(arg_index)
        .is_some_and(|t| string_utilities::param_is_rust_str_ref(t))
    {
        return format!("&{temp_name}");
    }
    let wants_owned_string = matches!(
        effective_param_ownership_for_arg(sig, arg_index),
        OwnershipMode::Owned
    ) && sig.formal_param_type(param_idx).is_some_and(|t| {
        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            && types::is_windjammer_text_type(t)
    });
    if wants_owned_string {
        return temp_name.to_string();
    }
    // Plain WJ text formal with Borrowed ownership → shared ref at call site.
    if matches!(
        effective_param_ownership_for_arg(sig, arg_index),
        OwnershipMode::Borrowed
    ) && sig
        .formal_param_type(param_idx)
        .is_some_and(|t| types::is_windjammer_text_type(t))
    {
        return format!("&{temp_name}");
    }
    // Converged plain text formal without owned emission → prefer borrow (readonly demotion).
    if sig.formal_param_type(param_idx).is_some_and(|t| {
        types::is_windjammer_text_type(t)
            && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
    }) && !plain_string_formal_passes_owned_at_call_site(sig, param_idx)
    {
        return format!("&{temp_name}");
    }
    temp_name.to_string()
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
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    #[test]
    fn registry_str_ref_param_emits_shared_borrow() {
        let mut sig = sig_with_formal(
            "TextBuffer::append_slice",
            vec![
                Type::Custom("TextBuffer".into()),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            vec![Type::Custom("TextBuffer".into()), Type::String],
            vec![OwnershipMode::MutBorrowed, OwnershipMode::Borrowed],
            true,
        );
        sig.emitted_rust_ref_params = Some(vec![false, true]);
        let pidx = sig.arg_param_index(0);
        assert!(
            callee_emits_shared_rust_ref_param(&sig, pidx),
            "emitted_rust_ref_params + Reference(str) must emit shared borrow at call sites"
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
        assert!(plain_string_formal_passes_owned_at_call_site(&sig, 0));
    }

    #[test]
    fn analyzer_bar_signature_plain_string_owned() {
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
        assert!(
            plain_string_formal_passes_owned_at_call_site(bar, 0),
            "formal={:?} param_types={:?}",
            bar.formal_param_types,
            bar.param_types
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
