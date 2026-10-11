//! inline_policy: keep `#[inline]` only on small leaf functions (PASSES.md P6).
//!
//! `#[inline]` is a pure optimizer hint with no semantic effect. It is removed
//! from `#[test]` functions, `main`, and functions whose body exceeds
//! `inline_max_body_bytes` (inlining those only inflates code size).

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::fn_walk::{for_each_fn, FnRef};
use syn::spanned::Spanned;
use syn::{Attribute, Meta};

fn is_plain_inline(a: &Attribute) -> bool {
    matches!(&a.meta, Meta::Path(p) if p.is_ident("inline"))
}

fn is_test(a: &Attribute) -> bool {
    a.path().is_ident("test") || a.path().is_ident("bench")
}

fn removable(f: &FnRef, ctx: &Ctx) -> bool {
    if f.attrs.iter().any(is_test) || f.sig.ident == "main" {
        return true;
    }
    ctx.idx
        .range(f.body.span())
        .map(|(s, e)| e - s > ctx.config.inline_max_body_bytes)
        .unwrap_or(false)
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let mut out = Vec::new();
    for_each_fn(ctx.file, |f| {
        if !removable(&f, ctx) {
            return;
        }
        for a in f.attrs.iter().filter(|a| is_plain_inline(a)) {
            let Some((s, e)) = ctx.idx.range(a.span()) else { continue };
            out.push(Edit::blank(ctx.idx.src(), s, e, "inline_policy"));
        }
    });
    out
}
