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
                let is_map_key_arg = crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                    && i == 0;

                // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                // Fix: Suppress clone when param expects Borrowed -> just add & to field
                let param_expects_borrowed = method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership_for_arg(i))
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

                for p in &closure_borrowed_params {
                    self.borrowed_iterator_vars.remove(p);
                }

                // Owned params need `.clone()` when the arg is a non-Copy binding
                if !is_map_key_arg
                    && !self.in_user_written_closure
                    && !matches!(arg_to_generate, Expression::Closure { .. })
                    && method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership_for_arg(i))
                    .is_some_and(|&o| matches!(o, OwnershipMode::Owned))
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
                            arg_str = format!("{}.to_string()", arg_str);
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
                let param_ownership = method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership_for_arg(i));
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

                    let param_type = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_type_for_arg(i));
                    let is_explicit_str_ref = if let Some(Type::Reference(inner)) = param_type {
                        matches!(**inner, Type::String) ||
                        matches!(**inner, Type::Custom(ref s) if s == "str")
                    } else {
                        false
                    };

                    if asref_str_module {
                        false
                    } else if is_explicit_str_ref {
                        matches!(param_ownership, Some(OwnershipMode::Owned))
                    } else if is_map_key_arg {
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
                                let wants_owned = method_signature
                                    .as_ref()
                                    .and_then(|sig| sig.param_type_for_arg(i))
                                    .is_some_and(|t| {
                                        crate::codegen::rust::string_utilities::param_is_owned_string_type(t)
                                    });
                                if wants_owned
                                    || crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(i, method, method_signature)
                                {
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
                        .is_some_and(crate::codegen::rust::string_utilities::param_is_owned_string_type);
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
                        if !is_map_key
                            && !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                                method_signature,
                                param_idx_for_sig,
                            )
                        {
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
                        if !param_is_mut_borrowed
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

                // TDD FIX: Strip unnecessary .clone() when method param is Borrowed
                // When a field like `ingredient.item_id` is auto-cloned by the
                // FieldAccess handler (because owner is borrowed), but the method
                // expects &String (Borrowed), the clone is wasteful:
                //   &ingredient.item_id.clone()  ← clones then borrows (wasteful)
                //   &ingredient.item_id          ← borrows directly (correct)
                // Strip the .clone() so should_add_ref can add & cleanly.
                if let Some(ref sig) = method_signature {
                    let sig_param_idx = sig.arg_param_index(i);
                    let param_is_borrowed = sig
                        .param_ownership
                        .get(sig_param_idx)
                        .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                    if param_is_borrowed {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                    }
                }

                // `if let Some(x) = &self.opt` — pass owned values via `.clone()`, not `&x` / `&mut x`.
                if let Expression::Identifier { name, .. } = arg_to_generate {
                    let inferred = self.infer_expression_type(arg_to_generate);
                    let is_ref_binding = inferred
                        .as_ref()
                        .is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        })
                        || self.match_arm_bindings.contains(name.as_str());
                    // Skip Copy types — they don't need cloning (e.g. i32 from enum destructure)
                    let is_copy = inferred
                        .as_ref()
                        .is_some_and(|t| {
                            crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
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

                // AUTO-MUT-BORROW: Add &mut when parameter expects MutBorrowed
                if let Some(ref sig) = method_signature {
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
                if !string_literal_converted && !to_string_stripped_for_str_param {
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
                            if let Expression::Cast { .. } = arg_to_generate {
                                arg_str = format!("&({})", arg_str);
                            } else {
                                arg_str = format!("&{}", arg_str);
                            }
                        }
                    }
                }

                let sig_param_idx = method_signature
                    .as_ref()
                    .map(|sig| if sig.has_self_receiver { i + 1 } else { i })
                    .unwrap_or(i);
                arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                    method_signature,
                    sig_param_idx,
                    arg_to_generate,
                    arg_str,
                    string_literal_converted || to_string_stripped_for_str_param,
                );

                // Borrow owned `string` locals for `&str` formals when qualified lookup failed.
                if i == 0
                    && !is_map_key_arg
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
                    let sig_param_idx = registry_sig
                        .as_ref()
                        .map(|sig| if sig.has_self_receiver { i + 1 } else { i })
                        .unwrap_or(i);
                    let wants_str = registry_sig.is_some_and(|sig| {
                        sig.param_types
                            .get(sig_param_idx)
                            .is_some_and(|pt| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(pt)
                                    || (crate::codegen::rust::types::is_windjammer_text_type(pt)
                                        && sig.param_ownership.get(sig_param_idx)
                                            .is_some_and(|&o| {
                                                matches!(
                                                    o,
                                                    crate::analyzer::OwnershipMode::Borrowed
                                                )
                                            }))
                            })
                    });
                    if wants_str {
                        if self
                            .infer_expression_type(arg_to_generate)
                            .as_ref()
                            .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                        {
                            arg_str = format!("&{arg_str}");
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
                if (is_auto_borrow || is_map_method) && i == 0 {
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
                            arg_str = format!("&{}", arg_str);
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
