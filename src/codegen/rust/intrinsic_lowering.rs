//! Rust lowering of language intrinsics (ADR-0014 Phase 1) that need more than the
//! generic macro conversion: messages for `panic`/`todo`/`unreachable`/`assert_eq`,
//! `assert` with interpolated messages, `debug(x)`, and brace-safe literal printing.
//!
//! Name knowledge comes from `crate::intrinsics::Intrinsic`; both the plain-call and
//! the deprecated `name!(..)` spellings arrive here as the same `Call` node.

use crate::intrinsics::Intrinsic;
use crate::parser::{Expression, Literal, Type};

use super::CodeGenerator;

/// Standard-library functions a `use` brings into scope by name: `use std::log::debug`,
/// `use std::log::{debug, info}` and `use std::log::*` (every intrinsic-named function the module exports).
pub(super) fn std_fn_names_imported_by(path: &[String], out: &mut std::collections::HashSet<String>) {
    let exports = |module: &str, name: &str| super::runtime_signatures::runtime_signature(&format!("{module}::{name}")).is_some();
    let joined = path.join("::");
    let Some(rest) = joined.strip_prefix("std::") else { return };
    if let Some((module, names)) = super::program_generation::parse_braced_use_segment(rest) {
        out.extend(names.into_iter().filter(|n| exports(module, n)).map(str::to_string));
    } else if let Some(module) = rest.strip_suffix("::*") {
        out.extend(Intrinsic::ALL.iter().map(|i| i.call_name()).filter(|n| exports(module, n)).map(str::to_string));
    } else if let Some((module, name)) = rest.rsplit_once("::") {
        if exports(module, name) {
            out.insert(name.to_string());
        }
    }
}

fn escape_format_braces(rust_literal: &str) -> String {
    rust_literal.replace('{', "{{").replace('}', "}}")
}

impl<'ast> CodeGenerator<'ast> {
    /// Lowering for registry intrinsics. `None` falls through to the generic path.
    pub(in crate::codegen::rust) fn try_generate_intrinsic_call(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, &Expression<'ast>)],
    ) -> Option<String> {
        let intr = Intrinsic::from_call_name(func_name)?;
        let user_defined = self
            .signature_registry
            .get_signature(func_name)
            .map(|sig| !sig.is_extern)
            .unwrap_or(false);
        if user_defined || self.idiom.imported_std_fn_names.contains(func_name) {
            return None;
        }

        match intr {
            Intrinsic::Print | Intrinsic::Println | Intrinsic::Eprint | Intrinsic::Eprintln => {
                self.lower_literal_print(intr, arguments)
            }
            Intrinsic::Panic | Intrinsic::Todo | Intrinsic::Unreachable => {
                let macro_name = intr.call_name();
                if arguments.is_empty() {
                    return Some(format!("{macro_name}!()"));
                }
                if arguments.len() > 1 && matches!(arguments[0].1, Expression::Literal { value: Literal::String(_), .. }) {
                    // `panic("bad {}: {}", a, b)`: a template and its arguments, exactly as `println` takes them
                    let mut parts = vec![self.format_macro_template_arg(arguments[0].1)];
                    parts.extend(arguments[1..].iter().map(|(_, a)| self.generate_expression(a)));
                    return Some(format!("{macro_name}!({})", parts.join(", ")));
                }
                let msg = self.intrinsic_message_parts(arguments[0].1);
                Some(format!("{macro_name}!({msg})"))
            }
            Intrinsic::AssertEq | Intrinsic::AssertNe if arguments.len() >= 3 => {
                let a = self.generate_expression(arguments[0].1);
                let b = self.generate_expression(arguments[1].1);
                let msg = self.intrinsic_message_parts(arguments[2].1);
                Some(format!("{}!({a}, {b}, {msg})", intr.call_name()))
            }
            Intrinsic::Assert
                if arguments.len() >= 2 && Self::is_format_node(arguments[1].1) =>
            {
                let cond = self.generate_expression(arguments[0].1);
                let msg = self.intrinsic_message_parts(arguments[1].1);
                Some(format!("assert!({cond}, {msg})"))
            }
            Intrinsic::Debug if arguments.len() == 1 => Some(self.lower_debug(arguments[0].1)),
            _ => None,
        }
    }

    fn is_format_node(expr: &Expression<'ast>) -> bool {
        matches!(expr, Expression::MacroInvocation { name, .. } if name == "format")
    }

    /// `println("a { b")` (a literal with braces and no arguments) must print the braces
    /// verbatim: Rust's `println!` would otherwise parse them as placeholders.
    fn lower_literal_print(
        &mut self,
        intr: Intrinsic,
        arguments: &[(Option<String>, &Expression<'ast>)],
    ) -> Option<String> {
        if arguments.len() != 1 || self.should_use_runtime_io() {
            return None;
        }
        let Expression::Literal {
            value: value @ Literal::String(s),
            ..
        } = arguments[0].1
        else {
            return None;
        };
        if !s.contains('{') && !s.contains('}') {
            return None;
        }
        let lit = escape_format_braces(&self.generate_literal(value));
        Some(format!("{}!({lit})", intr.call_name()))
    }

    /// `template, args..` pieces for a Rust formatting macro from a message expression.
    fn intrinsic_message_parts(&mut self, msg: &Expression<'ast>) -> String {
        match msg {
            Expression::MacroInvocation { name, args, .. } if name == "format" && !args.is_empty() => {
                let mut parts = vec![self.format_macro_template_arg(args[0])];
                for a in args.iter().skip(1) {
                    parts.push(self.generate_expression(a));
                }
                parts.join(", ")
            }
            Expression::Literal {
                value: value @ Literal::String(_),
                ..
            } => escape_format_braces(&self.generate_literal(value)),
            other => format!("\"{{}}\", {}", self.generate_expression(other)),
        }
    }

    /// `debug(x)`: print `[file:line:col] x = value` to stderr and return `x`.
    /// Place expressions are cloned unless known `Copy`, so the user's value is never moved.
    fn lower_debug(&mut self, arg: &Expression<'ast>) -> String {
        let inner = self.generate_expression(arg);
        let is_place = matches!(
            arg,
            Expression::Identifier { .. }
                | Expression::FieldAccess { .. }
                | Expression::Index { .. }
        );
        if is_place {
            let known_copy = self
                .infer_expression_type(arg)
                // `[1, 2, 3]` infers as a fixed array but lowers to a growable `Vec`.
                .is_some_and(|t| !matches!(t, Type::Array(..)) && self.is_type_copy(&t));
            if !known_copy {
                let shown = inner.replace('\\', "\\\\").replace('"', "\\\"");
                return format!(
                    "{{ let __wj_dbg = {inner}.clone(); eprintln!(\"[{{}}:{{}}:{{}}] {{}} = {{:#?}}\", file!(), line!(), column!(), \"{shown}\", &__wj_dbg); __wj_dbg }}"
                );
            }
        }
        format!("dbg!({inner})")
    }
}
