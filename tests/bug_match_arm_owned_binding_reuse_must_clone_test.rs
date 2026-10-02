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

//! P3.574 / ecosystem `wj-cron`: `Ok(expr) => { matches_cron(expr); matches_cron(expr) }`
//! must clone on reuse (E0382). Function-body reuse already clones; match-arm
//! pattern bindings (`Ok(expr)`) were not registered for auto-clone.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

pub struct CronExpr {
    pub minute: string,
    pub hour: string,
}

pub fn matches_field(field: string, value: int) -> bool {
    strings.contains(field, "*") || value == 0
}

pub fn matches_cron(expr: CronExpr, minute: int, hour: int) -> bool {
    matches_field(expr.minute, minute) && matches_field(expr.hour, hour)
}

pub fn parse_cron(s: string) -> Result<CronExpr, string> {
    Ok(CronExpr { minute: "*", hour: "*" })
}

pub fn test_reuse() -> bool {
    match parse_cron("x") {
        Ok(expr) => {
            let a = matches_cron(expr, 1, 2)
            let b = matches_cron(expr, 3, 4)
            a && b
        },
        Err(_) => false,
    }
}
"#;

#[test]
fn match_arm_owned_binding_reuse_must_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.574 transpile failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.574 lib.rs:\n{rs}");

    let check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3574_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    assert!(
        rs.contains("matches_cron(expr.clone()")
            || rs.contains("matches_cron(&expr")
            || (rs.contains("fn matches_cron(expr: &CronExpr")
                && rs.contains("matches_cron(expr,")),
        "P3.574 RED: match-arm reuse of owned CronExpr must clone or borrow:\n{rs}"
    );
    assert!(
        check.status.success(),
        "P3.574 RED: match-arm CronExpr reuse must cargo-check:\n{rs}\n{err}"
    );
}
