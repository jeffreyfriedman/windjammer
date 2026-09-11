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

//! FAILING REPRO — multipass app calling cross-crate `render(owned_template(), vars)` must
//! `cargo check` when helper returns owned `string` (`wj-sitegen` / `wj-template` class).
//!
//! Bug: codegen passes owned `String` from helper where crate metadata demotes template to `&str`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn multipass_cross_crate_render_owned_template_helper_must_cargo_check() {
    let tmp = TempDir::new().expect("tempdir");

    let tpl_src = tmp.path().join("tpl_src");
    fs::create_dir_all(&tpl_src).expect("mkdir tpl_src");
    fs::write(
        tpl_src.join("tpl_pkg.wj"),
        r#"
use std::collections::HashMap

pub fn render(template: string, vars: HashMap<string, string>) -> string {
    template
}
"#,
    )
    .unwrap();

    let tpl_gen = tmp.path().join("tpl_gen");
    let tpl_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            tpl_src.to_str().unwrap(),
            "--output",
            tpl_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("tpl_pkg build");
    assert!(
        tpl_build.status.success(),
        "tpl_pkg library build failed:\n{}",
        String::from_utf8_lossy(&tpl_build.stderr)
    );

    let metadata_path = tpl_gen.join("metadata.json");
    assert!(metadata_path.exists(), "tpl_pkg must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(&app_src).expect("mkdir app_src");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.tpl_pkg]\npath = \"{}\"\npackage = \"tpl_src\"\n",
            tpl_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("render.wj"),
        r#"
use std::collections::HashMap
use tpl_pkg::render

fn document_template() -> string {
    "<html><title>{{title}}</title><body>{{body}}</body></html>"
}

pub fn render_document(title: string, body: string) -> string {
    let mut vars = HashMap::new()
    vars.insert("title", title)
    vars.insert("body", body)
    render(document_template(), vars)
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("render.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("tpl_pkg={}", metadata_path.display()),
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
            "tpl_pkg = {{ path = \"{}\", package = \"tpl_src\" }}",
            tpl_gen.display()
        );
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("tpl_pkg"))
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

    let generated = fs::read_to_string(app_gen.join("render.rs")).expect("render.rs");
    assert!(
        !generated.contains("render(&document_template") && !generated.contains("render( &document"),
        "regression: must move owned template helper result, not borrow:\n{generated}"
    );

    let check = Command::new("cargo")
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "RED: multipass cross-crate render(document_template(), vars) must cargo-check; stderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
