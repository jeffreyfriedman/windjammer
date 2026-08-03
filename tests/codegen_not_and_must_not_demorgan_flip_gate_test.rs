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

//! FAILING REPRO — inside `for` loops, `!row.flag && row.n != 0` must not become
//! `!(row.flag && row.n != 0)`.
//!
//! Standalone functions may keep `!row.flag && …` correctly; the same condition
//! inside a `for row in rows` body has been observed as De Morgan–wrong.
//!
//! Language-only; no product/repo names.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn codegen_not_and_in_for_loop_must_not_demorgan_flip() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub struct Row {
    pub balanced: bool,
    pub discrepancy: int,
}

pub fn labels(rows: Vec<Row>) -> string {
    let mut out = ""
    for row in rows {
        let part = if !row.balanced && row.discrepancy != 0 {
            "off"
        } else {
            "ok"
        }
        out = "${out}${part}"
    }
    out
}

fn main() {
    let _ = labels(Vec::new())
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build must succeed. stderr=\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("t.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });
    assert!(!rs.is_empty(), "expected generated .rs under {}", out.display());

    assert!(
        !rs.contains("!(row.balanced &&")
            && !rs.contains("!(row.balanced&&")
            && !rs.contains("!( row.balanced &&"),
        "must not De Morgan-flip `!row.balanced && …` into `!(row.balanced && …)` inside for-loop. Got:\n{rs}"
    );
    assert!(
        rs.contains("!row.balanced &&")
            || rs.contains("!row.balanced&&")
            || (rs.contains("if !row.balanced") && rs.contains("discrepancy")),
        "expected `!row.balanced && …` (or nested equivalent) inside for-loop. Got:\n{rs}"
    );
}
