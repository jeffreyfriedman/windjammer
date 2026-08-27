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
))]

//! FAILING REPRO — cross-crate associated `Type::new(copy_int)` / `::new(copy_float)`
//! must pass Copy args **by value**, not `&arg`.
//!
//! Dogfood (`finance-screens` + path-dep `windjammer-ui`):
//!   `DaysToCloseMetric::new(days)` → tip emits `DaysToCloseMetric::new(&days)` (E0308)
//!   `Progress::new(progress_percent as float)` → `Progress::new(&(… as f64))` (E0308)
//!   `MoneyDisplay::new(cents)` → `MoneyDisplay::new(&cents)` (E0308)
//!
//! External UI crates often have **no WJ signature**; tip must not guess `&` for
//! owned Copy formals (`i64` / `f64`) at associated `::new` call sites.
//!
//! Language-only; no product/repo names in asserts beyond the pattern.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write_ext_crate(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "extmetric"
version = "0.0.0"
edition = "2021"
[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"
pub struct Metric {
    pub days: i64,
}

impl Metric {
    pub fn new(days: i64) -> Self {
        Self { days }
    }
}

pub struct Gauge {
    pub value: f64,
}

impl Gauge {
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}
"#,
    )
    .unwrap();
}

fn wj_bin() -> String {
    std::env::var("WJ_BIN").unwrap_or_else(|_| {
        let tip = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
        if tip.is_file() {
            tip.display().to_string()
        } else {
            "wj".to_string()
        }
    })
}

/// `wj build` may overwrite `Cargo.toml`; rewrite dep + lib after transpile.
fn cargo_check_view(out: &Path, ext: &Path, crate_name: &str, view_rs: &str) {
    let rs_check = view_rs.replace("use crate::extmetric::", "use extmetric::");
    fs::write(out.join("view.rs"), &rs_check).unwrap();
    fs::write(
        out.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{crate_name}"
version = "0.0.0"
edition = "2021"
[lib]
path = "lib.rs"
[dependencies]
extmetric = {{ path = "{}" }}
"#,
            ext.display()
        ),
    )
    .unwrap();
    fs::write(
        out.join("lib.rs"),
        "pub mod view;\npub use view::*;\n",
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(out)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed:\n{}\ngenerated=\n{rs_check}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn cross_crate_associated_new_copy_int_must_not_borrow() {
    let tmp = TempDir::new().unwrap();
    let ext = tmp.path().join("extmetric");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    write_ext_crate(&ext);
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod view\n").unwrap();
    fs::write(
        src.join("view.wj"),
        r#"
use extmetric::Metric

pub fn make(days: int) -> Metric {
    Metric::new(days)
}
"#,
    )
    .unwrap();

    let status = Command::new(wj_bin())
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--module-file",
            "--no-cargo",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("run wj");
    assert!(status.success(), "wj build should succeed");

    let rs = fs::read_to_string(out.join("view.rs")).expect("view.rs");
    assert!(
        !rs.contains("Metric::new(&days)") && !rs.contains("Metric::new(&(days)"),
        "Copy i64 into cross-crate ::new must not borrow. Got:\n{rs}"
    );
    assert!(
        rs.contains("Metric::new(days)"),
        "expected by-value Metric::new(days). Got:\n{rs}"
    );

    cargo_check_view(&out, &ext, "copy_new_int_gate", &rs);
}

#[test]
fn cross_crate_associated_new_copy_float_cast_must_not_borrow() {
    let tmp = TempDir::new().unwrap();
    let ext = tmp.path().join("extmetric");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    write_ext_crate(&ext);
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod view\n").unwrap();
    fs::write(
        src.join("view.wj"),
        r#"
use extmetric::Gauge

pub fn make(pct: int) -> Gauge {
    Gauge::new(pct as float)
}
"#,
    )
    .unwrap();

    let status = Command::new(wj_bin())
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--module-file",
            "--no-cargo",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("run wj");
    assert!(status.success(), "wj build should succeed");

    let rs = fs::read_to_string(out.join("view.rs")).expect("view.rs");
    assert!(
        !rs.contains("Gauge::new(&(") && !rs.contains("Gauge::new(&(pct"),
        "Copy f64 cast into cross-crate ::new must not borrow. Got:\n{rs}"
    );
    assert!(
        rs.contains("Gauge::new(") && !rs.contains("Gauge::new(&"),
        "expected by-value Gauge::new(…). Got:\n{rs}"
    );

    cargo_check_view(&out, &ext, "copy_new_float_gate", &rs);
}
