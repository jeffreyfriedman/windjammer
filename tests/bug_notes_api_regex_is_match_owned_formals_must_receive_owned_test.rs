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

//! Path-dep `is_match(pattern: String, text: String)` must receive owned args.
//!
//! Ecosystem `wj-notes-api` `note_matches_pattern` / `list_notes_for_query`:
//! `is_match("${pattern}", "")` emits `""` (`expected String, found &str`);
//! `is_match(pattern, note.body)` emits `&pattern, &note.body`.
//! Inverse of notes-api `log_tagged` under-borrow (that API demotes to `&str`).

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
fn regex_is_match_owned_formals_must_receive_owned() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let rx_src = tmp.path().join("rx_src");
    fs::create_dir_all(&rx_src).expect("mkdir rx_src");
    fs::write(
        rx_src.join("lib.wj"),
        r#"
// Keep String formals the way wj-regex does: consume into owned storage.
// Interpolation-only bodies demote to &str and miss the product bug.
fn keep(value: string) -> string {
    value
}

pub fn is_match(pattern: string, text: string) -> Result<bool, string> {
    let mut owned = Vec::new()
    owned.push(keep(pattern))
    owned.push(keep(text))
    Ok(owned.len() == 2)
}
"#,
    )
    .unwrap();
    let rx_gen = tmp.path().join("rx_gen");
    build_library(wj, &rx_src, &rx_gen);
    let rx_rs = fs::read_to_string(rx_gen.join("lib.rs")).unwrap_or_default();
    assert!(
        rx_rs.contains("pattern: String") && rx_rs.contains("text: String"),
        "fixture is_match must keep owned String formals:\n{rx_rs}"
    );

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("src").join("domain")).expect("mkdir src/domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"notes_rx\"\n\n[dependencies]\nrx_pkg = {{ path = \"{}\" }}\n",
            rx_gen.display()
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
use rx_pkg::is_match

fn own(value: string) -> string {
    value
}

pub fn validate_pattern(pattern: string) -> Result<bool, string> {
    let pattern = own(pattern)
    is_match("${pattern}", "")
}

pub fn title_matches(pattern: string, title: string) -> Result<bool, string> {
    let pattern = own(pattern)
    let title = own(title)
    is_match(pattern, title)
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
            &format!("rx_pkg={}", rx_gen.join("metadata.json").display()),
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
    let empty_owned = api_rs.contains("String::from(\"\")")
        || api_rs.contains("String::new()")
        || api_rs.contains("\"\".to_string()");
    assert!(
        empty_owned,
        "is_match owned text slot must own empty literal, not pass \"\":\n{api_rs}"
    );
    assert!(
        !api_rs.contains("is_match(&pattern") && !api_rs.contains("is_match(& pattern"),
        "is_match owned pattern must not over-borrow:\n{api_rs}"
    );
    assert!(
        !api_rs.contains(", &title") && !api_rs.contains(", & title"),
        "is_match owned title must not over-borrow:\n{api_rs}"
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("rx_pkg"))
            .map(str::to_string)
            .collect();
        let dep = format!(
            "rx_pkg = {{ path = \"{}\", package = \"rx_src\" }}",
            rx_gen.display()
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
        "is_match owned formals must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
