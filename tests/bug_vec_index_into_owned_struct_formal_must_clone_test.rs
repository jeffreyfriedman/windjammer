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

//! P3.683: LedgerKit `sort_reconciliation_queue` — `less(out[j], out[j+1])`
//! must clone non-Copy indexed elems into owned formals (not move → E0507).
//! Distinct from P3.575 owned-let of `rows[0]` then reuse.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn vec_index_into_owned_struct_formal_must_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.wj"),
        r#"
struct Item {
    pub code: string,
    pub rank: int,
}

pub fn item_less(a: Item, b: Item) -> bool {
    if a.rank != b.rank {
        return a.rank < b.rank
    }
    a.code < b.code
}

pub fn sort_items(items: Vec<Item>) -> Vec<Item> {
    let mut out = items
    let mut j: int = 0
    while j + 1 < out.len() {
        if !item_less(out[j], out[j + 1]) {
            let tmp = out[j]
            out[j] = out[j + 1]
            out[j + 1] = tmp
        }
        j = j + 1
    }
    out
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.683 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.683 emit:\n{rs}");

    let call_ok = rs.contains("out[")
        && (rs.contains(".clone()")
            || rs.contains("item_less(") && rs.contains("clone"));
    // Bare `item_less(out[…], out[…])` without clone is the RED shape.
    let bare_move = rs.lines().any(|l| {
        let t = l.trim();
        t.contains("item_less(")
            && t.contains("out[")
            && !t.contains(".clone()")
    });
    assert!(
        call_ok && !bare_move,
        "P3.683 RED: vec index into owned struct formal must clone:\n{rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--offline"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "P3.683 cargo-check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
