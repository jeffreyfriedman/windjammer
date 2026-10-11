//! string_append: `s = format!("{}<rest>", s, args..);` on a `String` binding
//! becomes an in-place append, turning quadratic string building linear
//! (PASSES.md P5).
//!
//! Preconditions: `s` is a local/param whose every binding is an owned
//! `String`; the format string is a plain (non-raw) literal beginning with an
//! unadorned `{}` consumed by the first argument `s`; the remainder uses only
//! positional placeholders (no `{0}`, no inline captures); no later argument
//! mentions `s`.

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::facts::{FnFacts, KnownTy};
use crate::optimizer::fn_walk::for_each_fn;
use crate::optimizer::mutants::Mutant;
use proc_macro2::TokenTree;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, Lit, Stmt, Token};

fn mentions(ts: proc_macro2::TokenStream, name: &str) -> bool {
    ts.into_iter().any(|t| match t {
        TokenTree::Ident(i) => i == name,
        TokenTree::Group(g) => mentions(g.stream(), name),
        _ => false,
    })
}

/// True when every `{..}` in `s` (outside `{{`/`}}`) is `{}` or `{:spec}`.
fn only_positional(s: &str) -> bool {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            '{' if b.get(i + 1) == Some(&'{') => i += 2,
            '}' if b.get(i + 1) == Some(&'}') => i += 2,
            '{' => {
                let Some(close) = b[i..].iter().position(|c| *c == '}') else { return false };
                let inner: String = b[i + 1..i + close].iter().collect();
                if !(inner.is_empty() || inner.starts_with(':')) || inner.contains('$') || inner.contains('*') {
                    return false;
                }
                i += close + 1;
            }
            _ => i += 1,
        }
    }
    true
}

struct V<'c, 'a> {
    ctx: &'c Ctx<'a>,
    facts: &'c FnFacts,
    out: &'c mut Vec<Edit>,
}

impl V<'_, '_> {
    fn rewrite(&mut self, assign: &syn::ExprAssign) -> Option<()> {
        let cfg = self.ctx.config;
        let Expr::Path(lhs) = &*assign.left else { return None };
        let name = lhs.path.get_ident()?.to_string();
        if !matches!(self.facts.unique(&name), Some(KnownTy::String)) {
            return None;
        }
        let Expr::Macro(m) = &*assign.right else { return None };
        if !m.mac.path.is_ident("format") {
            return None;
        }
        let args: Punctuated<Expr, Token![,]> =
            m.mac.parse_body_with(Punctuated::parse_terminated).ok()?;
        let mut it = args.iter();
        let Expr::Lit(fmt) = it.next()? else { return None };
        let Lit::Str(fmt_str) = &fmt.lit else { return None };
        let Expr::Path(first) = it.next()? else { return None };
        if !first.path.is_ident(&name) {
            return None;
        }
        let rest: Vec<&Expr> = it.collect();
        let value = fmt_str.value();
        let remainder = value.strip_prefix("{}")?;
        let lit_text = self.ctx.idx.text(fmt.span())?;
        let new_lit = lit_text.strip_prefix("\"{}")?;
        if !only_positional(remainder) || (remainder.is_empty() && rest.is_empty()) {
            return None;
        }
        if cfg.mutant != Some(Mutant::AppendIgnoresSelfReference)
            && rest.iter().any(|e| mentions(quote_tokens(e), &name))
        {
            return None;
        }
        let mut call = format!("write!({name}, \"{new_lit}");
        for e in &rest {
            call.push_str(", ");
            call.push_str(self.ctx.idx.text(e.span())?);
        }
        call.push(')');
        let text = format!("{{ use std::fmt::Write as _; {call}.expect(\"a formatting trait implementation returned an error\"); }}");
        let sp = assign.span();
        self.out.extend(self.ctx.edit(sp.start(), sp.end(), text, "string_append"));
        Some(())
    }
}

fn quote_tokens(e: &Expr) -> proc_macro2::TokenStream {
    use quote::ToTokens;
    e.to_token_stream()
}

impl<'ast> Visit<'ast> for V<'_, '_> {
    fn visit_stmt(&mut self, s: &'ast Stmt) {
        if let Stmt::Expr(Expr::Assign(a), Some(_)) = s {
            self.rewrite(a);
        }
        syn::visit::visit_stmt(self, s);
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
