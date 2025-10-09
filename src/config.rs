// Configuration module for wj.toml parsing and management
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Windjammer project configuration (wj.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WjConfig {
    pub package: PackageConfig,
    #[serde(default)]
    pub lib: Option<LibConfig>,
    #[serde(default)]
    pub dependencies: HashMap<String, DependencySpec>,
    #[serde(rename = "dev-dependencies")]
    #[serde(default)]
    pub dev_dependencies: HashMap<String, DependencySpec>,
    #[serde(default)]
    pub profile: HashMap<String, ProfileConfig>,
    #[serde(default)]
    pub target: HashMap<String, TargetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    #[serde(default = "default_edition")]
    pub edition: String,
}

fn default_edition() -> String {
    "2025".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibConfig {
    // Future: library-specific configuration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed {
        version: Option<String>,
        features: Option<Vec<String>>,
        path: Option<String>,
        git: Option<String>,
        branch: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(rename = "opt-level")]
    pub opt_level: Option<OptLevel>,
    pub lto: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptLevel {
    Int(u8),
    Str(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    pub enabled: Option<bool>,
    // Future: target-specific configuration
}

impl WjConfig {
    /// Load wj.toml from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read wj.toml: {}", e))?;

        Self::parse(&content)
    }

    /// Parse wj.toml from a string
    pub fn parse(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse wj.toml: {}", e))
    }

    /// Save wj.toml to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize wj.toml: {}", e))?;

        fs::write(path.as_ref(), content).map_err(|e| format!("Failed to write wj.toml: {}", e))
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dependencies.insert(name, spec);
    }

    /// Remove a dependency
    pub fn remove_dependency(&mut self, name: &str) -> bool {
        self.dependencies.remove(name).is_some()
    }

    /// Convert to Cargo.toml content
    pub fn to_cargo_toml(&self) -> String {
        let mut cargo_toml = String::new();

        // [package] section
        cargo_toml.push_str("[package]\n");
        cargo_toml.push_str(&format!("name = \"{}\"\n", self.package.name));
        cargo_toml.push_str(&format!("version = \"{}\"\n", self.package.version));
        cargo_toml.push_str("edition = \"2021\"  # Rust edition\n");
        cargo_toml.push_str("\n");

        // [lib] section if this is a library
        if self.lib.is_some() {
            cargo_toml.push_str("[lib]\n");
            cargo_toml.push_str("crate-type = [\"lib\"]\n");
            cargo_toml.push_str("\n");
        }

        // [dependencies] section
        if !self.dependencies.is_empty() {
            cargo_toml.push_str("[dependencies]\n");
            for (name, spec) in &self.dependencies {
                match spec {
                    DependencySpec::Simple(version) => {
                        cargo_toml.push_str(&format!("{} = \"{}\"\n", name, version));
                    }
                    DependencySpec::Detailed {
                        version,
                        features,
                        path,
                        git,
                        branch,
                    } => {
                        cargo_toml.push_str(&format!("{} = {{ ", name));
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

                        cargo_toml.push_str(&parts.join(", "));
                        cargo_toml.push_str(" }\n");
                    }
                }
            }
            cargo_toml.push_str("\n");
        }

        // [dev-dependencies] section
        if !self.dev_dependencies.is_empty() {
            cargo_toml.push_str("[dev-dependencies]\n");
            for (name, spec) in &self.dev_dependencies {
                match spec {
                    DependencySpec::Simple(version) => {
                        cargo_toml.push_str(&format!("{} = \"{}\"\n", name, version));
                    }
                    DependencySpec::Detailed {
                        version,
                        features,
                        path,
                        git,
                        branch,
                    } => {
                        cargo_toml.push_str(&format!("{} = {{ ", name));
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

                        cargo_toml.push_str(&parts.join(", "));
                        cargo_toml.push_str(" }\n");
                    }
                }
            }
            cargo_toml.push_str("\n");
        }

        // [profile.*] sections
        for (profile_name, profile) in &self.profile {
            cargo_toml.push_str(&format!("[profile.{}]\n", profile_name));

            if let Some(opt_level) = &profile.opt_level {
                match opt_level {
                    OptLevel::Int(level) => {
                        cargo_toml.push_str(&format!("opt-level = {}\n", level));
                    }
                    OptLevel::Str(level) => {
                        cargo_toml.push_str(&format!("opt-level = \"{}\"\n", level));
                    }
                }
            }

            if let Some(lto) = profile.lto {
                cargo_toml.push_str(&format!("lto = {}\n", lto));
            }

            cargo_toml.push_str("\n");
        }

        cargo_toml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_wj_toml() {
        let toml = r#"
[package]
name = "my-app"
version = "0.1.0"
edition = "2025"

[dependencies]
reqwest = "0.11"
"#;

        let config = WjConfig::parse(toml).unwrap();
        assert_eq!(config.package.name, "my-app");
        assert_eq!(config.package.version, "0.1.0");
        assert_eq!(config.dependencies.len(), 1);
    }

    #[test]
    fn test_parse_detailed_dependencies() {
        let toml = r#"
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
"#;

        let config = WjConfig::parse(toml).unwrap();
        assert!(config.dependencies.contains_key("serde"));
    }

    #[test]
    fn test_to_cargo_toml() {
        let mut config = WjConfig {
            package: PackageConfig {
                name: "test-app".to_string(),
                version: "0.1.0".to_string(),
                edition: "2025".to_string(),
            },
            lib: None,
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            profile: HashMap::new(),
            target: HashMap::new(),
        };

        config.add_dependency(
            "reqwest".to_string(),
            DependencySpec::Simple("0.11".to_string()),
        );

        let cargo_toml = config.to_cargo_toml();
        assert!(cargo_toml.contains("name = \"test-app\""));
        assert!(cargo_toml.contains("reqwest = \"0.11\""));
    }
}
