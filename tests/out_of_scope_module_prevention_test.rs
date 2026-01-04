// Bug #9B: Prevent declaring out-of-scope hand-written modules
//
// Problem:
// --------
// When compiling .wj files to src/components/generated/, the compiler discovers
// hand-written modules in src/ (like src/events/) and incorrectly declares them
// in src/components/generated/mod.rs with `pub mod events;`
//
// This causes errors because events.rs is not in the generated/ directory.
//
// Root Cause:
// -----------
// discover_hand_written_modules() searches the project root for .rs files, but
// doesn't check if those modules are within the scope of the output directory.
//
// When output is src/components/generated/, modules in src/ are "out of scope"
// and should not be declared.
//
// Solution:
// ---------
// 1. Pass output_dir to discover_hand_written_modules()
// 2. Skip declaring modules that exist outside the output directory tree
// 3. Only declare modules that are:
//    a) Within the output directory, OR
//    b) In the project root (for FFI interop)
//
// Test Strategy:
// --------------
// 1. Create project with src/events/ (hand-written module)
// 2. Create src_wj/components/ with .wj files
// 3. Set output to src/components/generated/
// 4. Verify src/components/generated/mod.rs does NOT declare `pub mod events;`
// 5. Verify button.wj compiles correctly

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
#[cfg_attr(tarpaulin, ignore)] // Skip during coverage: too slow, requires wj binary
fn test_out_of_scope_modules_not_declared() {
    // Create project structure:
    // project/
    //   src/
    //     events/          <- Hand-written module (out of scope)
    //       mod.rs
    //       dispatcher.rs
    //   src_wj/
    //     components/      <- Source .wj files
    //       mod.wj
    //       button.wj
    //   src/components/generated/  <- Output directory

    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    // Create hand-written events module in src/
    let src_dir = project_root.join("src");
    let events_dir = src_dir.join("events");
    fs::create_dir_all(&events_dir).unwrap();

    fs::write(
        events_dir.join("mod.rs"),
        "pub mod dispatcher;\npub use dispatcher::*;",
    )
    .unwrap();

    fs::write(
        events_dir.join("dispatcher.rs"),
        "pub struct ComponentEventDispatcher {}",
    )
    .unwrap();

    // Create .wj source files
    let src_wj_dir = project_root.join("src_wj");
    let components_src_dir = src_wj_dir.join("components");
    fs::create_dir_all(&components_src_dir).unwrap();

    fs::write(components_src_dir.join("mod.wj"), "").unwrap();

    fs::write(
        components_src_dir.join("button.wj"),
        "pub struct Button { pub label: string }",
    )
    .unwrap();

    // Create output directory
    let output_dir = src_dir.join("components").join("generated");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile
    compile_wj_project(&src_wj_dir, &output_dir).expect("Compilation should succeed");

    // Debug: List generated files
    eprintln!("=== Generated files in {:?} ===", output_dir);
    if let Ok(entries) = fs::read_dir(&output_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            eprintln!("  - {}", name);

            // Check if events directory was copied
            if name == "events" {
                let events_path = output_dir.join("events");
                eprintln!("    ⚠️  Events directory found! Listing contents:");
                if let Ok(sub_entries) = fs::read_dir(&events_path) {
                    for sub_entry in sub_entries.flatten() {
                        eprintln!("      - {}", sub_entry.file_name().to_string_lossy());
                    }
                }
            }
        }
    }

    // Verify events directory was NOT copied to output
    let events_output_path = output_dir.join("events");
    assert!(
        !events_output_path.exists(),
        "events directory should NOT be copied to output: {:?}",
        events_output_path
    );

    // Verify mod.rs was created
    let mod_rs_path = output_dir.join("components").join("mod.rs");
    assert!(
        mod_rs_path.exists(),
        "mod.rs should be generated: {:?}",
        mod_rs_path
    );

    // Verify mod.rs does NOT declare events module
    let mod_rs_content = fs::read_to_string(&mod_rs_path).unwrap();
    eprintln!("=== mod.rs content ===\n{}", mod_rs_content);

    assert!(
        !mod_rs_content.contains("pub mod events;"),
        "mod.rs should NOT declare out-of-scope module 'events': {}",
        mod_rs_content
    );

    assert!(
        !mod_rs_content.contains("pub use events::*;"),
        "mod.rs should NOT re-export out-of-scope module 'events': {}",
        mod_rs_content
    );

    // Verify button module IS declared (in-scope)
    assert!(
        mod_rs_content.contains("pub mod button;"),
        "mod.rs SHOULD declare in-scope module 'button': {}",
        mod_rs_content
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)] // Skip during coverage: too slow, requires wj binary
fn test_ffi_modules_in_project_root_are_declared() {
    // Create project structure:
    // project/
    //   ffi.rs           <- Hand-written FFI module in project root (IN SCOPE)
    //   src_wj/
    //     mod.wj
    //     button.wj
    //   out/             <- Output directory (crate root)

    let temp_dir = tempdir().unwrap();
    let project_root = temp_dir.path();

    // Create FFI module in project root (this SHOULD be declared)
    fs::write(
        project_root.join("ffi.rs"),
        "extern \"C\" { pub fn some_c_function(); }",
    )
    .unwrap();

    // Create .wj source files
    let src_wj_dir = project_root.join("src_wj");
    fs::create_dir_all(&src_wj_dir).unwrap();

    fs::write(src_wj_dir.join("mod.wj"), "").unwrap();

    fs::write(
        src_wj_dir.join("button.wj"),
        "pub struct Button { pub label: string }",
    )
    .unwrap();

    // Create output directory (crate root)
    let output_dir = project_root.join("out");
    fs::create_dir_all(&output_dir).unwrap();

    // Compile
    compile_wj_project(&src_wj_dir, &output_dir).expect("Compilation should succeed");

    // Verify lib.rs was created (crate root)
    let lib_rs_path = output_dir.join("lib.rs");
    assert!(
        lib_rs_path.exists(),
        "lib.rs should be generated at crate root: {:?}",
        lib_rs_path
    );

    // Verify lib.rs DOES declare ffi module (in-scope FFI)
    let lib_rs_content = fs::read_to_string(&lib_rs_path).unwrap();
    eprintln!("=== lib.rs content ===\n{}", lib_rs_content);

    assert!(
        lib_rs_content.contains("pub mod ffi;"),
        "lib.rs SHOULD declare FFI module in project root: {}",
        lib_rs_content
    );
}
