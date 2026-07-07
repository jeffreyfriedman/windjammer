//! Package manifest parsing (WJ-PKG-01).
//!
//! Converts `wj.toml` dependency sections into structured `PackageManifest`
//! values suitable for the dependency resolver.

use crate::config::{DependencySpec, WjConfig};
use std::collections::HashMap;
use std::path::Path;

/// Default registry URL for Windjammer packages.
pub const DEFAULT_REGISTRY: &str = "wj-registry.io";

/// A single package dependency declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    /// Registry host (defaults to `wj-registry.io`).
    pub registry: String,
    pub features: Vec<String>,
}

impl Dependency {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            registry: DEFAULT_REGISTRY.to_string(),
            features: Vec::new(),
        }
    }

    pub fn with_registry(mut self, registry: impl Into<String>) -> Self {
        self.registry = registry.into();
        self
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
}

/// Parsed package manifest with runtime and dev dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
}

impl PackageManifest {
    /// Build a manifest from an already-loaded `WjConfig`.
    pub fn from_config(config: &WjConfig) -> Self {
        let name = if !config.package.name.is_empty() {
            config.package.name.clone()
        } else {
            config
                .project
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_default()
        };

        let version = if !config.package.version.is_empty() {
            config.package.version.clone()
        } else {
            config
                .project
                .as_ref()
                .map(|p| p.version.clone())
                .unwrap_or_default()
        };

        Self {
            name,
            version,
            dependencies: parse_dependency_map(&config.dependencies),
            dev_dependencies: parse_dependency_map(&config.dev_dependencies),
        }
    }

    /// Load and parse a manifest from a `wj.toml` file path.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let config = WjConfig::load_from_file(path)?;
        Ok(Self::from_config(&config))
    }

    /// All dependencies (runtime + dev) by name.
    pub fn all_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .chain(self.dev_dependencies.iter())
    }
}

fn parse_dependency_map(map: &HashMap<String, DependencySpec>) -> Vec<Dependency> {
    let mut deps: Vec<Dependency> = map
        .iter()
        .map(|(name, spec)| dependency_from_spec(name, spec))
        .collect();
    deps.sort_by(|a, b| a.name.cmp(&b.name));
    deps
}

fn dependency_from_spec(name: &str, spec: &DependencySpec) -> Dependency {
    match spec {
        DependencySpec::Simple(version) => Dependency::new(name, version),
        DependencySpec::Detailed {
            version,
            features,
            path,
            git,
            branch,
            registry,
            ..
        } => {
            let version_str = resolve_version_string(version, path, git, branch);
            let mut dep = Dependency::new(name, version_str);
            if let Some(f) = features {
                dep.features = f.clone();
            }
            if let Some(r) = registry {
                dep.registry = r.clone();
            }
            dep
        }
    }
}

/// Resolve a version string from the various Cargo-style dependency forms.
fn resolve_version_string(
    version: &Option<String>,
    path: &Option<String>,
    git: &Option<String>,
    branch: &Option<String>,
) -> String {
    if let Some(v) = version {
        return v.clone();
    }
    if let Some(p) = path {
        return format!("path:{p}");
    }
    if let Some(g) = git {
        if let Some(b) = branch {
            return format!("git:{g}#{b}");
        }
        return format!("git:{g}");
    }
    "*".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_dependency() {
        let mut config = WjConfig::default();
        config.package.name = "demo".to_string();
        config.package.version = "0.1.0".to_string();
        config.add_dependency("serde".to_string(), DependencySpec::Simple("1.0".to_string()));

        let manifest = PackageManifest::from_config(&config);
        assert_eq!(manifest.name, "demo");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].name, "serde");
        assert_eq!(manifest.dependencies[0].version, "1.0");
        assert_eq!(manifest.dependencies[0].registry, DEFAULT_REGISTRY);
    }

    #[test]
    fn parse_detailed_dependency_with_features() {
        let mut config = WjConfig::default();
        config.add_dependency(
            "http".to_string(),
            DependencySpec::Detailed {
                version: Some("2.0".to_string()),
                features: Some(vec!["json".to_string()]),
                path: None,
                git: None,
                branch: None,
                registry: None,
            },
        );

        let manifest = PackageManifest::from_config(&config);
        assert_eq!(manifest.dependencies[0].features, vec!["json"]);
    }

    #[test]
    fn parse_dev_dependencies() {
        let mut config = WjConfig::default();
        config.dev_dependencies.insert(
            "test-utils".to_string(),
            DependencySpec::Simple("0.1".to_string()),
        );

        let manifest = PackageManifest::from_config(&config);
        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert_eq!(manifest.dev_dependencies[0].name, "test-utils");
    }
}
