//! Macro conversion helpers for expression generation
//!
//! Handles conversion of Windjammer function calls to Rust macros:
//! - Test macros (assert, assert_eq, panic, etc.)
//! - Test runtime functions (assert_gt, assert_contains, etc.)
//! - Print macros (print, println, eprintln, eprint)
//!
//! Philosophy: Windjammer code uses function-call syntax for ergonomics,
//! but the compiler converts them to Rust macros when appropriate.

use crate::parser::{BinaryOp, Expression, Literal};

use super::{string_analysis, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn try_generate_test_macro(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, &Expression<'ast>)],
    ) -> Option<String> {
        // Test macros that Windjammer converts to Rust macros
        let test_macros = [
            "assert",
            "assert_eq",
            "assert_ne",
            "assert_ok",
            "assert_err",
            "panic",
            "vec",
            "format",
            "write",
            "writeln",
            "dbg",
            "todo",
            "unimplemented",
            "unreachable",
        ];

        if !test_macros.contains(&func_name) {
            return None;
        }

        // Rust 2021: panic!(format!("...", args)) is invalid because
        // panic! requires a string literal as first arg.
        // Unwrap: panic(format!("...", a, b)) → panic!("...", a, b)
        if func_name == "panic" && arguments.len() == 1 {
            if let Expression::MacroInvocation {
                name: ref inner_name,
                args: ref inner_args,
                ..
            } = arguments[0].1
            {
                if inner_name == "format" {
                    let inner: Vec<String> = inner_args
                        .iter()
                        .map(|a| self.generate_expression(a))
                        .collect();
                    return Some(format!("panic!({})", inner.join(", ")));
                }
            }
        }

        let args: Vec<String> = arguments
            .iter()
            .map(|(_label, arg)| {
                let generated = self.generate_expression(arg);
                // Deref borrowed Copy params in assert_eq!/assert_ne!
                // to avoid E0277 (&i32 != i32 for PartialEq)
                if matches!(func_name, "assert_eq" | "assert_ne") {
                    if let Expression::Identifier { name, .. } = arg {
                        if self.inferred_borrowed_params.contains(name.as_str()) {
                            let param_type = self
                                .current_function_params
                                .iter()
                                .find(|p| p.name == *name)
                                .map(|p| &p.type_);
                            let is_copy_ref = param_type.is_some_and(|t| {
                                matches!(t, crate::parser::Type::Reference(inner)
                                    if self.is_type_copy(inner))
                            });
                            if is_copy_ref {
                                return format!("*{}", generated);
                            }
                        }
                    }
                }
                generated
            })
            .collect();
        Some(format!("{}!({})", func_name, args.join(", ")))
    }

    /// Try to qualify test assertion runtime functions
    /// Returns Some(code) if this is a test runtime function, None otherwise
    pub(in crate::codegen::rust) fn try_qualify_test_function(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, &Expression<'ast>)],
    ) -> Option<String> {
        // Test runtime functions that need windjammer_runtime::test:: qualification
        let test_functions = [
            "assert_gt",
            "assert_lt",
            "assert_gte",
            "assert_lte",
            "assert_approx",
            "assert_not_empty",
            "assert_empty",
            "assert_contains",
            "assert_is_some",
            "assert_is_none",
        ];

        if !test_functions.contains(&func_name) {
            return None;
        }

        let args: Vec<String> = arguments
            .iter()
            .enumerate()
            .map(|(idx, (_label, arg))| {
                let generated = self.generate_expression(arg);
                // assert_is_some and assert_is_none expect &Option, so add & for first arg
                if (func_name == "assert_is_some" || func_name == "assert_is_none") && idx == 0 {
                    format!("&{}", generated)
                } else {
                    generated
                }
            })
            .collect();
        Some(format!(
            "windjammer_runtime::test::{}({})",
            func_name,
            args.join(", ")
        ))
    }

    /// Try to convert print/println/eprintln/eprint to macros
    /// Returns Some(code) if this is a print function, None otherwise
    pub(in crate::codegen::rust) fn try_generate_print_macro(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, &Expression<'ast>)],
    ) -> Option<String> {
        // Print functions that convert to macros
        if !matches!(func_name, "print" | "println" | "eprintln" | "eprint") {
            return None;
        }

        // For print() -> println!(), otherwise keep the same name
        let target_macro = if func_name == "print" {
            "println"
        } else {
            func_name
        };

        // Check if the first argument is a format! macro (from string interpolation)
        if let Some((_, first_arg)) = arguments.first() {
            // Check for MacroInvocation (explicit format! calls)
            if let Expression::MacroInvocation {
                is_repeat: _,
                ref name,
                args: ref macro_args,
                ..
            } = **first_arg
            {
                if name == "format" && !macro_args.is_empty() {
                    // Unwrap the format! call and put its arguments directly into println!
                    // format!("text {}", var) -> println!("text {}", var)
                    let format_str = self.generate_expression(macro_args[0]);
                    let format_args: Vec<String> = macro_args[1..]
                        .iter()
                        .map(|arg| self.generate_expression(arg))
                        .collect();

                    let args_str = if format_args.is_empty() {
                        String::new()
                    } else {
                        format!(", {}", format_args.join(", "))
                    };

                    return Some(format!("{}!({}{})", target_macro, format_str, args_str));
                }
            }

            // Check for Binary expression with string concatenation (will become format!)
            if let Expression::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } = **first_arg
            {
                // Check if this is string concatenation
                let has_string_literal = matches!(
                    left,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                ) || matches!(
                    right,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                ) || string_analysis::contains_string_literal(left)
                    || string_analysis::contains_string_literal(right);

                if has_string_literal {
                    // Collect all parts of the concatenation
                    let mut parts = Vec::new();
                    string_analysis::collect_concat_parts_static(left, &mut parts);
                    string_analysis::collect_concat_parts_static(right, &mut parts);

                    // Generate format string and arguments
                    let format_str = "{}".repeat(parts.len());
                    let format_args: Vec<String> = parts
                        .iter()
                        .map(|expr| self.generate_expression(expr))
                        .collect();

                    return Some(format!(
                        "{}!(\"{}\", {})",
                        target_macro,
                        format_str,
                        format_args.join(", ")
                    ));
                }
            }
        }

        // No interpolation, just regular print
        // TDD FIX: Auto-format non-string arguments
        // println(value) where value: bool → println!("{}", value)
        // println("text") → println!("text") (string literals stay as-is)
        let args: Vec<String> = arguments
            .iter()
            .map(|(_label, arg)| self.generate_expression(arg))
            .collect();

        // Check if first argument is a string literal
        let first_arg_is_string_literal = arguments
            .first()
            .map(|(_, arg)| {
                matches!(
                    arg,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                )
            })
            .unwrap_or(false);

        if args.len() == 1 && !first_arg_is_string_literal {
            // Single non-string argument - format it
            Some(format!("{}!(\"{{}}\", {})", target_macro, args[0]))
        } else {
            // Multiple args or string literal - keep as-is
            Some(format!("{}!({})", target_macro, args.join(", ")))
        }
    }
}
