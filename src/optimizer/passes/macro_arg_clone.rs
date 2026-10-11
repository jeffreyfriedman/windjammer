//! macro_arg_clone (PASSES.md P8): `format!("{}", x.clone())` -> `format!("{}", x)`.
//!
//! Applies to the format family only (`format`, `print`, `println`, `eprint`,
//! `eprintln`, `write`, `writeln`, `panic`), whose arguments are taken by
//! reference. Only an argument that is itself `E.clone()` (no arguments) is touched.

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::mutants::Mutant;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{Expr, Token};

const FORMAT_FAMILY: &[&str] = &["format", "print", "println", "eprint", "eprintln", "write", "writeln", "panic"];

struct V<'c, 'a> {
    ctx: &'c Ctx<'a>,
    out: &'c mut Vec<Edit>,
}

impl V<'_, '_> {
    fn applies(&self, name: &str) -> bool {
        FORMAT_FAMILY.contains(&name)
            || (self.ctx.config.mutant == Some(Mutant::MacroCloneInVec) && name == "vec")
    }
}

impl<'ast> Visit<'ast> for V<'_, '_> {
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let name = m.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
        if self.applies(&name) {
            if let Ok(args) = m.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
                let skip = usize::from(name == "write" || name == "writeln");
                for a in args.iter().skip(skip) {
                    if let Expr::MethodCall(c) = a {
                        if c.method == "clone" && c.args.is_empty() {
                            self.out.extend(self.ctx.edit(
                                c.dot_token.spans[0].start(),
                                c.paren_token.span.close().end(),
                                String::new(),
                                "macro_arg_clone",
                            ));
                        }
                    }
                }
            }
        }
        syn::visit::visit_macro(self, m);
    }
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let mut out = Vec::new();
    V { ctx, out: &mut out }.visit_file(ctx.file);
    out
}
