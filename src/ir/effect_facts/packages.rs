//! Grants per package (ADR-0021 section 5): the application's own grant and one grant per dependency.
//!
//! ```toml
//! [capabilities]                       # the application
//! fs.read = ["./data/**"]
//! [capabilities.dependencies.json5]    # a dependency; effects recorded in .wj-capabilities.lock are allowed too
//! net.connect = []
//! ```
//!
//! Enforcement is opt-in: it applies when the manifest has a `[capabilities]` table or a lock file exists.
//! A dependency with no table and no lock entry gets the empty grant (time, random and console only).

use super::grant::Grant;
use super::lock::CapLock;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PackageGrants {
    /// The application's grant; `None` = the manifest has no `[capabilities]` table (the app is not checked).
    pub app: Option<Grant>,
    pub deps: BTreeMap<String, Grant>,
    /// Applied to a dependency with neither a table nor a lock entry.
    pub default_dep: Grant,
    /// Dependencies are checked (a `[capabilities]` table or a lock file exists).
    pub enforce_deps: bool,
}

impl PackageGrants {
    pub fn from_manifest_text(text: &str, root: &Path, lock: Option<&CapLock>) -> Result<PackageGrants, String> {
        let doc: toml::Value = toml::from_str(text).map_err(|e| format!("wj.toml: {e}"))?;
        let caps = match doc.get("capabilities") {
            None => None,
            Some(toml::Value::Table(t)) => Some(t),
            Some(_) => return Err("wj.toml: [capabilities] must be a table".to_string()),
        };
        let app = super::manifest::grant_from_manifest_text(text, root)?;
        let mut deps: BTreeMap<String, Grant> = BTreeMap::new();
        if let Some(toml::Value::Table(dt)) = caps.and_then(|t| t.get("dependencies")) {
            for (name, v) in dt {
                let table = v.as_table().ok_or_else(|| format!("[capabilities.dependencies.{name}] must be a table"))?;
                deps.insert(name.replace('-', "_"), super::manifest::from_table(table, root)?);
            }
        }
        if let Some(l) = lock {
            for (name, p) in &l.packages {
                let g = deps.entry(name.clone()).or_insert_with(|| Grant::sandbox(root));
                for r in &p.effects {
                    g.allow_ref(r);
                }
            }
        }
        Ok(PackageGrants { app, deps, default_dep: Grant::sandbox(root), enforce_deps: caps.is_some() || lock.is_some() })
    }

    /// The grant that governs code of `package` (`""` = the application). `None` = not checked.
    pub fn grant_for(&self, package: &str) -> Option<&Grant> {
        if package.is_empty() {
            self.app.as_ref()
        } else if self.enforce_deps {
            Some(self.deps.get(package).unwrap_or(&self.default_dep))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::effect_facts::lock::PackageLock;
    use crate::ir::effect_facts::EffectKind;

    fn root() -> std::path::PathBuf {
        std::env::current_dir().unwrap()
    }

    #[test]
    fn app_and_dependency_grants_are_separate() {
        let t = "[capabilities]\nnet.connect = [\"api.example.com\"]\n[capabilities.dependencies.json5]\nfs.read = [\"./cfg/**\"]\n";
        let g = PackageGrants::from_manifest_text(t, &root(), None).unwrap();
        assert!(g.grant_for("").unwrap().check(&EffectKind::NetConnect, Some("http://api.example.com/")).is_ok());
        let d = g.grant_for("json5").unwrap();
        assert!(d.check(&EffectKind::FsRead, Some("cfg/a.json")).is_ok());
        assert!(d.check(&EffectKind::NetConnect, Some("http://api.example.com/")).is_err(), "the app's grant is not the dependency's");
        assert!(g.grant_for("unlisted").unwrap().check(&EffectKind::FsRead, Some("cfg/a.json")).is_err(), "default is empty");
    }

    #[test]
    fn the_lock_file_allows_what_it_records() {
        let mut lock = CapLock::new();
        lock.packages.insert("json5".into(), PackageLock { rev: None, effects: ["net.connect(api.example.com)".to_string(), "fs.read(*)".to_string()].into() });
        let g = PackageGrants::from_manifest_text("[package]\nname = \"p\"\n", &root(), Some(&lock)).unwrap();
        assert!(g.app.is_none(), "no table: the app is not checked");
        assert!(g.enforce_deps, "but a lock file turns on dependency checks");
        let d = g.grant_for("json5").unwrap();
        assert!(d.check(&EffectKind::NetConnect, Some("http://api.example.com/")).is_ok());
        assert!(d.check(&EffectKind::NetConnect, Some("http://evil.example/")).is_err());
        assert!(d.check(&EffectKind::FsRead, Some("/etc/hosts")).is_ok());
    }

    #[test]
    fn no_table_and_no_lock_checks_nothing() {
        let g = PackageGrants::from_manifest_text("[package]\nname = \"p\"\n", &root(), None).unwrap();
        assert!(g.grant_for("").is_none() && g.grant_for("json5").is_none());
    }
}
