//! same_type_cast: remove no-op `as T` casts (PASSES.md P3).
//!
//! Sound cases: `(e as T) as T` (outer cast is the identity on type `T`), and
//! `x as T` where every binding of `x` in the function is declared `T`.

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::facts::{prim_name, FnFacts, KnownTy};
use crate::optimizer::fn_walk::for_each_fn;
use crate::optimizer::mutants::Mutant;
use syn::visit::Visit;
use syn::{Expr, ExprCast};

struct V<'c, 'a> {
    ctx: &'c Ctx<'a>,
    facts: &'c FnFacts,
    out: &'c mut Vec<Edit>,
}

impl V<'_, '_> {
    fn is_noop(&self, c: &ExprCast) -> bool {
        let Some(target) = prim_name(&c.ty) else { return false };
        let mut operand: &Expr = &c.expr;
        while let Expr::Paren(p) = operand {
            operand = &p.expr;
        }
        match operand {
            Expr::Cast(inner) => prim_name(&inner.ty).as_deref() == Some(target.as_str()),
            Expr::Path(p) => {
                let Some(id) = p.path.get_ident() else { return false };
                let name = id.to_string();
                if self.ctx.config.mutant == Some(Mutant::CastDropsAnyIdent) {
                    return matches!(self.facts.first_declared(&name), Some(KnownTy::Prim(_)));
                }
                matches!(self.facts.unique(&name), Some(KnownTy::Prim(t)) if *t == target)
            }
            _ => false,
        }
    }
}

impl<'ast> Visit<'ast> for V<'_, '_> {
    fn visit_expr_cast(&mut self, c: &'ast ExprCast) {
        if self.is_noop(c) {
            use syn::spanned::Spanned;
            let operand_end = c.expr.span().end();
            self.out.extend(self.ctx.edit(operand_end, c.ty.span().end(), String::new(), "same_type_cast"));
        }
        syn::visit::visit_expr_cast(self, c);
    }
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let mut out = Vec::new();
    for_each_fn(ctx.file, |f| {
        let facts = FnFacts::of(f.sig, f.body);
        V { ctx, facts: &facts, out: &mut out }.visit_block(f.body);
    });
    out
}
