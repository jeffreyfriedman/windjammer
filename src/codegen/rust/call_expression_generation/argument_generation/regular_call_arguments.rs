//! Plain function call argument lowering (ownership, FFI, casts).

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::super::{expression_helpers, CodeGenerator};

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn collect_regular_function_arguments<'ast>(
    gen: &mut CodeGenerator<'ast>,
    func_name: &str,
    func_str: &str,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
    signature: &Option<crate::analyzer::FunctionSignature>,
    signature_from_simple_fallback: bool,
    is_extern_call: bool,
) -> Vec<String> {
    arguments
        .iter()
        .enumerate()
        .flat_map(|(i, (_label, arg))| {
            // CRITICAL: Reset in_field_access_object for argument generation.
            // Arguments are independent expressions, NOT part of a field/method/index chain.
            // Without this, `process_property(prop.name, prop.value).as_str()` would
            // leak in_field_access_object from the MethodCall handler into prop.name/prop.value,
            // suppressing necessary .clone() calls.
            let scope = gen.arg_gen_scope();
            let mut arg_str = gen.generate_expression(arg);
            gen.restore_arg_gen_scope(scope);

            // TDD FIX: Cast int arguments to usize for stdlib methods
            // Vec::with_capacity(size) where size: int → Vec::with_capacity(size as usize)
            // Vec::with_capacity(10) where 10: int literal → Vec::with_capacity(10_usize)
            let method_part = func_name.rsplit("::").next().unwrap_or(func_name);
            if i == 0 && matches!(method_part, "with_capacity" | "reserve")
            {
                match arg {
                    Expression::Identifier { name, .. } => {
                        let already_usize = gen
                            .current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .is_some_and(|p| {
                                matches!(&p.type_, Type::Custom(n) if n == "usize")
                            })
                            || gen.local_var_types.get(name).is_some_and(|t| {
                                matches!(t, Type::Custom(n) if n == "usize")
                            });
                        if !already_usize {
                            arg_str = format!("{} as usize", arg_str);
                        }
                    }
                    Expression::Literal {
                        value: Literal::Int(val),
                        ..
                    } => {
                        // Literals: use usize suffix
                        arg_str = format!("{}_usize", val);
                    }
                    _ => {
                        // Other expressions (e.g., calculations): wrap in (expr) as usize
                        if !arg_str.ends_with("_usize") && !arg_str.contains(" as usize") {
                            arg_str = format!("({}) as usize", arg_str);
                        }
                    }
                }
            }

            // WINDJAMMER FFI: Convert string arguments for extern functions
            if is_extern_call {
                if let Some(ref sig) = signature {
                    if let Some(param_type) = sig.param_types.get(i) {
                        if matches!(param_type, Type::Custom(name) if name == "str") {
                            // Expand str to (ptr, len)
                            return vec![
                                format!("{}.as_bytes().as_ptr()", arg_str),
                                format!("{}.as_bytes().len()", arg_str),
                            ];
                        }
                        // string/String params → FfiString via string_to_ffi
                        // TDD FIX: Always use .to_string() - infer_expression_type returns
                        // declared param type (Type::String), not actual Rust type. When
                        // ownership infers Borrowed, param becomes &str in Rust, but we
                        // thought it was String and passed directly → E0308.
                        // .to_string() works for both &str and String (String::to_string = clone).
                        //
                        // TDD FIX: Strip redundant .to_string() before wrapping.
                        // Bug: User writes render_text(label.to_string(), x, y). Expression
                        // generation produces "label.to_string()", then we added another
                        // → string_to_ffi(label.to_string().to_string()). Fix: If arg_str
                        // already ends with .to_string(), don't add another.
                        if crate::codegen::rust::string_utilities::param_is_owned_string_type(param_type)
                        {
                            let inner = if arg_str.ends_with(".to_string()") {
                                arg_str.clone()
                            } else {
                                format!("{}.to_string()", arg_str)
                            };
                            return vec![format!(
                                "windjammer_runtime::ffi::string_to_ffi({})",
                                inner
                            )];
                        }
                    }
                }
            }

            // Auto-convert string literals to String for functions expecting owned String
            // THE WINDJAMMER WAY: Smart inference based on available information!
            if matches!(
                arg,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            ) {
                let asref_str_runtime = func_name
                    .split("::")
                    .next()
                    .is_some_and(super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str);

                // Check if the parameter expects an owned String
                let should_convert = if asref_str_runtime {
                    false
                } else if let Some(ref sig) = signature {
                    if sig.is_extern {
                        sig.param_types.get(i).is_some_and(|ty| {
                            crate::codegen::rust::string_utilities::param_is_owned_string_type(ty)
                        })
                    } else if sig.param_types.get(i).is_some_and(|ty| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
                    }) {
                        // Parameter type is &str (string optimization inferred this).
                        // String literals are already &str in Rust — no .to_string() needed.
                        false
                    } else if signature_from_simple_fallback && {
                        let qualifier = func_name.split("::").next().unwrap_or("");
                        qualifier.chars().next().is_some_and(|c| c.is_lowercase())
                    } {
                        // Fallback-resolved from module::function: the signature may
                        // be from a different module. Don't trust ownership for
                        // string coercion — the actual target may take &str.
                        false
                    } else if let Some(&ownership) = sig.param_ownership.get(i) {
                        // Convert if parameter expects owned String
                        matches!(ownership, OwnershipMode::Owned)
                    } else {
                        // No ownership info for this param
                        // THE WINDJAMMER WAY: Heuristic for constructors
                        // Functions named 'new' (or Type::new) taking string params likely expect String
                        func_name == "new" || func_name.ends_with("::new")
                    }
                } else {
                    // No signature found — check enum variant registry
                    // WINDJAMMER FIX: Enum variant constructors like GameEvent::ItemPickup("text")
                    // need .to_string() when the variant field is String type
                    if let Some(variant_types) = gen.enum_variant_types.get(func_name) {
                        // TDD FIX: Check for both Type::String and Type::Custom("String")
                        variant_types.get(i).is_some_and(|ty| {
                            matches!(ty, Type::String)
                                || matches!(ty, Type::Custom(name) if name == "String")
                        })
                    } else {
                        // Fallback heuristic for constructors
                        func_name == "new" || func_name.ends_with("::new")
                    }
                };

                if should_convert {
                    arg_str = format!("{}.to_string()", arg_str);
                }
            }

            // `const SCOPE_*: string` lowers to &'static str; callee params typed `String` need owned.
            if let Expression::Identifier { name, .. } = arg {
                let param_wants_owned_string = signature.as_ref().is_some_and(|sig| {
                    sig.param_types.get(i).is_some_and(|ty| {
                        crate::codegen::rust::string_utilities::param_is_owned_string_type(ty)
                    })
                });
                let is_string_const = crate::codegen::rust::string_utilities::is_string_const_identifier(
                    name,
                    gen.auto_clone_analysis.as_ref(),
                );
                if param_wants_owned_string && !arg_str.ends_with(".to_string()") && is_string_const
                {
                    arg_str = format!("{}.to_string()", arg_str);
                }
            }

            if let Some(ref sig) = signature {
                let all_params_borrowed = !sig.param_ownership.is_empty()
                    && sig
                        .param_ownership
                        .iter()
                        .all(|o| matches!(o, OwnershipMode::Borrowed));
                if all_params_borrowed
                    && matches!(arg, Expression::Identifier { .. })
                    && !arg_str.starts_with('&')
                    && !arg_str.ends_with(".clone()")
                {
                    let already_ref = if let Expression::Identifier { name, .. } = arg {
                        gen.identifier_already_ref(name)
                    } else {
                        false
                    };
                    let is_user_closure_param = if let Expression::Identifier { name, .. } = arg {
                        gen.in_user_written_closure && gen.user_closure_params.contains(name)
                    } else {
                        false
                    };
                    if !already_ref && !is_user_closure_param {
                        return vec![format!("&{}", arg_str)];
                    }
                }
            }

            // Check if this parameter expects a borrow
            // Skip ownership inference for extern function calls - they have explicit types
            if let Some(ref sig) = signature {
                if sig.is_extern {
                    // Auto-convert mut locals to &mut when FFI param is *mut T
                    // This eliminates Rust leakage: users write `ffi_fn(x)` not `ffi_fn(&mut x)`
                    if let Some(param_type) = sig.param_types.get(i) {
                        if matches!(
                            param_type,
                            crate::parser::ast::types::Type::RawPointer { mutable: true, .. }
                        ) {
                            return vec![format!("&mut {}", arg_str)];
                        }
                    }
                    return vec![arg_str];
                }

                // COLLISION GUARD: When the signature was resolved via a
                // simple-name fallback from a module-qualified call AND the
                // simple name has a collision, skip auto-borrow/auto-mutborrow.
                // The looked-up signature may be from the wrong module,
                // so applying its ownership blindly can produce incorrect
                // `&` or `&mut` prefixes.
                //
                // We only guard fallback-resolved signatures because:
                // - Direct qualified lookups are unambiguous (right signature)
                // - Bare-name calls within the same file are also unambiguous
                // - Only fallback from module::fn → fn is risky (wrong module)
                let simple_name = func_name.rsplit("::").next().unwrap_or(func_name);
                let has_ownership_collision = signature_from_simple_fallback
                    && (gen.signature_registry.has_collision(func_name)
                        || gen.signature_registry.has_collision(simple_name))
                    && {
                        // Validate collision: if the found signature's arg count
                        // matches the actual call, it's the right overload despite
                        // the collision. Only suppress ownership when arg count
                        // doesn't match (genuinely ambiguous signature).
                        let sig_args = if sig.has_self_receiver {
                            sig.param_ownership.len().saturating_sub(1)
                        } else {
                            sig.param_ownership.len()
                        };
                        sig_args != arguments.len()
                    };

                if let Some(&ownership) = sig.param_ownership.get(i) {
                    match ownership {
                        OwnershipMode::Borrowed if !has_ownership_collision => {
                            // PHASE 1: Generate &String parameters for correctness
                            // String literals need conversion: "foo" → &"foo".to_string()
                            let is_string_literal = matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            );

                            if is_string_literal {
                                // PHASE 2 CALL-SITE OPTIMIZATION: Check if parameter is &String vs &str
                                // In the AST, `string` parameters are Type::String (converted to &String by codegen)
                                // Explicit `&str` parameters are Type::Reference(Custom("str"))
                                let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                });

                                let asref_str_runtime = func_name
                                    .split("::")
                                    .next()
                                    .is_some_and(super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str);

                                if param_is_str_ref || asref_str_runtime {
                                    return vec![arg_str];
                                } else {
                                    let param_is_string = sig.param_types.get(i).is_some_and(|t| {
                                        crate::codegen::rust::string_utilities::param_is_owned_string_type(t)
                                    });
                                    if param_is_string {
                                        return vec![format!("&{}.to_string()", arg_str)];
                                    } else {
                                        return vec![arg_str];
                                    }
                                }
                            }

                            // TDD FIX: Check if parameter is already a reference type
                            // If param is &string, don't add another & (would be &&string)
                            let is_param_already_ref =
                                if let Expression::Identifier { name, .. } = arg {
                                    gen.current_function_params.iter().any(|param| {
                                        param.name == *name
                                            && matches!(
                                                &param.type_,
                                                Type::Reference(_)
                                                    | Type::MutableReference(_)
                                            )
                                    })
                                } else {
                                    false
                                };

                            // TDD FIX: Don't add & for Copy type parameters
                            // When signature says Borrowed but param type is Copy,
                            // codegen keeps it as owned (e.g., x: usize not x: &usize)
                            // So the call site should NOT add &
                            // BUT: Reference types (&Vec<T>, &[T]) are NOT treated as
                            // Copy here - if param type is &T, caller still needs &
                            let is_copy_param = sig
                                .param_types
                                .get(i)
                                .map(|t| {
                                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                        && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
                                })
                                .unwrap_or(false);

                            // TDD FIX (Bug #16): Don't add & to temp variables!
                            // Temp variables (like _temp0) hold OWNED values from format!()
                            // format!() returns String, not &str, so _temp0 is String
                            // If we add &, we get &String when we need String
                            let is_temp_variable = arg_str.starts_with("_temp")
                                && arg_str.chars().skip(5).all(|c| c.is_numeric());

                            // Strip .clone() when destination wants Borrowed — pass &field, not &field.clone()
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);

                            // Insert & if not already a reference and not a string literal and not a temp var
                            // THE WINDJAMMER WAY: Preserve user-written closure params
                            let is_user_closure_param =
                                if let Expression::Identifier { name, .. } = arg {
                                    gen.in_user_written_closure && gen.user_closure_params.contains(name)
                                } else {
                                    false
                                };

                            if !expression_helpers::is_reference_expression(arg)
                                && !is_param_already_ref
                                && !is_copy_param
                                && !is_temp_variable
                                && !is_user_closure_param
                            {
                                crate::codegen::rust::rust_coercion_rules::Coercion::Borrow
                                    .apply(&mut arg_str);
                                return vec![arg_str];
                            } else {
                                return vec![arg_str];
                            }
                        }
                        OwnershipMode::MutBorrowed if !has_ownership_collision => {
                            crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                                arg,
                                &mut arg_str,
                                &gen.current_function_params,
                                &gen.inferred_mut_borrowed_params,
                            );
                            return vec![arg_str];
                        }
                        OwnershipMode::Owned => {
                            let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            });
                            if param_is_str_ref {
                                // Owned String/local binding → borrow as &str via &String deref.
                                if !expression_helpers::is_reference_expression(arg)
                                    && !arg_str.starts_with('&')
                                {
                                    return vec![format!("&{}", arg_str)];
                                }
                                return vec![arg_str];
                            }

                            if let Expression::Identifier { name, .. } = arg {
                                arg_str = gen.maybe_auto_clone(name, &arg_str);

                                // Find the parameter type
                                let param_type = gen
                                    .current_function_params
                                    .iter()
                                    .find(|p| &p.name == name)
                                    .map(|p| &p.type_);

                                // Check if it's a reference parameter (&str, &String, &T, &mut T)
                                let inner_from_ref = match param_type {
                                    Some(Type::Reference(inner)) => Some(inner.as_ref()),
                                    Some(Type::MutableReference(inner)) => Some(inner.as_ref()),
                                    _ => None,
                                };
                                if let Some(inner_type) = inner_from_ref {
                                    if matches!(inner_type, Type::String)
                                        && !arg_str.ends_with(".to_string()")
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    } else if gen.is_type_copy(inner_type)
                                        && !arg_str.trim_start().starts_with('*')
                                    {
                                        arg_str = format!("*{}", arg_str);
                                    } else if !arg_str.ends_with(".clone()")
                                        && !arg_str.trim_start().starts_with('*')
                                    {
                                        arg_str = format!("{}.clone()", arg_str);
                                    }
                                } else {
                                    // TDD FIX: Check if it's from a borrowed iterator (for loop)
                                    // Example: for npc_id in npc_ids { Member::new(npc_id) }
                                    // npc_id is &String from iterator, needs .clone() for owned String
                                    //
                                    // CRITICAL: We're in OwnershipMode::Owned block, which means
                                    // the DESTINATION parameter wants an owned value (String, not &String).
                                    //
                                    // Windjammer `string` parameters lower to `&str`: `.clone()` keeps
                                    // `&str` (E0308). Use `.to_string()` for text types instead.
                                    let is_borrowed_iterator_var =
                                        gen.borrowed_iterator_vars.contains(name);

                                    let is_inferred_borrowed =
                                        gen.inferred_borrowed_params.contains(name);

                                    let is_inferred_mut_borrowed =
                                        gen.inferred_mut_borrowed_params.contains(name);

                                    if (is_borrowed_iterator_var
                                        || is_inferred_borrowed
                                        || is_inferred_mut_borrowed)
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        // `*ident` = owned Copy from &/&mut (see Identifier
                                        // in_owned_value_context); do not append .clone().
                                        if !arg_str.trim_start().starts_with('*') {
                                            let is_text = gen
                                                .infer_expression_type(arg)
                                                .as_ref()
                                                .is_some_and(|t| {
                                                    crate::codegen::rust::types::is_windjammer_text_type(t)
                                                });
                                            let is_phase2_str_param = gen
                                                .str_ref_optimized_params
                                                .contains(name.as_str());
                                            if is_text && !is_phase2_str_param {
                                                arg_str =
                                                    format!("{}.to_string()", arg_str);
                                            } else if !is_text {
                                                // Borrowed from iterator or inferred - use .clone()
                                                // This handles &T → T for non-text types
                                                arg_str = format!("{}.clone()", arg_str);
                                            }
                                        }
                                    }
                                }
                            }

                            if !gen.in_call_argument_generation {
                                gen.maybe_clone_borrowed_field_for_owned_param(arg, &mut arg_str);
                            }
                            gen.maybe_clone_index_for_owned_param(arg, &mut arg_str);
                        }
                        _ => {
                            // Collision guard triggered: Borrowed or MutBorrowed
                            // with a signature collision. Don't apply auto-borrow;
                            // pass the argument as-is and let downstream Rust
                            // compilation determine the correct behavior.
                        }
                    }
                }
            } else {
                // No signature found — still check auto-clone analysis.
                // The auto-clone analysis tracks data flow (value moved then
                // used later) independently of callee signatures (Some has no
                // registry entry).
                if let Expression::Identifier { name, .. } = arg {
                    arg_str = gen.maybe_auto_clone(name, &arg_str);
                }
            }

            // AUTO-CAST int → float: when parameter expects f32/f64 but argument is int.
            // Skip when signature has a collision (different types with same name).
            if let Some(ref sig) = signature {
                let has_collision = gen.signature_registry.has_collision(func_name)
                    || gen.signature_registry.has_collision(func_str);
                if !has_collision {
                    if let Some(param_ty) = sig.param_types.get(i) {
                        let arg_ty = gen.infer_expression_type(arg);
                        crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
                            &mut arg_str, arg, param_ty, arg_ty.as_ref(),
                        );
                    }
                }
            }

            // Coerce owned String → &str when callee expects explicit &str (Phase 2 / FFI wrappers).
            // Also handle stale metadata with empty param_ownership: Windjammer `string`
            // params lower to borrowed &str at the callee definition site.
            if let Some(ref sig) = signature {
                if let Some(param_ty) = sig.param_types.get(i) {
                    let param_is_str_ref = matches!(
                        param_ty,
                        Type::Reference(inner)
                            if matches!(**inner, Type::Custom(ref n) if n == "str")
                    );
                    let is_text_param = crate::codegen::rust::types::is_windjammer_text_type(param_ty);
                    let callee_borrows_string = !sig.is_extern
                        && !arg_str.contains("string_to_ffi(")
                        && (sig
                            .param_ownership
                            .get(i)
                            .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed))
                            || (sig.param_ownership.is_empty() && is_text_param));
                    let arg_already_ref =
                        if let Expression::Identifier { name, .. } = arg {
                            gen.identifier_already_ref(name)
                        } else {
                            false
                        };
                    if (param_is_str_ref || callee_borrows_string)
                        && !arg_str.contains("string_to_ffi(")
                        && !arg_str.starts_with('&')
                        && !arg_already_ref
                        && !matches!(
                            arg,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        )
                    {
                        arg_str = format!("&{}", arg_str);
                    }
                }
            }

            // Runtime std module auto-borrow: windjammer_runtime functions take &T
            // for non-Copy struct params (e.g. json::get(&value, ...) not json::get(value, ...)).
            // WJ stdlib declares owned params; the Rust side uses references.
            if !arg_str.starts_with('&') {
                let module = func_name.split("::").next().unwrap_or("");
                let param_type = signature
                    .as_ref()
                    .and_then(|sig| sig.param_types.get(i));
                let inferred_type = param_type.cloned().or_else(|| gen.infer_expression_type(arg));
                if let Some(ref ty) = inferred_type {
                    if super::super::super::stdlib_method_traits::runtime_std_param_needs_auto_borrow(
                        module, func_name, ty,
                    ) {
                        let already_ref = if let Expression::Identifier { name, .. } = arg {
                            gen.identifier_already_ref(name)
                        } else {
                            false
                        };
                        if !already_ref {
                            arg_str = format!("&{}", arg_str);
                        }
                    }
                }
            }

            vec![arg_str]
        })
        .collect()
}
