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

//! Cross-crate `get(query: String, key: &str)` must keep `"pretty"` as `&str`,
//! not `String::from("pretty")`.
//!
//! Ecosystem `wj-notes-api` `query_wants_pretty`: `qs_get(query, "pretty")`
//! emits E0308 `expected &str, found String`. Distinct from WDB-168 (same-crate).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn build_library(wj: &str, src_dir: &std::path::Path, out_dir: &std::path::Path) {
    let status = Command::new(wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("library build");
    assert!(
        status.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn qs_get_literal_into_demoted_key_must_not_string_from() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let qs_src = tmp.path().join("qs_src");
    fs::create_dir_all(&qs_src).expect("mkdir qs_src");
    fs::write(
        qs_src.join("lib.wj"),
        r#"
use std::strings

pub fn get(query: string, key: string) -> Option<string> {
    if strings.len(query) == 0 {
        return None
    }
    if key == "pretty" || key == "encoding" {
        return Some("1")
    }
    None
}
"#,
    )
    .unwrap();
    let qs_gen = tmp.path().join("qs_gen");
    build_library(wj, &qs_src, &qs_gen);
    let qs_rs = fs::read_to_string(qs_gen.join("lib.rs")).unwrap_or_default();
    assert!(
        qs_rs.contains("key: &str") || qs_rs.contains("key: & str"),
        "fixture get must demote key to &str:\n{qs_rs}"
    );

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("src").join("domain")).expect("mkdir src/domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"notes_qs\"\n\n[dependencies]\nqs_pkg = {{ path = \"{}\" }}\n",
            qs_gen.display()
        ),
    )
    .unwrap();
    fs::write(app_src.join("src").join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(
        app_src.join("src").join("domain").join("mod.wj"),
        "pub mod api\n",
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("api.wj"),
        r#"
use qs_pkg::get as qs_get

fn own(value: string) -> string {
    value
}

pub fn query_wants_pretty(query: string) -> bool {
    let query = own(query)
    match qs_get(query, "pretty") {
        None => false,
        Some(text) => text == "1" || text == "true",
    }
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(wj)
        .args([
            "build",
            app_src.join("src").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
            "--metadata",
            &format!("qs_pkg={}", qs_gen.join("metadata.json").display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let api_rs = fs::read_to_string(app_gen.join("domain").join("api.rs")).unwrap_or_default();
    eprintln!("api.rs:\n{api_rs}");
    assert!(
        !api_rs.contains("String::from(\"pretty\")")
            && !api_rs.contains("\"pretty\".to_string()"),
        "qs_get key &str must not own the pretty literal:\n{api_rs}"
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("qs_pkg"))
            .map(str::to_string)
            .collect();
        let dep = format!(
            "qs_pkg = {{ path = \"{}\", package = \"qs_src\" }}",
            qs_gen.display()
        );
        if let Some(idx) = lines.iter().position(|l| l.trim() == "[dependencies]") {
            lines.insert(idx + 1, dep);
        } else {
            lines.push("[dependencies]".to_string());
            lines.push(dep);
        }
        fs::write(&cargo_toml_path, format!("{}\n", lines.join("\n"))).unwrap();
    }

    let check = Command::new("cargo")
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "qs_get literal into demoted key must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

/// Product shape: path-dep is a `build/` tree with generated `lib.rs` and no
/// `metadata.json` / `--metadata` flags. Isolate above is GREEN with `--library`
/// metadata; notes-api product still emits `"pretty".to_string()`.
#[test]
fn qs_get_literal_from_generated_rs_path_dep_must_not_string_from() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let qs_build = tmp.path().join("qs_build");
    fs::create_dir_all(&qs_build).expect("mkdir qs_build");
    fs::write(
        qs_build.join("lib.rs"),
        r#"
#[inline]
pub fn get(query: String, key: &str) -> Option<String> {
    if key == "pretty" { Some(String::from("1")) } else { None }
}
"#,
    )
    .unwrap();
    fs::write(
        qs_build.join("Cargo.toml"),
        "[package]\nname = \"qs_src\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();

    let muter_build = tmp.path().join("muter_build");
    fs::create_dir_all(&muter_build).expect("mkdir muter_build");
    fs::write(
        muter_build.join("lib.rs"),
        "pub fn get(slot: &mut String) { let _ = slot; }\n",
    )
    .unwrap();

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("src").join("domain")).expect("mkdir src/domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"notes_qs_abi\"\n\n[dependencies]\nqs_pkg = {{ path = \"{}\" }}\nmuter_pkg = {{ path = \"{}\" }}\n",
            qs_build.display(),
            muter_build.display()
        ),
    )
    .unwrap();
    fs::write(app_src.join("src").join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(
        app_src.join("src").join("domain").join("mod.wj"),
        "pub mod api\n",
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("api.wj"),
        r#"
use qs_pkg::get as qs_get

fn own(value: string) -> string {
    value
}

pub fn query_wants_pretty(query: string) -> bool {
    let query = own(query)
    match qs_get(query, "pretty") {
        None => false,
        Some(text) => text == "1" || text == "true",
    }
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(wj)
        .current_dir(&app_src)
        .args([
            "build",
            "src",
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let api_rs = fs::read_to_string(app_gen.join("domain").join("api.rs")).unwrap_or_default();
    eprintln!("generated-rs path-dep api.rs:\n{api_rs}");
    assert!(
        !api_rs.contains("String::from(\"pretty\")")
            && !api_rs.contains("\"pretty\".to_string()"),
        "lib.rs-only path-dep qs_get key &str must stay bare:\n{api_rs}"
    );
}
