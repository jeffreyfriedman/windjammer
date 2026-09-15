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

//! FAILING REPRO — typed int + `len() as int` while must not emit usize into i64.
//!
//! Product tip api-check (LedgerKit): `_len as int` + `let mut idx: int = 0` →
//!   `idx += 1_usize` / `while` usize vs i64 (~100+ E0277 on tip HEAD).
//! Nested untyped increment isolate may GREEN; this product-shaped fixture stays RED.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/int_while_len_as_int_must_not_emit_usize_arith.wj");

fn bad_usize_into_i64(rs: &str) -> bool {
    rs.contains("+= 1_usize")
        || rs.contains("+= 1 as usize")
        || rs.contains("+ 1_usize")
        || rs.contains("+ 1usize")
        || rs.contains("< _len as usize")
        || rs.contains("< h_len as usize")
        || rs.contains("<= h_len as usize")
        || (rs.contains("while ")
            && rs.contains(" as usize")
            && (rs.contains(": i64") || rs.contains("0_i64") || rs.contains(" as i64")))
}

#[test]
fn int_while_len_as_int_must_not_emit_usize_arith() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("mod.wj"), SOURCE).unwrap();
    let out = tmp.path().join("build");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "wj --module-file build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let mut rs = String::new();
    for entry in fs::read_dir(&out).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            rs.push_str(&fs::read_to_string(&path).unwrap_or_default());
            rs.push('\n');
        }
    }

    let bad = bad_usize_into_i64(&rs);
    if bad {
        eprintln!("RED P3.267 int while len-as-int must not emit usize into i64:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.267: typed int + len() as int must not emit usize arith into i64. Generated:\n{rs}"
    );
}
