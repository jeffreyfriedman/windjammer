//! Method-call argument codegen.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use crate::codegen::rust::CodeGenerator;

/// Rust stdlib methods whose closure parameter receives `&T` (not owned `T`).
/// For these methods, closure params are references and comparisons need deref.
fn is_ref_closure_method(method: &str) -> bool {
    matches!(
        method,
        "retain"
            | "filter"
            | "any"
            | "all"
            | "find"
            | "position"
            | "rposition"
            | "take_while"
            | "skip_while"
            | "partition"
            | "inspect"
    )
}

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
                let receiver_is_map = self.infer_expression_type(object).as_ref().is_some_and(
                    crate::codegen::rust::stdlib_method_traits::is_map_type,
                ) || type_name.as_ref().is_some_and(
                    |n| crate::codegen::rust::stdlib_method_traits::is_map_type_name(n),
                );
                let receiver_is_set = self.infer_expression_type(object).as_ref().is_some_and(
                    crate::codegen::rust::stdlib_method_traits::is_set_type,
                ) || type_name.as_ref().is_some_and(
                    |n| crate::codegen::rust::stdlib_method_traits::is_set_type_name(n),
                );
                let is_map_key_arg = crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                    && i == 0
                    && receiver_is_map;
                let is_set_key_arg = crate::codegen::rust::stdlib_method_traits::is_set_lookup_method(method)
                    && i == 0
                    && receiver_is_set;
                let is_collection_key_arg = is_map_key_arg || is_set_key_arg;
                let receiver_type_name = type_name.as_deref();

                let is_external_module_method = matches!(
                    object,
                    Expression::Identifier { name, .. }
                        if name.chars().next().is_some_and(|c| c.is_lowercase())
                );
                let external_module_mut_reborrow = is_external_module_method
                    && i == 0
                    && matches!(
                        arg,
                        Expression::Identifier { name, .. }
                            if self.inferred_mut_borrowed_params.contains(name)
                    );

                let call_site_sig = self.mc_select_call_site_signature(
                    object,
                    method,
                    arguments,
                    method_signature,
                );

                let sig_for_effective = call_site_sig.as_ref().or(method_signature.as_ref());
                let effective_ownership = if external_module_mut_reborrow {
                    Some(OwnershipMode::MutBorrowed)
                } else {
                    sig_for_effective.map(|sig| {
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig, i, receiver_type_name,
                        )
                    })
                };

                // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                // Fix: Suppress clone when param expects Borrowed -> just add & to field
                let param_expects_borrowed =
                    effective_ownership.is_some_and(|o| matches!(o, OwnershipMode::Borrowed));

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
                        if let Expression::Identifier { .. } = &**operand {
                            operand
                        } else {
                            arg
                        }
                    } else if let Some(ref sig) = method_signature {
                        let sig_param_idx = sig.arg_param_index(i);
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

                // TDD FIX for E0277: Methods like retain/filter/any/all pass &T to
                // their closure, so closure params are references. Mark them as
                // borrowed so binary comparisons (id != val) generate *id != val.
                let closure_borrowed_params: Vec<String> =
                    if is_ref_closure_method(method) {
                        if let Expression::Closure { parameters, .. } = arg_to_generate {
                            let mut added = Vec::new();
                            for p in parameters.iter() {
                                if !self.borrowed_iterator_vars.contains(p) {
                                    self.borrowed_iterator_vars.insert(p.clone());
                                    added.push(p.clone());
                                }
                            }
                            added
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };

                let scope = self.arg_gen_scope();
                let mut arg_str = self.generate_expression(arg_to_generate);
                self.restore_arg_gen_scope(scope);
                arg_str = self
                    .peel_copy_ref_match_binding_for_value(arg_to_generate, &arg_str);

                for p in &closure_borrowed_params {
                    self.borrowed_iterator_vars.remove(p);
                }

                let callee_wants_str_borrow = call_site_sig.as_ref().is_some_and(|sig| {
                    let idx = sig.arg_param_index(i);
                    matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig, i, receiver_type_name,
                        ),
                        OwnershipMode::Borrowed,
                    ) || sig.param_types.get(idx).is_some_and(
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref,
                    )
                });

                // Owned params need `.clone()` when the arg is a non-Copy binding
                let arg_is_inferred_borrowed_param = matches!(
                    arg_to_generate,
                    Expression::Identifier { name, .. }
                        if self.inferred_borrowed_params.contains(name)
                            || self.inferred_mut_borrowed_params.contains(name)
                );
                let is_borrowed_iter_collecting_refs = matches!(
                    arg_to_generate,
                    Expression::Identifier { name, .. }
                        if self.borrowed_iterator_vars.contains(name)
                ) && matches!(
                    &self.current_function_return_type,
                    Some(Type::Vec(inner)) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_))
                );
                if !external_module_mut_reborrow
                    && !is_collection_key_arg
                    && !is_borrowed_iter_collecting_refs
                    && !self.in_user_written_closure
                    && !matches!(arg_to_generate, Expression::Closure { .. })
                    && !callee_wants_str_borrow
                    && effective_ownership.is_some_and(|o| matches!(o, OwnershipMode::Owned))
                    && (!arg_is_inferred_borrowed_param
                        || matches!(arg_to_generate, Expression::Identifier { name, .. }
                            if self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                            })))
                {
                    let is_copy = self
                        .infer_expression_type(arg_to_generate)
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t));
                    if !is_copy
                        && !arg_str.ends_with(".clone()")
                        && !arg_str.ends_with(".to_string()")
                        && !Self::is_enum_variant_or_constructor(arg_to_generate)
                        && matches!(
                            arg_to_generate,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        )
                    {
                        let is_text = self
                            .infer_expression_type(arg_to_generate)
                            .as_ref()
                            .is_some_and(|t| {
                                crate::codegen::rust::types::is_windjammer_text_type(t)
                            });
                        if is_text {
                            let already_owned_string = if let Expression::Identifier { name, .. } =
                                arg_to_generate
                            {
                                self.current_function_params.iter().any(|p| {
                                    p.name == *name
                                        && crate::codegen::rust::types::is_windjammer_text_type(
                                            &p.type_,
                                        )
                                        && !matches!(
                                            &p.type_,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                })
                            } else {
                                false
                            } || self.infer_expression_type(arg_to_generate).as_ref().is_some_and(
                                |t| matches!(t, Type::String),
                            );
                            let static_text_borrow = sig_for_effective.is_some_and(|sig| {
                                matches!(
                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                        sig, i, receiver_type_name,
                                    ),
                                    OwnershipMode::Borrowed,
                                )
                            });
                            if static_text_borrow {
                                let formal_is_plain_owned_string = sig_for_effective.is_some_and(|sig| {
                                    let idx = sig.arg_param_index(i);
                                    sig.formal_param_type(idx).is_some_and(|t| {
                                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                            && crate::codegen::rust::types::is_windjammer_text_type(t)
                                    })
                                });
                                if !formal_is_plain_owned_string
                                    && !arg_str.starts_with('&')
                                    && !arg_str.ends_with(".to_string()")
                                {
                                    let mut borrowed = arg_str.clone();
                                    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                        &mut borrowed,
                                    );
                                    arg_str = borrowed;
                                }
                            } else if !already_owned_string {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        } else {
                            arg_str = format!("{}.clone()", arg_str);
                        }
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
                    let sig_param_idx = sig.arg_param_index(i);
                    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                        let param_is_str_slice_ref = if let Type::Reference(inner) = param_type {
                            matches!(&**inner, Type::Custom(name) if name == "str")
                        } else {
                            false
                        };
                        let callee_wants_owned_string =
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ) == OwnershipMode::Owned
                                && sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                                });
                        if param_is_str_slice_ref && !callee_wants_owned_string {
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

                // CALLBACK BRIDGE: When a bare function identifier is passed as a
                // callback argument and the function's parameters have been auto-borrowed,
                // wrap it in a closure so the caller's owned args are correctly borrowed.
                // e.g. server.serve(handle_request) → server.serve(|__cb0| handle_request(&__cb0))
                if let Expression::Identifier { name, .. } = arg {
                    if let Some(func_sig) = self.signature_registry.get_signature(name) {
                        if !func_sig.has_self_receiver && !func_sig.is_extern {
                            let has_borrowed: Vec<usize> = func_sig
                                .param_ownership
                                .iter()
                                .enumerate()
                                .filter(|(_, o)| {
                                    matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                                })
                                .map(|(idx, _)| idx)
                                .collect();
                            if !has_borrowed.is_empty() {
                                let n = func_sig.param_ownership.len();
                                let wrapper: Vec<String> =
                                    (0..n).map(|j| format!("__cb{}", j)).collect();
                                let call: Vec<String> = (0..n)
                                    .map(|j| match func_sig.param_ownership[j] {
                                        OwnershipMode::MutBorrowed => format!("&mut __cb{}", j),
                                        OwnershipMode::Borrowed => format!("&__cb{}", j),
                                        _ => format!("__cb{}", j),
                                    })
                                    .collect();
                                arg_str = format!(
                                    "|{}| {}({})",
                                    wrapper.join(", "),
                                    name,
                                    call.join(", ")
                                );
                            }
                        }
                    }
                }

                // TDD FIX: String literal ownership conversion
                // Windjammer philosophy: "sword" should work whether parameter wants String or &String
                // CRITICAL: Do NOT convert for explicit &str parameters! Only for inferred &String.
                let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                let _param_ownership = method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership_for_arg(i));
                let string_literal_converted = if is_string_literal {
                    let effective_sig = type_name
                        .as_ref()
                        .and_then(|tn| {
                            self.lookup_method_signature_on_receiver_type(
                                tn,
                                method,
                                arguments.len(),
                            )
                        })
                        .or_else(|| method_signature.clone());

                    // Check what the parameter wants
                    let asref_str_module =
                        crate::codegen::rust::stdlib_method_traits::receiver_uses_asref_str_runtime_module(
                            None,
                            type_name.as_deref(),
                            |name| self.is_imported_runtime_std_module(name),
                        );

                    let param_type = effective_sig
                        .as_ref()
                        .and_then(|sig| sig.param_type_for_arg(i));
                    let is_explicit_str_ref = param_type
                        .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref);

                    let callee_param_is_rust_str = effective_sig.as_ref().is_some_and(|sig| {
                        let pi = sig.arg_param_index(i);
                        crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                            &Some(sig.clone()),
                            pi,
                        )
                    });

                    if asref_str_module {
                        false
                    } else if is_explicit_str_ref {
                        // Literal is already &str-compatible; never allocate .to_string().
                        false
                    } else if is_map_key_arg || callee_param_is_rust_str {
                        false
                    } else {
                        let needs_owned = crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
                            effective_sig.as_ref(),
                            i,
                            Some(method),
                            type_name.as_deref(),
                            Some(&self.enum_variant_types),
                            None,
                        );
                        if needs_owned {
                            arg_str = format!("{}.to_string()", arg_str);
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    false
                };

                if is_string_literal
                    && !string_literal_converted
                    && method_signature.as_ref().is_some_and(|sig| {
                        let pi = sig.arg_param_index(i);
                        !crate::codegen::rust::call_signature_resolution::static_impl_text_borrows_at_call_site(
                            sig, pi,
                        ) && matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Owned,
                        ) && sig.param_type_for_arg(i).is_some_and(
                            crate::codegen::rust::string_utilities::param_is_owned_string_type,
                        )
                    })
                {
                    arg_str = format!("{}.to_string()", arg_str);
                }

                // Runtime std modules (`strings.len(self.text)`) take `AsRef<str>` — borrow
                // owned string fields/vars instead of moving out of `&mut self`.
                if !is_string_literal {
                    let asref_str_module =
                        crate::codegen::rust::stdlib_method_traits::receiver_uses_asref_str_runtime_module(
                            None,
                            type_name.as_deref(),
                            |name| self.is_imported_runtime_std_module(name),
                        );
                    let param_is_string = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_type_for_arg(i))
                        .is_some_and(
                            crate::codegen::rust::string_utilities::param_is_owned_string_type,
                        );
                    let arg_is_string = crate::codegen::rust::string_utilities::expression_is_owned_string_for_asref_borrow(
                        arg_to_generate,
                        self.infer_expression_type(arg_to_generate).as_ref(),
                        &self.local_var_types,
                        &self.current_function_params,
                    );
                    if asref_str_module
                        && param_is_string
                        && (arg_is_string
                            || matches!(
                                arg_to_generate,
                                Expression::Identifier { .. } | Expression::FieldAccess { .. }
                            ))
                        && !arg_str.starts_with('&')
                        && !arg_str.ends_with(".clone()")
                    {
                        arg_str = format!("&{}", arg_str);
                    } else if type_name.as_deref().is_some_and(|tn| {
                        crate::codegen::rust::stdlib_method_traits::runtime_std_module_for_type(tn)
                            == Some("db")
                    }) && matches!(
                        method,
                        "query" | "execute" | "get_string" | "get_int" | "get_string_at"
                            | "get_int_at"
                    ) && i == 0
                        && matches!(
                            arg_to_generate,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        )
                        && !arg_str.starts_with('&')
                    {
                        arg_str = format!("&{}", arg_str);
                    }
                }

                // If we converted to owned String, do not re-borrow for stale Borrowed metadata.
                if string_literal_converted {
                    let effective_sig = type_name
                        .as_ref()
                        .and_then(|tn| {
                            self.lookup_method_signature_on_receiver_type(
                                tn,
                                method,
                                arguments.len(),
                            )
                        })
                        .or_else(|| method_signature.clone());
                    let still_borrowed = effective_sig.as_ref().is_some_and(|sig| {
                        let idx = sig.arg_param_index(i);
                        sig.param_types.get(idx).is_some_and(|ty| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
                        }) || crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                            &effective_sig,
                            idx,
                        )
                    });
                    if still_borrowed {
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
                    let wants_string = method_signature.as_ref().and_then(|sig| {
                        sig.param_type_for_arg(i).map(|ty| {
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
                        let is_map_key = crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                            && i == 0;
                        let param_idx_for_sig = method_signature.as_ref().map_or(i, |s| s.arg_param_index(i));
                        let callee_sig = call_site_sig
                            .clone()
                            .or(method_signature.clone())
                            .or_else(|| {
                                type_name.as_ref().and_then(|tn| {
                                    self.lookup_method_signature_on_receiver_type(
                                        tn,
                                        method,
                                        arguments.len(),
                                    )
                                })
                            });
                        let arg_is_owned_string_binding =
                            if let Expression::Identifier { name, .. } = arg_to_generate {
                                self.current_function_params.iter().any(|p| {
                                    p.name == *name
                                        && crate::codegen::rust::types::is_windjammer_text_type(
                                            &p.type_,
                                        )
                                        && !matches!(
                                            &p.type_,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                })
                            } else {
                                false
                            };
                        let callee_borrows = callee_sig.as_ref().is_some_and(|sig| {
                            matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                    sig, i, receiver_type_name,
                                ),
                                OwnershipMode::Borrowed,
                            )
                        }) || callee_sig.as_ref().is_some_and(|sig| {
                            sig.param_types
                                .get(sig.arg_param_index(i))
                                .is_some_and(
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref,
                                )
                        });
                        if !is_map_key
                            && !arg_is_owned_string_binding
                            && !callee_borrows
                            && !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                                &callee_sig,
                                param_idx_for_sig,
                            )
                        {
                            let expects_owned = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                i,
                                method,
                                &callee_sig,
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

                // HashMap::insert(key, value) — owned String keys from &str params/locals.
                if method == "insert"
                    && i == 0
                    && !arg_str.ends_with(".to_string()")
                    && !arg_str.starts_with('&')
                {
                    let param_wants_string = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_type_for_arg(i))
                        .is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_owned_string_type(t)
                        });
                    let arg_is_str_like = match arg_to_generate {
                        Expression::Identifier { name, .. } => {
                            let local_str = self.local_var_types.get(name).is_some_and(|t| {
                                if matches!(t, Type::String) {
                                    return true;
                                }
                                if let Type::Reference(inner) = t {
                                    return matches!(inner.as_ref(), Type::String)
                                        || matches!(inner.as_ref(), Type::Custom(s) if s == "str");
                                }
                                false
                            });
                            local_str || self.current_function_params.iter().any(|p| {
                                if p.name != *name {
                                    return false;
                                }
                                if p.type_ == Type::String {
                                    return true;
                                }
                                if let Type::Reference(inner) = &p.type_ {
                                    return matches!(inner.as_ref(), Type::String)
                                        || matches!(inner.as_ref(), Type::Custom(s) if s == "str");
                                }
                                false
                            })
                        }
                        _ => false,
                    };
                    if param_wants_string || arg_is_str_like {
                        arg_str = format!("{}.to_string()", arg_str);
                    }
                }

                // AUTO .clone(): Add .clone() when needed for borrowed values
                if let Expression::Identifier { name, .. } = arg {
                    let arg_is_inferred_borrowed_param = self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name);
                    let param_is_mut_borrowed = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_ownership_for_arg(i))
                        .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed))
                        || method_signature.as_ref().and_then(|sig| {
                            sig.param_type_for_arg(i).map(|t| {
                                matches!(t, Type::MutableReference(_))
                            })
                        }).unwrap_or(false);
                    let param_is_borrowed_map_key = i == 0
                        && crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                        && (method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership_for_arg(i))
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
                        if !arg_is_inferred_borrowed_param
                            && !external_module_mut_reborrow
                            && !param_is_mut_borrowed
                            && !param_is_borrowed_map_key
                            && !is_borrowed_iter_collecting_refs
                            && !param_expects_borrowed
                            && !is_auto_borrow_target
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
                    let param_expects_owned = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_ownership_for_arg(i))
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

                let arg_already_rust_ref = matches!(
                    arg_to_generate,
                    Expression::Identifier { name, .. }
                        if self.identifier_already_ref(name)
                            || self.str_ref_optimized_params.contains(name.as_str())
                );

                // Phase 3: unified call-site borrow lowering (param_expects_borrowed path)
                let call_site_sig = self.mc_select_call_site_signature(
                    object,
                    method,
                    arguments,
                    method_signature,
                );
                let mut borrow_decision =
                    crate::codegen::rust::call_site_borrow::CallSiteBorrowDecision::default();
                if let Some(ref sig) = call_site_sig {
                    borrow_decision =
                        crate::codegen::rust::call_site_borrow::should_borrow_at_call_site(
                            sig,
                            i,
                            arg_to_generate,
                            &arg_str,
                            method,
                            arg_already_rust_ref,
                            receiver_type_name,
                        );
                } else if let Some(receiver_tn) = self
                    .mc_infer_method_receiver_type_name(object)
                    .or_else(|| self.infer_type_name(object))
                {
                    let resolved_sig = self
                        .resolve_call_signature_with_global(
                            &format!("{receiver_tn}::{method}"),
                            Some(receiver_tn.as_str()),
                            arguments.len(),
                        )
                        .map(|r| r.sig)
                        .or_else(|| {
                            self.lookup_method_signature_on_receiver_type(
                                &receiver_tn,
                                method,
                                arguments.len(),
                            )
                        });
                    if let Some(sig) = resolved_sig {
                        borrow_decision =
                            crate::codegen::rust::call_site_borrow::should_borrow_at_call_site(
                                &sig,
                                i,
                                arg_to_generate,
                                &arg_str,
                                method,
                                arg_already_rust_ref,
                                receiver_type_name,
                            );
                    } else if is_collection_key_arg && !arg_str.starts_with('&')
                        && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_to_generate)
                    {
                        borrow_decision.add_ref = true;
                    }
                } else if is_collection_key_arg && !arg_str.starts_with('&')
                    && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_to_generate)
                {
                    borrow_decision.add_ref = true;
                }

                // Codegen-local guards preserved from pre-Phase-3 path
                if matches!(arg_to_generate, Expression::Identifier { name, .. }
                    if self.identifier_already_ref(name))
                {
                    borrow_decision.add_ref = false;
                }
                if matches!(arg_to_generate, Expression::Identifier { name, .. }
                    if self.str_ref_optimized_params.contains(name.as_str()))
                    && is_collection_key_arg
                {
                    borrow_decision.add_ref = false;
                }
                if matches!(arg_to_generate, Expression::StructLiteral { .. })
                    && (param_expects_borrowed || is_collection_key_arg)
                    && !arg_str.starts_with('&')
                {
                    borrow_decision.add_ref = false;
                }
                if let Some(ref sig) = call_site_sig {
                    let sig_param_idx = sig.arg_param_index(i);
                    let effective = crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                        sig, i, receiver_type_name,
                    );
                    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                        if matches!(param_ty, Type::Reference(_) | Type::MutableReference(_))
                            && effective != OwnershipMode::Owned
                            && !arg_str.starts_with('&')
                        {
                            let param_is_str_ref = crate::codegen::rust::string_utilities::param_is_rust_str_ref(
                                param_ty,
                            );
                            let arg_already_str_ref = matches!(
                                arg_to_generate,
                                Expression::Identifier { name, .. }
                                    if self.inferred_borrowed_params.contains(name)
                                        || self.str_ref_optimized_params.contains(name.as_str())
                                        || self.current_function_params.iter().any(|p| {
                                            p.name == *name
                                                && matches!(
                                                    &p.type_,
                                                    Type::Reference(inner)
                                                        if matches!(
                                                            inner.as_ref(),
                                                            Type::Custom(s) if s == "str"
                                                        )
                                                )
                                        })
                            );
                            let arg_is_str_literal = crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_to_generate);
                            if !(param_is_str_ref && (arg_already_str_ref || arg_is_str_literal)) && !arg_already_rust_ref {
                                borrow_decision.add_ref = true;
                            }
                        }
                    }
                }

                if borrow_decision.strip_clone {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                    // Owned-path may have added `.to_string()` before we knew callee takes &str.
                    if param_expects_borrowed && arg_str.ends_with(".to_string()")
                        && method_signature.as_ref().and_then(|sig| {
                            sig.param_type_for_arg(i)
                        }).is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        }) {
                            arg_str = arg_str[..arg_str.len() - 12].to_string();
                        }
                }

                if borrow_decision.add_ref
                    && !arg_str.starts_with('&')
                    && (!matches!(effective_ownership, Some(OwnershipMode::Owned))
                        || is_collection_key_arg)
                {
                    crate::codegen::rust::rust_coercion_rules::Coercion::Borrow
                        .apply(&mut arg_str);
                }

                // `if let Some(x) = &self.opt` — pass owned values via `.clone()`, not `&x` / `&mut x`.
                // Skip when callee expects a borrow (HashMap keys, &str params, etc.).
                // Match-arm bindings (owned enum payloads) need `&binding`, not `.clone()`.
                if !param_expects_borrowed && !is_collection_key_arg && !external_module_mut_reborrow {
                    if let Expression::Identifier { name, .. } = arg_to_generate {
                    if self.match_arm_bindings.contains(name.as_str()) {
                        // Fall through to should_add_ref / borrow_decision below.
                    } else {
                    let inferred = self.infer_expression_type(arg_to_generate);
                    let is_ref_binding = inferred
                        .as_ref()
                        .is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        })
                        || self.match_arm_bindings.contains(name.as_str());
                    // Skip Copy types — they don't need cloning (e.g. i32 from enum destructure,
                    // or &u32 from HashMap.get match arms after peel_copy_ref).
                    let is_copy = inferred.as_ref().is_some_and(|t| match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => self
                            .is_type_copy(inner.as_ref()),
                        other => crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(other),
                    });
                    if is_ref_binding && !is_copy {
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = sig.arg_param_index(i);
                            let param_is_mut_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
                            let param_is_owned = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::Owned));
                            if param_is_owned
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
                    }
                }

                // AUTO-MUT-BORROW: Add &mut when parameter expects MutBorrowed
                if external_module_mut_reborrow {
                    crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                        arg,
                        &mut arg_str,
                        &self.current_function_params,
                        &self.inferred_mut_borrowed_params,
                    );
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                } else if let Some(ref sig) = method_signature {
                    let sig_param_idx = sig.arg_param_index(i);
                    let ownership_is_mut = sig
                        .param_ownership
                        .get(sig_param_idx)
                        .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
                    let type_is_mut_ref = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                        matches!(t, Type::MutableReference(_))
                    });
                    let param_is_mut_borrowed = ownership_is_mut || type_is_mut_ref;
                    let param_wants_owned_value = sig.param_types.get(sig_param_idx).is_some_and(|ty| {
                        matches!(ty, Type::Custom(n) if n == "World" || n == "Entity")
                    });
                    if param_is_mut_borrowed && !param_wants_owned_value {
                        crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                            arg,
                            &mut arg_str,
                            &self.current_function_params,
                            &self.inferred_mut_borrowed_params,
                        );
                    }
                }

                // AUTO-REF: Add & when parameter expects reference but arg is owned
                // Skip when .to_string() was stripped for &str param — the result is
                // already a bare literal/value that is &str, adding & would create &&str.
                let callee_expects_owned = matches!(effective_ownership, Some(OwnershipMode::Owned))
                    || method_signature.as_ref().is_some_and(|sig| {
                        matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Owned,
                        )
                    })
                    || call_site_sig.as_ref().is_some_and(|sig| {
                        matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Owned,
                        )
                    });
                if !string_literal_converted
                    && !to_string_stripped_for_str_param
                    && !callee_expects_owned
                    && call_site_sig.is_none()
                {
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
                        &self.match_arm_bindings,
                        &self.str_ref_optimized_params,
                    );
                    if should_ref {
                        let push_owned_string = matches!(
                            method,
                            "push" | "insert" | "append" | "push_front" | "push_back" | "add"
                        ) && matches!(arg_to_generate, Expression::Identifier { name, .. }
                            if !self.inferred_borrowed_params.contains(name.as_str())
                                && !self.borrowed_iterator_vars.contains(name)
                                && self
                                    .infer_expression_type(arg_to_generate)
                                    .is_some_and(|t| matches!(t, Type::String)));
                        if !push_owned_string {
                            borrow_decision.add_ref = true;
                        }
                    }
                }

                // Map/set key methods: `key: &str` bindings must not become `&&str` (E0277).
                if is_collection_key_arg {
                    if let Expression::Identifier { name, .. } = arg_to_generate {
                        let key_already_str_ref = self.str_ref_optimized_params.contains(name.as_str())
                            || self.inferred_borrowed_params.contains(name)
                            || self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && (matches!(
                                        &p.type_,
                                        Type::Reference(inner)
                                            if matches!(inner.as_ref(), Type::Custom(s) if s == "str")
                                    ) || matches!(&p.type_, Type::Custom(s) if s == "str"))
                            });
                        if key_already_str_ref {
                            borrow_decision.add_ref = false;
                            if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                arg_str = arg_str.trim_start_matches('&').to_string();
                            }
                        }
                    }
                }

                if borrow_decision.add_ref
                    && !arg_str.starts_with('&')
                    && (!callee_expects_owned || is_collection_key_arg)
                    && (!matches!(effective_ownership, Some(OwnershipMode::Owned))
                        || is_collection_key_arg)
                    && !matches!(effective_ownership, Some(OwnershipMode::Borrowed))
                    && !method_signature.as_ref().is_some_and(|sig| {
                        let idx = sig.arg_param_index(i);
                        matches!(sig.param_ownership.get(idx), Some(OwnershipMode::Owned))
                            && !matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                    sig, i, receiver_type_name,
                                ),
                                OwnershipMode::Borrowed,
                            )
                    })
                {
                    if let Expression::Cast { .. } = arg_to_generate {
                        arg_str = format!("&({})", arg_str);
                    } else {
                        crate::codegen::rust::call_site_borrow::apply_call_site_borrow(
                            &crate::codegen::rust::call_site_borrow::CallSiteBorrowDecision {
                                add_ref: true,
                                ..Default::default()
                            },
                            &mut arg_str,
                        );
                    }
                }

                let sig_param_idx = method_signature
                    .as_ref()
                    .map(|sig| sig.arg_param_index(i))
                    .unwrap_or(i);
                arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                    method_signature,
                    sig_param_idx,
                    arg_to_generate,
                    arg_str,
                    string_literal_converted || to_string_stripped_for_str_param,
                );

                // Borrow owned `string` locals for `&str` formals when qualified lookup failed.
                if !is_map_key_arg
                    && !is_string_literal
                    && !arg_str.starts_with('&')
                    && !string_literal_converted
                    && !to_string_stripped_for_str_param
                    && !arg_str.ends_with(".to_string()")
                {
                    let receiver_type_name = type_name.clone().or_else(|| {
                        self.infer_expression_type(object).and_then(|t| match t {
                            Type::Custom(name) => Some(name),
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                if let Type::Custom(name) = inner.as_ref() {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                    });
                    let qualified = receiver_type_name
                        .as_deref()
                        .map(|tn| format!("{tn}::{method}"));
                    let registry_sig = receiver_type_name
                        .as_deref()
                        .and_then(|rt| {
                            self.signature_registry.find_method_on_receiver_type(
                                rt,
                                method,
                                arguments.len(),
                            )
                        })
                        .or_else(|| {
                            qualified
                                .as_ref()
                                .and_then(|q| self.signature_registry.get_signature(q))
                        })
                        .or_else(|| {
                            receiver_type_name.is_none().then(|| {
                                self.signature_registry
                                    .find_signature_by_name_and_arg_count(method, arguments.len())
                            }).flatten()
                        });
                    let sig_for_str_borrow = call_site_sig.as_ref().or(registry_sig);
                    let wants_str = sig_for_str_borrow.is_some_and(|sig| {
                        matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig,
                                i,
                                receiver_type_name.as_deref(),
                            ),
                            OwnershipMode::Borrowed,
                        )
                    });
                    if wants_str && !callee_expects_owned {
                        let param_is_registry_owned = sig_for_str_borrow.is_some_and(|sig| {
                            matches!(
                                sig.param_ownership.get(sig.arg_param_index(i)),
                                Some(OwnershipMode::Owned)
                            )
                        });
                        let formal_is_plain_owned_string = sig_for_str_borrow.is_some_and(|sig| {
                            let idx = sig.arg_param_index(i);
                            sig.formal_param_type(idx).is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && crate::codegen::rust::types::is_windjammer_text_type(t)
                            })
                        });
                        if param_is_registry_owned {
                            // trait/port metadata: owned string formal
                        } else if !formal_is_plain_owned_string {
                            let param_is_borrowed_text = sig_for_str_borrow.is_some_and(|sig| {
                                sig.param_type_for_arg(i).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                        || matches!(
                                            t,
                                            Type::Reference(inner)
                                                if crate::codegen::rust::types::is_windjammer_text_type(
                                                    inner,
                                                )
                                        )
                                })
                            });
                            if param_is_borrowed_text
                                && self
                                    .infer_expression_type(arg_to_generate)
                                    .as_ref()
                                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                            {
                                // String literals are already &str — adding & creates &&str.
                                // This covers both bare literals and "lit".to_string().
                                let is_string_literal_expr = matches!(
                                    arg_to_generate,
                                    Expression::Literal { value: Literal::String(_), .. }
                                ) || matches!(
                                    arg_to_generate,
                                    Expression::MethodCall { method: m, object, .. }
                                    if *m == "to_string" && matches!(
                                        &**object,
                                        Expression::Literal { value: Literal::String(_), .. }
                                    )
                                );
                                if !is_string_literal_expr {
                                    let mut borrowed = arg_str.clone();
                                    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                        &mut borrowed,
                                    );
                                    arg_str = borrowed;
                                }
                            }
                        }
                    }
                }

                // AUTO-BORROW: Methods that take &T or &[T] should auto-borrow
                // when given owned values. Eliminates Rust leakage in .wj files.
                let is_auto_borrow = matches!(method, "push_str" | "extend_from_slice");
                let is_map_method = crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                    && i == 0
                    && (self.infer_expression_type(object).as_ref().is_some_and(
                        crate::codegen::rust::stdlib_method_traits::is_map_type,
                    ) || self
                        .infer_type_name(object)
                        .as_ref()
                        .is_some_and(|n| crate::codegen::rust::stdlib_method_traits::is_map_type_name(n)));
                let is_set_method = crate::codegen::rust::stdlib_method_traits::is_set_lookup_method(method)
                    && i == 0
                    && (self.infer_expression_type(object).as_ref().is_some_and(
                        crate::codegen::rust::stdlib_method_traits::is_set_type,
                    ) || self
                        .infer_type_name(object)
                        .as_ref()
                        .is_some_and(|n| crate::codegen::rust::stdlib_method_traits::is_set_type_name(n)));
                if (is_auto_borrow || is_map_method || is_set_method) && i == 0 {
                    let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                    let arg_is_windjammer_str = match arg_to_generate {
                        Expression::Identifier { name, .. } => {
                            self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                        || matches!(
                                            &p.type_,
                                            Type::Reference(inner)
                                                if crate::codegen::rust::types::is_windjammer_text_type(inner)
                                        ))
                            }) || self.inferred_borrowed_params.contains(name)
                        }
                        _ => false,
                    };
                    let arg_already_ref = match arg_to_generate {
                        Expression::Identifier { name, .. } => self.identifier_already_ref(name),
                        _ => {
                            let arg_ty = self.infer_expression_type(arg);
                            arg_ty.as_ref().is_some_and(|t| {
                                matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    || matches!(t, Type::Custom(n) if n == "&str")
                            }) || match arg_to_generate {
                                Expression::Identifier { name, .. } =>
                                    self.borrowed_iterator_vars.contains(name),
                                _ => false,
                            }
                        }
                    };
                    if !is_string_literal
                        && !arg_is_windjammer_str
                        && !arg_str.starts_with('&')
                        && !arg_already_ref
                    {
                        let needs_borrow = matches!(arg,
                            Expression::Identifier { .. } |
                            Expression::FieldAccess { .. } |
                            Expression::MethodCall { .. } |
                            Expression::Tuple { .. } |
                            Expression::Binary { .. } |
                            Expression::Unary { .. } |
                            Expression::Cast { .. }
                        );
                        if needs_borrow {
                            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                &mut arg_str,
                            );
                        }
                    }
                }

                // AUTO-CAST int → float
                {
                    let effective_sig = method_signature.as_ref();
                    if let Some(sig) = effective_sig {
                        let sig_param_idx = sig.arg_param_index(i);
                        let type_name = self.infer_type_name(object);
                        let qualified_key = type_name
                            .as_ref()
                            .map(|tn| format!("{}::{}", tn, method));
                        let skip_cast = self.should_skip_int_to_float_auto_cast_with_global(
                            type_name.as_deref(),
                            method,
                            qualified_key.as_deref(),
                        );
                        let _receiver_is_float = receiver_float_type.as_ref().is_some_and(|t| {
                            matches!(t, Type::Float)
                                || matches!(t, Type::Custom(n) if n == "f32" || n == "f64")
                                || crate::codegen::rust::float_type_utilities::float_type_from_wj_ty(t)
                                    .is_some()
                        });
                        if !skip_cast {
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

                let effective_sig = type_name
                    .as_ref()
                    .and_then(|tn| {
                        self.lookup_method_signature_on_receiver_type(
                            tn,
                            method,
                            arguments.len(),
                        )
                    })
                    .or_else(|| method_signature.clone());

                crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
                    effective_sig.as_ref(),
                    i,
                    Some(method),
                    arg_to_generate,
                    &mut arg_str,
                    type_name.as_deref(),
                    Some(&self.enum_variant_types),
                    None,
                );

                crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                    call_site_sig
                        .as_ref()
                        .or(method_signature.as_ref())
                        .or(effective_sig.as_ref()),
                    i,
                    receiver_type_name,
                    arg_to_generate,
                    &mut arg_str,
                );

                if is_collection_key_arg {
                    let arg_already_rust_ref = matches!(
                        arg_to_generate,
                        Expression::Identifier { name, .. }
                            if self.identifier_already_ref(name)
                                || self.str_ref_optimized_params.contains(name.as_str())
                                || self.inferred_borrowed_params.contains(name)
                    );
                    crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
                        method,
                        i,
                        arg_to_generate,
                        &mut arg_str,
                        arg_already_rust_ref,
                        receiver_type_name,
                    );
                }

                arg_str
            })
            .collect();

        (args_vec, prev_float_target)
    }

    fn is_enum_variant_or_constructor(expr: &Expression) -> bool {
        match expr {
            Expression::StructLiteral { .. } => true,
            Expression::Identifier { name, .. } => Self::is_enum_variant_qualified_path(name),
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
