#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// TDD test: When a struct implements a trait from another file,
/// and the trait's methods use &mut self (recorded in metadata/registry),
/// the impl methods must also use &mut self.
///
/// Bug: game_renderer.rs generates `fn initialize(&self)` but the trait
/// (render_port.rs) requires `fn initialize(&mut self)` → E0053.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_uses_registry_self_ownership() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_trait_impl_cross_file_self_mut");

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    // Simulate cross-file trait impl: the trait methods have &mut self
    // in the signature registry, but the impl is compiled separately.
    let wj_source = r#"
trait Renderer {
    fn initialize()
    fn render()
    fn shutdown()
}

struct GameData {
    frame_count: i32
}

struct MyRenderer {
    data: GameData
}

impl Renderer for MyRenderer {
    fn initialize() {
        self.data.frame_count = 0
    }

    fn render() {
        self.data.frame_count = self.data.frame_count + 1
    }

    fn shutdown() {
        self.data.frame_count = -1
    }
}
"#;

    let wj_file = test_dir.join("trait_impl_mut.wj");
    fs::write(&wj_file, wj_source).unwrap();

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(test_dir.join("out"))
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(test_dir.join("out").join("trait_impl_mut.rs")).unwrap();

    // The trait methods should use &mut self since they mutate self.data
    assert!(
        generated.contains("fn initialize(&mut self)"),
        "Trait declaration should have &mut self for initialize.\nGenerated:\n{}",
        generated
    );

    // The impl methods should ALSO use &mut self to match the trait
    let impl_start = generated.find("impl Renderer for MyRenderer").unwrap_or(0);
    let impl_section = &generated[impl_start..];

    assert!(
        impl_section.contains("fn initialize(&mut self)"),
        "Impl method should match trait's &mut self for initialize.\nImpl section:\n{}",
        impl_section
    );

    assert!(
        impl_section.contains("fn render(&mut self)"),
        "Impl method should match trait's &mut self for render.\nImpl section:\n{}",
        impl_section
    );

    // Verify the generated code compiles with rustc
    let rs_file = test_dir.join("out").join("trait_impl_mut.rs");
    let rustc_out = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg(&rs_file)
        .arg("-o")
        .arg(test_dir.join("out").join("test.rlib"))
        .output()
        .expect("Failed to run rustc");
    assert!(
        rustc_out.status.success(),
        "Generated code doesn't compile with rustc:\n{}",
        String::from_utf8_lossy(&rustc_out.stderr)
    );
}
