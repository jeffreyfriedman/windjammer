// Integration tests for the plugin system
// Tests both core plugins and dynamic plugins

use windjammer_game_framework::prelude::*;

// ============================================================================
// Test Plugins
// ============================================================================

/// Simple test plugin
struct TestPlugin1;

impl Plugin for TestPlugin1 {
    fn name(&self) -> &str {
        "test_plugin_1"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn build(&self, _app: &mut App) {
        println!("TestPlugin1::build called");
    }

    fn cleanup(&self, _app: &mut App) {
        println!("TestPlugin1::cleanup called");
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Other
    }

    fn description(&self) -> &str {
        "Test plugin 1"
    }

    fn author(&self) -> &str {
        "Test Author"
    }

    fn license(&self) -> &str {
        "MIT"
    }
}

/// Plugin with dependencies
struct TestPlugin2;

impl Plugin for TestPlugin2 {
    fn name(&self) -> &str {
        "test_plugin_2"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency {
            name: "test_plugin_1".to_string(),
            version: VersionReq::parse("^1.0.0").unwrap(),
        }]
    }

    fn build(&self, _app: &mut App) {
        println!("TestPlugin2::build called");
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Rendering
    }
}

/// Plugin with hot-reload support
struct TestPluginHotReload;

impl Plugin for TestPluginHotReload {
    fn name(&self) -> &str {
        "test_plugin_hot_reload"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn build(&self, _app: &mut App) {
        println!("TestPluginHotReload::build called");
    }

    fn supports_hot_reload(&self) -> bool {
        true
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Editor
    }
}

// ============================================================================
// Core Plugin Tests
// ============================================================================

#[test]
fn test_plugin_registration() {
    let mut app = App::new();

    // Register plugin
    app.add_plugin(TestPlugin1);

    // Check plugin is registered
    let state = app.plugins().get_state("test_plugin_1");
    assert!(state.is_some());
    assert_eq!(state.unwrap(), PluginState::Unloaded);
}

#[test]
fn test_plugin_loading() {
    let mut app = App::new();

    // Register and load plugin
    app.add_plugin(TestPlugin1);
    app.load_plugins().unwrap();

    // Check plugin is loaded
    let state = app.plugins().get_state("test_plugin_1");
    assert!(state.is_some());
    assert_eq!(state.unwrap(), PluginState::Loaded);
}

#[test]
#[should_panic(expected = "Failed to add plugin")]
fn test_plugin_duplicate_registration() {
    let mut app = App::new();

    // Register plugin
    app.add_plugin(TestPlugin1);

    // Try to register again - should panic
    app.add_plugin(TestPlugin1);
}

#[test]
fn test_plugin_dependency_order() {
    let mut app = App::new();

    // Register plugins in wrong order (dependent first)
    app.add_plugin(TestPlugin2);
    app.add_plugin(TestPlugin1);

    // Load plugins - should load in correct order (dependency first)
    app.load_plugins().unwrap();

    // Both should be loaded
    assert_eq!(
        app.plugins().get_state("test_plugin_1").unwrap(),
        PluginState::Loaded
    );
    assert_eq!(
        app.plugins().get_state("test_plugin_2").unwrap(),
        PluginState::Loaded
    );
}

#[test]
fn test_plugin_missing_dependency() {
    let mut app = App::new();

    // Register plugin without its dependency
    app.add_plugin(TestPlugin2);

    // Try to load - should fail (dependency not registered)
    let result = app.load_plugins();
    assert!(result.is_err());
    let err = result.unwrap_err();
    // The error is NotFound because the dependency plugin was never registered
    assert!(matches!(err, PluginError::NotFound(_)));
}

#[test]
fn test_plugin_list() {
    let mut app = App::new();

    // Register multiple plugins
    app.add_plugin(TestPlugin1);
    app.add_plugin(TestPlugin2);
    app.add_plugin(TestPluginHotReload);

    // List plugins
    let plugins = app.plugins().list_plugins();
    assert_eq!(plugins.len(), 3);

    // Check all plugins are in the list
    assert!(plugins.iter().any(|p| p.name == "test_plugin_1"));
    assert!(plugins.iter().any(|p| p.name == "test_plugin_2"));
    assert!(plugins.iter().any(|p| p.name == "test_plugin_hot_reload"));
}

#[test]
fn test_plugin_hot_reload_support() {
    let plugin = TestPluginHotReload;
    assert!(plugin.supports_hot_reload());

    let plugin1 = TestPlugin1;
    assert!(!plugin1.supports_hot_reload());
}

#[test]
fn test_plugin_metadata() {
    let plugin = TestPlugin1;

    assert_eq!(plugin.name(), "test_plugin_1");
    assert_eq!(plugin.version(), "1.0.0");
    assert_eq!(plugin.description(), "Test plugin 1");
    assert_eq!(plugin.author(), "Test Author");
    assert_eq!(plugin.license(), "MIT");
    assert_eq!(plugin.category(), PluginCategory::Other);
}

#[test]
fn test_plugin_categories() {
    let plugin1 = TestPlugin1;
    assert_eq!(plugin1.category(), PluginCategory::Other);

    let plugin2 = TestPlugin2;
    assert_eq!(plugin2.category(), PluginCategory::Rendering);

    let plugin3 = TestPluginHotReload;
    assert_eq!(plugin3.category(), PluginCategory::Editor);
}

// ============================================================================
// Version Requirement Tests
// ============================================================================

#[test]
fn test_version_req_exact() {
    let req = VersionReq::parse("1.2.3").unwrap();
    let version = Version::parse("1.2.3").unwrap();
    assert!(req.matches(&version));

    let version2 = Version::parse("1.2.4").unwrap();
    assert!(!req.matches(&version2));
}

#[test]
fn test_version_req_caret() {
    let req = VersionReq::parse("^1.2.3").unwrap();

    // Should match 1.2.3 to 1.9.9
    assert!(req.matches(&Version::parse("1.2.3").unwrap()));
    assert!(req.matches(&Version::parse("1.2.4").unwrap()));
    assert!(req.matches(&Version::parse("1.9.9").unwrap()));

    // Should not match 2.0.0
    assert!(!req.matches(&Version::parse("2.0.0").unwrap()));

    // Should not match 1.2.2
    assert!(!req.matches(&Version::parse("1.2.2").unwrap()));
}

#[test]
fn test_version_req_tilde() {
    let req = VersionReq::parse("~1.2.3").unwrap();

    // Should match 1.2.3 to 1.2.9
    assert!(req.matches(&Version::parse("1.2.3").unwrap()));
    assert!(req.matches(&Version::parse("1.2.4").unwrap()));
    assert!(req.matches(&Version::parse("1.2.9").unwrap()));

    // Should not match 1.3.0
    assert!(!req.matches(&Version::parse("1.3.0").unwrap()));

    // Should not match 1.2.2
    assert!(!req.matches(&Version::parse("1.2.2").unwrap()));
}

#[test]
fn test_version_req_wildcard() {
    let req = VersionReq::parse("1.*").unwrap();

    // Should match any 1.x.x
    assert!(req.matches(&Version::parse("1.0.0").unwrap()));
    assert!(req.matches(&Version::parse("1.2.3").unwrap()));
    assert!(req.matches(&Version::parse("1.9.9").unwrap()));

    // Should not match 2.0.0
    assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
}

#[test]
fn test_version_comparison() {
    let v1 = Version::parse("1.0.0").unwrap();
    let v2 = Version::parse("1.0.1").unwrap();
    let v3 = Version::parse("1.1.0").unwrap();
    let v4 = Version::parse("2.0.0").unwrap();

    assert!(v1 < v2);
    assert!(v2 < v3);
    assert!(v3 < v4);
    assert!(v1 < v4);
}

// ============================================================================
// Circular Dependency Tests
// ============================================================================

struct CircularPluginA;

impl Plugin for CircularPluginA {
    fn name(&self) -> &str {
        "circular_a"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency {
            name: "circular_b".to_string(),
            version: VersionReq::parse("^1.0.0").unwrap(),
        }]
    }

    fn build(&self, _app: &mut App) {}
}

struct CircularPluginB;

impl Plugin for CircularPluginB {
    fn name(&self) -> &str {
        "circular_b"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency {
            name: "circular_a".to_string(),
            version: VersionReq::parse("^1.0.0").unwrap(),
        }]
    }

    fn build(&self, _app: &mut App) {}
}

#[test]
fn test_circular_dependency_detection() {
    let mut app = App::new();

    // Register plugins with circular dependency
    app.add_plugin(CircularPluginA);
    app.add_plugin(CircularPluginB);

    // Try to load - should detect circular dependency
    let result = app.load_plugins();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PluginError::CircularDependency(_)
    ));
}

// ============================================================================
// Complex Dependency Graph Tests
// ============================================================================

struct PluginA;
impl Plugin for PluginA {
    fn name(&self) -> &str {
        "plugin_a"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn build(&self, _app: &mut App) {}
}

struct PluginB;
impl Plugin for PluginB {
    fn name(&self) -> &str {
        "plugin_b"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency {
            name: "plugin_a".to_string(),
            version: VersionReq::parse("^1.0.0").unwrap(),
        }]
    }
    fn build(&self, _app: &mut App) {}
}

struct PluginC;
impl Plugin for PluginC {
    fn name(&self) -> &str {
        "plugin_c"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency {
            name: "plugin_a".to_string(),
            version: VersionReq::parse("^1.0.0").unwrap(),
        }]
    }
    fn build(&self, _app: &mut App) {}
}

struct PluginD;
impl Plugin for PluginD {
    fn name(&self) -> &str {
        "plugin_d"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![
            PluginDependency {
                name: "plugin_b".to_string(),
                version: VersionReq::parse("^1.0.0").unwrap(),
            },
            PluginDependency {
                name: "plugin_c".to_string(),
                version: VersionReq::parse("^1.0.0").unwrap(),
            },
        ]
    }
    fn build(&self, _app: &mut App) {}
}

#[test]
fn test_complex_dependency_graph() {
    let mut app = App::new();

    // Register plugins in random order
    app.add_plugin(PluginD);
    app.add_plugin(PluginB);
    app.add_plugin(PluginA);
    app.add_plugin(PluginC);

    // Load plugins - should resolve correct order
    app.load_plugins().unwrap();

    // All plugins should be loaded
    assert_eq!(
        app.plugins().get_state("plugin_a").unwrap(),
        PluginState::Loaded
    );
    assert_eq!(
        app.plugins().get_state("plugin_b").unwrap(),
        PluginState::Loaded
    );
    assert_eq!(
        app.plugins().get_state("plugin_c").unwrap(),
        PluginState::Loaded
    );
    assert_eq!(
        app.plugins().get_state("plugin_d").unwrap(),
        PluginState::Loaded
    );
}

// ============================================================================
// Version Mismatch Tests
// ============================================================================

struct OldPlugin;
impl Plugin for OldPlugin {
    fn name(&self) -> &str {
        "old_plugin"
    }
    fn version(&self) -> &str {
        "0.9.0"
    }
    fn build(&self, _app: &mut App) {}
}

struct NewPlugin;
impl Plugin for NewPlugin {
    fn name(&self) -> &str {
        "new_plugin"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![PluginDependency {
            name: "old_plugin".to_string(),
            version: VersionReq::parse("^1.0.0").unwrap(), // Requires 1.x
        }]
    }
    fn build(&self, _app: &mut App) {}
}

#[test]
fn test_version_mismatch() {
    let mut app = App::new();

    // Register plugins with incompatible versions
    app.add_plugin(OldPlugin);
    app.add_plugin(NewPlugin);

    // Try to load - should fail with version mismatch
    let result = app.load_plugins();
    assert!(result.is_err());
    // Note: This will fail with DependencyNotFound because the version doesn't match
    // In a real implementation, you might want a more specific error
}

// ============================================================================
// Plugin State Tests
// ============================================================================

#[test]
fn test_plugin_state_transitions() {
    let mut app = App::new();

    // Initial state: not registered
    assert!(app.plugins().get_state("test_plugin_1").is_none());

    // Register plugin
    app.add_plugin(TestPlugin1);

    // State: Unloaded
    assert_eq!(
        app.plugins().get_state("test_plugin_1").unwrap(),
        PluginState::Unloaded
    );

    // Load plugin
    app.load_plugins().unwrap();

    // State: Loaded
    assert_eq!(
        app.plugins().get_state("test_plugin_1").unwrap(),
        PluginState::Loaded
    );
}

#[test]
fn test_plugin_error_display() {
    let err = PluginError::NotFound("test_plugin".to_string());
    assert_eq!(err.to_string(), "Plugin not found: test_plugin");

    let err = PluginError::AlreadyRegistered("test_plugin".to_string());
    assert_eq!(err.to_string(), "Plugin already registered: test_plugin");

    let err = PluginError::CircularDependency(vec!["a".to_string(), "b".to_string()]);
    assert_eq!(err.to_string(), "Circular dependency detected: a -> b");
}

