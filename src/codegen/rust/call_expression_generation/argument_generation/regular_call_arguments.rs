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
            let prev_field_access_obj = gen.in_field_access_object;
            gen.in_field_access_object = false;

            // TDD FIX: Set call argument context to suppress premature .clone()
            // The FieldAccess handler normally adds .clone() for borrowed iterator vars,
            // but in call arguments, we need to let the ownership check below decide
            let prev_in_call_arg = gen.in_call_argument_generation;
            gen.in_call_argument_generation = true;

            // Return/match contexts set `coerce_string_literals_to_owned` and
            // `in_match_arm_needing_string` for the outer expression; nested call
            // arguments must use only parameter-type conversion (below), not context
            // coercion — avoids `"x".to_string().to_string()` and wrong `.to_string()`
            // on &str params, and prevents format!("...".to_string(), ...) in match arms.
            let prev_coerce_string_literals = gen.coerce_string_literals_to_owned;
            gen.coerce_string_literals_to_owned = false;
            let prev_match_arm_str = gen.in_match_arm_needing_string;
            gen.in_match_arm_needing_string = false;
            let mut arg_str = gen.generate_expression(arg);
            gen.coerce_string_literals_to_owned = prev_coerce_string_literals;
            gen.in_match_arm_needing_string = prev_match_arm_str;

            gen.in_call_argument_generation = prev_in_call_arg;
            gen.in_field_access_object = prev_field_access_obj;

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
                        if matches!(param_type, Type::String)
                            || matches!(param_type, Type::Custom(n) if n == "string" || n == "String")
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
                        // Extern functions have explicit types; ownership inference
                        // is meaningless (empty body defaults to Borrowed).
                        // Convert if parameter type is String.
                        sig.param_types.get(i).is_some_and(|ty| {
                            matches!(ty, Type::String)
                                || matches!(ty, Type::Custom(name) if name == "string" || name == "String")
                        })
                    } else if sig.param_types.get(i).is_some_and(|ty| {
                        matches!(ty, Type::Reference(inner) if
                            matches!(**inner, Type::Custom(ref s) if s == "str"))
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
                        matches!(ty, Type::String)
                            || matches!(ty, Type::Custom(n) if n == "string" || n == "String")
                    })
                });
                let is_string_const = name.starts_with("SCOPE_")
                    || gen
                        .auto_clone_analysis
                        .as_ref()
                        .is_some_and(|a| a.string_literal_vars.contains(name));
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
                        let n = name.to_string();
                        gen.inferred_borrowed_params.contains(&n)
                            || gen.str_ref_optimized_params.contains(&n)
                            || gen.current_function_params.iter().any(|param| {
                                param.name == *name
                                    && matches!(
                                        &param.type_,
                                        crate::parser::ast::types::Type::Reference(_)
                                            | crate::parser::ast::types::Type::MutableReference(_)
                                    )
                            })
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
                                    matches!(t, Type::Reference(inner) if matches!(**inner, Type::Custom(ref name) if name == "str"))
                                });

                                let asref_str_runtime = func_name
                                    .split("::")
                                    .next()
                                    .is_some_and(super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str);

                                if param_is_str_ref || asref_str_runtime {
                                    // Parameter is explicitly &str - pass literal directly (already a &str)
                                    // "World" is already &str in Rust, no conversion needed!
                                    return vec![arg_str];
                                } else {
                                    // Parameter is Type::String (becomes &String in Rust)
                                    // Check if it's actually a String type
                                    let param_is_string = sig.param_types.get(i).is_some_and(|t| {
                                        matches!(t, Type::String) || matches!(t, Type::Custom(ref name) if name == "string")
                                    });
                                    if param_is_string {
                                        // Convert &str literal to &String: "World" → &"World".to_string()
                                        return vec![format!("&{}.to_string()", arg_str)];
                                    } else {
                                        // Non-string type - pass directly
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

                            // TDD FIX: IDIOMATIC WINDJAMMER - Strip .clone() if present!
                            // When destination wants Borrowed, pass &field, NOT &field.clone()
                            // Example: has_item(ingredient.item_id) with has_item(item_id: string)
                            // Should generate: has_item(&ingredient.item_id)
                            // NOT: has_item(&ingredient.item_id.clone())
                            // The .clone() may have been added by generate_expression for borrowed iterator vars
                            if arg_str.ends_with(".clone()") {
                                arg_str = arg_str[..arg_str.len() - 8].to_string();
                            }

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
                                return vec![format!("&{}", arg_str)];
                            } else {
                                return vec![arg_str];
                            }
                        }
                        OwnershipMode::MutBorrowed if !has_ownership_collision => {
                            // TDD FIX: Don't add &mut if arg is already a &mut parameter
                            // Covers both explicitly declared &mut params AND
                            // params inferred as &mut through ownership analysis
                            let is_already_mut_ref =
                                if let Expression::Identifier { name, .. } = arg {
                                    // Check 1: Explicit &mut in AST type
                                    let explicit_mut_ref = gen.current_function_params.iter().any(|param| {
                                        param.name == *name
                                            && matches!(
                                                &param.type_,
                                                Type::MutableReference(_)
                                            )
                                    });
                                    // Check 2: Inferred &mut through ownership analysis
                                    let inferred_mut_ref =
                                        gen.inferred_mut_borrowed_params.contains(name.as_str());
                                    explicit_mut_ref || inferred_mut_ref
                                } else {
                                    false
                                };

                            // Insert &mut if not already a reference
                            if !expression_helpers::is_reference_expression(arg)
                                && !is_already_mut_ref
                            {
                                let mut_arg_str = if arg_str.ends_with(".clone()") {
                                    arg_str[..arg_str.len() - 8].to_string()
                                } else {
                                    arg_str
                                };
                                return vec![format!("&mut {}", mut_arg_str)];
                            }
                        }
                        OwnershipMode::Owned => {
                            // String optimization override: param_types may say &str
                            // while param_ownership is stale as Owned. Trust param_types.
                            let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                matches!(t, Type::Reference(inner) if
                                    matches!(**inner, Type::Custom(ref s) if s == "str"))
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
                                if let Some(ref analysis) = gen.auto_clone_analysis {
                                    if analysis
                                        .needs_clone(name, gen.current_statement_idx)
                                        .is_some()
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        let binding_is_copy = gen
                                            .current_function_params
                                            .iter()
                                            .find(|p| p.name == *name)
                                            .is_some_and(|p| gen.is_type_copy(&p.type_))
                                            || gen
                                                .local_var_types
                                                .get(name)
                                                .is_some_and(|t| gen.is_type_copy(t));
                                        if !binding_is_copy {
                                            let clone_base = if arg_str.contains(" as ")
                                                && !arg_str.starts_with('(')
                                            {
                                                format!("({})", arg_str)
                                            } else {
                                                arg_str.clone()
                                            };
                                            arg_str = format!("{}.clone()", clone_base);
                                        }
                                    }
                                }

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

                            // TDD FIX: AUTO-CLONE for borrowed_param.field
                            // When passing ingredient.item_id where ingredient is borrowed,
                            // we need to clone() IF destination wants Owned.
                            //
                            // We're ALREADY in OwnershipMode::Owned block,
                            // so destination wants owned. Safe to add .clone().
                            //
                            // This handles: for ingredient in &vec { func(ingredient.field) }
                            // where func(field: String) expects owned.
                            //
                            // Skip in call arguments: ownership lowering below already
                            // applies & / &mut / clone for the callee signature.
                            if !gen.in_call_argument_generation {
                            if let Expression::FieldAccess { .. } = arg {
                                // Trace through nested field accesses to find the root identifier
                                // Handles: stack.field, stack.item.id, stack.item.nested.deep
                                let root_name = gen.extract_root_identifier(arg);
                                if let Some(name) = root_name {
                                    let is_borrowed_iterator_var =
                                        gen.borrowed_iterator_vars.contains(&name);
                                    let is_explicitly_borrowed =
                                        gen.current_function_params.iter().any(|p| {
                                            p.name == name
                                                && matches!(
                                                    p.ownership,
                                                    crate::parser::OwnershipHint::Ref
                                                )
                                        });
                                    let is_inferred_borrowed =
                                        gen.inferred_borrowed_params.contains(&name);

                                    if (is_borrowed_iterator_var
                                        || is_explicitly_borrowed
                                        || is_inferred_borrowed)
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        let is_copy = gen
                                            .infer_expression_type(arg)
                                            .as_ref()
                                            .is_some_and(|t| gen.is_type_copy(t));
                                        if !is_copy {
                                            arg_str = format!("{}.clone()", arg_str);
                                        }
                                    }
                                }
                            }
                            }
                            // DOGFOODING FIX: Vec indexing &vec[idx] passed to owned param
                            // e.g. enterable.push(gen.buildings[i]) → need (.clone())
                            if let Expression::Index { .. } = arg {
                                if arg_str.starts_with("&") && !arg_str.ends_with(".clone()") {
                                    if let Some(inner) = gen.infer_expression_type(arg) {
                                        if !gen.is_type_copy(&inner) {
                                            arg_str = format!("({}).clone()", arg_str);
                                        }
                                    }
                                }
                            }
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
                // No signature found - don't auto-clone!
                // Without signature info, we can't know if destination wants Owned or Borrowed
                // Better to let Rust compiler catch the error than guess wrong
            }

            // AUTO-CAST int → float: regular Call path
            // Skip when the signature key has a collision (different types registered
            // the same function name with different param types). The auto-cast
            // cannot be trusted when the looked-up signature may be from a different
            // type in another module.
            if let Some(ref sig) = signature {
                let has_collision = gen.signature_registry.has_collision(func_name)
                    || gen.signature_registry.has_collision(func_str);
                if !has_collision {
                    if let Some(param_ty) = sig.param_types.get(i) {
                        let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                        let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                        if param_is_f32 || param_is_f64 {
                            let arg_ty = gen.infer_expression_type(arg);
                            let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                matches!(t, Type::Int)
                                    || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                            });
                            if arg_is_int
                                && !arg_str.contains(" as f32")
                                && !arg_str.contains(" as f64")
                            {
                                let target = if param_is_f32 { "f32" } else { "f64" };
                                arg_str = if arg_str.contains(' ')
                                    || matches!(arg, Expression::Binary { .. })
                                {
                                    format!("({}) as {}", arg_str, target)
                                } else {
                                    format!("{} as {}", arg_str, target)
                                };
                            }
                        }
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
                            gen.inferred_borrowed_params.contains(&name.to_string())
                                || gen.str_ref_optimized_params.contains(&name.to_string())
                                || gen.current_function_params.iter().any(|param| {
                                    param.name == *name
                                        && matches!(
                                            &param.type_,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                })
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

            vec![arg_str]
        })
        .collect()
}
