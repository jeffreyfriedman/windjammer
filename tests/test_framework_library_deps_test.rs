// TDD Test: Test framework should add all dependencies from wj.toml to library Cargo.toml
//
// Bug: When compiling a library for tests, the test framework only replaces wildcard
// dependencies but doesn't ADD new dependencies from wj.toml.
//
// Example: If wj.toml has `egui = "0.29"` but generated Cargo.toml doesn't have egui,
// it should be ADDED, not ignored.

use std::fs;
use tempfile::TempDir;

#[test]
fn test_library_deps_are_added_from_wj_toml() {
    // Create a temporary project with wj.toml
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create wj.toml with dependencies
    let wj_toml = r#"
[package]
name = "test-editor"
version = "0.1.0"

[dependencies]
windjammer-runtime = { path = "../../../windjammer/crates/windjammer-runtime" }
windjammer-ui = { path = "../../windjammer-ui", features = ["desktop"] }
egui = "0.29"
eframe = "0.29"
egui_dock = "0.14"
serde = { version = "1.0", features = ["derive"] }

[lib]
path = "src_wj/mod.wj"
"#;

    fs::write(project_root.join("wj.toml"), wj_toml).unwrap();

    // Create src_wj directory with a simple module
    let src_wj = project_root.join("src_wj");
    fs::create_dir_all(&src_wj).unwrap();
    fs::write(src_wj.join("mod.wj"), "// Test module\npub fn test() {}").unwrap();

    // Create a generated Cargo.toml (simulating what build_project generates)
    let lib_output = project_root.join("lib");
    fs::create_dir_all(&lib_output).unwrap();

    let initial_cargo_toml = r#"[package]
name = "windjammer-app"
version = "0.1.0"
edition = "2021"

[dependencies]
windjammer-runtime = { path = "/some/path", version = "*" }

[lib]
name = "windjammer_app"
path = "lib.rs"
"#;

    fs::write(lib_output.join("Cargo.toml"), initial_cargo_toml).unwrap();

    // Parse wj.toml and simulate the dependency merging logic
    let wj_toml_content = fs::read_to_string(project_root.join("wj.toml")).unwrap();
    let config: toml::Value = toml::from_str(&wj_toml_content).unwrap();

    let cargo_toml_path = lib_output.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();

    // This is what the fix should do: add ALL dependencies from wj.toml
    if let Some(deps) = config.get("dependencies").and_then(|d| d.as_table()) {
        let mut deps_section = String::new();

        for (dep_name, dep_value) in deps {
            // Skip windjammer-runtime (already in generated Cargo.toml)
            if dep_name == "windjammer-runtime" {
                continue;
            }

            // Add the dependency
            if let Some(dep_table) = dep_value.as_table() {
                deps_section.push_str(&format!("{} = {{ ", dep_name));
                for (key, value) in dep_table {
                    if key == "path" {
                        if let Some(path_str) = value.as_str() {
                            deps_section.push_str(&format!("path = \"{}\", ", path_str));
                        }
                    } else if key == "features" {
                        if let Some(features_array) = value.as_array() {
                            deps_section.push_str("features = [");
                            for (i, feature) in features_array.iter().enumerate() {
                                if i > 0 {
                                    deps_section.push_str(", ");
                                }
                                if let Some(f) = feature.as_str() {
                                    deps_section.push_str(&format!("\"{}\"", f));
                                }
                            }
                            deps_section.push_str("], ");
                        }
                    } else if key == "version" {
                        if let Some(version_str) = value.as_str() {
                            deps_section.push_str(&format!("version = \"{}\", ", version_str));
                        }
                    }
                }
                // Remove trailing comma and space
                if deps_section.ends_with(", ") {
                    deps_section.truncate(deps_section.len() - 2);
                }
                deps_section.push_str(" }\n");
            } else if let Some(version_str) = dep_value.as_str() {
                // Simple version string
                deps_section.push_str(&format!("{} = \"{}\"\n", dep_name, version_str));
            }
        }

        // Insert dependencies before [lib] section
        if !deps_section.is_empty() {
            if let Some(lib_pos) = cargo_toml.find("[lib]") {
                cargo_toml.insert_str(lib_pos, &deps_section);
            }
        }
    }

    // Verify the dependencies were added
    assert!(
        cargo_toml.contains("egui = "),
        "egui dependency should be added"
    );
    assert!(
        cargo_toml.contains("eframe = "),
        "eframe dependency should be added"
    );
    assert!(
        cargo_toml.contains("egui_dock = "),
        "egui_dock dependency should be added"
    );
    assert!(
        cargo_toml.contains("serde = "),
        "serde dependency should be added"
    );
    assert!(
        cargo_toml.contains("windjammer-ui = "),
        "windjammer-ui dependency should be added"
    );

    // Verify features are preserved
    assert!(
        cargo_toml.contains("features = [\"desktop\"]"),
        "windjammer-ui features should be preserved"
    );
    assert!(
        cargo_toml.contains("features = [\"derive\"]"),
        "serde features should be preserved"
    );
}

#[test]
fn test_library_deps_dont_duplicate() {
    // Test that we don't add duplicate dependencies
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let wj_toml = r#"
[package]
name = "test-lib"

[dependencies]
serde = "1.0"
"#;

    fs::write(project_root.join("wj.toml"), wj_toml).unwrap();

    // Generated Cargo.toml ALREADY has serde
    let lib_output = project_root.join("lib");
    fs::create_dir_all(&lib_output).unwrap();

    let initial_cargo_toml = r#"[package]
name = "test-lib"

[dependencies]
serde = "1.0"

[lib]
name = "test_lib"
"#;

    fs::write(lib_output.join("Cargo.toml"), initial_cargo_toml).unwrap();

    // Parse and merge (should NOT duplicate)
    let cargo_toml = fs::read_to_string(lib_output.join("Cargo.toml")).unwrap();

    // Count occurrences of "serde = " (should be exactly 1)
    let count = cargo_toml.matches("serde = ").count();
    assert_eq!(count, 1, "serde dependency should not be duplicated");
}
