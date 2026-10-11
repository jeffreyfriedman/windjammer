// ADR-0014 Phase 1: lower the deprecated macro spellings of language intrinsics to the
// canonical plain-call node, so both spellings produce the same AST.
//
//   println!("a {} b", x)   ==>  println("a {} b", x)-equivalent: Call(println, [format("a {} b", x)])
//   println("a ${x} b")     ==>  Call(println, [format("a {} b", x)])      (already canonical)
//   stderr.println(..)      ==>  Call(eprintln, [..])
//   dbg!(x)                 ==>  Call(debug, [x])
//
// Name knowledge lives in `crate::intrinsics`; nothing here matches on ad-hoc strings
// beyond asking the registry.

use crate::intrinsics::template::{template_has_placeholder, unescape_braces};
use crate::intrinsics::{stream_method_intrinsic, Intrinsic};
use crate::parser::ast::*;
use crate::parser_impl::Parser;

type Args = Vec<(Option<String>, &'static Expression<'static>)>;

impl Parser {
    /// `format!()` is Rust syntax: point at string interpolation.
    pub(in crate::parser) fn warn_if_format_macro(&mut self, name: &str) {
        if name != "format" {
            return;
        }
        let (file, line) = match self.current_location() {
            Some(loc) => (Some(loc.file.display().to_string()), Some(loc.line)),
            None => (None, None),
        };
        self.emit_warning(
            "format!() is Rust syntax. Use string interpolation instead: \"text ${expr}\"".to_string(),
            file,
            line,
            None,
        );
    }

    fn alloc_intrinsic_call(
        &mut self,
        intr: Intrinsic,
        arguments: Args,
    ) -> &'static Expression<'static> {
        let function = self.alloc_expr(Expression::Identifier {
            name: intr.call_name().to_string(),
            location: self.current_location(),
        });
        self.alloc_expr(Expression::Call {
            function,
            arguments,
            location: self.current_location(),
        })
    }

    /// `stderr.println(..)` / `stdout.print(..)` -> canonical intrinsic call.
    pub(in crate::parser) fn lower_stream_method_call(
        &mut self,
        object: &'static Expression<'static>,
        method: &str,
        arguments: &Args,
    ) -> Option<&'static Expression<'static>> {
        let Expression::Identifier { name, .. } = object else {
            return None;
        };
        let intr = stream_method_intrinsic(name, method)?;
        Some(self.alloc_intrinsic_call(intr, arguments.clone()))
    }

    /// Lower `name!(args)` for registry intrinsics. `None` keeps the macro node.
    pub(in crate::parser) fn lower_intrinsic_macro(
        &mut self,
        name: &str,
        args: &[&'static Expression<'static>],
    ) -> Option<&'static Expression<'static>> {
        let intr = Intrinsic::from_macro_name(name)?;
        match intr {
            // `format!` is already the interpolation node; `vec!` stays the list node.
            Intrinsic::Format | Intrinsic::Vec => None,
            Intrinsic::Debug => {
                if args.len() != 1 {
                    return None;
                }
                Some(self.alloc_intrinsic_call(intr, vec![(None, args[0])]))
            }
            _ => {
                let msg_idx = intr.message_arg_index()?;
                if args.len() < msg_idx {
                    return None;
                }
                let mut call_args: Args = args[..msg_idx].iter().map(|a| (None, *a)).collect();
                let message = self.lower_message_args(&args[msg_idx..])?;
                call_args.extend(message.into_iter().map(|a| (None, a)));
                Some(self.alloc_intrinsic_call(intr, call_args))
            }
        }
    }

    /// Message arguments of a print/panic/assert macro -> zero or one canonical argument.
    fn lower_message_args(
        &mut self,
        rest: &[&'static Expression<'static>],
    ) -> Option<Vec<&'static Expression<'static>>> {
        let Some(first) = rest.first() else {
            return Some(vec![]);
        };
        let Expression::Literal {
            value: Literal::String(tmpl),
            ..
        } = first
        else {
            return None; // `println!(x)` etc.: keep the macro (rustc-style error later)
        };
        if rest.len() == 1 && !template_has_placeholder(tmpl) {
            let plain = unescape_braces(tmpl);
            return Some(vec![self.alloc_expr(Expression::Literal {
                value: Literal::String(plain),
                location: self.current_location(),
            })]);
        }
        let node = self.alloc_expr(Expression::MacroInvocation {
            name: "format".to_string(),
            args: rest.to_vec(),
            delimiter: MacroDelimiter::Parens,
            is_repeat: false,
            location: self.current_location(),
        });
        Some(vec![node])
    }
}

impl Parser {
    /// ADR-0014: deprecation warnings (W0007/W0008) with spans and suggested replacements
    /// for `name!(..)` intrinsic spellings, `vec![..]` and `{}`-style format strings.
    pub(crate) fn emit_surface_syntax_warnings(&mut self) {
        if self.source.is_empty() {
            return;
        }
        let file = self.filename.clone();
        for f in crate::intrinsics::surface_fix::lint_source(&self.source) {
            // `format!` keeps its dedicated parse-time warning.
            if f.original.starts_with("format!") {
                continue;
            }
            let mut message = format!("[{}] {}", f.code, f.message);
            if let Some(rep) = &f.replacement {
                let rep: String = rep.lines().next().unwrap_or("").to_string();
                message.push_str(&format!("; suggested replacement: `{rep}`"));
            }
            self.emit_warning(message, Some(file.clone()), Some(f.line), Some(f.col));
        }
    }
}
