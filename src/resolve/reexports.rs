//! ADR-0040: a module re-exports a name only through an explicit `pub use`.
//!
//! Resolution still follows the lenient rule the generated Rust implements (`pub use self::child::*` for every
//! child). This pass resolves every `use` and every resolved path call a second time with the implicit rule switched
//! off (`Collected::explicit_only`) and reports each step the two resolutions disagree on: the module that would have
//! to re-export the name, the child that declares it, and the name. Decided by declaration (the def's own module and
//! that module's parent), never by name matching.

use super::collect::Collected;
use super::def::{DefId, DefKind};
use super::lookup::{Found, Ns};
use super::res::{CallKind, Res};
use super::uses::Binding;
use super::ResolvedProgram;
use crate::source_map::Location;

type Gap = (DefId, DefId, DefId, String);

/// One use that depends on the implicit child re-export rule.
#[derive(Debug, Clone)]
pub struct ImplicitReexport {
    /// The module that has to re-export the name (the parent of the declaring child).
    pub reexporter: DefId,
    /// The child module that declares the item.
    pub child: DefId,
    /// The declared item.
    pub def: DefId,
    pub name: String,
    /// The path as written at the use site.
    pub written: String,
    pub location: Option<Location>,
    /// The use site is inside `reexporter` itself (a plain `use` is enough there, not a `pub use`).
    pub inside_reexporter: bool,
}

impl ImplicitReexport {
    /// The line that makes the name explicit, relative to the re-exporting module: `pub use http::serve`.
    pub fn line(&self, r: &ResolvedProgram<'_>) -> String {
        let child = &r.defs().get(self.child).name;
        let kw = if self.inside_reexporter { "use" } else { "pub use" };
        format!("{kw} {child}::{}", self.name)
    }
    /// `serve` is not re-exported by `adapters`; it is declared in `adapters::http`.
    pub fn message(&self, r: &ResolvedProgram<'_>) -> String {
        self.message_in(r, &[])
    }
    /// `message` with module paths named relative to the crate the use lives in: `crate_root` is the module path of
    /// the crate's entry file (empty for a crate rooted at the program's root), stripped from every path printed, so
    /// a diagnostic never carries the checked file's location.
    pub fn message_in(&self, r: &ResolvedProgram<'_>, crate_root: &[String]) -> String {
        let d = r.defs();
        let m = relative_path(&d.get(self.reexporter).path, crate_root);
        let m = if m.is_empty() { "the crate root".to_string() } else { m };
        format!("`{}` is not re-exported by `{m}`; it is declared in `{}`", self.name, relative_path(&d.get(self.child).path, crate_root))
    }
}

/// `path` (`a::b::c`) without the leading module path `crate_root` (`["a", "b"]`): `c`.
pub fn relative_path(path: &str, crate_root: &[String]) -> String {
    if crate_root.is_empty() {
        return path.to_string();
    }
    let prefix = crate_root.join("::");
    if path == prefix {
        return String::new();
    }
    path.strip_prefix(&format!("{prefix}::")).unwrap_or(path).to_string()
}

impl Collected<'_> {
    /// `lookup_in_module` with the implicit rule off.
    fn lookup_explicit(&self, module: DefId, name: &str, ns: Ns) -> Vec<Found> {
        let prev = self.explicit_only.replace(true);
        let found = self.lookup_in_module(module, name, ns, &mut Vec::new());
        self.explicit_only.set(prev);
        found
    }

    /// Look `name` up in `module` leniently; every item only the implicit rule found is a gap. The flag says the
    /// explicit rule finds the name too.
    fn lookup_noting(&self, module: DefId, name: &str, ns: Ns, gaps: &mut Vec<Gap>) -> (Vec<Found>, bool) {
        let lenient = self.lookup_in_module(module, name, ns, &mut Vec::new());
        if lenient.is_empty() {
            return (lenient, false);
        }
        if !self.lookup_explicit(module, name, ns).is_empty() {
            return (lenient, true);
        }
        for f in &lenient {
            let Found::Def(d) = f else { continue };
            let def = self.defs.get(*d);
            if def.kind == DefKind::Module || def.module == module {
                continue;
            }
            let child = def.module;
            let Some(parent) = self.parent_of(child) else { continue };
            let g = (parent, child, *d, def.name.clone());
            if !gaps.contains(&g) {
                gaps.push(g);
            }
        }
        (lenient, false)
    }

    /// Walk `segs` from `module` like `resolve_use_path`, collecting the steps that need the implicit rule.
    /// Returns the gaps and whether the last segment resolved by the explicit rule (a path that is a module in one
    /// namespace and an item in the other, `use crate::inflate` with `inflate::inflate`, is explicit as the module).
    fn implicit_steps(&self, module: DefId, segs: &[String], ns: Ns) -> (Vec<Gap>, bool) {
        let mut gaps = Vec::new();
        let Some((first, rest)) = segs.split_first() else { return (gaps, false) };
        let first_ns = if rest.is_empty() { ns } else { Ns::Type };
        let (mut cur, mut explicit) = self.lookup_noting(module, first, first_ns, &mut gaps);
        if cur.is_empty() && module != self.root {
            (cur, explicit) = self.lookup_noting(self.root, first, first_ns, &mut gaps);
        }
        for (i, seg) in rest.iter().enumerate() {
            explicit = false;
            let seg_ns = if i + 1 == rest.len() { ns } else { Ns::Type };
            let mut next = Vec::new();
            for f in cur {
                if let Found::Def(d) = f {
                    if self.defs.get(d).kind == DefKind::Module {
                        let (found, ex) = self.lookup_noting(d, seg, seg_ns, &mut gaps);
                        explicit |= ex;
                        next.extend(found);
                    } else {
                        next.extend(self.member(d, seg, seg_ns, &mut Vec::new(), 0));
                        explicit = true;
                    }
                }
            }
            cur = next;
            if cur.is_empty() {
                break;
            }
        }
        (gaps, explicit)
    }
}

fn is_inside(c: &Collected<'_>, module: DefId, anc: DefId) -> bool {
    module == anc && c.modules.contains_key(&module)
}

/// Every `use` and resolved path call that relies on a module re-exporting a child's item without a `pub use`.
pub fn implicit_reexports(r: &ResolvedProgram<'_>) -> Vec<ImplicitReexport> {
    let c = &r.collected;
    let mut out: Vec<ImplicitReexport> = Vec::new();
    let mut push = |gaps: Vec<(DefId, DefId, DefId, String)>, module: DefId, written: String, at: Option<&Location>| {
        for (reexporter, child, def, name) in gaps {
            let dup = out.iter().any(|o| o.reexporter == reexporter && o.def == def && o.written == written && o.location.as_ref().map(|l| (&l.file, l.line)) == at.map(|l| (&l.file, l.line)));
            if !dup {
                out.push(ImplicitReexport { reexporter, child, def, name, written: written.clone(), location: at.cloned(), inside_reexporter: is_inside(c, module, reexporter) });
            }
        }
    };
    let mut scopes: Vec<_> = c.modules.iter().collect();
    scopes.sort_by_key(|(d, _)| **d);
    for (module, scope) in scopes {
        for u in &scope.uses {
            let (mut gaps, mut explicit) = c.implicit_steps(*module, &u.segments, Ns::Type);
            if u.binding != Binding::Glob {
                let (more, ex) = c.implicit_steps(*module, &u.segments, Ns::Value);
                explicit |= ex;
                for g in more {
                    if !gaps.contains(&g) {
                        gaps.push(g);
                    }
                }
            }
            // `use a::name` binds every namespace `name` has: explicit in one of them is explicit
            if explicit {
                gaps.retain(|g| u.segments.last() != Some(&g.3));
            }
            push(gaps, *module, u.segments.join("::"), u.location.as_ref());
        }
    }
    for call in &r.calls {
        if !matches!(call.kind, CallKind::Path | CallKind::Free) || !matches!(call.res, Res::Def(_)) {
            continue;
        }
        let segs: Vec<String> = call.callee.split("::").map(str::to_string).collect();
        push(c.implicit_steps(call.module, &segs, Ns::Value).0, call.module, call.callee.clone(), call.location.as_ref());
    }
    out
}
