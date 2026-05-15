// TDD: Test-Driven Development for @cfg decorator support
// Write tests FIRST, then implement the feature

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cfg_decorator_on_struct() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.wj");

    // Write Windjammer source with @cfg decorator
    // Note: Windjammer syntax uses ':' which codegen converts to '=' in Rust
    fs::write(
        &input_file,
        r#"
@cfg(feature: "desktop")
pub struct DesktopRenderer {
    window: int,
}

pub struct Button {
    label: string,
}
"#,
    )
    .unwrap();

    // Compile to Rust
    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success(), "Compilation should succeed");

    // Read generated Rust code
    let rust_code =
        fs::read_to_string(output_dir.join("input.rs")).expect("Generated Rust file should exist");

    println!("Generated Rust:\n{}", rust_code);

    // Verify @cfg decorator was converted to #[cfg(...)]
    assert!(
        rust_code.contains("#[cfg(feature = \"desktop\")]"),
        "Should generate #[cfg(feature = \"desktop\")] attribute"
    );
    assert!(
        rust_code.contains("pub struct DesktopRenderer"),
        "Struct should be generated"
    );

    // Button should NOT have cfg attribute
    assert!(
        !rust_code.contains("#[cfg(feature = \"desktop\")]\npub struct Button"),
        "Button should not have cfg attribute"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cfg_decorator_on_function() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.wj");

    fs::write(
        &input_file,
        r#"
@cfg(feature: "web")
pub fn web_only_function() {
    println!("Web only!")
}

pub fn regular_function() {
    println!("Regular")
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let rust_code = fs::read_to_string(output_dir.join("input.rs")).unwrap();

    assert!(
        rust_code.contains("#[cfg(feature = \"web\")]\npub fn web_only_function()"),
        "Function should have cfg attribute"
    );
    assert!(
        !rust_code.contains("#[cfg(feature = \"web\")]\npub fn regular_function()"),
        "Regular function should not have cfg attribute"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cfg_decorator_on_impl() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.wj");

    fs::write(
        &input_file,
        r#"
pub struct MyStruct {}

@cfg(feature: "desktop")
impl MyStruct {
    pub fn desktop_method(self) {}
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let rust_code = fs::read_to_string(output_dir.join("input.rs")).unwrap();

    assert!(
        rust_code.contains("#[cfg(feature = \"desktop\")]\nimpl MyStruct"),
        "Impl block should have cfg attribute"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_cfg_attributes() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.wj");

    // Test multiple cfg attributes (target_os)
    fs::write(
        &input_file,
        r#"
@cfg(target_os: "linux")
pub struct LinuxOnly {}

@cfg(target_os: "windows")
pub struct WindowsOnly {}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let rust_code = fs::read_to_string(output_dir.join("input.rs")).unwrap();

    assert!(
        rust_code.contains("#[cfg(target_os = \"linux\")]"),
        "Should support target_os cfg"
    );
    assert!(
        rust_code.contains("#[cfg(target_os = \"windows\")]"),
        "Should support multiple different cfg attributes"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_module_file_with_cfg_decorators() {
    // Test that --module-file respects @cfg decorators
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create modules with cfg decorators
    fs::write(src_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(
        src_dir.join("desktop.wj"),
        r#"
@cfg(feature: "desktop")
pub struct Desktop {}
"#,
    )
    .unwrap();

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

    // Check that desktop.rs has cfg attribute
    let desktop_rs = fs::read_to_string(output_dir.join("desktop.rs")).unwrap();
    assert!(
        desktop_rs.contains("#[cfg(feature = \"desktop\")]"),
        "desktop.rs should have cfg attribute from @cfg decorator"
    );

    // Check that mod.rs does NOT add extra cfg (already in the file)
    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).unwrap();
    println!("Generated mod.rs:\n{}", mod_rs);

    // mod.rs should just declare the module, cfg is in the file itself
    assert!(
        mod_rs.contains("pub mod button;"),
        "Should declare button module"
    );
    assert!(
        mod_rs.contains("pub mod desktop;"),
        "Should declare desktop module"
    );
}
