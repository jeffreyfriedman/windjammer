//! The capability lock file, `.wj-capabilities.lock` (ADR-0021 section 5, WJ-SEC-03): what each dependency's
//! code is known to do. A dependency upgrade that adds an effect is an *escalation*: `wj check` reports it
//! until `wj caps approve` records it. References are strings: `fs.read`, `net.connect(api.example.com)`,
//! `fs.read(*)` (a computed resource).

use super::analysis::{EffectFacts, EffectRef, Resource};
use super::kind::EffectKind;
use crate::resolve::{DefKind, ResolvedProgram};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const LOCK_FILE: &str = ".wj-capabilities.lock";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapLock {
    pub schema_version: u32,
    pub windjammer_version: String,
    pub packages: BTreeMap<String, PackageLock>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageLock {
    /// Pinned revision or version text, when known (a change here with the same effects is not an escalation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub effects: BTreeSet<String>,
}

/// A package gained an effect since the lock was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Escalation {
    pub package: String,
    pub effect: String,
}

impl CapLock {
    pub fn new() -> CapLock {
        CapLock { schema_version: 2, windjammer_version: env!("CARGO_PKG_VERSION").to_string(), packages: BTreeMap::new() }
    }

    pub fn load(dir: &Path) -> Result<Option<CapLock>, String> {
        let p = dir.join(LOCK_FILE);
        let Ok(text) = std::fs::read_to_string(&p) else { return Ok(None) };
        serde_json::from_str(&text).map(Some).map_err(|e| format!("{}: {e}", p.display()))
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(LOCK_FILE), text + "\n").map_err(|e| format!("{LOCK_FILE}: {e}"))
    }

    /// Does the lock allow `reference` for `package`? Exact, or the kind with any resource (`kind(*)`).
    pub fn covers(&self, package: &str, reference: &str) -> bool {
        let Some(p) = self.packages.get(package) else { return false };
        if p.effects.contains(reference) {
            return true;
        }
        match reference.split_once('(') {
            Some((kind, _)) => p.effects.contains(&format!("{kind}(*)")),
            None => false,
        }
    }

    /// References in `current` the lock does not cover. A package missing from the lock escalates by everything it does.
    pub fn escalations(&self, current: &BTreeMap<String, BTreeSet<String>>) -> Vec<Escalation> {
        let mut out = Vec::new();
        for (package, refs) in current {
            for r in refs {
                if !self.covers(package, r) {
                    out.push(Escalation { package: package.clone(), effect: r.clone() });
                }
            }
        }
        out
    }
}

pub fn reference(e: &EffectRef) -> String {
    match &e.resource {
        Resource::None => e.kind.name(),
        // network resources are recorded by host: that is what a grant pattern matches
        Resource::Lit(s) if matches!(e.kind, EffectKind::NetConnect | EffectKind::NetListen) => {
            format!("{}({})", e.kind, super::scope_match::host_port(s).map(|(h, _)| h).unwrap_or_else(|| s.clone()))
        }
        Resource::Lit(s) => format!("{}({})", e.kind, s),
        Resource::Computed | Resource::MintedOptional => format!("{}(*)", e.kind),
    }
}

/// The package a function belongs to: the leading module segment when it names a dependency, else `""` (the app).
pub fn package_of_path(path: &str, dep_names: &[String]) -> String {
    let first = path.split("::").next().unwrap_or("");
    if dep_names.iter().any(|d| d == first) {
        first.to_string()
    } else {
        String::new()
    }
}

/// Effects each dependency package performs *itself* (direct sink calls in its own code, closures by where they are
/// written). Empty sets are kept so a dependency with no effects is recorded as such.
pub fn package_effects(r: &ResolvedProgram<'_>, facts: &EffectFacts, dep_names: &[String]) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = dep_names.iter().map(|d| (d.clone(), BTreeSet::new())).collect();
    for f in facts.fns.values() {
        if !matches!(r.defs().get(f.def).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }) {
            continue;
        }
        let pkg = package_of_path(&f.path, dep_names);
        if pkg.is_empty() {
            continue;
        }
        for (e, o) in &f.effects {
            if o.via.is_none() {
                out.entry(pkg.clone()).or_default().insert(reference(e));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock() -> CapLock {
        let mut l = CapLock::new();
        l.packages.insert("json5".into(), PackageLock { rev: None, effects: ["fs.read(*)".to_string(), "net.connect(api.example.com)".to_string()].into() });
        l
    }

    #[test]
    fn covers_exact_and_any_resource() {
        let l = lock();
        assert!(l.covers("json5", "fs.read(/etc/hosts)"));
        assert!(l.covers("json5", "net.connect(api.example.com)"));
        assert!(!l.covers("json5", "net.connect(evil.example)"));
        assert!(!l.covers("json5", "process(ls)"));
        assert!(!l.covers("other", "fs.read(x)"), "an unknown package covers nothing");
    }

    #[test]
    fn an_upgrade_that_adds_an_effect_is_an_escalation() {
        let l = lock();
        let now: BTreeMap<String, BTreeSet<String>> =
            [("json5".to_string(), ["fs.read(x)".to_string(), "process(sh)".to_string()].into())].into();
        assert_eq!(l.escalations(&now), vec![Escalation { package: "json5".into(), effect: "process(sh)".into() }]);
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("wj_lock_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let l = lock();
        l.save(&dir).unwrap();
        assert_eq!(CapLock::load(&dir).unwrap().unwrap(), l);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(CapLock::load(&dir).unwrap().is_none());
    }
}
