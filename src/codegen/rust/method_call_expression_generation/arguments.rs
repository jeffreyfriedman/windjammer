//! Method-call argument codegen.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use crate::codegen::rust::expression_helpers;
use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn mc_build_method_call_arg_strings(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        type_name: Option<String>,
    ) -> (Vec<String>, Option<Type>) {
        // Float method argument context: for methods like clamp/max/min on float
        // receivers, arguments should use the same float type as the receiver.
        let prev_float_target = self.assignment_float_target_type.clone();
        let receiver_float_type = self.infer_expression_type(object);
        let is_float_method = crate::type_classification::is_float_receiver_method(method);
        if is_float_method {
            if let Some(ref rft) = receiver_float_type {
                match rft {
                    Type::Custom(n) if n == "f64" => {
                        self.assignment_float_target_type = Some(Type::Custom("f64".to_string()));
                    }
                    Type::Custom(n) if n == "f32" => {
                        self.assignment_float_target_type = Some(Type::Custom("f32".to_string()));
                    }
                    Type::Float => {
                        self.assignment_float_target_type = Some(Type::Custom("f64".to_string()));
                    }
                    _ => {}
                }
            }
        }

        let args_vec: Vec<String> = arguments
            .iter()
            .enumerate()
            .map(|(i, (_label, arg))| {
                // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                // Fix: Suppress clone when param expects Borrowed -> just add & to field
                let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                let param_expects_borrowed = method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                    .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Borrowed));

                let is_auto_borrow_target =
                    matches!(method, "push_str" | "extend_from_slice") && i == 0;

                let prev_suppress = self.suppress_borrowed_clone;
                if (param_expects_borrowed || is_auto_borrow_target)
                    && matches!(arg, Expression::FieldAccess { .. } | Expression::Identifier { .. })
                {
                    self.suppress_borrowed_clone = true;
                }

                // CRITICAL: Reset in_field_access_object for method argument generation.
                // Same rationale as function call arguments — method arguments are
                // independent expressions, not part of a field/method/index chain.
                // TDD FIX: STRIP explicit &ref when parameter expects owned value.
                // WINDJAMMER PHILOSOPHY: The developer shouldn't need to think about &.
                // If the user writes `&object.transform` but the method takes `Transform` (owned),
                // the compiler strips the & and passes by value (Copy types) or moves.
                // Example: self.render_transform(&object.transform) → self.render_transform(object.transform)
                //
                // TDD FIX: ALSO strip explicit & for HashMap/BTreeMap key methods with &String arguments.
                // HashMap<String, V>.contains_key() expects &str, not &&String.
                // User writes: map.contains_key(&key) where key is inferred as &String
                // Compiler generates: map.contains_key(key) which auto-derefs &String to &str ✅
                let arg_to_generate = if let Expression::Unary {
                    op: crate::parser::UnaryOp::Ref,
                    operand,
                    ..
                } = arg
                {
                    let is_hashmap_key_method =
                        crate::codegen::rust::stdlib_method_traits::is_map_key_method(method) && i == 0;

                    if is_hashmap_key_method {
                        // Strip explicit `&ident` for map keys: `should_add_ref` will add `&` back when the
                        // Rust type is owned or a Copy `K` that still needs `&K`. For `key: &str` / `&String`
                        // parameters, `should_add_ref` stays false → we emit `get(key)` not `get(&key)` (E0277).
                        if let Expression::Identifier { .. } = &**operand {
                            operand
                        } else {
                            arg
                        }
                    } else if let Some(ref sig) = method_signature {
                        let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                        let param_is_owned = sig
                            .param_ownership
                            .get(sig_param_idx)
                            .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Owned));
                        if param_is_owned {
                            operand // Strip & — generate the inner expression
                        } else {
                            arg // Keep the & — parameter expects a reference
                        }
                    } else {
                        arg // No signature info — keep as-is
                    }
                } else {
                    arg // Not a & expression — keep as-is
                };

                let prev_field_access_obj = self.in_field_access_object;
                self.in_field_access_object = false;
                let prev_in_call_arg = self.in_call_argument_generation;
                self.in_call_argument_generation = true;
                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                self.coerce_string_literals_to_owned = false;
                let prev_match_arm_str = self.in_match_arm_needing_string;
                self.in_match_arm_needing_string = false;
                let mut arg_str = self.generate_expression(arg_to_generate);
                self.in_match_arm_needing_string = prev_match_arm_str;
                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                self.in_call_argument_generation = prev_in_call_arg;
                self.in_field_access_object = prev_field_access_obj;

                // Owned params still need `.clone()` when the arg is a non-Copy binding; suppressing
                // auto-clone during `generate_expression` (above) skips spurious clones for
                // `&mut` pattern bindings (e.g. `world` from `if let Some(world) = &mut self.world`).
                let sig_param_idx_early = if method_signature
                    .as_ref()
                    .is_some_and(|s| s.has_self_receiver)
                {
                    i + 1
                } else {
                    i
                };
                if method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership.get(sig_param_idx_early))
                    .is_some_and(|&o| matches!(o, OwnershipMode::Owned))
                {
                    let is_copy = self
                        .infer_expression_type(arg_to_generate)
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t));
                    let is_mut_ref_binding = self
                        .infer_expression_type(arg_to_generate)
                        .as_ref()
                        .is_some_and(|t| {
                            matches!(t, Type::MutableReference(_))
                                || matches!(t, Type::Reference(_))
                        });
                    if !is_copy
                        && !is_mut_ref_binding
                        && !arg_str.ends_with(".clone()")
                        && !Self::is_enum_variant_or_constructor(arg_to_generate)
                        && matches!(
                            arg_to_generate,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        )
                    {
                        arg_str = format!("{}.clone()", arg_str);
                    }
                }

                // TDD FIX: PHASE 2 CALL-SITE OPTIMIZATION
                // Strip unnecessary .to_string() when parameter was optimized to &str
                // Example: User writes `loader.load("name".to_string())` but Phase 2 optimized
                // the signature from `fn load(self, name: String)` to `fn load(self, name: &str)`.
                // Result: Call site should be `loader.load("name")` not `loader.load("name".to_string())`
                //
                // IMPORTANT: Only strip for &str parameters, NOT &String parameters!
                // &String parameters still need .to_string() (creates String, then borrows it)
                let mut to_string_stripped_for_str_param = false;
                if let Some(ref sig) = method_signature {
                    let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                        let param_is_str_slice_ref = if let Type::Reference(inner) = param_type {
                            matches!(&**inner, Type::Custom(name) if name == "str")
                        } else {
                            false
                        };
                        if param_is_str_slice_ref {
                            if arg_str.ends_with(".to_string()") {
                                arg_str = arg_str[..arg_str.len() - 12].to_string();
                                to_string_stripped_for_str_param = true;
                            } else if arg_str.ends_with(".into()") {
                                arg_str = arg_str[..arg_str.len() - 7].to_string();
                                to_string_stripped_for_str_param = true;
                            }
                            // Strip .into() from nested expressions (match arms, if-else blocks)
                            // where string literals were coerced to String but callee wants &str.
                            if arg_str.contains(".into()") {
                                arg_str = arg_str.replace(".into()", "");
                                to_string_stripped_for_str_param = true;
                            }
                        }
                    }
                }

                // TDD FIX: Vec index methods require usize arguments.
                // Int inference may resolve the literal to i32/u32/i64/u64 due to
                // conflicting constraints. Fix at codegen level: rewrite any
                // integer suffix to _usize for the first argument of known
                // index-taking methods.
                if i == 0
                    && crate::codegen::rust::stdlib_method_traits::is_index_taking_method(method)
                {
                    let is_int_literal = matches!(
                        arg,
                        Expression::Literal {
                            value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                            ..
                        }
                    );
                    if is_int_literal {
                        let int_suffixes =
                            ["_i32", "_i64", "_u32", "_u64", "_i16", "_u16", "_i8", "_u8"];
                        for suffix in &int_suffixes {
                            if arg_str.ends_with(suffix) {
                                arg_str = format!(
                                    "{}_usize",
                                    &arg_str[..arg_str.len() - suffix.len()]
                                );
                                break;
                            }
                        }
                    }
                }

                // TDD FIX: AUTO-WRAP function pointers in iterator adapter methods.
                // Rust's .filter()/.any()/.find() on iter() yield &&T, expecting FnMut(&&T) -> bool,
                // but bare function pointers fn(&T) -> bool don't auto-deref.
                // THE WINDJAMMER WAY: Users write the natural `filter(predicate)` and the
                // compiler generates `filter(|__e| predicate(__e))`.
                if i == 0
                    && crate::codegen::rust::stdlib_method_traits::is_closure_taking_method(method)
                {
                    if let Expression::Identifier { name, .. } = arg {
                        // Wrap function pointer parameters: iter adapters expect FnMut(&&T),
                        // but fn(&T) -> bool does not auto-deref (E0631).
                        let is_fn_ptr_param = self.current_function_params.iter().any(|p| {
                            p.name == *name && matches!(p.type_, Type::FunctionPointer { .. })
                        });
                        if is_fn_ptr_param {
                            arg_str = format!("|__e| {}(__e)", arg_str);
                        }
                    }
                }

                // TDD FIX: String literal ownership conversion
                // Windjammer philosophy: "sword" should work whether parameter wants String or &String
                // CRITICAL: Do NOT convert for explicit &str parameters! Only for inferred &String.
                let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                let param_ownership = method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership.get(sig_param_idx));
                let string_literal_converted = if is_string_literal {
                    // Check what the parameter wants

                    let asref_str_module = match object {
                        Expression::Identifier { name, .. } => {
                            self.is_imported_runtime_std_module(name)
                        }
                        _ => type_name
                            .as_deref()
                            .is_some_and(|t| self.is_imported_runtime_std_module(t)),
                    };

                    // CRITICAL: Check if parameter is explicitly &str (not inferred &String)
                    // Explicit &str parameters should NOT get .to_string() conversion
                    let param_type = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_types.get(sig_param_idx));
                    let is_explicit_str_ref = if let Some(Type::Reference(inner)) = param_type {
                        matches!(**inner, Type::String) ||
                        matches!(**inner, Type::Custom(ref s) if s == "str")
                    } else {
                        false
                    };

                    if is_explicit_str_ref || asref_str_module {
                        // Explicit &str parameter - no conversion needed
                        false
                    } else {
                        match param_ownership {
                            Some(&OwnershipMode::Owned) | Some(&OwnershipMode::Borrowed) => {
                                // Runtime `strings::*` and similar APIs take `AsRef<str>` — borrow, don't move.
                                if asref_str_module {
                                    if matches!(
                                        arg_to_generate,
                                        Expression::FieldAccess { .. }
                                            | Expression::Identifier { .. }
                                    ) && !arg_str.starts_with('&')
                                    {
                                        arg_str = format!("&{}", arg_str);
                                    }
                                    false
                                } else {
                                // TDD FIX: Both Owned and Borrowed string params need .to_string()
                                // Owned → String needs .to_string()
                                // Borrowed → &String needs .to_string() (then & is added later)
                                // String literals are &str, must allocate to get String/&String
                                arg_str = format!("{}.to_string()", arg_str);
                                true // Mark that we converted
                                }
                            }
                            _ => {
                                // No signature info - use heuristic (fallback to old logic)
                                if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(i, method, method_signature) {
                                    arg_str = format!("{}.to_string()", arg_str);
                                    true
                                } else {
                                    false
                                }
                            }
                        }
                    }
                } else {
                    false
                };

                // Runtime std modules (`strings.len(self.text)`) take `AsRef<str>` — borrow
                // owned string fields/vars instead of moving out of `&mut self`.
                if !is_string_literal {
                    let asref_str_module = match object {
                        Expression::Identifier { name, .. } => {
                            self.is_imported_runtime_std_module(name)
                        }
                        _ => type_name
                            .as_deref()
                            .is_some_and(|t| self.is_imported_runtime_std_module(t)),
                    };
                    let arg_is_string = self
                        .infer_expression_type(arg_to_generate)
                        .as_ref()
                        .is_some_and(|t| crate::codegen::rust::string_utilities::param_is_owned_string_type(t));
                    if asref_str_module
                        && arg_is_string
                        && matches!(
                            arg_to_generate,
                            Expression::FieldAccess { .. } | Expression::Identifier { .. }
                        )
                        && !arg_str.starts_with('&')
                        && !arg_str.ends_with(".clone()")
                    {
                        arg_str = format!("&{}", arg_str);
                    }
                }

                // TDD FIX: If we converted string literal for Borrowed parameter,
                // we need to add & since .to_string() produces String but param wants &String
                if string_literal_converted {
                    if let Some(&OwnershipMode::Borrowed) = param_ownership {
                        // .to_string() produces String, but Borrowed param wants &String
                        // So we need to add &
                        arg_str = format!("&{}", arg_str);
                    }
                }

                // TDD FIX: AUTO-CONVERT &str → String for method calls
                // When passing a Phase 2 optimized &str parameter to a method expecting owned String, convert it
                // This handles cases like: HashMap::insert(key, value) where key is &str but insert expects String
                if let Expression::Identifier { name, .. } = arg_to_generate {
                    let is_string_const = crate::codegen::rust::string_utilities::is_string_const_identifier(
                        name,
                        self.auto_clone_analysis.as_ref(),
                    );
                    let sig_param_idx = if method_signature
                        .as_ref()
                        .is_some_and(|s| s.has_self_receiver)
                    {
                        i + 1
                    } else {
                        i
                    };
                    let wants_string = method_signature.as_ref().and_then(|sig| {
                        sig.param_types.get(sig_param_idx).map(|ty| {
                            crate::codegen::rust::string_utilities::param_is_owned_string_type(ty)
                        })
                    }).unwrap_or(false);
                    // `Vec<String>::push(SCOPE_*)` — push param is generic `T`; const is `&'static str`.
                    let needs_owned_string = wants_string
                        || (matches!(
                            method,
                            "push" | "insert" | "extend" | "append" | "push_front" | "push_back"
                                | "add" | "fill"
                        ) && is_string_const);
                    if needs_owned_string && is_string_const && !arg_str.ends_with(".to_string()")
                    {
                        arg_str = format!("{}.to_string()", arg_str);
                    }

                    let is_str_ref_optimized =
                        self.str_ref_optimized_params.contains(name.as_str());

                    if is_str_ref_optimized {
                        let sig_param_idx = if method_signature
                            .as_ref()
                            .is_some_and(|s| s.has_self_receiver)
                        {
                            i + 1
                        } else {
                            i
                        };
                        if !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                            method_signature,
                            sig_param_idx,
                        ) {
                            let expects_owned = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                i,
                                method,
                                method_signature,
                            );

                            if expects_owned
                                && !arg_str.ends_with(".to_string()")
                                && !arg_str.ends_with(".clone()")
                            {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }
                    }
                }

                // AUTO .clone(): Add .clone() when needed for borrowed values
                if let Expression::Identifier { name, .. } = arg {
                    let sig_param_idx = if method_signature
                        .as_ref()
                        .is_some_and(|s| s.has_self_receiver)
                    {
                        i + 1
                    } else {
                        i
                    };
                    let param_is_mut_borrowed = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                        .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed))
                        || method_signature.as_ref().and_then(|sig| {
                            sig.param_types.get(sig_param_idx).map(|t| {
                                matches!(t, Type::MutableReference(_))
                            })
                        }).unwrap_or(false);
                    let param_is_borrowed_map_key = i == 0
                        && crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                        && (method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                            .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed))
                            || self.borrowed_iterator_vars.contains(name));
                    // Borrowed iterator vars (for x in &vec) are already references.
                    // Cloning them produces owned values, changing the type. Skip
                    // auto-clone when the return type indicates we're collecting refs.
                    let is_borrowed_iter_collecting_refs =
                        self.borrowed_iterator_vars.contains(name)
                            && matches!(
                                &self.current_function_return_type,
                                Some(Type::Vec(inner)) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_))
                            );
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if !param_is_mut_borrowed
                            && !param_is_borrowed_map_key
                            && !is_borrowed_iter_collecting_refs
                            && analysis
                                .needs_clone(name, self.current_statement_idx)
                                .is_some()
                            && !arg_str.ends_with(".clone()")
                        {
                            arg_str = format!("{}.clone()", arg_str);
                        }
                    }
                }

                if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_clone(
                    arg,
                    &arg_str,
                    method,
                    i,
                    method_signature,
                    &self.borrowed_iterator_vars,
                    &self.current_function_params,
                    &self.inferred_borrowed_params,
                    &self.current_function_return_type,
                ) {
                    arg_str = format!("{}.clone()", arg_str);
                }

                // DOGFOODING FIX: Vec indexing vec[idx] passed to owned param (e.g. push)
                // should_add_clone handles Identifier/FieldAccess; Index needs explicit check
                // Vec::push uses stdlib heuristics (method_signature=None) - param 0 expects Owned
                if let Expression::Index { .. } = arg {
                    let sig_param_idx = method_signature
                        .as_ref()
                        .map(|s| if s.has_self_receiver { i + 1 } else { i })
                        .unwrap_or(i);
                    let param_expects_owned = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                        .is_some_and(|&o| matches!(o, OwnershipMode::Owned))
                        || (matches!(
                            method,
                            "push" | "insert" | "extend" | "append" | "push_front" | "push_back"
                                | "add" | "fill"
                        ) && i == 0);
                    if param_expects_owned && !arg_str.ends_with(".clone()") {
                        let inferred = self.infer_expression_type(arg);
                        let is_copy = inferred.as_ref().is_some_and(|t| self.is_type_copy(t));
                        let is_value_constructor = Self::is_enum_variant_or_constructor(arg);
                        if is_copy || is_value_constructor {
                            if arg_str.starts_with("&") {
                                arg_str = arg_str
                                    .strip_prefix('&')
                                    .unwrap_or(&arg_str)
                                    .to_string();
                            }
                        } else {
                            if arg_str.starts_with("&") {
                                arg_str = format!("({}).clone()", arg_str);
                            } else {
                                arg_str = format!("{}.clone()", arg_str);
                            }
                        }
                    }
                }

                // TDD FIX: Strip unnecessary .clone() when method param is Borrowed
                // When a field like `ingredient.item_id` is auto-cloned by the
                // FieldAccess handler (because owner is borrowed), but the method
                // expects &String (Borrowed), the clone is wasteful:
                //   &ingredient.item_id.clone()  ← clones then borrows (wasteful)
                //   &ingredient.item_id          ← borrows directly (correct)
                // Strip the .clone() so should_add_ref can add & cleanly.
                if let Some(ref sig) = method_signature {
                    let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                    let param_is_borrowed = sig
                        .param_ownership
                        .get(sig_param_idx)
                        .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                    if param_is_borrowed && arg_str.ends_with(".clone()") {
                        arg_str = arg_str[..arg_str.len() - 8].to_string();
                    }
                }

                // `if let Some(world) = &mut self.world` — pass owned `World`/`Entity`, not `&mut world`.
                if let Expression::Identifier { name, .. } = arg_to_generate {
                    if self.match_arm_bindings.contains(name.as_str()) {
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_mut_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
                            let wants_owned = sig.param_types.get(sig_param_idx).is_some_and(|ty| {
                                matches!(ty, Type::Custom(n) if n == "World" || n == "Entity")
                            });
                            if wants_owned
                                && !param_is_mut_borrowed
                                && !arg_str.ends_with(".clone()")
                            {
                                let base = arg_str
                                    .trim_start_matches("&mut ")
                                    .trim_start_matches('&');
                                arg_str = format!("{}.clone()", base);
                            }
                        }
                    }
                }

                // AUTO-MUT-BORROW: Add &mut when parameter expects MutBorrowed
                if let Some(ref sig) = method_signature {
                    let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                    let param_is_mut_borrowed = sig
                        .param_ownership
                        .get(sig_param_idx)
                        .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed))
                        || sig.param_types.get(sig_param_idx).is_some_and(|t| {
                            matches!(t, Type::MutableReference(_))
                        });
                    let param_wants_owned_value = sig.param_types.get(sig_param_idx).is_some_and(|ty| {
                        matches!(ty, Type::Custom(n) if n == "World" || n == "Entity")
                    });
                    if param_is_mut_borrowed && !param_wants_owned_value {
                        let is_already_mut_ref =
                            if let Expression::Identifier { name, .. } = arg {
                                let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                    param.name == *name
                                        && matches!(&param.type_, Type::MutableReference(_))
                                });
                                let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                explicit_mut_ref || inferred_mut_ref
                            } else {
                                false
                            };
                        if !expression_helpers::is_reference_expression(arg)
                            && !is_already_mut_ref
                        {
                            if arg_str.ends_with(".clone()") {
                                arg_str.truncate(arg_str.len() - 8);
                            }
                            if arg_str.starts_with("&") && !arg_str.starts_with("&mut ") {
                                arg_str = arg_str[1..].to_string();
                            }
                            crate::codegen::rust::rust_coercion_rules::Coercion::BorrowMut
                                .apply(&mut arg_str);
                        }
                    }
                }

                // AUTO-REF: Add & when parameter expects reference but arg is owned
                // Skip when .to_string() was stripped for &str param — the result is
                // already a bare literal/value that is &str, adding & would create &&str.
                if !string_literal_converted && !to_string_stripped_for_str_param {
                    // Use `arg_to_generate` (after stripping explicit `&` for map keys / owned params)
                    // so `should_add_ref` sees `key` not `&key` — otherwise the Unary(Ref) early-return
                    // skips HashMap `str` key handling and we emit `get(&key)` for `key: &str` (E0277).
                    let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                        arg_to_generate,
                        &arg_str,
                        method,
                        i,
                        method_signature,
                        &self.usize_variables,
                        &self.current_function_params,
                        &self.borrowed_iterator_vars,
                        &self.inferred_borrowed_params,
                        arguments.len(),
                        type_name.as_deref(),
                        Some(&self.local_var_types),
                        Some(&self.stdlib_method_signatures),
                        Some(&self.method_signatures_by_type),
                        &self.match_arm_bindings, // TDD FIX: Pass match arm bindings for E0308 fix
                    );
                    if should_ref {
                        if let Expression::Cast { .. } = arg_to_generate {
                            arg_str = format!("&({})", arg_str);
                        } else {
                            arg_str = format!("&{}", arg_str);
                        }
                    }
                }

                let sig_param_idx_str_field = method_signature.as_ref().map(|sig| {
                    if sig.has_self_receiver {
                        i + 1
                    } else {
                        i
                    }
                });
                if let Some(idx) = sig_param_idx_str_field {
                    arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                        method_signature,
                        idx,
                        arg_to_generate,
                        arg_str,
                        string_literal_converted || to_string_stripped_for_str_param,
                    );
                }

                // AUTO-BORROW: Methods that take &T or &[T] should auto-borrow
                // when given owned values. Eliminates Rust leakage in .wj files.
                let is_auto_borrow = matches!(method, "push_str" | "extend_from_slice");
                let is_map_method = matches!(
                    method,
                    "get" | "get_mut" | "contains_key" | "remove" | "get_key_value"
                )
                    && i == 0
                    && {
                        let obj_ty = self.infer_expression_type(object);
                        obj_ty.as_ref().is_some_and(|t| matches!(t,
                            Type::Parameterized(base, _) if base == "HashMap" || base == "BTreeMap" || base == "Map"
                        ))
                    };
                if (is_auto_borrow || is_map_method) && i == 0 {
                    let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                    let arg_already_ref = {
                        let arg_ty = self.infer_expression_type(arg);
                        let ty_is_ref = arg_ty.as_ref().is_some_and(|t| matches!(t,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) || matches!(t, Type::Custom(n) if n == "&str"));
                        let param_is_borrowed = match arg {
                            Expression::Identifier { name, .. } =>
                                self.inferred_borrowed_params.contains(&name.to_string()),
                            _ => false,
                        };
                        let is_borrowed_iter = match arg {
                            Expression::Identifier { name, .. } =>
                                self.borrowed_iterator_vars.contains(name),
                            _ => false,
                        };
                        ty_is_ref || param_is_borrowed || is_borrowed_iter
                    };
                    if !is_string_literal && !arg_str.starts_with('&') && !arg_already_ref {
                        let needs_borrow = matches!(arg,
                            Expression::Identifier { .. } |
                            Expression::FieldAccess { .. } |
                            Expression::MethodCall { .. }
                        );
                        if needs_borrow {
                            arg_str = format!("&{}", arg_str);
                        }
                    }
                }

                // AUTO-CAST int → float
                {
                    let effective_sig = method_signature.as_ref();
                    let qualified_method = self.infer_type_name(object)
                        .map(|tn| format!("{}::{}", tn, method));
                    let has_collision = qualified_method.as_deref()
                        .is_some_and(|q| self.signature_registry.has_collision(q))
                        || self.signature_registry.has_collision(method);
                    if let Some(sig) = effective_sig {
                        let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                        if !has_collision {
                            if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                                let arg_ty = self.infer_expression_type(arg);
                                crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
                                    &mut arg_str, arg, param_ty, arg_ty.as_ref(),
                                );
                            }
                        }
                    }
                }

                // Restore suppress flag
                self.suppress_borrowed_clone = prev_suppress;

                arg_str
            })
            .collect();

        (args_vec, prev_float_target)
    }

    fn is_enum_variant_or_constructor(expr: &Expression) -> bool {
        match expr {
            Expression::StructLiteral { .. } => true,
            Expression::Identifier { name, .. } => {
                Self::is_enum_variant_qualified_path(name)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } if arguments.is_empty() => {
                if let Expression::FieldAccess { object, .. } = &**function {
                    matches!(&**object, Expression::Identifier { name, .. } if name.chars().next().is_some_and(|c| c.is_uppercase()))
                } else {
                    false
                }
            }
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name.chars().next().is_some_and(|c| c.is_uppercase()))
            }
            _ => false,
        }
    }
}
