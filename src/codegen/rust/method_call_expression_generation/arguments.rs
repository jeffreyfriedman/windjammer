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

                const AUTO_BORROW_METHODS: &[&str] = &["push_str", "extend_from_slice"];
                let is_auto_borrow_target = AUTO_BORROW_METHODS.contains(&method) && i == 0;

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
                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                self.coerce_string_literals_to_owned = false;
                let prev_match_arm_str = self.in_match_arm_needing_string;
                self.in_match_arm_needing_string = false;
                let mut arg_str = self.generate_expression(arg_to_generate);
                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                self.in_match_arm_needing_string = prev_match_arm_str;
                self.in_field_access_object = prev_field_access_obj;

                // TDD FIX: PHASE 2 CALL-SITE OPTIMIZATION
                // Strip unnecessary .to_string() when parameter was optimized to &str
                // Example: User writes `loader.load("name".to_string())` but Phase 2 optimized
                // the signature from `fn load(self, name: String)` to `fn load(self, name: &str)`.
                // Result: Call site should be `loader.load("name")` not `loader.load("name".to_string())`
                //
                // IMPORTANT: Only strip for &str parameters, NOT &String parameters!
                // &String parameters still need .to_string() (creates String, then borrows it)
                if let Some(ref sig) = method_signature {
                    let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                        // Check if parameter is specifically &str (not &String!)
                        let param_is_str_slice_ref = if let Type::Reference(inner) = param_type {
                            matches!(&**inner, Type::Custom(name) if name == "str")
                        } else {
                            false
                        };
                        if param_is_str_slice_ref && arg_str.ends_with(".to_string()") {
                            // Strip .to_string() - &str accepts string literals directly
                            arg_str = arg_str[..arg_str.len() - 12].to_string();
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
                        // Only wrap bare function pointers — not params/locals (e.g. filter(asset_type)).
                        let is_param = self
                            .current_function_params
                            .iter()
                            .any(|p| p.name == *name);
                        let is_local = self
                            .local_variable_scopes
                            .iter()
                            .any(|scope| scope.contains(name));
                        if !is_param && !is_local {
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

                    if is_explicit_str_ref {
                        // Explicit &str parameter - no conversion needed
                        false
                    } else {
                        match param_ownership {
                            Some(&OwnershipMode::Owned) | Some(&OwnershipMode::Borrowed) => {
                                // TDD FIX: Both Owned and Borrowed string params need .to_string()
                                // Owned → String needs .to_string()
                                // Borrowed → &String needs .to_string() (then & is added later)
                                // String literals are &str, must allocate to get String/&String
                                arg_str = format!("{}.to_string()", arg_str);
                                true // Mark that we converted
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
                    let is_string_const = name.starts_with("SCOPE_")
                        || self
                            .auto_clone_analysis
                            .as_ref()
                            .is_some_and(|a| a.string_literal_vars.contains(name));
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
                            matches!(ty, Type::String)
                                || matches!(ty, Type::Custom(n) if n == "string" || n == "String")
                        })
                    }).unwrap_or(false);
                    // `Vec<String>::push(SCOPE_*)` — push param is generic `T`; const is `&'static str`.
                    let needs_owned_string =
                        wants_string || (method == "push" && is_string_const);
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
                        || (method == "push" && i == 0);
                    if param_expects_owned && !arg_str.ends_with(".clone()") {
                        let inferred = self.infer_expression_type(arg);
                        let is_copy = inferred.as_ref().is_some_and(|t| self.is_type_copy(t));
                        if is_copy {
                            if arg_str.starts_with("&") {
                                arg_str = arg_str
                                    .strip_prefix('&')
                                    .unwrap_or(&arg_str)
                                    .to_string();
                            }
                        } else {
                            // Non-Copy or unknown type: clone to prevent E0507
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
                            let wants_owned = sig.param_types.get(sig_param_idx).is_some_and(|ty| {
                                matches!(ty, Type::Custom(n) if n == "World" || n == "Entity")
                            });
                            if wants_owned && !arg_str.ends_with(".clone()") {
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
                        .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
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
                                arg_str = arg_str[..arg_str.len() - 8].to_string();
                            }
                            if arg_str.starts_with("&") && !arg_str.starts_with("&mut ") {
                                arg_str = arg_str[1..].to_string();
                            }
                            arg_str = format!("&mut {}", arg_str);
                        }
                    }
                }

                // AUTO-REF: Add & when parameter expects reference but arg is owned
                if !string_literal_converted {
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
                        string_literal_converted,
                    );
                }

                // AUTO-BORROW: Methods that take &T or &[T] should auto-borrow
                // when given owned values. Eliminates Rust leakage in .wj files.
                let auto_borrow_methods = ["push_str", "extend_from_slice"];
                let map_key_methods = ["remove", "get", "contains_key", "entry"];
                let is_map_method = map_key_methods.contains(&method)
                    && i == 0
                    && {
                        let obj_ty = self.infer_expression_type(object);
                        obj_ty.as_ref().is_some_and(|t| matches!(t,
                            Type::Parameterized(base, _) if base == "HashMap" || base == "BTreeMap" || base == "Map"
                        ))
                    };
                if (auto_borrow_methods.contains(&method) || is_map_method) && i == 0 {
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
                        ty_is_ref || param_is_borrowed
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

                // AUTO-CAST int → float: when parameter expects f32/f64 but argument is int
                // Skip when signature has a collision (different types with same name).
                {
                    let effective_sig = method_signature.as_ref()
                        .or_else(|| self.signature_registry.get_signature(method));
                    let has_collision = self.signature_registry.has_collision(method);
                    if let Some(sig) = effective_sig {
                        let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                        if !has_collision {
                            if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                                let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                if param_is_f32 || param_is_f64 {
                                    let arg_ty = self.infer_expression_type(arg);
                                    let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                        matches!(t, Type::Int)
                                            || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                                    });
                                    if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                        let target = if param_is_f32 { "f32" } else { "f64" };
                                        arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                            format!("({}) as {}", arg_str, target)
                                        } else {
                                            format!("{} as {}", arg_str, target)
                                        };
                                    }
                                }
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
}
