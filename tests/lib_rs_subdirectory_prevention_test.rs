// Bug #2B: Prevent lib.rs generation in subdirectories
//
// Problem:
// --------
// When compiling projects with output directories like src/components/generated/,
// the compiler generates lib.rs files in those subdirectories. This causes issues
// because lib.rs should only exist at the crate root.
//
// Root Cause:
// -----------
// Individual .wj files named "lib.wj" get compiled to "lib.rs" regardless of
// where the output directory is. The compiler doesn't check if the output
// directory is a subdirectory before allowing lib.rs generation.
//
// Solution:
// ---------
// 1. Detect if output directory is a subdirectory (contains ".../src/...")
// 2. If yes, rename "lib.wj" compilations to use a different name (e.g., "libmodule.rs")
// 3. Or skip compiling "lib.wj" files when in subdirectory mode
//
// Test Strategy:
// --------------
// 1. Create a project structure with src_wj/components/lib.wj
// 2. Set output to src/components/generated/
// 3. Compile and verify NO lib.rs is created in src/components/generated/
// 4. Verify lib.wj content is either skipped or renamed

use std::fs;
use std::path::Path;
use tempfile::tempdir;

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
fn test_lib_rs_not_generated_in_subdirectory() {
    // Create temp directory structure:
    // project/
    //   src_wj/
    //     components/
    //       lib.wj (contains: pub struct LibComponent {})
    //       button.wj (contains: pub struct Button {})
    //   src/
    //     components/
    //       generated/  <- output directory

    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    // Create source structure
    let src_wj_dir = project_root.join("src_wj");
    let components_src_dir = src_wj_dir.join("components");
    fs::create_dir_all(&components_src_dir).unwrap();

    // Create lib.wj (this should NOT generate lib.rs in subdirectory)
    fs::write(
        components_src_dir.join("lib.wj"),
        "pub struct LibComponent { pub value: int }",
    )
    .unwrap();

    // Create button.wj (for comparison)
    fs::write(
        components_src_dir.join("button.wj"),
        "pub struct Button { pub label: string }",
    )
    .unwrap();

    // Create mod.wj to make it a module
    fs::write(components_src_dir.join("mod.wj"), "").unwrap();

    // Create output directory
    let output_dir = project_root
        .join("src")
        .join("components")
        .join("generated");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile
    compile_wj_project(&src_wj_dir, &output_dir).expect("Compilation should succeed");

    // Debug: List all generated files
    eprintln!("=== Generated files in {:?} ===", output_dir);
    if let Ok(entries) = fs::read_dir(&output_dir) {
        for entry in entries.flatten() {
            eprintln!("  - {}", entry.file_name().to_string_lossy());
        }
    }

    // Also check the components subdirectory
    let components_output_dir = output_dir.join("components");
    if components_output_dir.exists() {
        eprintln!("=== Generated files in {:?} ===", components_output_dir);
        if let Ok(entries) = fs::read_dir(&components_output_dir) {
            for entry in entries.flatten() {
                eprintln!("  - {}", entry.file_name().to_string_lossy());
            }
        }
    }

    // Verify lib.rs was NOT created anywhere in the output tree
    let lib_rs_path = output_dir.join("lib.rs");
    let nested_lib_rs_path = output_dir.join("components").join("lib.rs");
    assert!(
        !lib_rs_path.exists(),
        "lib.rs should NOT be generated in subdirectory: {:?}",
        lib_rs_path
    );
    assert!(
        !nested_lib_rs_path.exists(),
        "lib.rs should NOT be generated in nested subdirectory: {:?}",
        nested_lib_rs_path
    );

    // Verify mod.rs or components/mod.rs WAS created
    let mod_rs_path = output_dir.join("mod.rs");
    let components_mod_rs_path = output_dir.join("components").join("mod.rs");
    assert!(
        mod_rs_path.exists() || components_mod_rs_path.exists(),
        "mod.rs should be generated somewhere in output: {:?} or {:?}",
        mod_rs_path,
        components_mod_rs_path
    );

    // Verify button.rs was created (might be in components/ subdirectory)
    let button_rs_path = output_dir.join("components").join("button.rs");
    let button_rs_alt_path = output_dir.join("button.rs");
    assert!(
        button_rs_path.exists() || button_rs_alt_path.exists(),
        "button.rs should be generated: {:?} or {:?}",
        button_rs_path,
        button_rs_alt_path
    );

    // Verify lib.wj content was either:
    // 1. Compiled to a different name (e.g., libmodule.rs), OR
    // 2. Not compiled at all (skipped)
    //
    // For now, let's verify it's NOT in lib.rs
    let mod_rs_content = fs::read_to_string(&mod_rs_path).unwrap();
    assert!(
        !mod_rs_content.contains("pub mod lib;"),
        "mod.rs should NOT declare 'pub mod lib;': {}",
        mod_rs_content
    );
}

#[test]
fn test_lib_rs_generated_at_crate_root() {
    // Create temp directory structure:
    // project/
    //   src_wj/
    //     lib.wj (contains: pub struct MyLib {})
    //     button.wj (contains: pub struct Button {})
    //   out/  <- output directory (crate root)

    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    // Create source structure
    let src_wj_dir = project_root.join("src_wj");
    fs::create_dir_all(&src_wj_dir).unwrap();

    // Create lib.wj (this SHOULD generate lib.rs at crate root)
    fs::write(
        src_wj_dir.join("lib.wj"),
        "pub struct MyLib { pub value: int }",
    )
    .unwrap();

    // Create button.wj
    fs::write(
        src_wj_dir.join("button.wj"),
        "pub struct Button { pub label: string }",
    )
    .unwrap();

    // Create mod.wj
    fs::write(src_wj_dir.join("mod.wj"), "").unwrap();

    // Create output directory (crate root, not a subdirectory)
    let output_dir = project_root.join("out");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile
    compile_wj_project(&src_wj_dir, &output_dir).expect("Compilation should succeed");

    // Verify lib.rs WAS created at crate root
    let lib_rs_path = output_dir.join("lib.rs");
    assert!(
        lib_rs_path.exists(),
        "lib.rs SHOULD be generated at crate root: {:?}",
        lib_rs_path
    );

    // Verify button.rs was created
    let button_rs_path = output_dir.join("button.rs");
    assert!(
        button_rs_path.exists(),
        "button.rs should be generated: {:?}",
        button_rs_path
    );

    // Verify lib.rs contains proper module declarations
    let lib_rs_content = fs::read_to_string(&lib_rs_path).unwrap();
    assert!(
        lib_rs_content.contains("pub mod button;"),
        "lib.rs should declare button module: {}",
        lib_rs_content
    );
}
