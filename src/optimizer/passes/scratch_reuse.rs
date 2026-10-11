//! scratch_reuse (auto-arena, OFF by default, PASSES.md P8): a per-iteration scratch buffer
//!
//! ```text
//! for .. { let mut buf = String::new(); ..uses of buf by reference.. }
//! ```
//! becomes
//! ```text
//! { let mut buf = String::new(); for .. { buf.clear(); ..same uses.. } }
//! ```
//! so the loop reuses one allocation (the buffer's capacity is the per-loop "arena", reset by `clear`).
//!
//! Preconditions (each checked syntactically):
//! 1. The `let` is a top-level statement of the loop body, binds `mut NAME` (optionally `: T`) to
//!    `String::new()` or `Vec::new()`, and NAME is not mentioned in the loop header nor before the `let`,
//!    and is bound nowhere else in the loop (no shadowing).
//! 2. Every later use of NAME borrows it: receiver of a non-consuming method (whitelist), `&NAME`,
//!    `&mut NAME`, `NAME[i]`, `NAME += ..`, or an argument of a format-family macro. Any other use (a move,
//!    an assignment, a closure capture, `for x in NAME`) rejects the loop.
//! 3. `Vec` only: the file has no `impl Drop` and the loop header does not lock or borrow (`lock(`,
//!    `borrow`), so the one observable difference (elements of iteration k are dropped when iteration k+1
//!    starts instead of when iteration k ends) is not observable.
//! The wrapping block ends right after the loop, so the buffer is freed when the loop exits (break, normal
//! exit), as before; the value of a `loop` expression is the value of the block.

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::mutants::Mutant;
use proc_macro2::TokenTree;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Block, Expr, Pat, Stmt};

const BORROWING_METHODS: &[&str] = &[
    "push", "push_str", "pop", "insert", "remove", "extend", "extend_from_slice", "len", "is_empty", "iter",
    "iter_mut", "clear", "sort", "sort_by", "sort_unstable", "sort_by_key", "dedup", "contains", "truncate",
    "as_str", "as_slice", "get", "get_mut", "first", "last", "join", "starts_with", "ends_with", "chars", "bytes",
    "retain", "reverse", "clone", "to_string", "lines", "split", "trim", "find", "swap", "capacity", "concat",
    "to_vec", "to_owned", "to_uppercase", "to_lowercase", "write_str", "write_fmt", "as_bytes", "binary_search",
];

const FORMAT_MACROS: &[&str] = &[
    "format", "print", "println", "eprint", "eprintln", "write", "writeln", "assert", "assert_eq", "assert_ne",
    "panic", "debug_assert", "debug_assert_eq",
];

fn mentions(ts: proc_macro2::TokenStream, name: &str) -> bool {
    ts.into_iter().any(|t| match t {
        TokenTree::Ident(i) => i == name,
        TokenTree::Group(g) => mentions(g.stream(), name),
        _ => false,
    })
}

fn tokens_of<T: quote::ToTokens>(t: &T) -> proc_macro2::TokenStream {
    t.to_token_stream()
}

fn is_name(e: &Expr, name: &str) -> bool {
    matches!(e, Expr::Path(p) if p.qself.is_none() && p.path.is_ident(name))
}

/// `String::new()` -> Some(false), `Vec::new()` -> Some(true).
fn scratch_ctor(e: &Expr) -> Option<bool> {
    let Expr::Call(c) = e else { return None };
    let Expr::Path(p) = &*c.func else { return None };
    let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
    if !c.args.is_empty() {
        return None;
    }
    match segs.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["String", "new"] => Some(false),
        ["Vec", "new"] => Some(true),
        _ => None,
    }
}

/// Finds uses of `name` that are not borrows (condition 2).
struct UseCheck<'n> {
    name: &'n str,
    bad: bool,
    bindings: usize,
    ignore_moves: bool,
}

impl<'ast> Visit<'ast> for UseCheck<'_> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        match e {
            Expr::MethodCall(m) if is_name(&m.receiver, self.name) => {
                if !BORROWING_METHODS.contains(&m.method.to_string().as_str()) {
                    self.bad = true;
                }
                for a in &m.args {
                    self.visit_expr(a);
                }
            }
            Expr::Reference(r) if is_name(&r.expr, self.name) => {}
            Expr::Index(i) if is_name(&i.expr, self.name) => self.visit_expr(&i.index),
            Expr::Binary(b) if matches!(b.op, syn::BinOp::AddAssign(_)) && is_name(&b.left, self.name) => {
                self.visit_expr(&b.right)
            }
            Expr::Path(_) if is_name(e, self.name) => self.bad |= !self.ignore_moves,
            Expr::Closure(c) if mentions(tokens_of(c), self.name) => self.bad = true,
            _ => syn::visit::visit_expr(self, e),
        }
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if mentions(m.tokens.clone(), self.name) {
            let is_fmt = m.path.get_ident().is_some_and(|i| FORMAT_MACROS.contains(&i.to_string().as_str()));
            self.bad |= !is_fmt;
        }
    }
    fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
        if p.ident == self.name {
            self.bindings += 1;
        }
        syn::visit::visit_pat_ident(self, p);
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {
        self.bad = true; // nested items can shadow anything; stay out
    }
}

struct V<'c, 'a> {
    ctx: &'c Ctx<'a>,
    out: &'c mut Vec<Edit>,
}

impl V<'_, '_> {
    /// (header tokens, body) of a loop statement.
    fn parts(e: &Expr) -> Option<(proc_macro2::TokenStream, &Block)> {
        match e {
            Expr::ForLoop(f) => {
                let mut h = tokens_of(&*f.pat);
                h.extend(tokens_of(&*f.expr));
                Some((h, &f.body))
            }
            Expr::While(w) => Some((tokens_of(&*w.cond), &w.body)),
            Expr::Loop(l) => Some((proc_macro2::TokenStream::new(), &l.body)),
            _ => None,
        }
    }

    fn try_loop(&mut self, lp: &Expr) -> Option<()> {
        let (header, body) = Self::parts(lp)?;
        let src = self.ctx.idx.src();
        for (i, st) in body.stmts.iter().enumerate() {
            let Stmt::Local(l) = st else { continue };
            let (pat, ty) = match &l.pat {
                Pat::Type(pt) => (&*pt.pat, Some(&*pt.ty)),
                p => (p, None),
            };
            let Pat::Ident(pi) = pat else { continue };
            let Some(init) = &l.init else { continue };
            if pi.mutability.is_none() || pi.by_ref.is_some() || pi.subpat.is_some() || init.diverge.is_some() {
                continue;
            }
            let Some(is_vec) = scratch_ctor(&init.expr) else { continue };
            let name = pi.ident.to_string();
            // 1. not mentioned before the let or in the header
            if mentions(header.clone(), &name) || body.stmts[..i].iter().any(|s| mentions(tokens_of(s), &name)) {
                continue;
            }
            // 2. later uses borrow; single binding
            let mut chk = UseCheck {
                name: &name,
                bad: false,
                bindings: 0,
                ignore_moves: self.ctx.config.mutant == Some(Mutant::ScratchIgnoresMoves),
            };
            for s in &body.stmts[i + 1..] {
                chk.visit_stmt(s);
            }
            if chk.bad || chk.bindings > 0 {
                continue;
            }
            // 3. Vec drop timing
            let h = header.to_string();
            if is_vec && (src.contains("impl Drop") || h.contains("lock") || h.contains("borrow")) {
                continue;
            }
            let (ls, le) = self.ctx.idx.range(lp.span())?;
            let (ss, se) = self.ctx.idx.range(st.span())?;
            let decl = match ty {
                Some(t) => format!("let mut {name}: {} = {};", self.ctx.idx.text(t.span())?, self.ctx.idx.text(init.expr.span())?),
                None => format!("let mut {name} = {};", self.ctx.idx.text(init.expr.span())?),
            };
            // Keep the line count (source maps): the replaced `let` keeps its newlines.
            let nl = "\n".repeat(src[ss..se].matches('\n').count());
            let text = format!("{{ {decl} {}{name}.clear();{nl}{} }}", &src[ls..ss], &src[se..le]);
            self.out.push(Edit { start: ls, end: le, text, pass: "scratch_reuse" });
            return Some(());
        }
        None
    }
}

impl<'ast> Visit<'ast> for V<'_, '_> {
    fn visit_block(&mut self, b: &'ast Block) {
        for st in &b.stmts {
            if let Stmt::Expr(e, _) = st {
                if self.try_loop(e).is_some() {
                    continue; // nested loops are handled in a later round
                }
            }
            syn::visit::visit_stmt(self, st);
        }
    }
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let mut out = Vec::new();
    V { ctx, out: &mut out }.visit_file(ctx.file);
    out
}
