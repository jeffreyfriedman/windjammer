//! temp_clone and copy_clone: remove `.clone()` whose removal cannot change
//! behaviour (see docs/optimization/PASSES.md, P1 and P2).

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::facts::{FnFacts, KnownTy};
use crate::optimizer::fn_walk::for_each_fn;
use crate::optimizer::mutants::Mutant;
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, Lit};

fn is_str_lit(e: &Expr) -> bool {
    matches!(e, Expr::Lit(l) if matches!(l.lit, Lit::Str(_)))
}

fn call_path(e: &Expr) -> Option<Vec<String>> {
    if let Expr::Path(p) = e {
        return Some(p.path.segments.iter().map(|s| s.ident.to_string()).collect());
    }
    None
}

/// Receiver is a freshly constructed owned value with no other owner.
pub fn is_owned_temporary(e: &Expr) -> bool {
    match e {
        Expr::Macro(m) => m.mac.path.is_ident("vec"),
        Expr::Call(c) => {
            let p = call_path(&c.func).unwrap_or_default();
            let p: Vec<&str> = p.iter().map(String::as_str).collect();
            match p.as_slice() {
                ["String", "new"] | ["Vec", "new"] => c.args.is_empty(),
                ["String", "from"] => c.args.len() == 1 && is_str_lit(&c.args[0]),
                _ => false,
            }
        }
        Expr::MethodCall(m) => m.method == "to_string" && m.args.is_empty() && is_str_lit(&m.receiver),
        _ => false,
    }
}

struct V<'c, 'a> {
    ctx: &'c Ctx<'a>,
    facts: Option<&'c FnFacts>,
    out: &'c mut Vec<Edit>,
}

impl V<'_, '_> {
    fn removable(&self, call: &ExprMethodCall) -> Option<&'static str> {
        if call.method != "clone" || !call.args.is_empty() {
            return None;
        }
        let cfg = self.ctx.config;
        if cfg.temp_clone && is_owned_temporary(&call.receiver) {
            return Some("temp_clone");
        }
        if cfg.mutant == Some(Mutant::TempCloneOnAnyIdent) && matches!(&*call.receiver, Expr::Path(_)) {
            return Some("temp_clone");
        }
        if cfg.copy_clone {
            if let (Expr::Path(p), Some(f)) = (&*call.receiver, self.facts) {
                if let Some(id) = p.path.get_ident() {
                    let name = id.to_string();
                    let ty = if cfg.mutant == Some(Mutant::CopyCloneIgnoresShadowing) {
                        f.first_declared(&name)
                    } else {
                        f.unique(&name)
                    };
                    if matches!(ty, Some(KnownTy::Prim(_))) {
                        return Some("copy_clone");
                    }
                }
            }
        }
        None
    }
}

impl<'ast> Visit<'ast> for V<'_, '_> {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if let Some(pass) = self.removable(call) {
            let e = self.ctx.edit(
                call.dot_token.spans[0].start(),
                call.paren_token.span.close().end(),
                String::new(),
                pass,
            );
            self.out.extend(e);
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let mut out = Vec::new();
    for_each_fn(ctx.file, |f| {
        let facts = FnFacts::of(f.sig, f.body);
        V { ctx, facts: Some(&facts), out: &mut out }.visit_block(f.body);
    });
    out
}
