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

//! Multipass path-dep: owned locals into read-only `string` formals that demote to
//! `&str` must auto-borrow (`log_tagged(&level, …, &message)`, `slugify(&title)`,
//! `parse_level(&probe)`).
//!
//! Ecosystem `wj-notes-api` → `wj-log` / `wj-inflect`: E0308 `expected &str, found String`.
//! Inverse of `bug_todo_cli_cross_crate_validate_field_must_auto_borrow_test`
//! (mixed formals: borrow field, keep owned value).

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
fn notes_api_owned_into_demoted_str_must_auto_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let log_src = tmp.path().join("log_src");
    fs::create_dir_all(&log_src).expect("mkdir log_src");
    fs::write(
        log_src.join("lib.wj"),
        r#"
pub fn parse_level(text: string) -> Option<int> {
    match text {
        "info" | "INFO" | "Info" => Some(1),
        _ => None,
    }
}

pub fn log_tagged(level: string, tag: string, message: string) {
    match level {
        "info" | "INFO" => (),
        _ => (),
    }
    match tag {
        _ => (),
    }
    match message {
        _ => (),
    }
}
"#,
    )
    .unwrap();
    let log_gen = tmp.path().join("log_gen");
    build_library(wj, &log_src, &log_gen);
    let log_meta = log_gen.join("metadata.json");
    assert!(log_meta.exists(), "log_pkg must emit metadata.json");
    let log_rs = fs::read_to_string(log_gen.join("lib.rs")).unwrap_or_default();
    assert!(
        log_rs.contains("text: &str") || log_rs.contains("text: & str"),
        "fixture parse_level must demote to &str:\n{log_rs}"
    );

    let inflect_src = tmp.path().join("inflect_src");
    fs::create_dir_all(&inflect_src).expect("mkdir inflect_src");
    fs::write(
        inflect_src.join("lib.wj"),
        r#"
pub fn slugify(text: string) -> string {
    match text {
        "" => "",
        _ => "x",
    }
}
"#,
    )
    .unwrap();
    let inflect_gen = tmp.path().join("inflect_gen");
    build_library(wj, &inflect_src, &inflect_gen);
    let inflect_rs = fs::read_to_string(inflect_gen.join("lib.rs")).unwrap_or_default();
    assert!(
        inflect_rs.contains("text: &str") || inflect_rs.contains("text: & str"),
        "fixture slugify must demote to &str:\n{inflect_rs}"
    );

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("src").join("domain")).expect("mkdir src/domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"notes_app\"\n\n[dependencies]\nlog_pkg = {{ path = \"{}\" }}\ninflect_pkg = {{ path = \"{}\" }}\n",
            log_gen.display(),
            inflect_gen.display()
        ),
    )
    .unwrap();
    fs::write(app_src.join("src").join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(
        app_src.join("src").join("domain").join("mod.wj"),
        "pub mod api\npub mod config\npub mod store\n",
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("api.wj"),
        r#"
use log_pkg::log_tagged

fn own(value: string) -> string {
    value
}

pub fn emit_access_log(level: string, message: string) {
    let level = own(level)
    let message = own(message)
    log_tagged(level, "notes", message)
}
"#,
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("config.wj"),
        r#"
use log_pkg::parse_level

fn own(value: string) -> string {
    value
}

pub fn normalize_log_level(text: string) -> string {
    let owned = own(text)
    let probe = "${owned}"
    match parse_level(probe) {
        Some(_) => owned,
        None => "info",
    }
}
"#,
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("store.wj"),
        r#"
use inflect_pkg::slugify

fn own(value: string) -> string {
    value
}

pub fn title_slug(title: string) -> string {
    let title = own(title)
    slugify(title)
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
            &format!("log_pkg={}", log_meta.display()),
            "--metadata",
            &format!("inflect_pkg={}", inflect_gen.join("metadata.json").display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let api_rs = fs::read_to_string(app_gen.join("domain").join("api.rs")).unwrap_or_default();
    let config_rs =
        fs::read_to_string(app_gen.join("domain").join("config.rs")).unwrap_or_default();
    let store_rs = fs::read_to_string(app_gen.join("domain").join("store.rs")).unwrap_or_default();
    eprintln!("api.rs:\n{api_rs}\nconfig.rs:\n{config_rs}\nstore.rs:\n{store_rs}");

    assert!(
        api_rs.contains("log_tagged(&level") || api_rs.contains("log_tagged(& level"),
        "log_tagged must borrow owned level:\n{api_rs}"
    );
    assert!(
        api_rs.contains("&message") || api_rs.contains("& message"),
        "log_tagged must borrow owned message:\n{api_rs}"
    );
    assert!(
        config_rs.contains("parse_level(&probe") || config_rs.contains("parse_level(& probe"),
        "parse_level must borrow owned probe:\n{config_rs}"
    );
    assert!(
        store_rs.contains("slugify(&title") || store_rs.contains("slugify(& title"),
        "slugify must borrow owned title:\n{store_rs}"
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| {
                let t = line.trim_start();
                !t.starts_with("log_pkg") && !t.starts_with("inflect_pkg")
            })
            .map(str::to_string)
            .collect();
        let deps = [
            format!(
                "log_pkg = {{ path = \"{}\", package = \"log_src\" }}",
                log_gen.display()
            ),
            format!(
                "inflect_pkg = {{ path = \"{}\", package = \"inflect_src\" }}",
                inflect_gen.display()
            ),
        ];
        if let Some(idx) = lines.iter().position(|l| l.trim() == "[dependencies]") {
            for (i, dep) in deps.into_iter().enumerate() {
                lines.insert(idx + 1 + i, dep);
            }
        } else {
            lines.push("[dependencies]".to_string());
            lines.extend(deps);
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
        "notes-api owned→&str path-dep must cargo-check.\napi:\n{api_rs}\nconfig:\n{config_rs}\nstore:\n{store_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
