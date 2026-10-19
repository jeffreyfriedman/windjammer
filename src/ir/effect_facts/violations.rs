//! Effects against a grant and against declarations: the findings behind WJ0040, WJ0041, WJ0042, WJ0044.

use super::analysis::{EffectFacts, EffectRef, FnEffects, Resource};
use super::grant::Denial;
use super::lock::package_of_path;
use super::packages::PackageGrants;
use super::kind::EffectKind;
use crate::resolve::{DefKind, ResolvedProgram};
use crate::source_map::Location;

#[derive(Debug, Clone)]
pub enum Finding {
    /// WJ0040 / WJ0041: the function performs an effect its package's grant does not allow.
    NotGranted { package: String, function: String, effect: EffectRef, denial: Denial, entry: Vec<String>, sink: String, site: Option<Location> },
    /// WJ0042: the function performs an effect its `@uses` / `@pure` does not declare.
    Undeclared { function: String, effect: EffectKind, chain: Vec<(String, Option<Location>, String)>, site: Option<Location>, def_site: Option<Location> },
    /// WJ0044: a call to an `extern` fn that declares no effects.
    UndeclaredExtern { function: String, callee: String, site: Option<Location> },
}

fn has_body(r: &ResolvedProgram<'_>, f: &FnEffects) -> bool {
    matches!(r.defs().get(f.def).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }) && r.collected.decls.contains_key(&f.def)
}

/// Check every function against the grant of the package it belongs to and against its own declaration.
/// Only the *direct* origin of an effect is reported per function chain root, so one bad sink call is
/// reported once, at the function that contains it, not once per transitive caller.
pub fn check(r: &ResolvedProgram<'_>, facts: &EffectFacts, grants: &PackageGrants, dep_names: &[String]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in facts.fns.values().filter(|f| has_body(r, f)) {
        for (e, o) in &f.effects {
            if o.via.is_some() {
                continue; // reported where it is introduced
            }
            let package = package_of_path(&f.path, dep_names);
            if let Some(g) = grants.grant_for(&package) {
                let res = match &e.resource {
                    Resource::Lit(s) => Some(s.as_str()),
                    _ => None,
                };
                // a computed resource cannot be decided statically: it gets a runtime guard (only an effect that is not granted at all is an error)
                let verdict = match &e.resource {
                    // a capability value for an effect the package may not have simply carries no such ability
                    Resource::MintedOptional => Ok(()),
                    Resource::Computed => g.granted_at_all(&e.kind),
                    _ => g.check(&e.kind, res),
                };
                if let Err(denial) = verdict {
                    if matches!(e.kind, EffectKind::Ffi) {
                        out.push(Finding::UndeclaredExtern { function: f.path.clone(), callee: o.callee.clone(), site: o.site.clone() });
                    } else {
                        out.push(Finding::NotGranted { package: package.clone(), function: f.path.clone(), effect: e.clone(), denial, entry: facts.entry_path(f.def), sink: o.callee.clone(), site: o.site.clone() });
                    }
                }
            }
        }
        if let Some(declared) = &f.declared {
            for (e, o) in &f.effects {
                if !declared.contains(&e.kind) {
                    out.push(Finding::Undeclared {
                        function: f.path.clone(),
                        effect: e.kind.clone(),
                        chain: facts.chain(f.def, e),
                        site: o.site.clone(),
                        def_site: f.location.clone(),
                    });
                }
            }
        }
    }
    out
}
