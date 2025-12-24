// Test: Source Maps Must Be Portable (No Absolute Paths)
// Verifies that generated source maps use relative paths

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_source_maps_use_relative_paths() {
    // Create a temporary workspace
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create source directory
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");

    // Write a simple Windjammer program
    let source_file = src_dir.join("test.wj");
    fs::write(
        &source_file,
        r#"
struct Point {
    x: f32,
    y: f32,
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    println("Point: {}, {}", p.x, p.y)
}
"#,
    )
    .expect("Failed to write source file");

    // Compile to build directory
    let build_dir = workspace.join("build");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(workspace)
        .args(["build", "src/test.wj", "--output", "build", "--no-cargo"])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the generated source map
    let source_map_file = build_dir.join("test.rs.map");
    assert!(source_map_file.exists(), "Source map file not created");

    let source_map_json = fs::read_to_string(&source_map_file).expect("Failed to read source map");

    println!("Source map:\n{}", source_map_json);

    // Verify no absolute paths (except workspace_root which is for reference)
    // Parse the JSON
    let source_map: serde_json::Value =
        serde_json::from_str(&source_map_json).expect("Failed to parse source map JSON");

    // Check mappings
    let mappings = source_map["mappings"]
        .as_array()
        .expect("mappings should be an array");

    for mapping in mappings {
        let rust_file = mapping["rust_file"]
            .as_str()
            .expect("rust_file should be string");
        let wj_file = mapping["wj_file"]
            .as_str()
            .expect("wj_file should be string");

        // Paths should be relative (not start with / on Unix or C:\ on Windows)
        assert!(
            !rust_file.starts_with('/')
                && !rust_file.starts_with("C:\\")
                && !rust_file.starts_with("c:\\"),
            "rust_file should be relative: {}",
            rust_file
        );
        assert!(
            !wj_file.starts_with('/')
                && !wj_file.starts_with("C:\\")
                && !wj_file.starts_with("c:\\"),
            "wj_file should be relative: {}",
            wj_file
        );

        // Paths should use forward slashes or be simple filenames
        assert!(
            rust_file.contains("build") || rust_file.ends_with(".rs"),
            "rust_file should point to build directory: {}",
            rust_file
        );
        assert!(
            wj_file.contains("src") || wj_file.ends_with(".wj"),
            "wj_file should point to src directory: {}",
            wj_file
        );
    }

    // Verify workspace_root is set (for reference)
    assert!(
        source_map["workspace_root"].is_string(),
        "workspace_root should be set"
    );

    println!("✓ Source map uses relative paths");
}

#[test]
fn test_source_map_resolves_across_machines() {
    // This test verifies that a source map created on one machine
    // can be loaded and used on another machine with a different filesystem layout

    // Create a temporary workspace
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();

    // Create source directory
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");

    // Write a simple Windjammer program
    let source_file = src_dir.join("test.wj");
    fs::write(
        &source_file,
        r#"
fn add(x: int, y: int) -> int {
    x + y
}

fn main() {
    let result = add(2, 3)
    println("Result: {}", result)
}
"#,
    )
    .expect("Failed to write source file");

    // Compile from workspace directory
    let build_dir = workspace.join("build");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(workspace)
        .args(["build", "src/test.wj", "--output", "build", "--no-cargo"])
        .output()
        .expect("Failed to run wj");

    assert!(output.status.success(), "Compilation failed");

    // Load the source map
    let source_map_file = build_dir.join("test.rs.map");
    let source_map = windjammer::source_map::SourceMap::load_from_file(&source_map_file)
        .expect("Failed to load source map");

    // Simulate being on a different machine by loading from a different working directory
    // The paths in the source map should still be resolvable relative to the workspace

    // Look up a mapping (line 1 of the generated Rust)
    let rust_file = Path::new("build/test.rs");
    let mapping = source_map.lookup(rust_file, 1);

    assert!(mapping.is_some(), "Should find mapping for line 1");

    let mapping = mapping.unwrap();

    // Verify the mapping points to the correct Windjammer source
    assert!(
        mapping.wj_file.to_string_lossy().contains("test.wj"),
        "Should map to test.wj"
    );
    assert!(mapping.wj_line > 0, "Should have valid line number");

    // Verify the paths are relative and can be resolved from the workspace
    let resolved_wj_file = workspace.join(&mapping.wj_file);
    assert!(
        resolved_wj_file.exists(),
        "Windjammer source file should exist when resolved from workspace: {:?}",
        resolved_wj_file
    );

    let resolved_rust_file = workspace.join(&mapping.rust_file);
    assert!(
        resolved_rust_file.exists(),
        "Rust file should exist when resolved from workspace: {:?}",
        resolved_rust_file
    );

    println!("✓ Source map resolves correctly across different contexts");
}
