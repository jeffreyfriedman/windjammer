//! bounded_counter (PASSES.md P10): arithmetic on a `while` loop counter that provably cannot overflow is written with
//! `wrapping_*`, so the overflow check the release profile keeps (`overflow-checks = true`) is not paid where it can
//! never fire.
//!
//! Shape: `let mut i = L;` (L a non-negative integer literal) and `while i < B [&& ..] { ..; i += 1; .. }` where `i`
//! has one known integer type in the function, the `i += 1` is a top-level statement of the loop body and the only
//! write to `i` anywhere in the body.
//! Rewrites:
//! - (a) `i += 1` -> `i = i.wrapping_add(1)`: `i < B` held at the loop head and `i` is unchanged until the increment,
//!   so `i + 1 <= B <= T::MAX`.
//! - (b) with `B` an integer literal: `i * c`, `i + c` and their compositions with literals in statements before the
//!   increment -> `wrapping_mul`/`wrapping_add` when the largest value, computed with `0 <= i <= B - 1`, fits `T`
//!   (`i` starts at a non-negative literal and only ever grows by one, so it is never negative).

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::facts::{FnFacts, KnownTy};
use crate::optimizer::fn_walk::for_each_fn;
use crate::optimizer::mutants::Mutant;
use proc_macro2::TokenTree;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Block, Expr, ExprWhile, Lit, Pat, Stmt};

const FORMAT_FAMILY: &[&str] = &["format", "print", "println", "eprint", "eprintln", "write", "writeln", "panic", "assert", "assert_eq", "assert_ne"];

fn int_max(ty: &str) -> Option<u128> {
    Some(match ty {
        "i8" => i8::MAX as u128,
        "i16" => i16::MAX as u128,
        "i32" => i32::MAX as u128,
        "i64" => i64::MAX as u128,
        "i128" => i128::MAX as u128,
        "isize" => isize::MAX as u128,
        "u8" => u8::MAX as u128,
        "u16" => u16::MAX as u128,
        "u32" => u32::MAX as u128,
        "u64" => u64::MAX as u128,
        "u128" => u128::MAX,
        "usize" => usize::MAX as u128,
        _ => return None,
    })
}

fn int_lit(e: &Expr) -> Option<u128> {
    match e {
        Expr::Lit(l) => match &l.lit {
            Lit::Int(i) => i.base10_parse::<u128>().ok(),
            _ => None,
        },
        Expr::Paren(p) => int_lit(&p.expr),
        _ => None,
    }
}

fn is_ident(e: &Expr, name: &str) -> bool {
    matches!(e, Expr::Path(p) if p.path.is_ident(name))
}

/// Every write (or possible write) to `name` inside a block, and every re-binding of it.
struct Writes<'n> {
    name: &'n str,
    writes: usize,
    /// writes other than `name += 1`
    other_writes: usize,
    rebinds: bool,
    escapes: bool,
    /// every `let` of `name` has a non-negative integer literal initializer
    lets_non_negative: bool,
}

fn tokens_mention(ts: proc_macro2::TokenStream, name: &str) -> bool {
    ts.into_iter().any(|t| match t {
        TokenTree::Ident(i) => i == name,
        TokenTree::Group(g) => tokens_mention(g.stream(), name),
        _ => false,
    })
}

impl<'ast> Visit<'ast> for Writes<'_> {
    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        if is_ident(&a.left, self.name) {
            self.writes += 1;
            self.other_writes += 1;
        }
        syn::visit::visit_expr_assign(self, a);
    }
    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        let compound = matches!(
            b.op,
            BinOp::AddAssign(_) | BinOp::SubAssign(_) | BinOp::MulAssign(_) | BinOp::DivAssign(_) | BinOp::RemAssign(_)
                | BinOp::BitXorAssign(_) | BinOp::BitAndAssign(_) | BinOp::BitOrAssign(_) | BinOp::ShlAssign(_) | BinOp::ShrAssign(_)
        );
        if compound && is_ident(&b.left, self.name) {
            self.writes += 1;
            if !(matches!(b.op, BinOp::AddAssign(_)) && int_lit(&b.right) == Some(1)) {
                self.other_writes += 1;
            }
        }
        syn::visit::visit_expr_binary(self, b);
    }
    fn visit_expr_reference(&mut self, r: &'ast syn::ExprReference) {
        if r.mutability.is_some() && is_ident(&r.expr, self.name) {
            self.escapes = true;
        }
        syn::visit::visit_expr_reference(self, r);
    }
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if is_ident(&m.receiver, self.name) && m.method.to_string().ends_with("_assign") {
            self.escapes = true;
        }
        syn::visit::visit_expr_method_call(self, m);
    }
    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        if tokens_mention(quote::quote!(#c), self.name) {
            self.escapes = true;
        }
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let name = m.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
        if !FORMAT_FAMILY.contains(&name.as_str()) && tokens_mention(m.tokens.clone(), self.name) {
            self.escapes = true;
        }
    }
    fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
        if p.ident == self.name {
            self.rebinds = true;
        }
        syn::visit::visit_pat_ident(self, p);
    }
    fn visit_local(&mut self, l: &'ast syn::Local) {
        let p = match &l.pat {
            Pat::Type(pt) => &*pt.pat,
            p => p,
        };
        if matches!(p, Pat::Ident(pi) if pi.ident == self.name)
            && !l.init.as_ref().is_some_and(|i| i.diverge.is_none() && int_lit(&i.expr).is_some())
        {
            self.lets_non_negative = false;
        }
        syn::visit::visit_local(self, l);
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

/// `i < B` among the top-level `&&` operands of the condition: B. (`le`: the mutant that also accepts `i <= B`.)
fn bound_of<'e>(cond: &'e Expr, name: &str, le: bool) -> Option<&'e Expr> {
    match cond {
        Expr::Binary(b) if matches!(b.op, BinOp::And(_)) => bound_of(&b.left, name, le).or_else(|| bound_of(&b.right, name, le)),
        Expr::Binary(b) if (matches!(b.op, BinOp::Lt(_)) || (le && matches!(b.op, BinOp::Le(_)))) && is_ident(&b.left, name) => Some(&b.right),
        Expr::Binary(b) if matches!(b.op, BinOp::Gt(_)) && is_ident(&b.right, name) => Some(&b.left),
        Expr::Paren(p) => bound_of(&p.expr, name, le),
        _ => None,
    }
}

/// Index of the top-level `name += 1;` statement.
fn increment_at(body: &Block, name: &str) -> Option<usize> {
    body.stmts.iter().position(|s| match s {
        Stmt::Expr(Expr::Binary(b), Some(_)) => {
            matches!(b.op, BinOp::AddAssign(_)) && is_ident(&b.left, name) && int_lit(&b.right) == Some(1)
        }
        _ => false,
    })
}

fn scan<'n>(block: &Block, name: &'n str) -> Writes<'n> {
    let mut w = Writes { name, writes: 0, other_writes: 0, rebinds: false, escapes: false, lets_non_negative: true };
    w.visit_block(block);
    w
}

/// The counter is never negative: every binding of it starts at a non-negative literal and every write anywhere in
/// the function is `name += 1` (it only grows), with no `&mut` or other escape.
fn never_negative(fn_body: &Block, name: &str) -> bool {
    let w = scan(fn_body, name);
    w.lets_non_negative && w.other_writes == 0 && !w.escapes
}

/// Largest value of `e` over `0 <= name <= hi`, when `e` is built from `name`, int literals, `+` and `*`.
fn max_value(e: &Expr, name: &str, hi: u128) -> Option<u128> {
    match e {
        Expr::Paren(p) => max_value(&p.expr, name, hi),
        Expr::Path(_) if is_ident(e, name) => Some(hi),
        Expr::Lit(_) => int_lit(e),
        Expr::Binary(b) => {
            let (l, r) = (max_value(&b.left, name, hi)?, max_value(&b.right, name, hi)?);
            match b.op {
                BinOp::Add(_) => l.checked_add(r),
                BinOp::Mul(_) => l.checked_mul(r),
                _ => None,
            }
        }
        _ => None,
    }
}

fn mentions(e: &Expr, name: &str) -> bool {
    tokens_mention(quote::quote!(#e), name)
}

/// Rewrite an affine expression of `name` with wrapping operations (only when it mentions `name`).
fn wrapping_text(e: &Expr, name: &str, idx: &crate::optimizer::edit::SourceIndex) -> Option<String> {
    match e {
        Expr::Paren(p) => Some(format!("({})", wrapping_text(&p.expr, name, idx)?)),
        Expr::Path(_) | Expr::Lit(_) => idx.text(e.span()).map(str::to_string),
        Expr::Binary(b) => {
            let op = match b.op {
                BinOp::Add(_) => "wrapping_add",
                BinOp::Mul(_) => "wrapping_mul",
                _ => return None,
            };
            let l = wrapping_text(&b.left, name, idx)?;
            let r = wrapping_text(&b.right, name, idx)?;
            // the receiver must carry the counter's type: put the side that mentions the counter first
            if mentions(&b.left, name) {
                Some(format!("({l}).{op}({r})"))
            } else {
                Some(format!("({r}).{op}({l})"))
            }
        }
        _ => None,
    }
}

/// Outermost `+`/`*` expressions over the counter and literals that fit `max` (rule (b)).
struct Affine<'c, 'a> {
    ctx: &'c Ctx<'a>,
    name: &'c str,
    hi: u128,
    max: u128,
    out: &'c mut Vec<Edit>,
}

impl<'ast> Visit<'ast> for Affine<'_, '_> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        if let Expr::Binary(b) = e {
            if matches!(b.op, BinOp::Add(_) | BinOp::Mul(_)) && mentions(e, self.name) {
                if let Some(m) = max_value(e, self.name, self.hi) {
                    if m <= self.max {
                        if let Some(t) = wrapping_text(e, self.name, self.ctx.idx) {
                            let sp = e.span();
                            self.out.extend(self.ctx.edit(sp.start(), sp.end(), t, "bounded_counter"));
                            return;
                        }
                    }
                }
            }
        }
        syn::visit::visit_expr(self, e);
    }
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    fn visit_macro(&mut self, _: &'ast syn::Macro) {}
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

struct Loops<'c, 'a> {
    ctx: &'c Ctx<'a>,
    /// the counter is a parameter: its starting value is unknown, so rule (b) does not apply
    params: Vec<String>,
    facts: &'c FnFacts,
    fn_body: &'c Block,
    out: &'c mut Vec<Edit>,
}

impl Loops<'_, '_> {
    fn try_loop(&mut self, w: &ExprWhile) {
        let mutant = self.ctx.config.mutant;
        // the counter: the left side of `i < B`
        let Some(name) = counter_name(&w.cond) else { return };
        let Some(KnownTy::Prim(ty)) = self.facts.unique(&name) else { return };
        let Some(max) = int_max(ty) else { return };
        let Some(bound) = bound_of(&w.cond, &name, mutant == Some(Mutant::CounterAcceptsLessEqual)) else { return };
        let Some(at) = increment_at(&w.body, &name) else { return };
        let wr = scan(&w.body, &name);
        let only_write = wr.writes == 1 || mutant == Some(Mutant::CounterIgnoresOtherWrites);
        if !only_write || wr.rebinds || wr.escapes {
            return;
        }
        // (a) the increment
        if let Stmt::Expr(Expr::Binary(b), _) = &w.body.stmts[at] {
            let one = self.ctx.idx.text(b.right.span()).unwrap_or("1").to_string();
            let sp = b.span();
            self.out.extend(self.ctx.edit(sp.start(), sp.end(), format!("{name} = {name}.wrapping_add({one})"), "bounded_counter"));
        }
        // (b) affine uses before the increment, for a literal bound and a counter that starts non-negative
        if let Some(b) = int_lit(bound) {
            if b == 0 || self.params.contains(&name) || !never_negative(self.fn_body, &name) {
                return;
            }
            let max = if mutant == Some(Mutant::CounterIgnoresTypeMax) { u128::MAX } else { max };
            let mut a = Affine { ctx: self.ctx, name: &name, hi: b - 1, max, out: &mut *self.out };
            for s in &w.body.stmts[..at] {
                a.visit_stmt(s);
            }
        }
    }
}

fn counter_name(cond: &Expr) -> Option<String> {
    match cond {
        Expr::Binary(b) if matches!(b.op, BinOp::And(_)) => counter_name(&b.left).or_else(|| counter_name(&b.right)),
        Expr::Binary(b) if matches!(b.op, BinOp::Lt(_) | BinOp::Le(_)) => match &*b.left {
            Expr::Path(p) => p.path.get_ident().map(|i| i.to_string()),
            _ => None,
        },
        Expr::Binary(b) if matches!(b.op, BinOp::Gt(_)) => match &*b.right {
            Expr::Path(p) => p.path.get_ident().map(|i| i.to_string()),
            _ => None,
        },
        Expr::Paren(p) => counter_name(&p.expr),
        _ => None,
    }
}

impl<'ast> Visit<'ast> for Loops<'_, '_> {
    fn visit_expr_while(&mut self, w: &'ast ExprWhile) {
        self.try_loop(w);
        syn::visit::visit_expr_while(self, w);
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let mut out = Vec::new();
    for_each_fn(ctx.file, |f| {
        let facts = FnFacts::of(f.sig, f.body);
        let params = f
            .sig
            .inputs
            .iter()
            .filter_map(|a| match a {
                syn::FnArg::Typed(pt) => match &*pt.pat {
                    Pat::Ident(pi) => Some(pi.ident.to_string()),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let mut l = Loops { ctx, params, facts: &facts, fn_body: f.body, out: &mut out };
        l.visit_block(f.body);
    });
    out
}
