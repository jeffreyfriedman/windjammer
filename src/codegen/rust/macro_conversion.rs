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
                    let mut inner: Vec<String> = Vec::with_capacity(inner_args.len());
                    if !inner_args.is_empty() {
                        inner.push(self.format_macro_template_arg(inner_args[0]));
                        for a in inner_args.iter().skip(1) {
                            inner.push(self.generate_expression(a));
                        }
                    }
                    return Some(format!("panic!({})", inner.join(", ")));
                }
            }
        }

        let args: Vec<String> = if func_name == "format" && !arguments.is_empty() {
            let mut out = Vec::with_capacity(arguments.len());
            out.push(self.format_macro_template_arg(arguments[0].1));
            for (_label, arg) in arguments.iter().skip(1) {
                out.push(self.generate_expression(arg));
            }
            out
        } else {
            arguments
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
                .collect()
        };
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

    /// Use windjammer_runtime::io for print builtins when the compilation unit links
    /// the runtime (stdlib imports). Standalone `--no-cargo` snippets keep `println!`.
    pub(in crate::codegen::rust) fn should_use_runtime_io(&self) -> bool {
        !self.runtime_std_module_imports.is_empty()
    }

    /// Map Windjammer print builtins to windjammer_runtime::io (no Rust `println!` leakage).
    fn runtime_io_print_fn(func_name: &str) -> &'static str {
        match func_name {
            "print" => "windjammer_runtime::io::print",
            "println" => "windjammer_runtime::io::println",
            "eprint" => "windjammer_runtime::io::eprint",
            "eprintln" => "windjammer_runtime::io::eprintln",
            _ => "windjammer_runtime::io::println",
        }
    }

    fn emit_runtime_print_call(io_fn: &str, format_literal: &str, format_args: &[String]) -> String {
        if format_args.is_empty() {
            format!("{io_fn}({format_literal})")
        } else {
            format!(
                "{io_fn}(&format!({}{}))",
                format_literal,
                format!(", {}", format_args.join(", "))
            )
        }
    }

    /// Convert print/println/eprintln/eprint to windjammer_runtime::io calls.
    /// Returns Some(code) if this is a print function, None otherwise
    pub(in crate::codegen::rust) fn try_generate_print_macro(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, &Expression<'ast>)],
    ) -> Option<String> {
        if !matches!(func_name, "print" | "println" | "eprintln" | "eprint") {
            return None;
        }

        if !self.should_use_runtime_io() {
            return self.try_generate_print_rust_macro(func_name, arguments);
        }

        let io_fn = Self::runtime_io_print_fn(func_name);

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
                    let format_str = self.format_macro_template_arg(macro_args[0]);
                    let format_args: Vec<String> = macro_args[1..]
                        .iter()
                        .map(|arg| self.generate_expression(arg))
                        .collect();
                    return Some(Self::emit_runtime_print_call(
                        io_fn,
                        &format_str,
                        &format_args,
                    ));
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
                    let mut parts = Vec::new();
                    string_analysis::collect_concat_parts_static(left, &mut parts);
                    string_analysis::collect_concat_parts_static(right, &mut parts);

                    let format_str = format!("\"{}\"", "{}".repeat(parts.len()));
                    let format_args: Vec<String> = parts
                        .iter()
                        .map(|expr| self.generate_expression(expr))
                        .collect();

                    return Some(Self::emit_runtime_print_call(
                        io_fn,
                        &format_str,
                        &format_args,
                    ));
                }
            }
        }

        let args: Vec<String> = arguments
            .iter()
            .map(|(_label, arg)| self.generate_expression(arg))
            .collect();

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

        if args.is_empty() {
            return Some(format!("{io_fn}(\"\")"));
        }

        if args.len() == 1 && first_arg_is_string_literal {
            // Single string literal — pass directly as &str
            Some(format!("{io_fn}({})", args[0]))
        } else if args.len() == 1 && !first_arg_is_string_literal {
            // Single non-string value — format it
            Some(Self::emit_runtime_print_call(io_fn, "\"{}\"", &args))
        } else if first_arg_is_string_literal {
            // println("fmt {}", a, b) — first arg is format template
            Some(Self::emit_runtime_print_call(
                io_fn,
                &args[0],
                &args[1..],
            ))
        } else {
            // Multiple non-literal args — join with spaces
            let format_str = format!("\"{}\"", " ".repeat(args.len()));
            Some(Self::emit_runtime_print_call(io_fn, &format_str, &args))
        }
    }

    /// Legacy Rust macro lowering for standalone builds without windjammer_runtime linked.
    fn try_generate_print_rust_macro(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, &Expression<'ast>)],
    ) -> Option<String> {
        let target_macro = if func_name == "print" {
            "println"
        } else {
            func_name
        };

        if let Some((_, first_arg)) = arguments.first() {
            if let Expression::MacroInvocation {
                is_repeat: _,
                ref name,
                args: ref macro_args,
                ..
            } = **first_arg
            {
                if name == "format" && !macro_args.is_empty() {
                    let format_str = self.format_macro_template_arg(macro_args[0]);
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

            if let Expression::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } = **first_arg
            {
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
                    let mut parts = Vec::new();
                    string_analysis::collect_concat_parts_static(left, &mut parts);
                    string_analysis::collect_concat_parts_static(right, &mut parts);
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

        let args: Vec<String> = arguments
            .iter()
            .map(|(_label, arg)| self.generate_expression(arg))
            .collect();

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
            Some(format!("{}!(\"{{}}\", {})", target_macro, args[0]))
        } else {
            Some(format!("{}!({})", target_macro, args.join(", ")))
        }
    }
}
