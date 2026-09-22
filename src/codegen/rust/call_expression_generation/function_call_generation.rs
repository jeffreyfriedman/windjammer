//! Plain function call lowering (after `Call(FieldAccess)` is handled elsewhere).

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::CodeGenerator;

/// Apply `&mut` at call sites from emitted callee metadata and converged registry signatures.
fn apply_callee_mut_borrow_to_call_args<'ast>(
    gen: &CodeGenerator<'ast>,
    func_name: &str,
    signature: &Option<crate::analyzer::FunctionSignature>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
    args: &mut [String],
) {
    let simple_name = func_name.rsplit("::").next().unwrap_or(func_name);
    let registry_sig = gen
        .get_signature_with_global(func_name)
        .cloned()
        .or_else(|| gen.get_signature_with_global(simple_name).cloned())
        .or_else(|| signature.as_ref().cloned());
    let emitted_indices = gen
        .function_emitted_mut_arg_indices
        .get(func_name)
        .or_else(|| gen.function_emitted_mut_arg_indices.get(simple_name));

    for (i, arg_str) in args.iter_mut().enumerate() {
        let Some((_, arg_expr)) = arguments.get(i) else {
            continue;
        };
        if !matches!(arg_expr, Expression::Identifier { .. }) {
            continue;
        }
        let refreshed_sig =
            gen.refresh_call_site_signature_for_arg(registry_sig.clone(), func_name, i);
        let local_emitted_mut = emitted_indices.is_some_and(|indices| indices.contains(&i));
        let callee_expects_mut = refreshed_sig.as_ref().is_some_and(|sig| {
            crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                sig, i,
            )
        });
        let is_mutable_raw_pointer_formal = refreshed_sig
            .as_ref()
            .is_some_and(|sig| {
                let pidx = sig.arg_param_index(i);
                sig.formal_param_type(pidx)
                    .or_else(|| sig.param_types.get(pidx))
                    .is_some_and(
                        crate::codegen::rust::call_signature_resolution::param_type_is_mutable_raw_pointer,
                    )
            })
            || gen.get_signature_with_global(func_name).is_some_and(|sig| {
                let pidx = sig.arg_param_index(i);
                sig.formal_param_type(pidx)
                    .or_else(|| sig.param_types.get(pidx))
                    .is_some_and(
                        crate::codegen::rust::call_signature_resolution::param_type_is_mutable_raw_pointer,
                    )
            })
            || gen.get_signature_with_global(simple_name).is_some_and(|sig| {
                let pidx = sig.arg_param_index(i);
                sig.formal_param_type(pidx)
                    .or_else(|| sig.param_types.get(pidx))
                    .is_some_and(
                        crate::codegen::rust::call_signature_resolution::param_type_is_mutable_raw_pointer,
                    )
            });
        let owned_contract = !is_mutable_raw_pointer_formal
            && (refreshed_sig
                .as_ref()
                .is_some_and(|sig| {
                    let pidx = sig.arg_param_index(i);
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        sig, pidx,
                    ) || matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                            sig, i,
                        ),
                        OwnershipMode::Owned,
                    )
                })
                || gen.get_signature_with_global(func_name).is_some_and(|sig| {
                    let pidx = sig.arg_param_index(i);
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        sig, pidx,
                    ) || matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                            sig, i,
                        ),
                        OwnershipMode::Owned,
                    )
                })
                || gen.get_signature_with_global(simple_name).is_some_and(|sig| {
                    let pidx = sig.arg_param_index(i);
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        sig, pidx,
                    ) || matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                            sig, i,
                        ),
                        OwnershipMode::Owned,
                    )
                }));
        if !local_emitted_mut
            && !callee_expects_mut
            && crate::codegen::rust::call_signature_resolution::has_ownership_collision_for_call(
                gen, func_name,
            )
        {
            continue;
        }
        // Owned defining-module formals win over stale mut-arg index bookkeeping.
        let needs_mut = (local_emitted_mut || callee_expects_mut) && !owned_contract;
        if owned_contract && arg_str.starts_with("&mut ") {
            *arg_str =
                crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str).to_string();
            continue;
        }
        if needs_mut {
            // Always peel `&mut place.clone()` temps, even when already `&mut`-prefixed.
            crate::codegen::rust::expression_utilities::sanitize_mut_borrow_clone_temp(arg_str);
            if arg_str.starts_with("&mut ") {
                continue;
            }
            if let Expression::Identifier { name, .. } = arg_expr {
                if gen.identifier_already_mut_ref(name)
                    || gen.emitted_rust_ref_formals.contains(name)
                {
                    // Binding is already `&mut T` / `&T` — peel stale shared `&`.
                    *arg_str =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str)
                            .to_string();
                    continue;
                }
            }
            // Skip re-borrow when user wrote `&mut x` or arg already carries a borrow.
            if arg_str.starts_with('&') {
                continue;
            }
            // Mut borrow needs an lvalue — never `&mut binding.clone()` (WDB-336/337/342).
            crate::codegen::rust::expression_utilities::apply_mut_borrow_prefix(arg_str);
        }
    }
}

/// After IR call-site passes, ensure `"lit"` becomes `"lit".to_string()` for owned String formals.
fn apply_owned_string_literal_coercion<'ast>(
    gen: &CodeGenerator<'ast>,
    func_name: &str,
    signature: &Option<crate::analyzer::FunctionSignature>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
    args: &mut [String],
) {
    // Signature-driven for all callees — including inline `mod` and lowercase
    // `module::fn` paths. Do not blanket-skip by module spelling.
    let simple_name = func_name.rsplit("::").next().unwrap_or(func_name);
    let allow_simple_fallback = gen.inline_module_qualified_call(func_name)
        || !crate::codegen::rust::call_signature_resolution::is_lowercase_user_module_qualified_call(
            func_name,
        );

    for (i, arg_str) in args.iter_mut().enumerate() {
        let Some((_, arg_expr)) = arguments.get(i) else {
            continue;
        };
        if !matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) || arg_str.ends_with(".to_string()")
            || arg_str.contains("string_to_ffi(")
        {
            continue;
        }
        // Prefer defining-module / scanned-runtime `&str` over multipass WJ stubs
        // (`std/strings.wj` owned `delimiter: string` shadowing runtime `&str`).
        let sig = gen.refresh_call_site_signature_for_arg(
            signature.clone().or_else(|| {
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name).cloned()),
                    if allow_simple_fallback {
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(simple_name).cloned())
                    } else {
                        None
                    },
                    if allow_simple_fallback {
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.find_unique_signature_ending_with(simple_name).cloned())
                    } else {
                        None
                    },
                    gen.get_signature_with_global(func_name).cloned(),
                    gen.signature_registry.get_signature(func_name).cloned(),
                    if allow_simple_fallback {
                        gen.signature_registry.get_signature(simple_name).cloned()
                    } else {
                        None
                    },
                ])
            }),
            func_name,
            i,
        );
        if gen.preregistered_free_call_arg_expects_borrow(func_name, i) {
            continue;
        }
        if sig.as_ref().is_some_and(|s| {
            let pidx = s.arg_param_index(i);
            matches!(
                s.param_ownership.get(pidx),
                Some(crate::analyzer::OwnershipMode::Borrowed)
            ) && s.formal_param_type(pidx).is_some_and(|t| {
                crate::codegen::rust::types::is_windjammer_text_type(t)
                    && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            })
        }) {
            continue;
        }
        if crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
            sig.as_ref(),
            i,
        ) {
            continue;
        }
        if sig.as_ref().is_some_and(|s| {
            let idx = s.arg_param_index(i);
            crate::ir::signature_bridge::call_site_expects_shared_borrow(s, idx)
                || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(s, idx)
                || crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
                    s, i,
                )
        }) {
            continue;
        }
        // `@string_ref` / `&String`: `"lit"` → `&"lit".to_string()`.
        let is_string_ref_formal = sig.as_ref().is_some_and(|s| {
            s.string_ref_string_formal_for_arg(i)
                || s.param_type_for_arg(i)
                    .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref)
                || s.formal_param_type_for_arg(i)
                    .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref)
        });
        if is_string_ref_formal {
            let base = arg_str.trim_start_matches('&');
            let owned = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(base);
            *arg_str = format!("&{owned}");
            continue;
        }
        if crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
            sig.as_ref(),
            i,
            func_name.rsplit("::").next(),
            func_name
                .split("::")
                .next()
                .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase())),
            Some(&gen.enum_variant_types),
        ) || sig.as_ref().is_some_and(|s| {
            let idx = s.arg_param_index(i);
            if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(s, idx) {
                return false;
            }
            if matches!(
                s.param_ownership.get(idx),
                Some(crate::analyzer::OwnershipMode::Borrowed)
            ) && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string_for_call_arg(
                s, i,
            ) {
                return false;
            }
            if !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(s, idx)
                && !crate::codegen::rust::string_utilities::string_literal_needs_to_string(s, i)
            {
                return false;
            }
            let text_formal = s
                .formal_param_type(idx)
                .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
            let owned_contract = matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership(s, idx),
                OwnershipMode::Owned,
            ) || matches!(
                s.param_ownership.get(idx),
                Some(OwnershipMode::Owned)
            );
            let not_str_ref = !s
                .param_types
                .get(idx)
                .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref);
            text_formal && owned_contract && not_str_ref
        }) {
            let base = arg_str.trim_start_matches('&');
            *arg_str = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(base);
        }
    }
}

/// Map static impl calls to `Type::method` + receiver context for signature lookup.
///
/// `Self::method` and `Type::method` (when `Type` is the enclosing impl struct) must both
/// supply receiver type. Without it, `resolve_call_signature` falls through declaration
/// stubs to arg-count suffix matches and mis-lowers borrows (e.g. `grid.clone()` for
/// `FpsCamera::collides_aabb` in library builds).
fn signature_lookup_for_call<'ast>(
    gen: &CodeGenerator<'ast>,
    func_name: &str,
) -> (String, Option<String>) {
    if gen.in_impl_block {
        if let Some(ref tn) = gen.current_struct_name {
            if let Some(method) = func_name.strip_prefix("Self::") {
                return (format!("{tn}::{method}"), Some(tn.clone()));
            }
            if let Some((qualifier, method)) = func_name.rsplit_once("::") {
                if qualifier == tn.as_str()
                    && qualifier
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_uppercase())
                    && !method.contains("::")
                {
                    return (func_name.to_string(), Some(tn.clone()));
                }
            }
        }
    }
    if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(func_name)
    {
        if let Some((receiver, _method)) = func_name.rsplit_once("::") {
            return (func_name.to_string(), Some(receiver.to_string()));
        }
    }
    let lookup = gen.signature_lookup_callee_name(func_name);
    (lookup.into_owned(), None)
}

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn generate_plain_function_call<'ast>(
    gen: &mut CodeGenerator<'ast>,
    func_name: &str,
    function: &Expression<'ast>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> String {
    let mut func_str = gen.generate_expression(function);

    if let Some(rt_method) = {
        let normalized = func_name
            .replace('.', "::")
            .strip_prefix("std::")
            .unwrap_or(func_name)
            .to_string();
        crate::analyzer::SignatureRegistry::resolve_runtime_emit_method_name_chain(
            &normalized,
            &gen.signature_registry,
            gen.global_signature_registry.as_deref(),
        )
    } {
        let normalized = func_name.replace('.', "::");
        let module = normalized
            .strip_prefix("std::")
            .unwrap_or(normalized.as_str())
            .rsplit_once("::")
            .map(|(m, _)| m)
            .unwrap_or("random");
        func_str = format!("{module}::{rt_method}");
    }

    // Bare `min(a, b)` on floats → Rust float min (no unqualified `min` in scope).
    if func_name == "min" && arguments.len() == 2 {
        use crate::type_inference::FloatType;
        let lc = gen.float_class_for_binary_operand(arguments[0].1);
        let rc = gen.float_class_for_binary_operand(arguments[1].1);
        func_str = match (lc, rc) {
            (Some(FloatType::F64), _) | (_, Some(FloatType::F64)) => "f64::min".to_string(),
            _ => "f32::min".to_string(),
        };
    }

    // E0282 turbofish: `Type::new()` → `Type::<T>::new()` when return type provides args.
    // Skip when suppress_collection_turbofish is set (let binding already has type ascription).
    // Skip in call-argument position: the callee's parameter type is the source of truth
    // (regression-060: `decode_records(Vec::new())` must not become `Vec::<WalRecord>::new()`).
    if arguments.is_empty()
        && !gen.suppress_collection_turbofish
        && !gen.in_call_argument_generation
    {
        if let Some((type_name, "new")) = func_str.rsplit_once("::") {
            if crate::type_classification::type_name_leaf(type_name) == "Vec" {
                if let Some(Type::Vec(inner)) = &gen.current_function_return_type {
                    func_str = format!("Vec::<{}>::new", gen.type_to_rust(inner));
                }
            } else if crate::type_classification::is_set_type_name(type_name) {
                if let Some(Type::Parameterized(base, args)) = &gen.current_function_return_type {
                    if crate::type_classification::is_set_type_name(base) && args.len() == 1 {
                        let leaf = crate::type_classification::type_name_leaf(type_name);
                        func_str = format!("{leaf}::<{}>::new", gen.type_to_rust(&args[0]));
                    }
                }
            } else if crate::type_classification::is_map_type_name(type_name) {
                if let Some(Type::Parameterized(base, args)) = &gen.current_function_return_type {
                    if crate::type_classification::is_map_type_name(base) && args.len() == 2 {
                        let map_name = if gen.import_aliases.contains("Map")
                            && crate::type_classification::type_name_leaf(type_name) == "Map"
                        {
                            "Map"
                        } else if crate::type_classification::type_name_leaf(type_name) == "Map" {
                            "HashMap"
                        } else {
                            crate::type_classification::type_name_leaf(type_name)
                        };
                        func_str = format!(
                            "{}::<{}, {}>::new",
                            map_name,
                            gen.type_to_rust(&args[0]),
                            gen.type_to_rust(&args[1])
                        );
                    }
                }
            }
        }
    }

    // In an impl block, bare function calls to sibling methods need qualified dispatch.
    // Instance methods (take self) → self.method(args)
    // Static methods → Self::method(args)
    if gen.in_impl_block
        && !func_name.contains("::")
        && gen.current_impl_methods.contains(func_name)
    {
        if gen.current_impl_instance_methods.contains(func_name) {
            func_str = format!("self.{}", func_str);
        } else {
            func_str = format!("Self::{}", func_str);
        }
    }

    // E0282 turbofish: Some(expr) → Some::<T>(expr)
    // Only needed when the type parameter is truly ambiguous
    // (e.g. numeric literals outside a typed context). In return
    // position or when the inner type involves references/structs,
    // Rust infers the type from the function signature.
    if func_str == "Some" && arguments.len() == 1 {
        if let Some(Type::Option(inner)) = &gen.current_function_return_type {
            let inner_rust = gen.type_to_rust(inner);
            let is_ambiguous_primitive = matches!(
                inner.as_ref(),
                Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
            );
            if is_ambiguous_primitive {
                func_str = format!("Some::<{}>", inner_rust);
            }
        }
    }

    // WINDJAMMER PHILOSOPHY: Some/Ok/Err with string literals need .to_string()
    // Some("literal") -> Some("literal".to_string())
    // Ok("literal") -> Ok("literal".to_string())
    // Err("literal") -> Err("literal".to_string())
    // Also: Some(borrowed_iterator_var) -> Some(borrowed_iterator_var.clone())

    // Language-level payload constructors (`Some`/`Ok`/`Err`, `Type::Variant`) and
    // tuple structs need owned values. Signature-driven for named calls; AST for
    // Option/Result/enum (not a method-name ownership heuristic).
    let is_payload_ctor =
        crate::type_classification::is_language_level_payload_call_name(func_name);
    let is_tuple_struct_constructor = !is_payload_ctor
        && !func_name.contains("::")
        && func_name.chars().next().is_some_and(|c| c.is_uppercase())
        && gen.tuple_struct_names.contains(func_name);

    if is_payload_ctor || is_tuple_struct_constructor {
        // Enum variant constructors need owned values (Some(T), Ok(T), Err(E)).
        // Set owned context so index expressions use .clone() instead of &,
        // BUT only for arguments that aren't already explicit references.
        let prev_owned_context = gen.in_owned_value_context;
        let generated_args: Vec<String> = arguments
            .iter()
            .map(|(_label, arg)| {
                let is_explicit_ref = matches!(
                    arg,
                    Expression::Unary {
                        op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                        ..
                    }
                );
                if !is_explicit_ref {
                    gen.in_owned_value_context = true;
                }
                let prev_in_call_arg = gen.in_call_argument_generation;
                gen.in_call_argument_generation = true;
                let result = gen.generate_expression(arg);
                gen.in_call_argument_generation = prev_in_call_arg;
                gen.in_owned_value_context = prev_owned_context;
                result
            })
            .collect();

        let has_format_arg = generated_args
            .iter()
            .any(|arg_str| arg_str.contains("format!("));

        if has_format_arg {
            // Extract format!() macros to temp variables
            let mut temp_decls = String::new();
            let mut temp_counter = 0;
            let fixed_args: Vec<String> = generated_args
                .iter()
                .map(|arg_str| {
                    if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                        // Strip leading & if present
                        let format_expr = if arg_str.starts_with("&") {
                            arg_str.strip_prefix("&").unwrap()
                        } else {
                            arg_str
                        };
                        // Extract to temp var
                        let temp_name = format!("_temp{}", temp_counter);
                        temp_counter += 1;
                        temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));

                        // TDD FIX: Don't add & for owned parameters
                        // Err(format!(...)) should be Err(_temp0), not Err(&_temp0)
                        // Original arg didn't have &, so pass owned value
                        if arg_str.starts_with("&") {
                            format!("&{}", temp_name)
                        } else {
                            temp_name
                        }
                    } else {
                        arg_str.clone()
                    }
                })
                .collect();

            return format!(
                "{{ {}{}({}) }}",
                temp_decls,
                func_str,
                fixed_args.join(", ")
            );
        }

        let args: Vec<String> = generated_args
            .iter()
            .enumerate()
            .map(|(i, arg_str)| {
                // Get the original argument expression for type checking
                let arg = &arguments[i].1;
                let result = arg_str.clone();

                // Auto-convert string literals / &str slices to String for Option/Result
                // wrappers when the payload is owned string (`Option<string>`, `Result<string, _>`).
                let payload_wants_owned_string =
                    match (&gen.current_function_return_type, func_name) {
                        (Some(Type::Option(inner)), "Some") => {
                            crate::codegen::rust::string_utilities::type_is_owned_string(inner)
                        }
                        (Some(Type::Result(ok, _)), "Ok") => {
                            crate::codegen::rust::string_utilities::type_is_owned_string(ok)
                        }
                        (Some(Type::Result(_, err)), "Err") => {
                            crate::codegen::rust::string_utilities::type_is_owned_string(err)
                        }
                        _ => false,
                    };
                if matches!(
                    arg,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                ) {
                    // WDB-341: `coerce_string_literals_to_owned` may already emit
                    // `String::from("…")` — never stack `.to_string()` on top.
                    crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(&result)
                } else if payload_wants_owned_string
                    && !crate::codegen::rust::string_utilities::already_owned_string_expr(&result)
                    && result.starts_with('&')
                {
                    // `Some(&s[i..j])` → owned String payload (type-driven, not method names)
                    crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(&result)
                } else if let Expression::Identifier { name, .. } = arg {
                    // BUGFIX: Don't clone if function returns Option<&T>, Option<&mut T>, or Result<&T, E>
                    // When returning Option<&Squad>, Some(squad) should NOT become Some(squad.clone())

                    // Check if return type is Option<&T> or Option<&mut T> (reference inside)
                    let returns_option_ref = match &gen.current_function_return_type {
                        Some(Type::Option(inner_type)) => {
                            matches!(**inner_type, Type::Reference(_) | Type::MutableReference(_))
                        }
                        _ => false,
                    };

                    // Check if return type is Result<&T, E> or Result<&mut T, E>
                    let returns_result_ref = match &gen.current_function_return_type {
                        Some(Type::Result(ok_type, _err_type)) => {
                            matches!(**ok_type, Type::Reference(_) | Type::MutableReference(_))
                        }
                        _ => false,
                    };

                    // AUTO-CONVERT: Borrowed variables in enum constructors need
                    // ownership conversion since the wrapper takes ownership.
                    // &str params → .to_string(), other borrowed → .clone()
                    // UNLESS returning Option<&T>, Result<&T, E>, etc.
                    if !returns_option_ref
                        && !returns_result_ref
                        && !result.ends_with(".clone()")
                        && !result.ends_with(".to_string()")
                        && !result.trim_start().starts_with('*')
                    {
                        if gen.str_ref_optimized_params.contains(name.as_str()) {
                            format!("{}.to_string()", result)
                        } else if gen.callee_param_field_extracts_by_name(func_name, i) {
                            result
                        } else if gen.param_used_in_prior_field_extract_call(name) {
                            result
                        } else if gen.borrowed_iterator_vars.contains(name)
                            || gen.inferred_borrowed_params.contains(name.as_str())
                        {
                            format!("{}.clone()", result)
                        } else {
                            gen.maybe_auto_clone(name, &result)
                        }
                    } else {
                        result
                    }
                } else {
                    result
                }
            })
            .collect();
        let cast_suffix = gen.current_function_return_type.as_ref().and_then(|t| {
            if let Type::Option(inner) = t {
                match inner.as_ref() {
                    Type::Int => Some(" as i64"),
                    Type::Int32 => Some(" as i32"),
                    Type::Custom(n) if n == "int" || n == "i64" => Some(" as i64"),
                    Type::Custom(n) if n == "i32" => Some(" as i32"),
                    _ => None,
                }
            } else {
                None
            }
        });
        let args = if let Some(suffix) = cast_suffix {
            if func_str.starts_with("Some::<") && arguments.len() == 1 {
                let (_, inner) = &arguments[0];
                if gen.expression_produces_usize(inner) || gen.infer_expression_type_is_usize(inner)
                {
                    args.into_iter()
                        .map(|a| format!("{a}{suffix}"))
                        .collect::<Vec<_>>()
                } else {
                    args
                }
            } else {
                args
            }
        } else {
            args
        };
        return format!("{}({})", func_str, args.join(", "));
    }

    // Function pointer signature extraction: when calling a function pointer
    // parameter (e.g., has_item(arg1, arg2)), build the signature from the
    // parameter's type instead of registry lookup.
    let mut signature = gen
        .current_function_params
        .iter()
        .find(|p| p.name == func_name)
        .and_then(|param| {
            if let Type::FunctionPointer {
                params,
                return_type,
            } = &param.type_
            {
                let param_ownership: Vec<OwnershipMode> = params
                    .iter()
                    .map(|ty| match ty {
                        Type::String => OwnershipMode::Borrowed,
                        Type::Custom(name) if name == "string" => OwnershipMode::Borrowed,
                        Type::Reference(_) | Type::MutableReference(_) => OwnershipMode::Borrowed,
                        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => {
                            OwnershipMode::Owned
                        }
                        Type::Custom(name)
                            if matches!(
                                name.as_str(),
                                "i32"
                                    | "i64"
                                    | "u32"
                                    | "u64"
                                    | "f32"
                                    | "f64"
                                    | "bool"
                                    | "char"
                                    | "usize"
                                    | "isize"
                            ) =>
                        {
                            OwnershipMode::Owned
                        }
                        _ => OwnershipMode::Owned,
                    })
                    .collect();
                // Mirror Rust `fn(&str, …)` emission so call sites suppress field clones.
                let emitted_rust_ref_params = Some(
                    param_ownership
                        .iter()
                        .map(|o| matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed))
                        .collect(),
                );

                Some(crate::analyzer::FunctionSignature {
                    name: func_name.to_string(),
                    param_types: params.clone(),
                    formal_param_types: params.clone(),
                    param_ownership,
                    return_type: return_type.as_ref().map(|t| (**t).clone()),
                    return_ownership: OwnershipMode::Owned,
                    has_self_receiver: false,
                    is_extern: false,
                    emitted_rust_ref_params,
                    string_ref_string_formal_params: None,
                    field_extract_params: None,
                    forwarding_borrow_params: None,
                })
            } else {
                None
            }
        });

    // Unified signature resolution: local registry first, then converged library-wide registry.
    let mut resolved_via_fallback = false;
    let (sig_lookup_name, sig_receiver_type) = signature_lookup_for_call(gen, func_name);
    if signature.is_none() {
        if let Some(ref tn) = sig_receiver_type {
            let method = sig_lookup_name.rsplit("::").next().unwrap_or(func_name);
            signature = gen.lookup_method_signature_on_receiver_type(tn, method, arguments.len());
        }
    }
    if signature.is_none() {
        if let Some(r) = gen.resolve_call_signature_with_global(
            &sig_lookup_name,
            sig_receiver_type.as_deref(),
            arguments.len(),
        ) {
            let method = sig_lookup_name.rsplit("::").next().unwrap_or(func_name);
            let accept = sig_receiver_type.as_ref().is_none_or(|tn| {
                crate::codegen::rust::call_signature_resolution::accept_method_resolution_for_receiver(
                    &r, tn, method,
                )
            });
            if accept {
                resolved_via_fallback = matches!(
                    r.resolution_method,
                    crate::codegen::rust::call_signature_resolution::ResolutionMethod::ArgCountValidated
                );
                // Module-qualified lowercase calls (draw::draw_text): if
                // the resolution's qualified_key comes from a different
                // module (e.g., rendering_api::draw_text matched via
                // unqualified fallback), treat as fallback to prevent
                // trusting wrong ownership metadata.
                if !resolved_via_fallback && func_name.contains("::") {
                    let qualifier = func_name.split("::").next().unwrap_or("");
                    if qualifier
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_lowercase())
                    {
                        let key_qualifier = r.qualified_key.split("::").next().unwrap_or("");
                        if key_qualifier != qualifier
                            && !r.qualified_key.contains(&format!("{}::", qualifier))
                        {
                            resolved_via_fallback = true;
                        }
                        // ExactQualified resolution that only exists in a
                        // fallback chain (not in the codegen registry's own
                        // signatures map) means the key was synthesized
                        // during registry merges — don't trust ownership.
                        if !resolved_via_fallback
                            && matches!(
                                r.resolution_method,
                                crate::codegen::rust::call_signature_resolution::ResolutionMethod::ExactQualified
                            )
                            && !gen.signature_registry.has_signature_locally(&r.qualified_key)
                        {
                            resolved_via_fallback = true;
                        }
                    }
                }
                signature = Some(r.sig);
                if let Some(ref mut sig) = signature {
                    // Prefer defining-module codegen refresh from the converged global registry.
                    if let Some(global) = gen.global_signature_registry.as_ref() {
                        crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                            sig,
                            global,
                            &[
                                r.qualified_key.clone(),
                                sig_lookup_name.clone(),
                                func_name.to_string(),
                                func_name
                                    .rsplit("::")
                                    .next()
                                    .unwrap_or(func_name)
                                    .to_string(),
                            ],
                        );
                    }
                }
            }
        }
    }

    // Bare same-module user API beats stdlib homonym (`join` vs `strings::join`).
    if !func_name.contains("::") {
        if let Some(ref sig) = signature {
            signature = Some(
                crate::codegen::rust::signature_promotion::local_user_fn_beats_runtime_std_homonym(
                    &gen.signature_registry,
                    func_name,
                    sig.clone(),
                ),
            );
        }
    }

    // Extern detection: resolved signature is authoritative. For fallback
    // resolutions on module-qualified calls, only trust explicit is_extern.
    let is_extern_call = if resolved_via_fallback && func_name.contains("::") {
        signature.as_ref().is_some_and(|sig| sig.is_extern)
    } else if let Some(ref sig) = signature {
        sig.is_extern
    } else if func_name.contains("::") {
        // Module-qualified call without a resolved signature — check if the
        // base name (after the last `::`) is a known extern function. This
        // handles cross-module extern calls like `api::gpu_create_buffer()`
        // where signature resolution may miss the extern flag.
        let base_name = func_name.rsplit("::").next().unwrap_or(func_name);
        if gen.extern_function_names.contains(base_name) {
            true
        } else if let Some(ref global) = gen.global_signature_registry {
            // Cross-crate extern: check global registry for any qualified key
            // ending with this base name that is marked extern.
            if global
                .get_signature(func_name)
                .or_else(|| global.get_signature(base_name))
                .is_some_and(|s| s.is_extern)
            {
                true
            } else {
                let module_prefix = func_name.split("::").next().unwrap_or("");
                !module_prefix.is_empty() && gen.ffi_module_aliases.contains(module_prefix)
            }
        } else {
            // Check if the call is through a module imported from an ffi path.
            // E.g., `use engine::ffi::input` → input::func() is extern.
            let module_prefix = func_name.split("::").next().unwrap_or("");
            !module_prefix.is_empty() && gen.ffi_module_aliases.contains(module_prefix)
        }
    } else {
        gen.extern_function_names.contains(func_name)
    };

    let mut args: Vec<String> = super::argument_generation::collect_regular_function_arguments(
        gen,
        func_name,
        func_str.as_str(),
        arguments,
        &signature,
        resolved_via_fallback,
        is_extern_call,
    );

    // Path-dep import aliases: dependency metadata `Borrowed` / `emitted_rust_ref_params`
    // must auto-borrow owned locals at the cross-crate boundary (apps/wj-find).
    // Mixed formals (`field: &str`, `value: String`) must peel `&` only on owned slots
    // (apps/wj-todo-cli → wj-validate) — never blanket-borrow every string arg.
    let lookup_callee = gen.signature_lookup_callee_name(func_name);
    let import_alias = gen.is_import_alias_cross_crate_call(func_name);
    let cross_crate_import =
        import_alias || lookup_callee.as_ref().contains("::");
    if cross_crate_import {
        let lookup_ref = lookup_callee.as_ref();
        let simple = lookup_ref.rsplit("::").next().unwrap_or(lookup_ref);
        let dep_sig = gen
            .global_signature_registry
            .as_ref()
            .and_then(|global_reg| {
                global_reg
                    .get_signature(lookup_ref)
                    .or_else(|| global_reg.get_signature(simple))
                    .or_else(|| global_reg.find_unique_signature_ending_with(simple))
            })
            .or_else(|| signature.as_ref())
            .or_else(|| gen.get_signature_with_global(func_name))
            .or_else(|| gen.get_signature_with_global(simple));
        if let Some(gs) = dep_sig {
            for (i, (_, arg)) in arguments.iter().enumerate() {
                let Some(arg_str) = args.get_mut(i) else {
                    continue;
                };
                let Expression::Identifier { name, .. } = arg else {
                    continue;
                };
                let pidx = gs.arg_param_index(i);
                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(gs, pidx)
                    || gen.preregistered_free_call_arg_emits_owned(func_name, i)
                    || (matches!(
                        gs.param_ownership.get(pidx),
                        Some(OwnershipMode::Owned)
                    ) && gs.param_types.get(pidx).is_some_and(|t| {
                        matches!(t, Type::String)
                            || crate::codegen::rust::types::is_windjammer_text_type(t)
                                && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    }))
                {
                    if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                        *arg_str = crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                            arg_str,
                        );
                    }
                    continue;
                }
                if !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(gs, pidx) {
                    continue;
                }
                if !arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                    *arg_str = format!("&{name}");
                }
            }
        }
    }

    // Legacy heuristic auto-borrow deleted (Phase 5): IR call-site coercion owns this.

    apply_callee_mut_borrow_to_call_args(gen, func_name, &signature, arguments, &mut args);
    // Terminal sanitize: never leave `&mut place.clone()` temps (WDB-336/337/342).
    for arg in &mut args {
        crate::codegen::rust::expression_utilities::sanitize_mut_borrow_clone_temp(arg);
    }
    apply_owned_string_literal_coercion(gen, func_name, &signature, arguments, &mut args);

    let needs_format_temp = |arg_str: &str| -> bool {
        arg_str.contains("format!(")
            || arg_str.contains("write!(&mut __s,")
            || (arg_str.contains("string_to_ffi(")
                && (arg_str.contains("format!(") || arg_str.contains("write!(&mut __s,")))
    };
    let has_format_arg = args.iter().any(|arg_str| needs_format_temp(arg_str));

    /// Strip `string_to_ffi(...)` wrapper for temp extraction of the inner expression.
    fn unwrap_string_to_ffi(arg_str: &str) -> (&str, bool) {
        const PREFIX: &str = "windjammer_runtime::ffi::string_to_ffi(";
        if let Some(rest) = arg_str.strip_prefix(PREFIX) {
            if let Some(inner) = rest.strip_suffix(')') {
                return (inner, true);
            }
        }
        (arg_str, false)
    }

    fn extract_format_like_arg(
        arg_str: &str,
        arg_index: usize,
        sig: Option<&crate::analyzer::FunctionSignature>,
        temp_decls: &mut String,
        temp_counter: &mut i32,
    ) -> Option<String> {
        let (inner, was_ffi) = unwrap_string_to_ffi(arg_str);
        let has_borrow_prefix = inner.starts_with('&');
        let format_expr = if has_borrow_prefix {
            &inner[1..]
        } else {
            inner
        };
        let needs_extract = format_expr.starts_with("format!(")
            || format_expr.starts_with("{") && format_expr.contains("write!(&mut __s,");
        if !needs_extract {
            return None;
        }
        let temp_name = format!("_temp{}", temp_counter);
        *temp_counter += 1;
        temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));
        let pass_expr = crate::codegen::rust::call_site_borrow::format_temp_arg_pass_expr(
            sig,
            arg_index,
            &temp_name,
            has_borrow_prefix,
        );
        Some(if was_ffi {
            format!("windjammer_runtime::ffi::string_to_ffi({})", pass_expr)
        } else {
            pass_expr
        })
    }

    // WINDJAMMER FFI: Extern functions returning string use FfiString - wrap with ffi_to_string
    let returns_string = signature
        .as_ref()
        .and_then(|s| s.return_type.as_ref())
        .is_some_and(|t| {
            matches!(t, Type::String)
                || matches!(t, Type::Custom(n) if n == "string" || n == "String")
        });

    // WINDJAMMER PHILOSOPHY: Auto-wrap extern function calls in unsafe blocks
    // THE WINDJAMMER WAY: Users shouldn't have to write `unsafe` manually
    let call_result = if has_format_arg {
        // Extract format!() macros to temp variables
        let mut temp_decls = String::new();
        let mut temp_counter = 0i32;
        let fixed_args: Vec<String> = args
            .iter()
            .enumerate()
            .map(|(arg_idx, arg_str)| {
                if let Some(fixed) = extract_format_like_arg(
                    arg_str,
                    arg_idx,
                    signature.as_ref(),
                    &mut temp_decls,
                    &mut temp_counter,
                ) {
                    fixed
                } else {
                    arg_str.clone()
                }
            })
            .collect();

        let call_expr = format!("{}({})", func_str, fixed_args.join(", "));

        // Wrap in unsafe block if extern, otherwise regular block
        // Parenthesize so the block can be used as a sub-expression (e.g., in comparisons)
        if is_extern_call && !gen.in_unsafe_block {
            format!("(unsafe {{ {}{}  }})", temp_decls, call_expr)
        } else {
            format!("{{ {}{} }}", temp_decls, call_expr)
        }
    } else {
        // No format!() args - generate normally with optional unsafe wrapper
        let call_str = format!("{}({})", func_str, args.join(", "));
        if is_extern_call && !gen.in_unsafe_block {
            format!("(unsafe {{ {} }})", call_str)
        } else {
            call_str
        }
    };

    // Wrap extern string return with ffi_to_string
    if is_extern_call && returns_string {
        format!("windjammer_runtime::ffi::ffi_to_string({})", call_result)
    } else {
        call_result
    }
}
