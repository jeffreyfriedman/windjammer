//! TDD Test: mod.wj types/traits/structs must be compiled in multi-file builds
//!
//! Bug: In multi-file projects, mod.wj files are skipped during compilation
//! (main.rs line 686). Only `pub mod` and `pub use` declarations are scraped
//! by module_system.rs, so any traits, structs, impls, or functions defined
//! in mod.wj are silently dropped from the generated output.
//!
//! Expected: A mod.wj with `pub mod sub` + `pub trait Foo` + `pub struct Bar`
//! should produce a mod.rs containing all three.

#[path = "../../common/test_utils.rs"]
mod test_utils;

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mod_wj_trait_and_struct_codegen() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let plugin_dir = src_dir.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    // Create plugin/mod.wj with both module declarations AND code (trait + struct)
    fs::write(
        plugin_dir.join("mod.wj"),
        r#"
pub mod audio

pub trait Plugin {
    fn name(self) -> String
    fn version(self) -> String
}

pub struct App {
    pub plugin_count: i32,
}

impl App {
    pub fn new() -> App {
        App { plugin_count: 0 }
    }
}
"#,
    )
    .unwrap();

    // Create plugin/audio.wj that uses the trait from mod.wj
    fs::write(
        plugin_dir.join("audio.wj"),
        r#"
pub struct AudioPlugin {
    pub name_str: String,
}

impl AudioPlugin {
    pub fn new() -> AudioPlugin {
        AudioPlugin { name_str: String::from("audio") }
    }
}
"#,
    )
    .unwrap();

    // Create a root file that uses the plugin module
    fs::write(
        src_dir.join("main.wj"),
        r#"
pub mod plugin

pub fn create_app() -> i32 {
    42
}
"#,
    )
    .unwrap();

    // Compile the project
    let out_dir = tmp.path().join("build");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Compilation failed.\nstderr: {}",
        stderr
    );

    // Read the generated plugin/mod.rs
    let mod_rs_path = out_dir.join("plugin").join("mod.rs");
    assert!(
        mod_rs_path.exists(),
        "plugin/mod.rs should exist. Out dir contents: {:?}",
        fs::read_dir(&out_dir)
            .map(|e| e.flatten().map(|e| e.path()).collect::<Vec<_>>())
            .unwrap_or_default()
    );

    let mod_rs_content = fs::read_to_string(&mod_rs_path).unwrap();

    // mod.rs MUST contain the module declaration
    assert!(
        mod_rs_content.contains("pub mod audio"),
        "mod.rs must contain `pub mod audio`.\nActual content:\n{}",
        mod_rs_content
    );

    // mod.rs MUST contain the trait definition
    assert!(
        mod_rs_content.contains("pub trait Plugin"),
        "mod.rs must contain `pub trait Plugin` from mod.wj.\nActual content:\n{}",
        mod_rs_content
    );

    // mod.rs MUST contain the struct definition
    assert!(
        mod_rs_content.contains("pub struct App"),
        "mod.rs must contain `pub struct App` from mod.wj.\nActual content:\n{}",
        mod_rs_content
    );

    // mod.rs MUST contain the impl block
    assert!(
        mod_rs_content.contains("impl App"),
        "mod.rs must contain `impl App` from mod.wj.\nActual content:\n{}",
        mod_rs_content
    );

    // mod.rs must NOT have duplicate `pub mod audio;` declarations
    let pub_mod_count = mod_rs_content.matches("pub mod audio").count();
    assert!(
        pub_mod_count == 1,
        "mod.rs should have exactly 1 `pub mod audio` declaration, found {}.\nActual content:\n{}",
        pub_mod_count,
        mod_rs_content
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mod_wj_code_compiles_with_rustc() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let plugin_dir = src_dir.join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    // mod.wj with trait + struct
    fs::write(
        plugin_dir.join("mod.wj"),
        r#"
pub mod audio

pub trait Plugin {
    fn name(self) -> String
    fn version(self) -> String
}

pub struct App {
    pub plugin_count: i32,
}

impl App {
    pub fn new() -> App {
        App { plugin_count: 0 }
    }
}
"#,
    )
    .unwrap();

    // plugin/audio.wj
    fs::write(
        plugin_dir.join("audio.wj"),
        r#"
pub struct AudioPlugin {
    pub volume: f32,
}
"#,
    )
    .unwrap();

    // Root file
    fs::write(
        src_dir.join("lib.wj"),
        r#"
pub mod plugin
"#,
    )
    .unwrap();

    // Compile
    let out_dir = tmp.path().join("build");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "wj compilation failed.\nstderr: {}",
        stderr
    );

    // Verify the generated Rust actually compiles
    let rustc_output = std::process::Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(tmp.path().join("verify.rmeta"))
        .arg(out_dir.join("lib.rs"))
        .output()
        .expect("Failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    assert!(
        rustc_output.status.success(),
        "Generated Rust from mod.wj should compile with rustc.\nrustc stderr:\n{}",
        rustc_stderr
    );
}
