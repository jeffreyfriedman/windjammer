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
    },
}

/// Main Windjammer configuration (wj.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WjConfig {
    #[serde(default)]
    pub package: PackageConfig,

    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,

    #[serde(default)]
    pub dev_dependencies: HashMap<String, DependencySpec>,

    /// Backend configuration for WASM proxy (optional)
    #[serde(default)]
    pub backend: Option<BackendConfig>,
}

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub edition: String,
}

/// Backend proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub url: String,
    #[serde(default)]
    pub api_key: Option<String>,
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
