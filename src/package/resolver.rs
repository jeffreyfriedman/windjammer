//! Dependency resolution (WJ-PKG-01).
//!
//! Stub implementation of PubGrub-style dependency resolution. Full version
//! selection and conflict reporting will be added in a future phase.

use super::manifest::{Dependency, PackageManifest};
use std::collections::HashMap;

/// A fully resolved dependency with a concrete version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: String,
    pub registry: String,
    pub features: Vec<String>,
}

impl ResolvedDependency {
    pub fn from_dependency(dep: &Dependency) -> Self {
        Self {
            name: dep.name.clone(),
            version: dep.version.clone(),
            registry: dep.registry.clone(),
            features: dep.features.clone(),
        }
    }
}

/// Outcome of a dependency resolution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionResult {
    /// All dependencies resolved successfully.
    Success(Vec<ResolvedDependency>),
    /// Resolution failed with one or more errors.
    Failure(Vec<ResolutionError>),
}

/// Errors encountered during dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionError {
    /// A dependency could not be found in any registry.
    PackageNotFound { name: String, registry: String },
    /// No version satisfies the requested constraint.
    VersionConflict {
        name: String,
        requested: String,
        available: Vec<String>,
    },
    /// Circular dependency detected in the dependency graph.
    CircularDependency { chain: Vec<String> },
    /// Generic resolution failure with a human-readable message.
    Other { message: String },
}

impl std::fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionError::PackageNotFound { name, registry } => {
                write!(f, "package '{name}' not found in registry '{registry}'")
            }
            ResolutionError::VersionConflict {
                name,
                requested,
                available,
            } => write!(
                f,
                "no version of '{name}' satisfies '{requested}' (available: {})",
                available.join(", ")
            ),
            ResolutionError::CircularDependency { chain } => {
                write!(f, "circular dependency: {}", chain.join(" -> "))
            }
            ResolutionError::Other { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ResolutionError {}

/// PubGrub-style dependency resolver (stub).
#[derive(Debug, Default)]
pub struct Resolver {
    /// Optional pinned versions for stub resolution (name -> version).
    pinned_versions: HashMap<String, String>,
}

impl Resolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pin a package to a specific version for stub resolution.
    pub fn pin(&mut self, name: impl Into<String>, version: impl Into<String>) {
        self.pinned_versions.insert(name.into(), version.into());
    }

    /// Resolve all dependencies declared in the manifest.
    ///
    /// Stub behavior: accepts declared versions as-is without contacting a
    /// registry. Path and git dependencies pass through unchanged.
    pub fn resolve(&self, manifest: &PackageManifest) -> Result<Vec<ResolvedDependency>, Vec<ResolutionError>> {
        let mut resolved = Vec::new();
        let mut errors = Vec::new();
        let mut seen: HashMap<String, String> = HashMap::new();

        for dep in manifest.dependencies.iter().chain(manifest.dev_dependencies.iter()) {
            if let Some(first) = seen.get(&dep.name) {
                if first != &dep.version {
                    errors.push(ResolutionError::VersionConflict {
                        name: dep.name.clone(),
                        requested: dep.version.clone(),
                        available: vec![first.clone()],
                    });
                    continue;
                }
            } else {
                seen.insert(dep.name.clone(), dep.version.clone());
            }

            if dep.version.is_empty() {
                errors.push(ResolutionError::Other {
                    message: format!("dependency '{}' has no version constraint", dep.name),
                });
                continue;
            }

            let version = self
                .pinned_versions
                .get(&dep.name)
                .cloned()
                .unwrap_or_else(|| dep.version.clone());

            resolved.push(ResolvedDependency {
                name: dep.name.clone(),
                version,
                registry: dep.registry.clone(),
                features: dep.features.clone(),
            });
        }

        if errors.is_empty() {
            Ok(resolved)
        } else {
            Err(errors)
        }
    }

    /// Resolve and wrap the result in a `ResolutionResult`.
    pub fn resolve_with_result(&self, manifest: &PackageManifest) -> ResolutionResult {
        match self.resolve(manifest) {
            Ok(deps) => ResolutionResult::Success(deps),
            Err(errs) => ResolutionResult::Failure(errs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::Dependency;

    #[test]
    fn resolve_simple_manifest() {
        let manifest = PackageManifest {
            name: "app".to_string(),
            version: "1.0.0".to_string(),
            dependencies: vec![Dependency::new("serde", "1.0")],
            dev_dependencies: vec![],
        };

        let resolver = Resolver::new();
        let resolved = resolver.resolve(&manifest).expect("should resolve");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "serde");
        assert_eq!(resolved[0].version, "1.0");
    }

    #[test]
    fn detect_version_conflict() {
        let manifest = PackageManifest {
            name: "app".to_string(),
            version: "1.0.0".to_string(),
            dependencies: vec![Dependency::new("serde", "1.0")],
            dev_dependencies: vec![Dependency::new("serde", "2.0")],
        };

        let resolver = Resolver::new();
        let err = resolver.resolve(&manifest).expect_err("should conflict");
        assert_eq!(err.len(), 1);
        assert!(matches!(err[0], ResolutionError::VersionConflict { .. }));
    }
}
