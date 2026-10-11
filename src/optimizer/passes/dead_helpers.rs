//! dead_helpers (PASSES.md P7): within one self-contained Rust file,
//!  * remove `#[repr(C)]` from non-`pub` structs when the file has no
//!    layout-sensitive constructs;
//!  * remove an inherent impl that contains only `to_bytes` for a non-`pub`
//!    struct when `to_bytes` is referenced nowhere else in the file.
//!
//! A non-`pub` item cannot be named from another module, so whole-file token
//! evidence is complete evidence.

use super::Ctx;
use crate::optimizer::edit::Edit;
use crate::optimizer::mutants::Mutant;
use proc_macro2::TokenTree;
use std::collections::{HashMap, HashSet};
use syn::spanned::Spanned;
use syn::{Attribute, Item, Meta, Visibility};

const LAYOUT_SENSITIVE: &[&str] = &[
    "unsafe", "extern", "transmute", "size_of", "align_of", "offset_of", "as_ptr", "as_mut_ptr",
    "from_raw_parts", "MaybeUninit", "addr_of", "bytemuck", "Pod", "Zeroable",
];

fn idents(ts: proc_macro2::TokenStream, counts: &mut HashMap<String, usize>) {
    for t in ts {
        match t {
            TokenTree::Ident(i) => *counts.entry(i.to_string()).or_default() += 1,
            TokenTree::Group(g) => idents(g.stream(), counts),
            _ => {}
        }
    }
}

fn is_repr_c(a: &Attribute) -> bool {
    if let Meta::List(l) = &a.meta {
        return l.path.is_ident("repr") && l.tokens.to_string() == "C";
    }
    false
}

fn private_structs(items: &[Item], out: &mut HashSet<String>) {
    for it in items {
        match it {
            Item::Struct(s) if matches!(s.vis, Visibility::Inherited) => {
                out.insert(s.ident.to_string());
            }
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    private_structs(inner, out);
                }
            }
            _ => {}
        }
    }
}

pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    use quote::ToTokens;
    let mut counts = HashMap::new();
    idents(ctx.file.to_token_stream(), &mut counts);
    let sensitive = LAYOUT_SENSITIVE.iter().any(|w| counts.contains_key(*w));
    let sensitive = sensitive && ctx.config.mutant != Some(Mutant::ReprIgnoresUnsafe);
    let mut private = HashSet::new();
    private_structs(&ctx.file.items, &mut private);
    let mut out = Vec::new();
    walk(&ctx.file.items, ctx, &private, &counts, sensitive, &mut out);
    out
}

fn walk(
    items: &[Item],
    ctx: &Ctx,
    private: &HashSet<String>,
    counts: &HashMap<String, usize>,
    sensitive: bool,
    out: &mut Vec<Edit>,
) {
    for it in items {
        match it {
            Item::Struct(s) if matches!(s.vis, Visibility::Inherited) && !sensitive => {
                for a in s.attrs.iter().filter(|a| is_repr_c(a)) {
                    push_line_removal(ctx, a.span(), out);
                }
            }
            Item::Impl(im) if im.trait_.is_none() && im.items.len() == 1 => {
                let name = match &*im.self_ty {
                    syn::Type::Path(p) => p.path.get_ident().map(|i| i.to_string()),
                    _ => None,
                };
                let sole_to_bytes = matches!(&im.items[0], syn::ImplItem::Fn(f) if f.sig.ident == "to_bytes");
                let referenced = counts.get("to_bytes").copied().unwrap_or(0) > 1;
                let ignore = ctx.config.mutant == Some(Mutant::HelpersIgnoreReferences);
                if sole_to_bytes && name.is_some_and(|n| private.contains(&n)) && (!referenced || ignore) {
                    push_line_removal(ctx, im.span(), out);
                }
            }
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    walk(inner, ctx, private, counts, sensitive, out);
                }
            }
            _ => {}
        }
    }
}

fn push_line_removal(ctx: &Ctx, span: proc_macro2::Span, out: &mut Vec<Edit>) {
    let Some((s, e)) = ctx.idx.range(span) else { return };
    out.push(Edit::blank(ctx.idx.src(), s, e, "dead_helpers"));
}
