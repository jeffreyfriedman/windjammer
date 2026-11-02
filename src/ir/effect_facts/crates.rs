//! Effects of calls into Rust crate dependencies (ADR-0026, ADR-0021 §11).
//!
//! A crate call is a sink of unknown authority: it performs what its signature surface declares
//! (`@uses(..)`, nothing for `@pure`; trusted claims, like `extern fn`), and otherwise `ffi(<crate>)`, which only
//! `[capabilities] ffi = ["<crate>"]` grants. That includes calls into crates with no surface at all.
//! The table is built by the caller from `wj.toml` and the surfaces; this module holds no I/O.

use super::analysis::Resource;
use super::kind::EffectKind;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, Clone)]
pub struct CrateEffects {
    /// crate (path spelling) -> function -> declared effects (`None`: no `@uses`/`@pure`)
    crates: BTreeMap<String, BTreeMap<String, Option<BTreeSet<EffectKind>>>>,
}

impl CrateEffects {
    /// Register a Rust crate dependency (calls into it are attributed even without declarations).
    pub fn add_crate(&mut self, name: &str) {
        self.crates.entry(name.to_string()).or_default();
    }

    /// Record what the surface of `krate` declares for `func`.
    pub fn declare(&mut self, krate: &str, func: &str, effects: Option<BTreeSet<EffectKind>>) {
        self.crates.entry(krate.to_string()).or_default().insert(func.to_string(), effects);
    }

    pub fn is_empty(&self) -> bool {
        self.crates.is_empty()
    }

    /// Effects of a call to `path` (`crate::f`, or deeper), or `None` when its head is not a Rust crate.
    pub fn uses(&self, path: &str) -> Option<Vec<(EffectKind, Resource)>> {
        let (head, rest) = path.split_once("::").unwrap_or((path, ""));
        let fns = self.crates.get(head)?;
        let declared = if rest.contains("::") { None } else { fns.get(rest).cloned().flatten() };
        Some(match declared {
            Some(ks) => ks.into_iter().map(|k| (k, Resource::None)).collect(),
            None => vec![(EffectKind::Ffi, Resource::Lit(head.to_string()))],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_pure_undeclared_and_unknown() {
        let mut c = CrateEffects::default();
        c.add_crate("bare");
        c.declare("strsim", "levenshtein", Some(BTreeSet::new()));
        c.declare("strsim", "osa", None);
        c.declare("netc", "get", Some([EffectKind::NetConnect].into_iter().collect()));
        assert_eq!(c.uses("strsim::levenshtein"), Some(vec![]));
        let ffi = |n: &str| Some(vec![(EffectKind::Ffi, Resource::Lit(n.to_string()))]);
        assert_eq!(c.uses("strsim::osa"), ffi("strsim"));
        assert_eq!(c.uses("strsim::not_declared"), ffi("strsim"));
        assert_eq!(c.uses("bare::anything"), ffi("bare"));
        assert_eq!(c.uses("strsim::Type::method"), ffi("strsim"));
        assert_eq!(c.uses("netc::get"), Some(vec![(EffectKind::NetConnect, Resource::None)]));
        assert_eq!(c.uses("std::fs::read"), None);
    }
}
