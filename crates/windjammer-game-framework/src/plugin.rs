//! Plugin System
//!
//! Windjammer's plugin system enables extensibility through three types of plugins:
//! 1. **Core Plugins** (Rust, compile-time, zero-cost)
//! 2. **Dynamic Plugins** (C FFI, runtime, hot-reload)
//! 3. **Script Plugins** (Windjammer, easy to write)
//!
//! ## Example
//!
//! ```rust
//! use windjammer_game_framework::{Plugin, App};
//!
//! pub struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn name(&self) -> &str {
//!         "my_plugin"
//!     }
//!
//!     fn version(&self) -> &str {
//!         "1.0.0"
//!     }
//!
//!     fn build(&self, app: &mut App) {
//!         // Add systems, resources, etc.
//!     }
//! }
//!
//! // Register plugin
//! let mut app = App::new();
//! app.add_plugin(MyPlugin);
//! ```

use std::collections::HashMap;
use std::fmt;

/// Plugin trait
///
/// All plugins must implement this trait to be registered with the plugin manager.
pub trait Plugin: Send + Sync {
    /// Plugin name (unique identifier)
    ///
    /// This should be a unique identifier for the plugin, typically in snake_case.
    /// Example: "advanced_physics", "custom_renderer"
    fn name(&self) -> &str;

    /// Plugin version (semantic versioning)
    ///
    /// Should follow semver format: "MAJOR.MINOR.PATCH"
    /// Example: "1.0.0", "2.1.3"
    fn version(&self) -> &str;

    /// Plugin dependencies (other plugins required)
    ///
    /// List other plugins that must be loaded before this plugin.
    /// The plugin manager will ensure dependencies are loaded in the correct order.
    fn dependencies(&self) -> Vec<PluginDependency> {
        Vec::new()
    }

    /// Build plugin (called once at startup)
    ///
    /// This is where you register systems, resources, and other plugin functionality.
    fn build(&self, app: &mut App);

    /// Cleanup plugin (called at shutdown)
    ///
    /// Optional cleanup when the plugin is unloaded.
    fn cleanup(&self, _app: &mut App) {}

    /// Hot-reload support (optional)
    ///
    /// Return true if this plugin supports hot-reloading.
    fn supports_hot_reload(&self) -> bool {
        false
    }

    /// Plugin category (for organization)
    fn category(&self) -> PluginCategory {
        PluginCategory::Other
    }

    /// Plugin description
    fn description(&self) -> &str {
        ""
    }

    /// Plugin author
    fn author(&self) -> &str {
        ""
    }

    /// Plugin license
    fn license(&self) -> &str {
        ""
    }
}

/// Plugin dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDependency {
    /// Plugin name
    pub name: String,
    /// Version requirement (semver)
    pub version: VersionReq,
}

impl PluginDependency {
    /// Create a new plugin dependency
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Result<Self, String> {
        Ok(Self {
            name: name.into(),
            version: VersionReq::parse(&version.into())?,
        })
    }
}

/// Version requirement (simplified semver)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    /// Minimum version (inclusive)
    pub min: Version,
    /// Maximum version (exclusive)
    pub max: Option<Version>,
}

impl VersionReq {
    /// Parse a version requirement string
    ///
    /// Supports:
    /// - Exact: "1.0.0"
    /// - Caret: "^1.0.0" (>=1.0.0, <2.0.0)
    /// - Tilde: "~1.0.0" (>=1.0.0, <1.1.0)
    /// - Wildcard: "1.*" (>=1.0.0, <2.0.0)
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();

        // Caret requirement: ^1.0.0 means >=1.0.0, <2.0.0
        if let Some(version_str) = s.strip_prefix('^') {
            let min = Version::parse(version_str)?;
            let max = Version {
                major: min.major + 1,
                minor: 0,
                patch: 0,
            };
            return Ok(Self {
                min,
                max: Some(max),
            });
        }

        // Tilde requirement: ~1.0.0 means >=1.0.0, <1.1.0
        if let Some(version_str) = s.strip_prefix('~') {
            let min = Version::parse(version_str)?;
            let max = Version {
                major: min.major,
                minor: min.minor + 1,
                patch: 0,
            };
            return Ok(Self {
                min,
                max: Some(max),
            });
        }

        // Wildcard: 1.* means >=1.0.0, <2.0.0
        if s.contains('*') {
            let parts: Vec<&str> = s.split('.').collect();
            if parts.len() != 3 {
                return Err(format!("Invalid wildcard version: {}", s));
            }

            let major = parts[0]
                .parse::<u32>()
                .map_err(|_| format!("Invalid major version: {}", parts[0]))?;

            let min = Version {
                major,
                minor: 0,
                patch: 0,
            };
            let max = Version {
                major: major + 1,
                minor: 0,
                patch: 0,
            };
            return Ok(Self {
                min,
                max: Some(max),
            });
        }

        // Exact version
        let min = Version::parse(s)?;
        Ok(Self { min, max: None })
    }

    /// Check if a version satisfies this requirement
    pub fn matches(&self, version: &Version) -> bool {
        if version < &self.min {
            return false;
        }

        if let Some(ref max) = self.max {
            if version >= max {
                return false;
            }
        }

        true
    }
}

/// Semantic version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parse a version string
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Plugin category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCategory {
    /// Rendering plugins (shaders, pipelines, materials)
    Rendering,
    /// Physics plugins (engines, collision, controllers)
    Physics,
    /// Audio plugins (effects, spatial audio, music)
    Audio,
    /// AI plugins (behavior trees, pathfinding, ML)
    AI,
    /// Editor plugins (tools, inspectors, importers)
    Editor,
    /// Asset plugins (importers, processors, optimizers)
    Assets,
    /// Networking plugins (netcode, matchmaking, voice)
    Networking,
    /// Platform plugins (console, mobile, VR/AR)
    Platform,
    /// Other plugins
    Other,
}

impl fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginCategory::Rendering => write!(f, "Rendering"),
            PluginCategory::Physics => write!(f, "Physics"),
            PluginCategory::Audio => write!(f, "Audio"),
            PluginCategory::AI => write!(f, "AI"),
            PluginCategory::Editor => write!(f, "Editor"),
            PluginCategory::Assets => write!(f, "Assets"),
            PluginCategory::Networking => write!(f, "Networking"),
            PluginCategory::Platform => write!(f, "Platform"),
            PluginCategory::Other => write!(f, "Other"),
        }
    }
}

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is not loaded
    Unloaded,
    /// Plugin is loaded and active
    Loaded,
    /// Plugin failed to load
    Failed,
}

/// Plugin manager
///
/// Manages plugin registration, loading, and lifecycle.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_states: HashMap<String, PluginState>,
    dependency_graph: DependencyGraph,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            plugin_states: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
        }
    }

    /// Register a plugin
    pub fn add_plugin<P: Plugin + 'static>(&mut self, plugin: P) -> Result<(), PluginError> {
        let name = plugin.name().to_string();

        // Check if plugin already registered
        if self.plugin_states.contains_key(&name) {
            return Err(PluginError::AlreadyRegistered(name));
        }

        // Check dependencies exist (will be validated during load)
        for dep in plugin.dependencies() {
            if !self.plugin_states.contains_key(&dep.name) {
                // Dependency will be checked during load
            }
        }

        // Add to dependency graph
        self.dependency_graph
            .add_node(&name, plugin.dependencies());

        // Store plugin
        self.plugins.push(Box::new(plugin));
        self.plugin_states.insert(name, PluginState::Unloaded);

        Ok(())
    }

    /// Load all plugins in dependency order
    pub fn load_all(&mut self, app: &mut App) -> Result<(), PluginError> {
        // Topological sort of dependency graph
        let load_order = self.dependency_graph.topological_sort()?;

        for plugin_name in load_order {
            self.load_plugin(&plugin_name, app)?;
        }

        Ok(())
    }

    /// Load a specific plugin
    fn load_plugin(&mut self, name: &str, app: &mut App) -> Result<(), PluginError> {
        // Find plugin
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // Check dependencies are loaded
        for dep in plugin.dependencies() {
            let dep_state = self
                .plugin_states
                .get(&dep.name)
                .ok_or_else(|| PluginError::DependencyNotFound(dep.name.clone()))?;

            if *dep_state != PluginState::Loaded {
                return Err(PluginError::DependencyNotLoaded(dep.name.clone()));
            }

            // Check version compatibility
            let dep_plugin = self
                .plugins
                .iter()
                .find(|p| p.name() == dep.name)
                .ok_or_else(|| PluginError::DependencyNotFound(dep.name.clone()))?;

            let dep_version = Version::parse(dep_plugin.version())
                .map_err(|e| PluginError::InvalidVersion(dep.name.clone(), e))?;

            if !dep.version.matches(&dep_version) {
                return Err(PluginError::VersionMismatch {
                    plugin: name.to_string(),
                    dependency: dep.name.clone(),
                    required: format!("{:?}", dep.version),
                    found: dep_version.to_string(),
                });
            }
        }

        // Build plugin
        plugin.build(app);

        // Mark as loaded
        self.plugin_states
            .insert(name.to_string(), PluginState::Loaded);

        Ok(())
    }

    /// Unload a plugin
    pub fn unload(&mut self, name: &str, app: &mut App) -> Result<(), PluginError> {
        // Check if plugin is loaded
        let state = self
            .plugin_states
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        if *state != PluginState::Loaded {
            return Err(PluginError::NotLoaded(name.to_string()));
        }

        // Check if other plugins depend on this
        if self.dependency_graph.has_dependents(name) {
            return Err(PluginError::HasDependents(name.to_string()));
        }

        // Find plugin
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // Cleanup plugin
        plugin.cleanup(app);

        // Mark as unloaded
        self.plugin_states
            .insert(name.to_string(), PluginState::Unloaded);

        Ok(())
    }

    /// Get plugin state
    pub fn get_state(&self, name: &str) -> Option<PluginState> {
        self.plugin_states.get(name).copied()
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|p| PluginInfo {
                name: p.name().to_string(),
                version: p.version().to_string(),
                category: p.category(),
                description: p.description().to_string(),
                author: p.author().to_string(),
                state: *self.plugin_states.get(p.name()).unwrap_or(&PluginState::Unloaded),
            })
            .collect()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin information
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub category: PluginCategory,
    pub description: String,
    pub author: String,
    pub state: PluginState,
}

/// Plugin error
#[derive(Debug, Clone)]
pub enum PluginError {
    /// Plugin not found
    NotFound(String),
    /// Plugin already registered
    AlreadyRegistered(String),
    /// Plugin not loaded
    NotLoaded(String),
    /// Dependency not found
    DependencyNotFound(String),
    /// Dependency not loaded
    DependencyNotLoaded(String),
    /// Plugin has dependents (cannot unload)
    HasDependents(String),
    /// Circular dependency detected
    CircularDependency(Vec<String>),
    /// Version mismatch
    VersionMismatch {
        plugin: String,
        dependency: String,
        required: String,
        found: String,
    },
    /// Invalid version string
    InvalidVersion(String, String),
    /// Failed to load plugin
    LoadFailed(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::NotFound(name) => write!(f, "Plugin not found: {}", name),
            PluginError::AlreadyRegistered(name) => {
                write!(f, "Plugin already registered: {}", name)
            }
            PluginError::NotLoaded(name) => write!(f, "Plugin not loaded: {}", name),
            PluginError::DependencyNotFound(name) => {
                write!(f, "Dependency not found: {}", name)
            }
            PluginError::DependencyNotLoaded(name) => {
                write!(f, "Dependency not loaded: {}", name)
            }
            PluginError::HasDependents(name) => {
                write!(f, "Plugin has dependents and cannot be unloaded: {}", name)
            }
            PluginError::CircularDependency(cycle) => {
                write!(f, "Circular dependency detected: {}", cycle.join(" -> "))
            }
            PluginError::VersionMismatch {
                plugin,
                dependency,
                required,
                found,
            } => write!(
                f,
                "Version mismatch: {} requires {} {} but found {}",
                plugin, dependency, required, found
            ),
            PluginError::InvalidVersion(plugin, error) => {
                write!(f, "Invalid version for {}: {}", plugin, error)
            }
            PluginError::LoadFailed(msg) => write!(f, "Failed to load plugin: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

/// Dependency graph for topological sorting
struct DependencyGraph {
    nodes: HashMap<String, Vec<PluginDependency>>,
}

impl DependencyGraph {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    fn add_node(&mut self, name: &str, dependencies: Vec<PluginDependency>) {
        self.nodes.insert(name.to_string(), dependencies);
    }

    fn has_dependents(&self, name: &str) -> bool {
        self.nodes
            .values()
            .any(|deps| deps.iter().any(|dep| dep.name == name))
    }

    fn topological_sort(&self) -> Result<Vec<String>, PluginError> {
        let mut result = Vec::new();
        let mut visited = HashMap::new();
        let mut stack = Vec::new();

        for node in self.nodes.keys() {
            if !visited.contains_key(node) {
                self.visit(node, &mut visited, &mut stack, &mut result)?;
            }
        }

        Ok(result)
    }

    fn visit(
        &self,
        node: &str,
        visited: &mut HashMap<String, bool>,
        stack: &mut Vec<String>,
        result: &mut Vec<String>,
    ) -> Result<(), PluginError> {
        if let Some(&in_stack) = visited.get(node) {
            if in_stack {
                // Circular dependency detected
                let cycle_start = stack.iter().position(|n| n == node).unwrap();
                let cycle: Vec<String> = stack[cycle_start..].to_vec();
                return Err(PluginError::CircularDependency(cycle));
            }
            return Ok(());
        }

        visited.insert(node.to_string(), true);
        stack.push(node.to_string());

        if let Some(dependencies) = self.nodes.get(node) {
            for dep in dependencies {
                self.visit(&dep.name, visited, stack, result)?;
            }
        }

        stack.pop();
        visited.insert(node.to_string(), false);
        result.push(node.to_string());

        Ok(())
    }
}

/// Application context for plugins
///
/// This is a simplified version - the real App will have more functionality.
pub struct App {
    // Placeholder for now - will be expanded with actual ECS, resources, etc.
    pub(crate) plugin_manager: PluginManager,
}

impl App {
    /// Create a new application
    pub fn new() -> Self {
        Self {
            plugin_manager: PluginManager::new(),
        }
    }

    /// Add a plugin
    pub fn add_plugin<P: Plugin + 'static>(&mut self, plugin: P) -> &mut Self {
        self.plugin_manager
            .add_plugin(plugin)
            .expect("Failed to add plugin");
        self
    }

    /// Load all plugins
    pub fn load_plugins(&mut self) -> Result<(), PluginError> {
        // We need to temporarily take ownership of the plugin manager to avoid borrow checker issues
        let mut plugin_manager = std::mem::replace(&mut self.plugin_manager, PluginManager::new());
        let result = plugin_manager.load_all(self);
        self.plugin_manager = plugin_manager;
        result
    }

    /// Get plugin manager
    pub fn plugins(&self) -> &PluginManager {
        &self.plugin_manager
    }

    /// Get plugin manager (mutable)
    pub fn plugins_mut(&mut self) -> &mut PluginManager {
        &mut self.plugin_manager
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        name: String,
        dependencies: Vec<PluginDependency>,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn dependencies(&self) -> Vec<PluginDependency> {
            self.dependencies.clone()
        }

        fn build(&self, _app: &mut App) {}
    }

    #[test]
    fn test_version_parsing() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_requirement_caret() {
        let req = VersionReq::parse("^1.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_version_requirement_tilde() {
        let req = VersionReq::parse("~1.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.0.5").unwrap()));
        assert!(!req.matches(&Version::parse("1.1.0").unwrap()));
    }

    #[test]
    fn test_plugin_registration() {
        let mut app = App::new();
        let plugin = TestPlugin {
            name: "test".to_string(),
            dependencies: Vec::new(),
        };

        app.add_plugin(plugin);
        assert_eq!(app.plugins().get_state("test"), Some(PluginState::Unloaded));
    }

    #[test]
    fn test_plugin_loading() {
        let mut app = App::new();
        let plugin = TestPlugin {
            name: "test".to_string(),
            dependencies: Vec::new(),
        };

        app.add_plugin(plugin);
        app.load_plugins().unwrap();
        assert_eq!(app.plugins().get_state("test"), Some(PluginState::Loaded));
    }

    #[test]
    fn test_dependency_order() {
        let mut app = App::new();

        // Plugin B depends on Plugin A
        let plugin_a = TestPlugin {
            name: "plugin_a".to_string(),
            dependencies: Vec::new(),
        };
        let plugin_b = TestPlugin {
            name: "plugin_b".to_string(),
            dependencies: vec![PluginDependency::new("plugin_a", "^1.0.0").unwrap()],
        };

        app.add_plugin(plugin_b); // Add B first
        app.add_plugin(plugin_a); // Add A second

        app.load_plugins().unwrap();

        // Both should be loaded
        assert_eq!(
            app.plugins().get_state("plugin_a"),
            Some(PluginState::Loaded)
        );
        assert_eq!(
            app.plugins().get_state("plugin_b"),
            Some(PluginState::Loaded)
        );
    }

    #[test]
    fn test_circular_dependency() {
        let mut app = App::new();

        // Plugin A depends on Plugin B
        let plugin_a = TestPlugin {
            name: "plugin_a".to_string(),
            dependencies: vec![PluginDependency::new("plugin_b", "^1.0.0").unwrap()],
        };
        // Plugin B depends on Plugin A (circular!)
        let plugin_b = TestPlugin {
            name: "plugin_b".to_string(),
            dependencies: vec![PluginDependency::new("plugin_a", "^1.0.0").unwrap()],
        };

        app.add_plugin(plugin_a);
        app.add_plugin(plugin_b);

        let result = app.load_plugins();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginError::CircularDependency(_)
        ));
    }
}

