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

//! P3.644: product `wj-csv` thin wrappers must auto-borrow into stdlib formals.
//!
//! ```ignore
//! pub fn parse(text: string) -> … { csv.parse(text) }   // formal: &str
//! pub fn write(rows: Vec<Vec<string>>) -> … { csv.write(rows) }  // formal: &[Vec<String>]
//! ```
//! Tip emits `csv::parse(text)` / `csv::write(rows)` → E0308.
//! Isolate `bug_std_csv_write_owned_rows_auto_borrow` can false-GREEN; gate the
//! product package. Do not rename wrappers to dodge homonyms.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn csv_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-csv");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-csv");
            if uncle.join("src").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[test]
fn csv_stdlib_owned_args_must_auto_borrow_product() {
    let pkg = csv_pkg().unwrap_or_else(|| {
        panic!(
            "wj-csv src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&pkg)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build wj-csv");
    assert!(
        build.status.success(),
        "P3.644 wj-csv transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.644 wj-csv lib.rs:\n{lib}");

    let bad_parse = lib
        .lines()
        .any(|l| l.trim() == "csv::parse(text)" || l.contains("csv::parse(text)"));
    // Allow csv::parse(&text) in parse_with_headers; forbid bare owned in thin parse.
    let parse_fn = {
        let start = lib.find("pub fn parse(").unwrap_or(0);
        &lib[start..lib.len().min(start + 200)]
    };
    let thin_parse_bare = parse_fn.contains("csv::parse(text)") && !parse_fn.contains("csv::parse(&text)");

    let bad_write = lib.contains("csv::write(rows)") && !lib.contains("csv::write(&rows)");

    assert!(
        !thin_parse_bare && !bad_write,
        "P3.644 RED: csv::parse/write must auto-borrow owned args:\n  parse region: {parse_fn}\n  bad_parse={bad_parse} thin_bare={thin_parse_bare} bad_write={bad_write}\n{lib}"
    );
}
