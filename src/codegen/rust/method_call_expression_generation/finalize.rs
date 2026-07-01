//! Final Rust emission for method calls.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub(in crate::codegen::rust) fn mc_finalize_method_call_expression(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        type_args: &Option<Vec<Type>>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        obj_str: String,
        args: Vec<String>,
        prev_float_target: Option<Type>,
    ) -> String {
        let resolved_signature =
            self.mc_select_call_site_signature(object, method, arguments, method_signature);
        let receiver_type_name = self
            .mc_infer_method_receiver_type_name(object)
            .or_else(|| self.infer_type_name(object));
        let args = if let Some(ref sig) = resolved_signature {
            let receiver_is_map = receiver_type_name
                .as_ref()
                .is_some_and(|n| crate::codegen::rust::stdlib_method_traits::is_map_type_name(n))
                || self
                    .infer_expression_type(object)
                    .as_ref()
                    .is_some_and(crate::codegen::rust::stdlib_method_traits::is_map_type);
            let receiver_is_set = receiver_type_name
                .as_ref()
                .is_some_and(|n| crate::codegen::rust::stdlib_method_traits::is_set_type_name(n))
                || self
                    .infer_expression_type(object)
                    .as_ref()
                    .is_some_and(crate::codegen::rust::stdlib_method_traits::is_set_type);
            args.into_iter()
                .enumerate()
                .map(|(i, mut arg_str)| {
                    if arg_str.contains("string_to_ffi(") {
                        return arg_str;
                    }
                    let sig_param_idx = sig.arg_param_index(i);
                    let ownership =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig, i, receiver_type_name.as_deref(),
                        );
                    let param_is_copy = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                        self.is_type_copy(t)
                    });
                    let is_collection_key = i == 0
                        && ((crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                            && receiver_is_map)
                            || (crate::codegen::rust::stdlib_method_traits::is_set_lookup_method(
                                method,
                            ) && receiver_is_set));
                    let apply_borrow = |arg_str: &mut String| {
                        if matches!(
                            sig.param_ownership.get(sig_param_idx),
                            Some(OwnershipMode::Owned)
                        ) {
                            return;
                        }
                        if sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                && crate::codegen::rust::types::is_windjammer_text_type(t)
                        }) {
                            return;
                        }
                        let param_is_str_ref = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        });
                        let arg_is_string_literal = matches!(
                            arguments.get(i).map(|(_, e)| e),
                            Some(Expression::Literal {
                                value: Literal::String(_),
                                ..
                            })
                        );
                        // After .to_string() stripping, a MethodCall like "lit".to_string()
                        // becomes bare "lit" in arg_str. Check if arg_str is now a string
                        // literal — it's already &str, adding & would create &&str.
                        let arg_str_is_bare_literal = arg_str.starts_with('"')
                            || arg_str.starts_with("r\"")
                            || arg_str.starts_with("r#\"");
                        if param_is_str_ref || arg_is_string_literal || arg_str_is_bare_literal
                            || (param_is_copy && !is_collection_key) {
                            return;
                        }
                        if let Some((_, arg_expr)) = arguments.get(i) {
                            let inner = match arg_expr {
                                Expression::Unary { op: UnaryOp::Ref, operand, .. } => operand,
                                other => other,
                            };
                            if let Expression::Identifier { name, .. } = inner {
                                if self.identifier_already_ref(name)
                                    || self.str_ref_optimized_params.contains(name.as_str())
                                {
                                    return;
                                }
                            }
                        }
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(arg_str);
                        if !arg_str.starts_with('&') {
                            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                arg_str,
                            );
                        }
                    };
                    match ownership {
                        crate::analyzer::OwnershipMode::MutBorrowed
                            if !arg_str.starts_with("&mut ") =>
                        {
                            if let Some((_, arg_expr)) = arguments.get(i) {
                                if let Expression::Identifier { name, .. } = arg_expr {
                                    if self.identifier_already_mut_ref(name) {
                                        return arg_str;
                                    }
                                }
                            }
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                &mut arg_str,
                            );
                            if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                format!("&mut {}", arg_str.trim_start_matches('&'))
                            } else {
                                format!("&mut {arg_str}")
                            }
                        }
                        crate::analyzer::OwnershipMode::Borrowed
                            if !arg_str.starts_with('&')
                                && !matches!(
                                    sig.param_ownership.get(sig_param_idx),
                                    Some(OwnershipMode::Owned)
                                ) =>
                        {
                            if sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                matches!(t, Type::String)
                                    || matches!(t, Type::Custom(n) if n == "string" || n == "String")
                            }) {
                                arg_str
                            } else {
                                apply_borrow(&mut arg_str);
                                arg_str
                            }
                        }
                        _ if sig.param_types.get(sig_param_idx).is_some_and(|t| {
                            matches!(t, Type::Reference(_))
                        }) && !matches!(
                            ownership,
                            crate::analyzer::OwnershipMode::Owned,
                        ) && !arg_str.starts_with('&') =>
                        {
                            apply_borrow(&mut arg_str);
                            arg_str
                        }
                        crate::analyzer::OwnershipMode::Owned => {
                            let param_is_str_ref = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            });
                            if arg_str.starts_with('&') && !param_is_str_ref && !is_collection_key {
                                arg_str.trim_start_matches('&').to_string()
                            } else {
                                arg_str
                            }
                        }
                        _ => arg_str,
                    }
                })
                .collect()
        } else {
            args
        };

        // E0499 FIX: Extract temporaries when receiver and arguments both borrow self.
        // Pattern: self.field.method(self.other_method()) generates two &mut self borrows.
        // Fix: { let __wj_tmp0 = self.other_method(); self.field.method(__wj_tmp0) }
        let receiver_borrows_self = self.codegen_expression_traces_to_self(object);
        let mut self_borrow_temps: Vec<(String, String)> = Vec::new();
        let args = if receiver_borrows_self {
            let needs_extraction = arguments
                .iter()
                .any(|(_label, arg)| self.expression_borrows_self(arg));
            if needs_extraction {
                args.into_iter()
                    .enumerate()
                    .map(|(i, arg_str)| {
                        let (_label, arg_expr) = &arguments[i];
                        if self.expression_borrows_self(arg_expr) {
                            let temp_name = format!("__wj_tmp{}", i);
                            self_borrow_temps.push((temp_name.clone(), arg_str));
                            temp_name
                        } else {
                            arg_str
                        }
                    })
                    .collect()
            } else {
                args
            }
        } else {
            args
        };

        // Restore float target type after argument generation
        self.assignment_float_target_type = prev_float_target;

        // Generate turbofish if present, or infer for collect() from return type
        let turbofish = if let Some(types) = type_args {
            let type_strs: Vec<String> = types.iter().map(|t| self.type_to_rust(t)).collect();
            format!("::<{}>", type_strs.join(", "))
        } else if method == "collect" {
            if let Some(target_ty) = &self.collect_target_type {
                format!("::<{}>", self.type_to_rust(target_ty))
            } else if let Some(ret_ty) = &self.current_function_return_type {
                format!("::<{}>", self.type_to_rust(ret_ty))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Special case: empty method name means turbofish on a function call (func::<T>())
        if method.is_empty() {
            return format!("{}{}({})", obj_str, turbofish, args.join(", "));
        }

        // Special case: substring(start, end) -> &text[start..end]
        if method == "substring" && args.len() == 2 {
            return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
        }

        // Special case: contains() with String argument needs .as_str()
        // String::contains() expects &str, not String
        if method == "contains" && args.len() == 1 {
            // Check if argument is a method call that returns String (like to_lowercase())
            if let Some((_label, arg)) = arguments.first() {
                if matches!(arg, Expression::MethodCall { method: m, .. } if
                    m == "to_lowercase" || m == "to_uppercase" ||
                    m == "to_string" || m == "trim" || m == "clone")
                {
                    // The argument is String, needs .as_str()
                    return format!("{}.{}({}.as_str())", obj_str, method, args[0]);
                }
            }
        }

        // Determine separator: :: for static/module calls, . for instance methods
        // - Type/Module (starts with uppercase): use ::
        // - Variable (starts with lowercase): use .
        let separator = match object {
            Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
            Expression::Identifier { name, .. } => {
                // Enum variant paths parse as one identifier: `ShaderFile::HiZCull.to_path()`
                if Self::is_enum_variant_qualified_path(name) {
                    "."
                } else {
                    // Check for known module/crate names that should use ::
                    // Note: Avoid common variable names like "path", "config" which are used as variables
                    // Only unambiguous module/type names — never short names used as variables (io, log, fs, …).
                    let known_modules = [
                        "std",
                        "serde_json",
                        "serde",
                        "tokio",
                        "reqwest",
                        "sqlx",
                        "chrono",
                        "sha2",
                        "bcrypt",
                        "base64",
                        "rand",
                        "Vec",
                        "String",
                        "Option",
                        "Result",
                        "Box",
                        "Arc",
                        "Mutex",
                        "Utc",
                        "Local",
                        "DEFAULT_COST",
                    ];

                    // Type, `Self`, or module (uppercase) vs variable (lowercase)
                    if name == "Self"
                        || name.chars().next().is_some_and(|c| c.is_uppercase())
                        || name.contains('.')
                        || known_modules.contains(&name.as_str())
                        || self.is_imported_runtime_std_module(name)
                    {
                        "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                    } else {
                        "." // x.abs(), value.method()
                    }
                }
            }
            Expression::FieldAccess { ref object, .. } => {
                // Type::Variant.method() in Windjammer (enum variant receiver) must lower to
                // `(Type::Variant).method()` in Rust — not `Type::Variant::method()`.
                match object {
                    Expression::Identifier { name, .. }
                        if name.chars().next().is_some_and(|c| c.is_uppercase()) =>
                    {
                        "." // ShaderFile::HiZCull.to_path() → (ShaderFile::HiZCull).to_path()
                    }
                    Expression::Identifier { name, .. } if name == "std" => "::",
                    _ => ".", // self.field.method()
                }
            }
            _ => ".", // Instance method on expressions
        };

        // SPECIAL CASE: .slice() method is our desugared slice syntax [start..end]
        // Convert it back to proper Rust slice syntax
        // For strings, we need to add & to get &str (a reference)
        if method == "slice" && args.len() == 2 {
            return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
        }

        // E0308: Borrowed Windjammer `string` parameters lower to `&str`. `.clone()` on `&str`
        // is still `&str`, but users mean an owned copy → emit `.to_string()`.
        if method == "clone" && arguments.is_empty() {
            if let Expression::Identifier { name, .. } = object {
                if self.inferred_borrowed_params.contains(name.as_str())
                    && self
                        .current_function_params
                        .iter()
                        .find(|p| p.name == *name)
                        .is_some_and(|p| {
                            crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        })
                {
                    return format!("{}.to_string()", obj_str);
                }
            }
        }

        // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
        // DISABLED: This optimization was too aggressive and removed needed clones
        // TODO: Make this more conservative - only remove clone when we can prove
        // the value is Copy or when it's the last use
        // if method == "clone" && arguments.is_empty() {
        //     if let Expression::Identifier { name: ref var_name, location: None } = **object {
        //         if self.clone_optimizations.contains(var_name) {
        //             // Skip the .clone(), just return the variable (or borrow if needed)
        //             return obj_str;
        //         }
        //     }
        // }

        // UI FRAMEWORK: Check if we need to add .to_vnode() for .child() methods
        // DISABLED: Too aggressive - needs type checking to determine if parameter expects VNode
        // TODO: Re-enable with proper type checking when VNode type bindings are implemented
        let processed_args = args;

        // WINDJAMMER STDLIB → RUST TRANSLATION
        // Some Windjammer methods don't exist in Rust and need translation.
        //
        // reversed() → into_iter().rev().collect::<Vec<_>>()
        if method == "reversed" && processed_args.is_empty() {
            return format!("{}.into_iter().rev().collect::<Vec<_>>()", obj_str);
        }
        // enumerate() → iter().enumerate()
        // Rust Vec doesn't have .enumerate() — only iterators do.
        // But if the object already ends with .iter(), .iter_mut(), or
        // .into_iter(), don't add a redundant .iter() prefix.
        if method == "enumerate" && processed_args.is_empty() {
            let already_iterator = obj_str.ends_with(".iter()")
                || obj_str.ends_with(".iter_mut()")
                || obj_str.ends_with(".into_iter()");
            if already_iterator {
                return format!("{}.enumerate()", obj_str);
            } else {
                return format!("{}.iter().enumerate()", obj_str);
            }
        }

        // TDD FIX (Bug #3): Extract format!() / write!-block macros in method arguments too
        let needs_format_temp = |arg_str: &str| -> bool {
            arg_str.contains("format!(") || arg_str.contains("write!(&mut __s,")
        };
        let has_format_arg = processed_args
            .iter()
            .any(|arg_str| needs_format_temp(arg_str));

        let base_expr = if has_format_arg {
            // Extract format!() macros to temp variables
            let mut temp_decls = String::new();
            let mut temp_counter = 0i32;
            let fixed_args: Vec<String> = processed_args
                .iter()
                .enumerate()
                .map(|(arg_idx, arg_str)| {
                    let has_borrow_prefix = arg_str.starts_with('&');
                    let inner = if has_borrow_prefix {
                        &arg_str[1..]
                    } else {
                        arg_str.as_str()
                    };
                    let needs_extract = inner.starts_with("format!(")
                        || (inner.starts_with('{') && inner.contains("write!(&mut __s,"));
                    if needs_extract {
                        let temp_name = format!("_temp{}", temp_counter);
                        temp_counter += 1;
                        temp_decls.push_str(&format!("let {} = {}; ", temp_name, inner));

                        let param_wants_owned_string = method_signature
                            .as_ref()
                            .map(|sig| {
                                let idx = sig.arg_param_index(arg_idx);
                                matches!(
                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                                        sig, idx,
                                    ),
                                    OwnershipMode::Owned,
                                ) && sig.formal_param_type(idx).is_some_and(|t| {
                                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                                })
                            })
                            .unwrap_or(false)
                            || self.mc_method_param_expects_owned_string_from_global(
                                object,
                                method,
                                arg_idx,
                                arguments.len(),
                            );
                        let param_wants_str_ref = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_type_for_arg(arg_idx))
                            .is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            });
                        // When the method expects &str (push_str, extend_from_slice),
                        // add & to pass borrowed temp. Owned String params take the temp directly.
                        let method_needs_borrow =
                            matches!(method, "push_str" | "extend_from_slice");
                        if param_wants_owned_string {
                            temp_name
                        } else if has_borrow_prefix || method_needs_borrow || param_wants_str_ref
                        {
                            format!("&{}", temp_name)
                        } else {
                            temp_name
                        }
                    } else {
                        arg_str.clone()
                    }
                })
                .collect();

            // Wrap in block: { let _temp0 = format!(...); obj.method(&_temp0, ...) }
            format!(
                "{{ {}{}{}{}{}({}) }}",
                temp_decls,
                obj_str,
                separator,
                method,
                turbofish,
                fixed_args.join(", ")
            )
        } else {
            format!(
                "{}{}{}{}({})",
                obj_str,
                separator,
                method,
                turbofish,
                processed_args.join(", ")
            )
        };

        // E0499 FIX: Wrap in block with temporaries if self-borrow extraction was needed
        let base_expr = if !self_borrow_temps.is_empty() {
            let mut temp_decls = String::new();
            for (name, value) in &self_borrow_temps {
                temp_decls.push_str(&format!("let {} = {}; ", name, value));
            }
            format!("{{ {}{} }}", temp_decls, base_expr)
        } else {
            base_expr
        };

        base_expr
    }

    /// `Type::Variant` in expressions is parsed as a single qualified identifier, not FieldAccess.
    pub(in crate::codegen::rust) fn is_enum_variant_qualified_path(name: &str) -> bool {
        let mut parts = name.split("::");
        let type_name = parts.next();
        let variant = parts.next();
        parts.next().is_none()
            && type_name.is_some_and(|t| t.chars().next().is_some_and(|c| c.is_uppercase()))
            && variant.is_some_and(|v| {
                !v.is_empty()
                    && !v.starts_with('<')
                    && v.chars().all(|c| c.is_alphanumeric() || c == '_')
            })
    }
}
