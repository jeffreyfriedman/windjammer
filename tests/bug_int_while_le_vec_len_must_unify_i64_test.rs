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

//! `wj-glob` `match_segs`: `>= pats.len()` / `>= texts.len()` emit `len() as i64`,
//! but `while k <= texts.len()` leaves raw `usize` (E0308 i64 vs usize).
//!
//! `bug_int_index_while_len_must_not_emit_usize_add_test` only gates `ti >=`
//! and treats transpile-ok as success — it false-greens this `<=` hole.
//! Product: `packages/wj-glob` `$WJ test` (tip p3505 18:50).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
fn match_segs(pats: Vec<string>, pi: int, texts: Vec<string>, ti: int) -> bool {
    if pi >= pats.len() {
        return ti >= texts.len()
    }
    let pat = "${pats[pi]}"
    if pat == "**" {
        let mut k = ti
        while k <= texts.len() {
            if match_segs(pats, pi + 1, texts, k) {
                return true
            }
            k = k + 1
        }
        return false
    }
    if ti >= texts.len() {
        return false
    }
    match_segs(pats, pi + 1, texts, ti + 1)
}

pub fn ok(pats: Vec<string>, texts: Vec<string>) -> bool {
    match_segs(pats, 0, texts, 0)
}
"#;

fn le_len_ununified(rs: &str) -> bool {
    rs.lines().any(|line| {
        line.contains("k <=")
            && line.contains("texts.len()")
            && !line.contains("as i64")
            && !line.contains("as usize")
    })
}

#[test]
fn int_while_le_vec_len_must_unify_i64() {
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
        "wj build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    if !check.status.success() || le_len_ununified(&generated) {
        eprintln!("P3.516 RED:\n{generated}\n{err}");
    }
    assert!(
        !le_len_ununified(&generated),
        "while k <= texts.len() must unify like >= (len as i64 or k as usize):\n{generated}"
    );
    assert!(
        check.status.success(),
        "wj-glob match_segs while k <= len must cargo-check:\n{generated}\n{err}"
    );
}
