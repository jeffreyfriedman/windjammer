//! Callee path resolution: `f`, `Type::assoc`, `Self::assoc`, `Trait::method`, `mod::f`, `Enum::V`.
//!
//! Free names resolve through scope only: a method never competes with a free function.

use super::collect::Collected;
use super::def::{DefId, DefKind};
use super::lookup::{Found, Ns};
use super::prelude::{is_prelude_type, is_prelude_value};
use super::res::{Reason, Res};

/// Lexical context a body is resolved in.
pub struct Ctx<'c> {
    pub module: DefId,
    pub impl_idx: Option<usize>,
    pub trait_def: Option<DefId>,
    pub generics: &'c [String],
}

enum Start {
    Def(DefId),
    Done(Res),
}

impl<'a> Collected<'a> {
    /// Resolve an unqualified name in value position (locals are the caller's concern).
    pub fn resolve_free(&self, ctx: &Ctx<'_>, name: &str) -> Res {
        let found = self.lookup(ctx.module, name, Ns::Value);
        if !found.is_empty() {
            return found_to_res(found, name);
        }
        if is_prelude_value(name) {
            return Res::Prim(name.to_string());
        }
        Res::Unresolved(Reason::NotFound(name.to_string()))
    }

    /// Resolve a `::`-qualified callee (two or more segments).
    pub fn resolve_path(&self, ctx: &Ctx<'_>, segs: &[&str]) -> Res {
        let text = segs.join("::");
        let mut cur = match self.path_start(ctx, segs[0], &text) {
            Start::Def(d) => d,
            Start::Done(r) => return r,
        };
        let last = segs.len() - 1;
        for seg in &segs[1..last] {
            let found = self.member(cur, seg, Ns::Type, &mut Vec::new(), 0);
            match found_to_res(found, &text) {
                Res::Def(d) => cur = d,
                other => return other,
            }
        }
        self.finish_member(cur, segs[last], &text)
    }

    fn path_start(&self, ctx: &Ctx<'_>, first: &str, text: &str) -> Start {
        if first == "Self" {
            return self.self_start(ctx, text);
        }
        if ctx.generics.iter().any(|g| g == first) {
            return Start::Done(Res::Unresolved(Reason::GenericDispatch(text.to_string())));
        }
        let found = self.lookup(ctx.module, first, Ns::Type);
        if !found.is_empty() {
            return match found_to_res(found, text) {
                Res::Def(d) => Start::Def(d),
                other => Start::Done(other),
            };
        }
        if is_prelude_type(first) {
            return Start::Done(Res::Prim(text.to_string()));
        }
        if is_crate_like(first) {
            return Start::Done(Res::Unresolved(Reason::External(text.to_string())));
        }
        Start::Done(Res::Unresolved(Reason::NotFound(first.to_string())))
    }

    fn self_start(&self, ctx: &Ctx<'_>, text: &str) -> Start {
        if let Some(idx) = ctx.impl_idx {
            let info = &self.impls[idx];
            return match info.self_def {
                Some(d) => Start::Def(d),
                None => Start::Done(Res::Unresolved(Reason::External(info.type_name.clone()))),
            };
        }
        match ctx.trait_def {
            Some(t) => Start::Def(t),
            None => Start::Done(Res::Unresolved(Reason::NotFound(text.to_string()))),
        }
    }

    fn finish_member(&self, def: DefId, name: &str, text: &str) -> Res {
        match self.defs.get(def).kind {
            DefKind::Module => {
                let found = self.member(def, name, Ns::Value, &mut Vec::new(), 0);
                found_to_res(found, text)
            }
            DefKind::Enum => match self.variant_named(def, name) {
                v if !v.is_empty() => found_to_res(v, text),
                _ => self.assoc_item(def, name, text),
            },
            DefKind::Trait => self.trait_member(def, name, text),
            DefKind::TypeAlias => match self.follow_alias(def) {
                Some(t) if t != def => self.finish_member(t, name, text),
                _ => Res::Unresolved(Reason::NoAssocItem(text.to_string())),
            },
            _ => self.assoc_item(def, name, text),
        }
    }

    fn trait_member(&self, trait_def: DefId, name: &str, text: &str) -> Res {
        let ids: Vec<DefId> = self
            .members
            .get(&trait_def)
            .into_iter()
            .flatten()
            .copied()
            .filter(|m| self.defs.get(*m).name == name)
            .collect();
        ids_to_res(ids, Reason::NoAssocItem(text.to_string()))
    }

    /// `Type::name`: inherent impls win; then trait impls; then trait default methods.
    fn assoc_item(&self, ty: DefId, name: &str, text: &str) -> Res {
        let impls: Vec<usize> = self.impls_of.get(&ty).cloned().unwrap_or_default();
        let named = |idx: &usize| -> Vec<DefId> {
            self.impls[*idx]
                .fns
                .iter()
                .copied()
                .filter(|f| self.defs.get(*f).name == name)
                .collect()
        };
        let inherent: Vec<DefId> = impls
            .iter()
            .filter(|i| self.impls[**i].trait_name.is_none())
            .flat_map(named)
            .collect();
        if !inherent.is_empty() {
            return ids_to_res(inherent, Reason::NoAssocItem(text.to_string()));
        }
        let mut via_trait: Vec<DefId> = impls
            .iter()
            .filter(|i| self.impls[**i].trait_name.is_some())
            .flat_map(named)
            .collect();
        if via_trait.is_empty() {
            via_trait = self.default_methods_for(&impls, name);
        }
        ids_to_res(via_trait, Reason::NoAssocItem(text.to_string()))
    }

    fn default_methods_for(&self, impls: &[usize], name: &str) -> Vec<DefId> {
        let mut out = Vec::new();
        for i in impls {
            let Some(t) = self.impls[*i].trait_def else {
                continue;
            };
            for m in self.members.get(&t).into_iter().flatten() {
                if self.defs.get(*m).name == name && !out.contains(m) {
                    out.push(*m);
                }
            }
        }
        out
    }
}

fn ids_to_res(mut ids: Vec<DefId>, none: Reason) -> Res {
    ids.dedup();
    match ids.len() {
        0 => Res::Unresolved(none),
        1 => Res::Def(ids[0]),
        _ => Res::Ambiguous(ids),
    }
}

/// Collapse lookup results: one def, several (ambiguous), external, or nothing.
pub(crate) fn found_to_res(found: Vec<Found>, text: &str) -> Res {
    let defs: Vec<DefId> = found
        .iter()
        .filter_map(|f| match f {
            Found::Def(d) => Some(*d),
            Found::External(_) => None,
        })
        .collect();
    if !defs.is_empty() {
        return ids_to_res(defs, Reason::NotFound(text.to_string()));
    }
    match found.first() {
        Some(Found::External(p)) => Res::Unresolved(Reason::External(p.join("::"))),
        _ => Res::Unresolved(Reason::NotFound(text.to_string())),
    }
}

/// Rust-style naming: lowercase-initial first segment of a qualified path names a crate/module
/// (types are capitalised), so an unknown one is an extern crate path, not a typo'd type.
fn is_crate_like(seg: &str) -> bool {
    seg.chars().next().is_some_and(|c| c.is_lowercase())
}
