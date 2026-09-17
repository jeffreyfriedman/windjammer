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

//! P3.329: `wj-timefmt` product multipass must not emit:
//! - `while month <= 12_i32 as i32` (i64 month vs i32 literal)
//! - `let time_and_tz = &parts[1].to_string()` into owned `String` formal
//!
//! Isolate month-loop fixtures can GREEN; full package shape is tip RED.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/timefmt_product_shape.wj");
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read fixture {p:?}: {e}"))
}

fn bad_timefmt(rs: &str) -> bool {
    rs.contains("12_i32 as i32")
        || rs.contains("<= 12_i32")
        || rs.contains("&parts[1].to_string()")
        || rs.contains("= &parts[")
}

#[test]
fn module_file_timefmt_product_must_not_mix_i32_month_or_ref_string() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), fixture()).unwrap();
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
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    if bad_timefmt(&generated) {
        eprintln!("P3.329 RED (snippet):\n{}", &generated[..generated.len().min(2500)]);
    }
    assert!(
        !bad_timefmt(&generated),
        "RED P3.329: timefmt product must not emit i32 month sentinel or &String into String:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("12_i32") || l.contains("time_and_tz") || l.contains("parts[1]"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (timefmt product shape):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
