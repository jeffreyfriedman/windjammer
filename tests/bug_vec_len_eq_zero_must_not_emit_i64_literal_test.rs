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

//! FAILING REPRO — product multipass `rows.len() == 0` emits `0_i64` (LedgerKit).
//!
//! Shallow isolate often rewrites to `is_empty()` (GREEN). Full LedgerKit
//! `--module-file` still emits `rows.len() == 0_i64` → E0277 usize vs i64.
//! This fixture keeps the comparison form and asserts no i64 zero literal.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::db::{Connection, Row}

fn int_to_string_pg(value: int) -> string {
    if value == 0 {
        return "0"
    }
    let mut n = value
    let mut digits = ""
    while n > 0 {
        let d = n % 10
        if d == 0 {
            digits = "0" + digits
        } else {
            digits = "1" + digits
        }
        n = n / 10
    }
    digits
}

trait Port {
    fn create(self) -> Result<string, string>
}

struct PgPort {}

impl Port for PgPort {
    fn create(self) -> Result<string, string> {
        let _ = int_to_string_pg(1)
        let conn = Connection::open("postgres://x")?
        let rows = conn.query("select 1", vec![])?
        // Keep len()==0 form — must not emit 0_i64 against usize len().
        if rows.len() == 0 {
            return Err("insert returned no rows")
        }
        Ok("ok" + "")
    }
}
"#;

fn bad_len_zero_i64(rs: &str) -> bool {
    rs.contains("len() == 0_i64")
        || rs.contains("len()==0_i64")
        || rs.contains(".len() == 0_i32")
}

#[test]
fn vec_len_eq_zero_must_not_emit_i64_literal() {
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

    let bad = bad_len_zero_i64(&rs);
    if bad {
        eprintln!("RED P3.305 vec len==0 must not emit 0_i64:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.305: rows.len() == 0 must emit is_empty() or 0_usize, not 0_i64. Generated:\n{rs}"
    );
}
