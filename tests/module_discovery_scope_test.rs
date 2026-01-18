// This test file is part of the TDD process for Bug #12: Module discovery scope.
//
// Problem:
// --------
// The Windjammer compiler incorrectly discovers hand-written .rs files from the
// parent directory (e.g., src/app.rs) and:
// 1. Copies them to the generated output directory (e.g., src/components/generated/app.rs)
// 2. Declares them in the generated mod.rs (e.g., `pub mod app;`)
//
// This causes duplicate symbol errors when these files are already declared as
// top-level modules in src/lib.rs.
//
// Root Cause:
// -----------
// The module discovery logic in `discover_hand_written_modules` and/or the
// directory copying logic in `copy_dir_recursive` doesn't properly scope its
// search to only the immediate directory being compiled.
//
// Solution:
// ---------
// When generating src/components/generated/mod.rs, the compiler should only
// discover hand-written .rs files that are WITHIN src/components/, not from
// the parent src/ directory.
//
// Test Strategy:
// --------------
// 1. Create a project structure that mimics windjammer-ui:
//    - src/lib.rs (declares `pub mod app;`)
//    - src/app.rs (hand-written top-level module)
//    - src/components_wj/button.wj (Windjammer component)
//    - Output to src/components/generated/
//
// 2. Compile with `wj build`
//
// 3. Assert:
//    - src/components/generated/mod.rs does NOT contain `pub mod app;`
//    - src/components/generated/app.rs does NOT exist
//    - src/components/generated/button.rs DOES exist
//    - src/components/generated/mod.rs DOES contain `pub mod button;`

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

// Helper function to compile a Windjammer project
fn compile_wj_project(source_dir: &Path, output_dir: &Path) -> Result<(), String> {
    use std::process::Command;

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            source_dir.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo", // Skip cargo build to focus on wj compilation
        ])
        .output()
        .map_err(|e| format!("Failed to run wj CLI: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)] // Skip during coverage runs due to timeout
fn test_parent_directory_modules_not_discovered() {
    // Scenario (mimics windjammer-ui structure):
    //   src/
    //     lib.rs (declares `pub mod app;`)
    //     app.rs (hand-written top-level module)
    //     components_wj/
    //       button.wj (Windjammer component)
    //       mod.wj
    //     components/
    //       generated/  <- output directory
    //
    // Expected behavior:
    //   - generated/mod.rs should NOT declare `pub mod app;`
    //   - generated/app.rs should NOT exist
    //   - generated/button.rs SHOULD exist
    //   - generated/mod.rs SHOULD declare `pub mod button;`

    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    // Create src/lib.rs with top-level app module
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("lib.rs"),
        r#"// Top-level library file
pub mod app;  // Hand-written top-level module

// Re-export generated components
pub mod components {
    pub mod generated;
}
"#,
    )
    .unwrap();

    // Create src/app.rs (hand-written top-level module)
    fs::write(
        src_dir.join("app.rs"),
        r#"// Hand-written app module (top-level in src/lib.rs)
pub struct App {
    pub name: String,
}

impl App {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
"#,
    )
    .unwrap();

    // Create src/components_wj/button.wj (Windjammer component)
    let components_wj_dir = src_dir.join("components_wj");
    fs::create_dir_all(&components_wj_dir).unwrap();
    fs::write(
        components_wj_dir.join("button.wj"),
        r#"pub struct Button {
    pub label: string
}

impl Button {
    pub fn new(label: string) -> Button {
        Button { label }
    }

    pub fn render(&self) -> string {
        format!("<button>{}</button>", self.label)
    }
}
"#,
    )
    .unwrap();
    fs::write(components_wj_dir.join("mod.wj"), "").unwrap();

    // Create output directory structure
    let output_dir = src_dir.join("components").join("generated");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile
    compile_wj_project(&components_wj_dir, &output_dir).expect("Compilation should succeed");

    // Debug: List all generated files
    eprintln!("=== Generated files in {:?} ===", output_dir);
    if let Ok(entries) = fs::read_dir(&output_dir) {
        for entry in entries.flatten() {
            eprintln!("  - {}", entry.file_name().to_string_lossy());
        }
    }

    // CRITICAL ASSERTIONS: Parent directory modules should NOT be discovered

    // 1. app.rs should NOT be copied to generated directory
    let generated_app_rs = output_dir.join("app.rs");
    assert!(
        !generated_app_rs.exists(),
        "BUG: app.rs from parent src/ was incorrectly copied to generated/: {:?}",
        generated_app_rs
    );

    // 2. mod.rs should NOT declare app module
    let mod_rs_path = output_dir.join("mod.rs");
    assert!(
        mod_rs_path.exists(),
        "mod.rs should be generated: {:?}",
        mod_rs_path
    );

    let mod_rs_content = fs::read_to_string(&mod_rs_path).unwrap();
    eprintln!("=== mod.rs content ===\n{}", mod_rs_content);

    assert!(
        !mod_rs_content.contains("pub mod app;"),
        "BUG: mod.rs incorrectly declares parent directory module 'app'"
    );
    assert!(
        !mod_rs_content.contains("pub use app::*;"),
        "BUG: mod.rs incorrectly re-exports parent directory module 'app'"
    );

    // POSITIVE ASSERTIONS: Actual components should be discovered

    // 3. button.rs SHOULD exist
    let button_rs_path = output_dir.join("button.rs");
    assert!(
        button_rs_path.exists(),
        "button.rs should be generated: {:?}",
        button_rs_path
    );

    // 4. mod.rs SHOULD declare button module
    assert!(
        mod_rs_content.contains("pub mod button;"),
        "mod.rs should declare button module"
    );
    assert!(
        mod_rs_content.contains("pub use button::*;"),
        "mod.rs should re-export button module"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)] // Skip during coverage runs due to timeout
fn test_sibling_hand_written_modules_are_discovered() {
    // Scenario:
    //   src/
    //     components_wj/
    //       button.wj
    //       mod.wj
    //     components/
    //       platform.rs  <- Hand-written sibling (should be discovered)
    //       generated/   <- output directory
    //
    // Expected behavior:
    //   - generated/mod.rs SHOULD declare `pub mod platform;` (sibling in components/)
    //   - generated/mod.rs should NOT declare anything from src/

    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    // Create src structure
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create src/components_wj/button.wj
    let components_wj_dir = src_dir.join("components_wj");
    fs::create_dir_all(&components_wj_dir).unwrap();
    fs::write(
        components_wj_dir.join("button.wj"),
        "pub struct Button { pub label: string }",
    )
    .unwrap();
    fs::write(components_wj_dir.join("mod.wj"), "").unwrap();

    // Create src/components/platform.rs (hand-written sibling)
    let components_dir = src_dir.join("components");
    fs::create_dir_all(&components_dir).unwrap();
    fs::write(
        components_dir.join("platform.rs"),
        r#"pub fn get_platform_name() -> String {
    "test".to_string()
}
"#,
    )
    .unwrap();

    // Create output directory
    let output_dir = components_dir.join("generated");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile
    compile_wj_project(&components_wj_dir, &output_dir).expect("Compilation should succeed");

    // Check mod.rs
    let mod_rs_path = output_dir.join("mod.rs");
    let mod_rs_content = fs::read_to_string(&mod_rs_path).unwrap();
    eprintln!("=== mod.rs content ===\n{}", mod_rs_content);

    // Platform SHOULD be discovered (it's a sibling in components/)
    assert!(
        mod_rs_content.contains("pub mod platform;"),
        "mod.rs should declare sibling platform module"
    );
}

