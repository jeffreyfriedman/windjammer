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
    feature = "integration_tests",
))]

//! P3.262 (`wj-config` → `wj-toml`): library multipass demotes `from_toml(text: string)`
//! to `&str`, then calls cross-crate `parse(text: String)` without auto-own → E0308.
//!
//! Ecosystem interim: `parse("${text}")` in `wj-config::from_toml`.
//! Related: WDB-170 (same-crate `.clone()`), P3.233 (app demoted → cross-crate owned).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn multipass_lib_demoted_str_into_cross_crate_owned_parse_must_cargo_check() {
    let tmp = TempDir::new().expect("tempdir");

    let pkg_src = tmp.path().join("toml_src");
    fs::create_dir_all(&pkg_src).expect("mkdir toml_src");
    fs::write(
        pkg_src.join("toml_pkg.wj"),
        r#"
use std::collections::HashMap

pub fn parse(text: string) -> HashMap<string, string> {
    let mut map = HashMap::new()
    map.insert("raw", text)
    map
}
"#,
    )
    .unwrap();

    let pkg_gen = tmp.path().join("toml_gen");
    let pkg_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            pkg_src.to_str().unwrap(),
            "--output",
            pkg_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("toml_pkg build");
    assert!(
        pkg_build.status.success(),
        "toml_pkg library build failed:\n{}",
        String::from_utf8_lossy(&pkg_build.stderr)
    );
    let metadata_path = pkg_gen.join("metadata.json");
    assert!(metadata_path.exists(), "toml_pkg must emit metadata.json");

    let lib_src = tmp.path().join("config_src");
    fs::create_dir_all(&lib_src).expect("mkdir config_src");
    fs::write(
        lib_src.join("lib.wj"),
        r#"
use std::collections::HashMap
use toml_pkg::parse

pub fn from_toml(text: string) -> HashMap<string, string> {
    parse(text)
}
"#,
    )
    .unwrap();

    let lib_gen = tmp.path().join("config_gen");
    let lib_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            lib_src.to_str().unwrap(),
            "--output",
            lib_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
            "--metadata",
            &format!("toml_pkg={}", metadata_path.display()),
        ])
        .output()
        .expect("config lib build");
    assert!(
        lib_build.status.success(),
        "config library transpile failed:\n{}",
        String::from_utf8_lossy(&lib_build.stderr)
    );

    let generated = fs::read_to_string(lib_gen.join("lib.rs")).expect("read lib.rs");

    let cargo_toml_path = lib_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");
        let dep_line = format!(
            "toml_pkg = {{ path = \"{}\", package = \"toml_src\" }}",
            pkg_gen.display()
        );
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("toml_pkg"))
            .map(str::to_string)
            .collect();
        if let Some(idx) = lines.iter().position(|l| l.trim() == "[dependencies]") {
            lines.insert(idx + 1, dep_line);
        } else {
            lines.push(String::new());
            lines.push("[dependencies]".to_string());
            lines.push(dep_line);
        }
        fs::write(&cargo_toml_path, format!("{}\n", lines.join("\n"))).expect("patch Cargo.toml");
    }

    let check = Command::new("cargo")
        .current_dir(&lib_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "P3.262 RED: demoted &str into cross-crate owned parse must cargo-check.\ngenerated:\n{generated}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
