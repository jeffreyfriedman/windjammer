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

//! FAILING REPRO — multipass app calling cross-crate `join_url(base_url, path)` must
//! `cargo check` when both arguments are owned `string` formals (wj-sitegen feeds).
//!
//! Bug: codegen passes owned `String` local for first param where crate metadata expects `&str`.
//! Workaround in ecosystem: local `join_page_url` instead of dogfooding `wj-url`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn multipass_cross_crate_join_url_owned_base_must_cargo_check() {
    let tmp = TempDir::new().expect("tempdir");

    let url_src = tmp.path().join("url_src");
    fs::create_dir_all(&url_src).expect("mkdir url_src");
    fs::write(
        url_src.join("url_pkg.wj"),
        r#"
pub fn join_url(base: string, relative: string) -> Result<string, string> {
    Ok("${base}/${relative}")
}
"#,
    )
    .unwrap();

    let url_gen = tmp.path().join("url_gen");
    let url_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            url_src.to_str().unwrap(),
            "--output",
            url_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("url_pkg build");
    assert!(
        url_build.status.success(),
        "url_pkg library build failed:\n{}",
        String::from_utf8_lossy(&url_build.stderr)
    );

    let metadata_path = url_gen.join("metadata.json");
    assert!(metadata_path.exists(), "url_pkg must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(&app_src).expect("mkdir app_src");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.url_pkg]\npath = \"{}\"\npackage = \"url_src\"\n",
            url_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("feeds.wj"),
        r#"
use url_pkg::join_url

pub fn page_url(base_url: string, html_path: string) -> Result<string, string> {
    join_url(base_url, html_path)
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("feeds.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("url_pkg={}", metadata_path.display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app build failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).expect("read app Cargo.toml");
        let dep_line = format!(
            "url_pkg = {{ path = \"{}\", package = \"url_src\" }}",
            url_gen.display()
        );
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("url_pkg"))
            .map(str::to_string)
            .collect();
        if let Some(idx) = lines.iter().position(|l| l.trim() == "[dependencies]") {
            lines.insert(idx + 1, dep_line);
        } else {
            lines.push(String::new());
            lines.push("[dependencies]".to_string());
            lines.push(dep_line);
        }
        fs::write(&cargo_toml_path, format!("{}\n", lines.join("\n"))).expect("patch app Cargo.toml");
    }

    let generated = fs::read_to_string(app_gen.join("feeds.rs")).expect("feeds.rs");
    assert!(
        !generated.contains("join_url(&base_url") && !generated.contains("join_url( &base_url"),
        "regression: must not borrow owned base when metadata says owned:\n{generated}"
    );

    let check = Command::new("cargo")
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "RED: multipass cross-crate join_url(base_url, path) must cargo-check; stderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
