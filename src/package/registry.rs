//! Registry client interface (WJ-PKG-01).
//!
//! Stub implementation — no actual HTTP calls. Future phases will implement
//! fetching metadata and downloading packages from `wj-registry.io`.

use super::manifest::DEFAULT_REGISTRY;
use std::collections::HashMap;
use std::path::PathBuf;

/// Metadata for a package available in a registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageMetadata {
    pub name: String,
    pub latest_version: String,
    pub versions: Vec<String>,
    pub description: Option<String>,
}

/// Client for interacting with a Windjammer package registry.
#[derive(Debug, Clone)]
pub struct RegistryClient {
    pub registry_url: String,
    /// Stub cache of known packages (for testing and offline development).
    stub_packages: HashMap<String, PackageMetadata>,
}

impl RegistryClient {
    pub fn new(registry_url: impl Into<String>) -> Self {
        Self {
            registry_url: registry_url.into(),
            stub_packages: HashMap::new(),
        }
    }

    pub fn default_registry() -> Self {
        Self::new(DEFAULT_REGISTRY)
    }

    /// Register stub metadata for a package (testing / offline use).
    pub fn register_stub(&mut self, metadata: PackageMetadata) {
        self.stub_packages.insert(metadata.name.clone(), metadata);
    }

    /// Fetch metadata for a package from the registry.
    ///
    /// Stub: returns cached stub data or a synthetic entry for unknown packages.
    pub fn fetch_package_metadata(
        &self,
        name: &str,
    ) -> Result<PackageMetadata, RegistryError> {
        if let Some(meta) = self.stub_packages.get(name) {
            return Ok(meta.clone());
        }

        // Stub fallback: synthesize metadata so resolver tests can proceed.
        Ok(PackageMetadata {
            name: name.to_string(),
            latest_version: "0.0.0".to_string(),
            versions: vec!["0.0.0".to_string()],
            description: None,
        })
    }

    /// Download a specific version of a package.
    ///
    /// Stub: returns an empty directory path without performing any I/O.
    pub fn download_package(
        &self,
        name: &str,
        version: &str,
    ) -> Result<PathBuf, RegistryError> {
        if name.is_empty() {
            return Err(RegistryError::InvalidPackageName);
        }
        if version.is_empty() {
            return Err(RegistryError::InvalidVersion {
                name: name.to_string(),
                version: version.to_string(),
            });
        }

        Ok(PathBuf::from(format!(
            "/tmp/wj-packages/{}/{}/{}",
            self.registry_url, name, version
        )))
    }
}

/// Errors from registry operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    PackageNotFound { name: String },
    InvalidPackageName,
    InvalidVersion { name: String, version: String },
    NetworkUnavailable { registry: String },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::PackageNotFound { name } => {
                write!(f, "package '{name}' not found in registry")
            }
            RegistryError::InvalidPackageName => write!(f, "invalid package name"),
            RegistryError::InvalidVersion { name, version } => {
                write!(f, "invalid version '{version}' for package '{name}'")
            }
            RegistryError::NetworkUnavailable { registry } => {
                write!(f, "registry '{registry}' is unavailable (stub mode)")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_stub_metadata() {
        let mut client = RegistryClient::default_registry();
        client.register_stub(PackageMetadata {
            name: "serde".to_string(),
            latest_version: "1.0.0".to_string(),
            versions: vec!["1.0.0".to_string(), "0.9.0".to_string()],
            description: Some("serialization".to_string()),
        });

        let meta = client.fetch_package_metadata("serde").unwrap();
        assert_eq!(meta.latest_version, "1.0.0");
    }

    #[test]
    fn download_package_stub_path() {
        let client = RegistryClient::default_registry();
        let path = client.download_package("serde", "1.0.0").unwrap();
        assert!(path.to_string_lossy().contains("serde"));
        assert!(path.to_string_lossy().contains("1.0.0"));
    }
}
