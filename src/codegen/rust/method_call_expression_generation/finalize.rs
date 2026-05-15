//! Final Rust emission for method calls.

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
        obj_str: String,
        args: Vec<String>,
        prev_float_target: Option<Type>,
    ) -> String {
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

        // Determine separator: :: for static calls, . for instance methods
        // - Type/Module (starts with uppercase): use ::
        // - Variable (starts with lowercase): use .
        let separator = match object {
            Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
            Expression::Identifier { name, .. } => {
                // Check for known module/crate names that should use ::
                // Note: Avoid common variable names like "path", "config" which are used as variables
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
                    // Stdlib modules (avoid common variable names)
                    "mime",
                    "http",
                    "fs",
                    "strings",
                    // NOTE: "json" removed - it's a common variable name!
                    // Use "serde_json" for the module instead
                    "regex",
                    "cli",
                    "log",
                    "crypto",
                    "io",
                    "env",
                    "time",
                    "sync",
                    "thread",
                    "collections",
                    "cmp",
                ];

                // Type or module (uppercase) vs variable (lowercase)
                if name.chars().next().is_some_and(|c| c.is_uppercase())
                    || name.contains('.')
                    || known_modules.contains(&name.as_str())
                {
                    "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                } else {
                    "." // x.abs(), value.method()
                }
            }
            Expression::FieldAccess { ref object, .. } => {
                // Check if this is a module path (e.g., std::fs) or a field access (e.g., self.count)
                // If the object is an identifier that looks like a module, use ::
                // Otherwise, use . for instance methods on fields
                match object {
                    Expression::Identifier { name, .. }
                        if (name.chars().next().is_some_and(|c| c.is_uppercase()) || name == "std") => {
                            "::" // Module::path::method() -> static method
                        }
                    _ => ".", // Default to instance method
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

        // TDD FIX (Bug #3): Extract format!() macros in method arguments too
        let has_format_arg = processed_args
            .iter()
            .any(|arg_str| arg_str.contains("format!("));

        let base_expr = if has_format_arg {
            // Extract format!() macros to temp variables
            let mut temp_decls = String::new();
            let mut temp_counter = 0;
            let fixed_args: Vec<String> = processed_args
                .iter()
                .map(|arg_str| {
                    if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                        // Strip leading & if present (was added by argument processing)
                        let format_expr = if arg_str.starts_with("&") {
                            arg_str.strip_prefix("&").unwrap()
                        } else {
                            arg_str
                        };
                        // Extract to temp var
                        let temp_name = format!("_temp{}", temp_counter);
                        temp_counter += 1;
                        temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));

                        // When the method expects &str (push_str, extend_from_slice),
                        // add & to pass borrowed temp. Otherwise, pass owned value.
                        let method_needs_borrow =
                            matches!(method, "push_str" | "extend_from_slice");
                        if arg_str.starts_with("&") || method_needs_borrow {
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
}
