//! What a compiled program needs from the capability model (ADR-0021 section 8, Rust backend).
//!
//! * **Proven**: every sink call has a literal resource inside the grant, or a resource-less effect (time, random,
//!   console) the grant allows: the compiler has already decided, the program carries no check.
//! * **Runtime guard**: a computed path, host or name (or a capability value minted from a scoped grant) can only be
//!   decided at run time: the program installs the grant once at the start of `main`
//!   (`windjammer_runtime::caps::install`) and the runtime's std wrappers check against it.

use super::analysis::{EffectFacts, Resource};
use super::grant::Grant;
use super::kind::EffectKind;
use super::lock::package_of_path;
use super::packages::PackageGrants;
use crate::resolve::{DefKind, ResolvedProgram};
use std::path::Path;

fn resourceful(k: &EffectKind) -> bool {
    matches!(
        k,
        EffectKind::FsRead | EffectKind::FsWrite | EffectKind::NetConnect | EffectKind::NetListen | EffectKind::EnvRead | EffectKind::EnvWrite | EffectKind::Process
    )
}

fn is_any(g: &Grant, k: &EffectKind) -> bool {
    matches!(g.scope(k), Some(super::grant::Scope::Any))
}

/// True when some sink call in the application cannot be proven inside the grant at compile time.
pub fn needs_runtime_guards(r: &ResolvedProgram<'_>, facts: &EffectFacts, grants: &PackageGrants, dep_names: &[String]) -> bool {
    let Some(app) = grants.app.as_ref() else { return false };
    facts.fns.values().any(|f| {
        let real = matches!(r.defs().get(f.def).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }) && r.collected.decls.contains_key(&f.def);
        real && f.effects.iter().any(|(e, o)| {
            o.via.is_none() && matches!(e.resource, Resource::Computed) && resourceful(&e.kind) && !is_any(grants.grant_for(&package_of_path(&f.path, dep_names)).unwrap_or(app), &e.kind)
        })
    })
}

/// Dependencies whose computed-resource uses a compiled program cannot hold to their own narrower grant: the runtime
/// holds the whole program to ONE grant, so a dependency's scope for that effect must equal the application's (or be
/// unrestricted). Returns `(package, effect)`.
pub fn compiled_dependency_gaps(r: &ResolvedProgram<'_>, facts: &EffectFacts, grants: &PackageGrants, dep_names: &[String]) -> Vec<(String, EffectKind)> {
    let Some(app) = grants.app.as_ref() else { return Vec::new() };
    let mut out: Vec<(String, EffectKind)> = Vec::new();
    for f in facts.fns.values() {
        let pkg = package_of_path(&f.path, dep_names);
        if pkg.is_empty() || !matches!(r.defs().get(f.def).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }) {
            continue;
        }
        let Some(dep) = grants.grant_for(&pkg) else { continue };
        for (e, o) in &f.effects {
            if o.via.is_some() || !matches!(e.resource, Resource::Computed) || !resourceful(&e.kind) {
                continue;
            }
            let same = is_any(dep, &e.kind) || dep.scope(&e.kind) == app.scope(&e.kind);
            if !same && !out.contains(&(pkg.clone(), e.kind.clone())) {
                out.push((pkg.clone(), e.kind.clone()));
            }
        }
    }
    out
}

/// The Rust statement that installs `grant` in a compiled program (`root` anchors relative patterns).
pub fn render_install(grant: &Grant, root: &Path) -> String {
    let q = |s: &str| format!("{s:?}");
    let allow: Vec<String> = grant
        .entries()
        .iter()
        .map(|(k, ps)| format!("({}, &[{}])", q(k), ps.iter().map(|p| q(p)).collect::<Vec<_>>().join(", ")))
        .collect();
    let deny: Vec<String> = grant.denied().iter().map(|d| q(d)).collect();
    format!(
        "windjammer_runtime::caps::install(&windjammer_runtime::caps::Spec {{ root: {}, allow: &[{}], deny: &[{}] }})",
        q(&root.display().to_string()),
        allow.join(", "),
        deny.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_statement_carries_the_grant() {
        let mut g = Grant::sandbox(Path::new("/proj"));
        g.allow(EffectKind::FsRead, "./data/**");
        g.allow_all(EffectKind::NetConnect);
        g.deny(EffectKind::Console);
        let s = render_install(&g, Path::new("/proj"));
        assert!(s.contains(r#"root: "/proj""#), "{s}");
        assert!(s.contains(r#"("fs.read", &["./data/**"])"#), "{s}");
        assert!(s.contains(r#"("net.connect", &["*"])"#), "{s}");
        assert!(s.contains(r#"deny: &["console"]"#), "{s}");
    }
}
