//! Macro and string expression generation
//!
//! Handles generation of:
//! - Macro invocations (println!, format!, vec!, etc.)
//! - String concatenation operations
//! - Format string optimization

use crate::parser::{Expression, Type};

use super::{string_analysis, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for macro invocation expression
    /// Handles format!, println!, vec!, and other macros with special semantics
    pub(in crate::codegen::rust) fn generate_macro_invocation(
        &mut self,
        is_repeat: bool,
        name: &str,
        args: &[&Expression<'ast>],
        delimiter: &crate::parser::MacroDelimiter,
    ) -> String {
        use crate::parser::{Literal, MacroDelimiter};

        // PHASE 4 OPTIMIZATION: Check for format! with capacity hints
        if name == "format" {
            if let Some(&capacity) = self.string_capacity_hints.get(&self.current_statement_idx) {
                // Clone capacity to avoid borrow issues
                let capacity_val = capacity;
                // Generate optimized String::with_capacity + write! instead of format!
                self.needs_write_import = true;
                // write! expects the first argument to be a &str format template, not String.
                let arg_strs: Vec<String> = if args.is_empty() {
                    Vec::new()
                } else {
                    let prev_suppress = self.suppress_string_conversion.get();
                    self.suppress_string_conversion.set(true);
                    let fmt = self.generate_expression(args[0]);
                    self.suppress_string_conversion.set(prev_suppress);
                    let rest: Vec<String> = args[1..]
                        .iter()
                        .map(|e| self.generate_expression(e))
                        .collect();
                    let mut v = Vec::with_capacity(1 + rest.len());
                    v.push(fmt);
                    v.extend(rest);
                    v
                };

                return format!(
                    "{{\n{}    let mut __s = String::with_capacity({});\n{}    write!(&mut __s, {}).unwrap();\n{}    __s\n{}}}",
                    self.indent(),
                    capacity_val,
                    self.indent(),
                    arg_strs.join(", "),
                    self.indent(),
                    self.indent()
                );
            }
        }

        // Special case: if this is println!/eprintln!/print!/eprint! and first arg is format!, flatten it
        let should_flatten = (name == "println"
            || name == "eprintln"
            || name == "print"
            || name == "eprint")
            && !args.is_empty()
            && matches!(&args[0], Expression::MacroInvocation { name: macro_name, .. } if macro_name == "format");

        // Macro arguments must never have context-level string coercion applied.
        // format!("...".to_string(), ...) is invalid Rust (requires literal first arg).
        let prev_coerce = self.coerce_string_literals_to_owned;
        self.coerce_string_literals_to_owned = false;
        let prev_match_arm = self.in_match_arm_needing_string;
        self.in_match_arm_needing_string = false;

        let arg_strs: Vec<String> = if should_flatten {
            // Flatten format! macro arguments into the print macro
            if let Expression::MacroInvocation {
                is_repeat: _,
                args: format_args,
                ..
            } = &args[0]
            {
                format_args
                    .iter()
                    .map(|e| self.generate_expression(e))
                    .collect()
            } else {
                args.iter().map(|e| self.generate_expression(e)).collect()
            }
        } else {
            // Special case: if this is println!/eprintln!/print!/eprint! with a single non-literal arg,
            // wrap it with "{}" to make it valid Rust: println!(var) -> println!("{}", var)
            // Also wrap format!() calls: println!(format!(...)) -> println!("{}", format!(...))
            if (name == "println" || name == "eprintln" || name == "print" || name == "eprint")
                && args.len() == 1
                && !matches!(
                    &args[0],
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                )
            {
                vec!["\"{}\"".to_string(), self.generate_expression(args[0])]
            } else {
                args.iter().map(|e| self.generate_expression(e)).collect()
            }
        };

        self.coerce_string_literals_to_owned = prev_coerce;
        self.in_match_arm_needing_string = prev_match_arm;

        let (open, close) = match delimiter {
            MacroDelimiter::Parens => ("(", ")"),
            MacroDelimiter::Brackets => ("[", "]"),
            MacroDelimiter::Braces => ("{", "}"),
        };

        // WINDJAMMER FIX: vec![value; count] repeat syntax
        // The parser sets is_repeat=true for vec![x; n] syntax
        // Use semicolon for repeat, comma for regular args
        let separator = if is_repeat { "; " } else { ", " };

        // WINDJAMMER FIX: String literal coercion in vec![]
        // In Windjammer, `string` maps to Rust `String`, so vec!["a", "b"] must
        // become vec!["a".to_string(), "b".to_string()] for Vec<String>.
        // Only apply when: macro is vec, brackets delimiter, has string literal args.
        let final_arg_strs: Vec<String> =
            if name == "vec" && matches!(delimiter, MacroDelimiter::Brackets) && !is_repeat {
                arg_strs
                    .iter()
                    .enumerate()
                    .map(|(idx, s)| {
                        // Check if the original arg is a string literal
                        if idx < args.len() {
                            if let Expression::Literal {
                                value: Literal::String(_),
                                ..
                            } = &args[idx]
                            {
                                // Add .to_string() if not already present
                                if !s.ends_with(".to_string()") {
                                    return format!("{}.to_string()", s);
                                }
                            }
                        }
                        s.clone()
                    })
                    .collect()
            } else {
                arg_strs
            };

        format!(
            "{}!{}{}{}",
            name,
            open,
            final_arg_strs.join(separator),
            close
        )
    }

    pub(in crate::codegen::rust) fn generate_string_concat(
        &mut self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
    ) -> String {
        let mut parts = Vec::new();
        string_analysis::collect_concat_parts_static(left, &mut parts);
        string_analysis::collect_concat_parts_static(right, &mut parts);

        let use_additive = parts
            .iter()
            .any(string_analysis::expression_produces_string);

        if use_additive {
            let mut acc = self.generate_expression(&parts[0]);
            for p in parts.iter().skip(1) {
                let rhs = self.generate_expression(p);
                let amp = string_analysis::expression_produces_string(p)
                    || self.infer_expression_type(p).as_ref().is_some_and(|ty| {
                        matches!(ty, Type::String)
                            || matches!(
                                ty,
                                Type::Custom(n) if n == "string" || n == "String"
                            )
                    });
                acc = if amp {
                    format!("{} + &{}", acc, rhs)
                } else {
                    format!("{} + {}", acc, rhs)
                };
            }
            return acc;
        }

        let format_str = "{}".repeat(parts.len());
        let mut args = Vec::new();
        for expr in &parts {
            args.push(self.generate_expression(expr));
        }
        format!("format!(\"{}\", {})", format_str, args.join(", "))
    }
}
