// Test that desktop-only modules get proper #[cfg(feature = "desktop")] gates

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_desktop_module_feature_gates() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create some test modules
    fs::write(src_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(
        src_dir.join("desktop_renderer.wj"),
        "pub struct DesktopRenderer {}",
    )
    .unwrap();
    fs::write(src_dir.join("app_docking.wj"), "pub struct AppDocking {}").unwrap();
    fs::write(src_dir.join("app_reactive.wj"), "pub struct AppReactive {}").unwrap();

    // Compile with --module-file flag
    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success(), "wj build should succeed");

    // Read the generated mod.rs
    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).expect("mod.rs should be generated");

    println!("Generated mod.rs:\n{}", mod_rs);

    // Verify feature gates are applied correctly
    assert!(
        mod_rs.contains("#[cfg(feature = \"desktop\")]\npub mod desktop_renderer;"),
        "desktop_renderer should have feature gate"
    );
    assert!(
        mod_rs.contains("#[cfg(feature = \"desktop\")]\npub mod app_docking;"),
        "app_docking should have feature gate"
    );

    // Verify non-desktop modules don't have feature gates
    assert!(
        mod_rs.contains("pub mod button;")
            && !mod_rs.contains("#[cfg(feature = \"desktop\")]\npub mod button;"),
        "button should NOT have feature gate"
    );

    // app_reactive is an exception - it should NOT have a feature gate
    assert!(
        mod_rs.contains("pub mod app_reactive;")
            && !mod_rs.contains("#[cfg(feature = \"desktop\")]\npub mod app_reactive;"),
        "app_reactive should NOT have feature gate (exception)"
    );

    // Verify re-exports also have feature gates
    assert!(
        mod_rs.contains("#[cfg(feature = \"desktop\")]\npub use desktop_renderer::*;"),
        "desktop_renderer re-export should have feature gate"
    );
    assert!(
        mod_rs.contains("#[cfg(feature = \"desktop\")]\npub use app_docking::*;"),
        "app_docking re-export should have feature gate"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_feature_gates_for_regular_modules() {
    // Create a temporary directory with only non-desktop modules
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    fs::write(src_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(src_dir.join("input.wj"), "pub struct Input {}").unwrap();
    fs::write(src_dir.join("text.wj"), "pub struct Text {}").unwrap();

    // Compile
    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).expect("mod.rs should be generated");

    // Verify NO feature gates are present
    assert!(
        !mod_rs.contains("#[cfg(feature = \"desktop\")]"),
        "No feature gates should be present for non-desktop modules"
    );
}
