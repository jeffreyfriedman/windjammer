//! with_capacity: `let mut v = Vec::new(); for _ in 0_usize..N { v.push(..); }`
//! becomes `Vec::with_capacity(N)` (PASSES.md P4).
//!
//! Preconditions: the loop statement immediately follows the `let`; the range
//! starts at literal `0_usize`; `N` is side-effect free; the body pushes to `v`
//! unconditionally at its top level and contains no early exit; the file never
//! observes `.capacity()`.

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::mutants::Mutant;
use proc_macro2::TokenTree;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Block, Expr, ExprForLoop, Lit, Local, Pat, RangeLimits, Stmt};

/// Free of side effects and of moves of non-Copy values.
fn is_pure(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(l.lit, Lit::Int(_)),
        Expr::Path(p) => p.path.get_ident().is_some(),
        Expr::Field(f) => is_pure_place(&f.base),
        Expr::Paren(p) => is_pure(&p.expr),
        Expr::Binary(b) => {
            matches!(b.op, BinOp::Add(_) | BinOp::Sub(_) | BinOp::Mul(_)) && is_pure(&b.left) && is_pure(&b.right)
        }
        Expr::Cast(c) => is_pure(&c.expr),
        Expr::MethodCall(m) => m.method == "len" && m.args.is_empty() && is_pure_place(&m.receiver),
        _ => false,
    }
}

fn is_pure_place(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => p.path.get_ident().is_some(),
        Expr::Field(f) => is_pure_place(&f.base),
        Expr::Paren(p) => is_pure_place(&p.expr),
        _ => false,
    }
}

fn tokens_have_exit(ts: proc_macro2::TokenStream) -> bool {
    ts.into_iter().any(|t| match t {
        TokenTree::Ident(i) => i == "break" || i == "return" || i == "continue",
        TokenTree::Punct(p) => p.as_char() == '?',
        TokenTree::Group(g) => tokens_have_exit(g.stream()),
        _ => false,
    })
}

struct ExitScan {
    found: bool,
    ignore_break: bool,
}

impl<'ast> Visit<'ast> for ExitScan {
    fn visit_expr_break(&mut self, _: &'ast syn::ExprBreak) {
        self.found |= !self.ignore_break;
    }
    fn visit_expr_continue(&mut self, _: &'ast syn::ExprContinue) {
        self.found |= !self.ignore_break;
    }
    fn visit_expr_return(&mut self, _: &'ast syn::ExprReturn) {
        self.found = true;
    }
    fn visit_expr_try(&mut self, _: &'ast syn::ExprTry) {
        self.found = true;
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if tokens_have_exit(m.tokens.clone()) {
            self.found = true;
        }
    }
}

fn has_exit(body: &Block, ignore_break: bool) -> bool {
    let mut s = ExitScan { found: false, ignore_break };
    s.visit_block(body);
    s.found
}

fn is_vec_new(e: &Expr) -> bool {
    if let Expr::Call(c) = e {
        if let Expr::Path(p) = &*c.func {
            let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            return segs == ["Vec", "new"] && c.args.is_empty();
        }
    }
    false
}

fn pushes_unconditionally(body: &Block, name: &str) -> bool {
    body.stmts.iter().any(|s| match s {
        Stmt::Expr(Expr::MethodCall(m), Some(_)) => {
            m.method == "push" && matches!(&*m.receiver, Expr::Path(p) if p.path.is_ident(name))
        }
        _ => false,
    })
}

struct V<'c, 'a> {
    ctx: &'c Ctx<'a>,
    out: &'c mut Vec<Edit>,
}

impl V<'_, '_> {
    fn try_pair(&mut self, local: &Local, next: &Stmt) {
        let cfg = self.ctx.config;
        let Pat::Ident(pi) = (match &local.pat {
            Pat::Type(pt) => &*pt.pat,
            p => p,
        }) else {
            return;
        };
        let Some(init) = &local.init else { return };
        if !is_vec_new(&init.expr) || init.diverge.is_some() {
            return;
        }
        let Stmt::Expr(Expr::ForLoop(fl), _) = next else { return };
        let name = pi.ident.to_string();
        let Some(cap) = self.capacity_text(fl, &name) else { return };
        let sp = init.expr.span();
        self.out.extend(self.ctx.edit(sp.start(), sp.end(), format!("Vec::with_capacity({cap})"), "with_capacity"));
        let _ = cfg;
    }

    fn capacity_text(&self, fl: &ExprForLoop, name: &str) -> Option<String> {
        let cfg = self.ctx.config;
        let Expr::Range(r) = &*fl.expr else { return None };
        if !matches!(r.limits, RangeLimits::HalfOpen(_)) {
            return None;
        }
        let (from, to) = (r.start.as_deref()?, r.end.as_deref()?);
        let zero = matches!(from, Expr::Lit(l) if matches!(&l.lit, Lit::Int(i) if i.base10_digits() == "0" && i.suffix() == "usize"));
        let any_start = cfg.mutant == Some(Mutant::CapacityAnyStart);
        if !(zero || any_start) || !is_pure(to) || !is_pure(from) {
            return None;
        }
        if !pushes_unconditionally(&fl.body, name)
            || has_exit(&fl.body, cfg.mutant == Some(Mutant::CapacityIgnoresBreak))
        {
            return None;
        }
        if self.ctx.idx.src().contains(".capacity(") {
            return None;
        }
        let to_text = self.ctx.idx.text(to.span())?;
        if zero {
            Some(to_text.to_string())
        } else {
            Some(format!("({}) - ({})", to_text, self.ctx.idx.text(from.span())?))
        }
    }
}

impl<'ast> Visit<'ast> for V<'_, '_> {
    fn visit_block(&mut self, b: &'ast Block) {
        for w in b.stmts.windows(2) {
            if let Stmt::Local(l) = &w[0] {
                self.try_pair(l, &w[1]);
            }
        }
        syn::visit::visit_block(self, b);
    }
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let mut out = Vec::new();
    V { ctx, out: &mut out }.visit_file(ctx.file);
    out
}
