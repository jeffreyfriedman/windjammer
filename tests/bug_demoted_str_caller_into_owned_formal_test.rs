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

//! P3.233 (`wj-scheduler` schedule wrappers): multipass app demotes a local `string`
//! formal to `&str`, then calls a cross-crate helper whose formal stays owned
//! `String` (moves into a struct field) without auto-own → E0308.
//!
//! Dogfood workaround: `parse_cron("${expr}")` at the call site.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn multipass_demoted_str_into_cross_crate_owned_formal_must_cargo_check() {
    let tmp = TempDir::new().expect("tempdir");

    let pkg_src = tmp.path().join("cron_src");
    fs::create_dir_all(&pkg_src).expect("mkdir cron_src");
    fs::write(
        pkg_src.join("cron_pkg.wj"),
        r#"
pub struct CronExpr {
    pub source: string,
}

pub fn parse_cron(expr: string) -> Result<CronExpr, string> {
    if expr == "" {
        return Err("empty")
    }
    Ok(CronExpr { source: expr })
}
"#,
    )
    .unwrap();

    let pkg_gen = tmp.path().join("cron_gen");
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
        .expect("cron_pkg build");
    assert!(
        pkg_build.status.success(),
        "cron_pkg library build failed:\n{}",
        String::from_utf8_lossy(&pkg_build.stderr)
    );

    let metadata_path = pkg_gen.join("metadata.json");
    assert!(metadata_path.exists(), "cron_pkg must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("domain")).expect("mkdir domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.cron_pkg]\npath = \"{}\"\npackage = \"cron_src\"\n",
            pkg_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("domain").join("schedule.wj"),
        r#"
use cron_pkg::parse_cron

pub fn check_schedule(expr: string) -> Result<string, string> {
    match parse_cron(expr) {
        Ok(cron) => Ok(cron.source),
        Err(e) => Err(e),
    }
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("domain").join("schedule.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("cron_pkg={}", metadata_path.display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).expect("read app Cargo.toml");
        let dep_line = format!(
            "cron_pkg = {{ path = \"{}\", package = \"cron_src\" }}",
            pkg_gen.display()
        );
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("cron_pkg"))
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

    let generated = fs::read_to_string(app_gen.join("schedule.rs")).unwrap_or_else(|_| {
        // module-file may nest
        fs::read_to_string(app_gen.join("domain").join("schedule.rs")).unwrap_or_default()
    });

    let check = Command::new("cargo")
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "RED P3.233: demoted &str into cross-crate owned String must cargo-check.\ngenerated:\n{generated}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
