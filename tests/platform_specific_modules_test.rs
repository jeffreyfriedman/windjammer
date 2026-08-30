#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

// Bug #11: Platform-specific module declarations
//
// When generated or hand-written `.rs` files start with `#![cfg(...)]`, `mod.rs` /
// `lib.rs` must emit matching `#[cfg(...)]` on `pub mod` so non-target builds skip them.
//
// Tests inject cfg-gated `.rs` after the first compile (simulates hand-written output).

use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn compile_wj_project(source_dir: &Path, output_dir: &Path) -> Result<(), String> {
    use std::process::Command;

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            source_dir.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| format!("Failed to run wj: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_wasm_specific_module_declaration() {
    // This test verifies that when .rs files have #![cfg(...)] attributes,
    // the module system detects them and adds #[cfg(...)] to module declarations
    //
    // Strategy: Compile a simple project, then manually add a cfg-gated .rs file,
    // and verify that re-running the compiler adds the cfg to mod.rs

    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    // Create simple .wj project
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(src_dir.join("mod.wj"), "").unwrap();

    fs::write(
        src_dir.join("button.wj"),
        "pub struct Button { pub label: string }",
    )
    .unwrap();

    let output_dir = project_root.join("out");
    fs::create_dir_all(&output_dir).unwrap();

    // First compilation
    compile_wj_project(&src_dir, &output_dir).expect("First compilation should succeed");

    // Now manually add a WASM-specific .rs file (simulating hand-written or generated code)
    fs::write(
        output_dir.join("examples_wasm.rs"),
        r#"#![cfg(target_arch = "wasm32")]

pub fn run_wasm_example() {
    println!("Running WASM example!");
}
"#,
    )
    .unwrap();

    // Second compilation to regenerate mod.rs
    compile_wj_project(&src_dir, &output_dir).expect("Second compilation should succeed");

    // Verify lib.rs was updated
    let lib_rs_path = output_dir.join("lib.rs");
    let lib_rs_content = fs::read_to_string(&lib_rs_path).unwrap();
    eprintln!("=== lib.rs content ===\n{}", lib_rs_content);

    // Verify button module is declared normally (no cfg)
    assert!(
        lib_rs_content.contains("pub mod button;"),
        "lib.rs should declare button module: {}",
        lib_rs_content
    );

    // Module system detects #![cfg(...)] in sibling .rs files and gates the mod declaration.
    assert!(
        lib_rs_content.contains("#[cfg(target_arch = \"wasm32\")]")
            && lib_rs_content.contains("pub mod examples_wasm;"),
        "lib.rs should gate examples_wasm with #[cfg(target_arch = \"wasm32\")]:\n{lib_rs_content}"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_desktop_specific_module_declaration() {
    // Same strategy as WASM: inject cfg-gated `.rs` after first compile.
    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(src_dir.join("mod.wj"), "").unwrap();

    fs::write(
        src_dir.join("core.wj"),
        "pub struct Core { pub id: int }",
    )
    .unwrap();

    let output_dir = project_root.join("out");
    fs::create_dir_all(&output_dir).unwrap();

    compile_wj_project(&src_dir, &output_dir).expect("First compilation should succeed");

    fs::write(
        output_dir.join("desktop_app.rs"),
        r#"#![cfg(feature = "desktop")]

pub struct DesktopApp {
    pub window_title: String,
}
"#,
    )
    .unwrap();

    compile_wj_project(&src_dir, &output_dir).expect("Second compilation should succeed");

    let lib_rs_path = output_dir.join("lib.rs");
    let lib_rs_content = fs::read_to_string(&lib_rs_path).unwrap();
    eprintln!("=== lib.rs content ===\n{}", lib_rs_content);

    assert!(
        lib_rs_content.contains(r#"#[cfg(feature = "desktop")]"#)
            && lib_rs_content.contains("pub mod desktop_app;"),
        "lib.rs should gate desktop_app with #[cfg(feature = \"desktop\")]:\n{lib_rs_content}"
    );
}
