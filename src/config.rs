// Configuration file parsing for Windjammer projects (wj.toml and windjammer.toml)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Dependency specification (matches Cargo.toml format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string: "1.0.0"
    Simple(String),
    /// Detailed specification with features, path, git, etc.
    Detailed {
        version: Option<String>,
        features: Option<Vec<String>>,
        path: Option<String>,
        git: Option<String>,
        branch: Option<String>,
        /// Windjammer registry host (defaults to wj-registry.io).
        registry: Option<String>,
    },
}

/// Main Windjammer configuration (wj.toml or windjammer.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WjConfig {
    #[serde(default)]
    pub project: Option<ProjectConfig>,

    #[serde(default)]
    pub package: PackageConfig,

    #[serde(default)]
    pub sources: Option<SourcesConfig>,

    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,

    #[serde(default, alias = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, DependencySpec>,

    /// Backend configuration for WASM proxy (optional)
    #[serde(default)]
    pub backend: Option<BackendConfig>,

    /// Application capability declarations for the effect system (WJ-SEC-01).
    /// Declares which side-effects this application is allowed to perform.
    #[serde(default)]
    pub app_capabilities: Option<AppCapabilities>,
}

/// Project metadata (for windjammer.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
}

/// Package metadata (for wj.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub edition: String,
}

/// Source configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourcesConfig {
    /// Source roots - directories where compiler looks for modules
    /// e.g., ["windjammer-game-core/src", "lib/src"]
    #[serde(default)]
    pub roots: Vec<String>,
}

/// Backend proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub url: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Application capability declarations (WJ-SEC-01).
///
/// Declares which side-effects this application is allowed to perform.
/// The compiler verifies at build time that all code paths stay within
/// these declared capabilities.
///
/// Example `wj.toml`:
/// ```toml
/// [app_capabilities]
/// allow = ["fs_read", "fs_write", "net_egress"]
/// deny = ["process_spawn"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppCapabilities {
    /// Effects this application is allowed to perform.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Effects explicitly denied (takes precedence over allow).
    #[serde(default)]
    pub deny: Vec<String>,
}

impl AppCapabilities {
    /// Convert the declared capability strings to an `EffectSet`.
    pub fn to_effect_set(&self) -> crate::ir::safety_type::EffectSet {
        use crate::ir::safety_type::EffectSet;
        let mut set = EffectSet::pure();
        for cap in &self.allow {
            if let Some(effect) = parse_effect_name(cap) {
                set.insert(effect);
            }
        }
        set
    }

    /// Return the set of explicitly denied effects.
    pub fn denied_effects(&self) -> Vec<crate::ir::safety_type::Effect> {
        self.deny.iter().filter_map(|s| parse_effect_name(s)).collect()
    }
}

/// Parse a capability name string into an `Effect`.
fn parse_effect_name(name: &str) -> Option<crate::ir::safety_type::Effect> {
    use crate::ir::safety_type::Effect;
    match name {
        "fs_read" | "filesystem_read" => Some(Effect::FsRead),
        "fs_write" | "filesystem_write" => Some(Effect::FsWrite),
        "net_egress" | "network_egress" | "network" => Some(Effect::NetEgress),
        "net_ingress" | "network_ingress" => Some(Effect::NetIngress),
        "process_spawn" | "process" => Some(Effect::ProcessSpawn),
        "env_read" | "environment_read" => Some(Effect::EnvRead),
        "env_write" | "environment_write" => Some(Effect::EnvWrite),
        "ffi" => Some(Effect::Ffi),
        other => Some(Effect::Custom(other.to_string())),
    }
}

impl WjConfig {
    /// Load configuration from a file
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Save configuration to a file
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dependencies.insert(name, spec);
    }

    /// Remove a dependency
    pub fn remove_dependency(&mut self, name: &str) -> bool {
        self.dependencies.remove(name).is_some()
    }

    /// Convert this config into a package manifest for dependency resolution.
    pub fn to_package_manifest(&self) -> crate::package::PackageManifest {
        crate::package::PackageManifest::from_config(self)
    }

    /// Convert to Cargo.toml format
    pub fn to_cargo_toml(&self) -> String {
        let mut output = String::new();

        // Package section
        output.push_str("[package]\n");
        output.push_str(&format!("name = \"{}\"\n", self.package.name));
        output.push_str(&format!("version = \"{}\"\n", self.package.version));
        output.push_str(&format!(
            "edition = \"{}\"\n",
            if self.package.edition.is_empty() {
                "2021"
            } else {
                &self.package.edition
            }
        ));

        if !self.package.authors.is_empty() {
            output.push_str(&format!("authors = {:?}\n", self.package.authors));
        }

        output.push('\n');

        // Dependencies
        if !self.dependencies.is_empty() {
            output.push_str("[dependencies]\n");
            for (name, spec) in &self.dependencies {
                match spec {
                    DependencySpec::Simple(version) => {
                        output.push_str(&format!("{} = \"{}\"\n", name, version));
                    }
                    DependencySpec::Detailed {
                        version,
                        features,
                        path,
                        git,
                        branch,
                        registry: _,
                    } => {
                        output.push_str(&format!("{} = {{ ", name));
                        let mut parts = Vec::new();

                        if let Some(v) = version {
                            parts.push(format!("version = \"{}\"", v));
                        }
                        if let Some(f) = features {
                            parts.push(format!("features = {:?}", f));
                        }
                        if let Some(p) = path {
                            parts.push(format!("path = \"{}\"", p));
                        }
                        if let Some(g) = git {
                            parts.push(format!("git = \"{}\"", g));
                        }
                        if let Some(b) = branch {
                            parts.push(format!("branch = \"{}\"", b));
                        }

                        output.push_str(&parts.join(", "));
                        output.push_str(" }\n");
                    }
                }
            }
            output.push('\n');
        }

        // Dev dependencies
        if !self.dev_dependencies.is_empty() {
            output.push_str("[dev-dependencies]\n");
            for (name, spec) in &self.dev_dependencies {
                match spec {
                    DependencySpec::Simple(version) => {
                        output.push_str(&format!("{} = \"{}\"\n", name, version));
                    }
                    DependencySpec::Detailed {
                        version,
                        features,
                        path,
                        git,
                        branch,
                        registry: _,
                    } => {
                        output.push_str(&format!("{} = {{ ", name));
                        let mut parts = Vec::new();

                        if let Some(v) = version {
                            parts.push(format!("version = \"{}\"", v));
                        }
                        if let Some(f) = features {
                            parts.push(format!("features = {:?}", f));
                        }
                        if let Some(p) = path {
                            parts.push(format!("path = \"{}\"", p));
                        }
                        if let Some(g) = git {
                            parts.push(format!("git = \"{}\"", g));
                        }
                        if let Some(b) = branch {
                            parts.push(format!("branch = \"{}\"", b));
                        }

                        output.push_str(&parts.join(", "));
                        output.push_str(" }\n");
                    }
                }
            }
        }

        output
    }
}

/// Windjammer project configuration (windjammer.toml) - for runtime settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindjammerConfig {
    /// Backend configuration for WASM proxy
    #[serde(default)]
    pub backend: Option<BackendConfig>,

    /// Custom key-value pairs
    #[serde(flatten)]
    pub custom: HashMap<String, toml::Value>,
}

impl WindjammerConfig {
    /// Load from a file
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Get backend URL if configured
    pub fn backend_url(&self) -> Option<&str> {
        self.backend.as_ref().map(|b| b.url.as_str())
    }
}
