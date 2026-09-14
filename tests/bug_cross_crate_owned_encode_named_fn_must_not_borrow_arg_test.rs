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

//! cargo-bin / multipass: cross-crate free fn named `encode` with owned `String`
//! formal still emits `encode(&arg)` → E0308 expected `String`, found `&String`.
//!
//! Same metadata shape as `hex` / `encode_text` (Owned, emitted_rust_ref_params=false)
//! but the bare name `encode` over-borrows. Ecosystem workaround: `encode_text`.
//!
//! Dogfood: `wj-notes-api` GET `?encoding=base64` via `wj_base64::encode_text`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cross_crate_owned_encode_named_fn_must_not_borrow_arg() {
    let tmp = TempDir::new().expect("tempdir");

    let pkg_src = tmp.path().join("b64_src");
    fs::create_dir_all(&pkg_src).expect("mkdir b64_src");
    fs::write(
        pkg_src.join("b64_pkg.wj"),
        r#"
pub fn encode(text: string) -> string {
    text
}

pub fn encode_text(text: string) -> string {
    encode(text)
}
"#,
    )
    .unwrap();

    let pkg_gen = tmp.path().join("b64_gen");
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
        .expect("b64_pkg build");
    assert!(
        pkg_build.status.success(),
        "b64_pkg library build failed:\n{}",
        String::from_utf8_lossy(&pkg_build.stderr)
    );

    let metadata_path = pkg_gen.join("metadata.json");
    assert!(metadata_path.exists(), "b64_pkg must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("domain")).expect("mkdir domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.b64_pkg]\npath = \"{}\"\npackage = \"b64_src\"\n",
            pkg_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("domain").join("export.wj"),
        r#"
use b64_pkg::encode

pub fn wrap(text: string) -> string {
    let text = text
    encode(text)
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("domain").join("export.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("b64_pkg={}", metadata_path.display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&pkg_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("export.rs")).unwrap_or_else(|_| {
        fs::read_to_string(app_gen.join("domain").join("export.rs")).unwrap_or_default()
    });

    assert!(
        !generated.contains("encode(&") && !generated.contains("encode(&text)"),
        "RED: owned cross-crate encode formal must not borrow arg.\ngenerated:\n{generated}"
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).expect("read app Cargo.toml");
        let dep_line = format!(
            "b64_pkg = {{ path = \"{}\", package = \"b64_src\" }}",
            pkg_gen.display()
        );
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("b64_pkg"))
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
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "RED: cross-crate owned encode must cargo-check.\ngenerated:\n{generated}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
