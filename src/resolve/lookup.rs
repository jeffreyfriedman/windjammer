//! Scope lookup: items, imports (explicit, aliased, glob, re-exported) and `crate`/`self`/`super`.
//!
//! Lookup is lazy and cycle-guarded; it never consults methods (those live in impls).

use super::collect::{base_type_name, Collected};
use super::def::{Def, DefId, DefKind};
use super::uses::Binding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ns {
    Type,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Def(DefId),
    /// Resolves outside the program; full path attached.
    External(Vec<String>),
}

type Guard = Vec<(DefId, String, Ns)>;

const MAX_ALIAS_DEPTH: u8 = 8;

fn accepts(def: &Def, ns: Ns) -> bool {
    match ns {
        Ns::Type => def.in_type_ns(),
        Ns::Value => def.in_value_ns(),
    }
}

fn dedupe(v: Vec<Found>) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::with_capacity(v.len());
    for f in v {
        if !out.contains(&f) {
            out.push(f);
        }
    }
    out
}

impl<'a> Collected<'a> {
    /// Look `name` up in `module`'s scope (items, imports, globs).
    pub fn lookup(&self, module: DefId, name: &str, ns: Ns) -> Vec<Found> {
        self.lookup_in_module(module, name, ns, &mut Vec::new())
    }

    /// Resolve a (possibly qualified) type path as written in `module`.
    pub fn resolve_type_path(&self, module: DefId, segs: &[String]) -> Vec<Found> {
        self.resolve_use_path(module, segs, Ns::Type, &mut Vec::new())
    }

    pub fn parent_of(&self, module: DefId) -> Option<DefId> {
        self.modules.get(&module).and_then(|m| m.parent)
    }

    fn lookup_in_module(&self, module: DefId, name: &str, ns: Ns, guard: &mut Guard) -> Vec<Found> {
        if let Some(special) = self.special_segment(module, name) {
            return if ns == Ns::Type { special } else { Vec::new() };
        }
        let key = (module, name.to_string(), ns);
        if guard.contains(&key) {
            return Vec::new();
        }
        guard.push(key);
        let mut found = self.lookup_direct(module, name, ns, guard);
        if found.is_empty() {
            found = self.lookup_globs(module, name, ns, guard);
        }
        if found.is_empty() {
            found = self.lookup_child_reexports(module, name, ns, guard);
        }
        guard.pop();
        dedupe(found)
    }

    fn special_segment(&self, module: DefId, name: &str) -> Option<Vec<Found>> {
        let target = match name {
            "self" => Some(module),
            "crate" => Some(self.root),
            "super" => self.parent_of(module),
            _ => return None,
        };
        Some(target.map(Found::Def).into_iter().collect())
    }

    fn lookup_direct(&self, module: DefId, name: &str, ns: Ns, guard: &mut Guard) -> Vec<Found> {
        let scope = &self.modules[&module];
        let mut out: Vec<Found> = scope
            .items
            .get(name)
            .into_iter()
            .flatten()
            .filter(|d| accepts(self.defs.get(**d), ns))
            .map(|d| Found::Def(*d))
            .collect();
        for u in &scope.uses {
            if u.binding == Binding::Named(name.to_string()) {
                out.extend(self.resolve_use_path(module, &u.segments, ns, guard));
            }
        }
        out
    }

    /// A module re-exports the public items of its child modules (`pub mod http` makes `crate::adapters::serve`
    /// the same function as `crate::adapters::http::serve`): the generated Rust module declares
    /// `pub use self::child::*` for every child, and the interpreter binds the same names.
    fn lookup_child_reexports(&self, module: DefId, name: &str, ns: Ns, guard: &mut Guard) -> Vec<Found> {
        let mut out = Vec::new();
        let mut children: Vec<DefId> = self.modules[&module]
            .items
            .values()
            .flatten()
            .copied()
            .filter(|d| {
                let def = self.defs.get(*d);
                def.kind == DefKind::Module && def.is_pub && self.parent_of(*d) == Some(module)
            })
            .collect();
        children.sort();
        for child in children {
            let scope = &self.modules[&child];
            if let Some(defs) = scope.items.get(name) {
                out.extend(
                    defs.iter()
                        .filter(|d| {
                            let def = self.defs.get(**d);
                            def.is_pub && def.kind != DefKind::Module && accepts(def, ns)
                        })
                        .map(|d| Found::Def(*d)),
                );
            }
        }
        let _ = guard;
        out
    }

    fn lookup_globs(&self, module: DefId, name: &str, ns: Ns, guard: &mut Guard) -> Vec<Found> {
        let mut out = Vec::new();
        let mut external_glob: Option<Vec<String>> = None;
        for u in &self.modules[&module].uses {
            if u.binding != Binding::Glob {
                continue;
            }
            for target in self.resolve_use_path(module, &u.segments, Ns::Type, guard) {
                match target {
                    Found::Def(d) => out.extend(self.member(d, name, ns, guard, 0)),
                    Found::External(p) => external_glob = Some(p),
                }
            }
        }
        if out.is_empty() {
            if let Some(mut p) = external_glob {
                p.push(name.to_string());
                out.push(Found::External(p));
            }
        }
        out
    }

    /// Resolve `segs` starting from `module`'s scope; the final segment is looked up in `ns`.
    pub(crate) fn resolve_use_path(
        &self,
        module: DefId,
        segs: &[String],
        ns: Ns,
        guard: &mut Guard,
    ) -> Vec<Found> {
        let Some((first, rest)) = segs.split_first() else {
            return Vec::new();
        };
        let first_ns = if rest.is_empty() { ns } else { Ns::Type };
        let mut cur = self.lookup_in_module(module, first, first_ns, guard);
        if cur.is_empty() && module != self.root {
            cur = self.lookup_in_module(self.root, first, first_ns, guard);
        }
        if cur.is_empty() {
            return vec![Found::External(segs.to_vec())];
        }
        for (i, seg) in rest.iter().enumerate() {
            let is_last = i + 1 == rest.len();
            let seg_ns = if is_last { ns } else { Ns::Type };
            let was_root = cur.contains(&Found::Def(self.root));
            cur = self.step(cur, seg, seg_ns, guard);
            if cur.is_empty() && (!is_last || was_root) {
                // A module (file) we were not given: outside the program, not a typo.
                return vec![Found::External(segs[..=i + 1].to_vec())];
            }
        }
        cur
    }

    fn step(&self, cur: Vec<Found>, seg: &str, ns: Ns, guard: &mut Guard) -> Vec<Found> {
        let mut next = Vec::new();
        for f in cur {
            match f {
                Found::Def(d) => next.extend(self.member(d, seg, ns, guard, 0)),
                Found::External(mut p) => {
                    p.push(seg.to_string());
                    next.push(Found::External(p));
                }
            }
        }
        dedupe(next)
    }

    /// Named member of a module (scope lookup), enum (variant) or type alias (target's members).
    pub(crate) fn member(
        &self,
        def: DefId,
        name: &str,
        ns: Ns,
        guard: &mut Guard,
        depth: u8,
    ) -> Vec<Found> {
        let d = self.defs.get(def);
        match d.kind {
            DefKind::Module => self.lookup_in_module(def, name, ns, guard),
            DefKind::Enum if ns == Ns::Value => self.variant_named(def, name),
            DefKind::TypeAlias if depth < MAX_ALIAS_DEPTH => {
                let Some(target) = self.alias_target_def(def) else {
                    return Vec::new();
                };
                self.member(target, name, ns, guard, depth + 1)
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn variant_named(&self, enum_def: DefId, name: &str) -> Vec<Found> {
        self.members
            .get(&enum_def)
            .into_iter()
            .flatten()
            .filter(|v| self.defs.get(**v).name == name)
            .map(|v| Found::Def(*v))
            .collect()
    }

    fn alias_target_def(&self, alias: DefId) -> Option<DefId> {
        let d = self.defs.get(alias);
        let target = d.alias_target.as_ref()?;
        let segs: Vec<String> = target.split("::").map(str::to_string).collect();
        match self.resolve_type_path(d.module, &segs).as_slice() {
            [Found::Def(t)] => Some(*t),
            _ => None,
        }
    }

    /// Strip type aliases: `type A = B` -> `B`'s def.
    pub fn follow_alias(&self, mut def: DefId) -> Option<DefId> {
        for _ in 0..MAX_ALIAS_DEPTH {
            if self.defs.get(def).kind != DefKind::TypeAlias {
                return Some(def);
            }
            def = self.alias_target_def(def)?;
        }
        None
    }

    /// Phase 2: bind each `impl` to its self type / trait and index impls by type.
    pub fn link_impls(&mut self) {
        let links: Vec<(Option<DefId>, Option<DefId>)> = self
            .impls
            .iter()
            .map(|i| {
                let ty = self
                    .single_def(i.module, &i.type_name)
                    .and_then(|d| self.follow_alias(d));
                let tr = i
                    .trait_name
                    .as_deref()
                    .and_then(|t| self.single_def(i.module, t));
                (ty, tr)
            })
            .collect();
        for (idx, (ty, tr)) in links.into_iter().enumerate() {
            self.impls[idx].self_def = ty;
            self.impls[idx].trait_def = tr.filter(|d| self.defs.get(*d).kind == DefKind::Trait);
            if let Some(t) = ty {
                self.impls_of.entry(t).or_default().push(idx);
            }
        }
    }

    fn single_def(&self, module: DefId, type_name: &str) -> Option<DefId> {
        let segs: Vec<String> = base_type_name(type_name)
            .split("::")
            .map(str::to_string)
            .collect();
        match self.resolve_type_path(module, &segs).as_slice() {
            [Found::Def(d)] => Some(*d),
            _ => None,
        }
    }
}

impl<'a> Collected<'a> {
    /// Resolve a qualified path whose last segment is in the value namespace.
    pub fn lookup_path_value(&self, module: DefId, segs: &[String]) -> Vec<Found> {
        self.resolve_use_path(module, segs, Ns::Value, &mut Vec::new())
    }
}
