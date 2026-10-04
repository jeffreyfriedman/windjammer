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

//! P3.642: reused owned `String` into owned formal must clone on the **first** use.
//!
//! Product `wj-todo-cli` `stats` / `export`:
//! ```ignore
//! let snapshot = encode_store(store)
//! match decode_store(snapshot) {          // tip: move (no clone)
//!     Ok(copy) => {
//!         … match decode_store(snapshot)  // tip: snapshot.clone() — too late → E0382
//! ```
//! Tip clones only the second call. First move into owned `decode_store(text: String)`
//! invalidates the later `.clone()`. Distinct from P3.615 (`.into()` move) /
//! P3.616 (`.into().clone()` inference). Do not reshape todo-cli with manual clones.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn todo_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-todo-cli");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-todo-cli");
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

fn first_decode_moves_snapshot(rs: &str) -> bool {
    // Look for decode_store(snapshot) without .clone() before any later reuse.
    let mut saw_move = false;
    let mut saw_later_clone = false;
    for line in rs.lines() {
        if !line.contains("decode_store(") || !line.contains("snapshot") {
            continue;
        }
        if line.contains("decode_store(snapshot.clone()")
            || line.contains("decode_store(&snapshot")
        {
            if saw_move {
                saw_later_clone = true;
            }
            continue;
        }
        if line.contains("decode_store(snapshot)") || line.contains("decode_store(snapshot,") {
            if saw_move || saw_later_clone {
                // second bare move also bad
                return true;
            }
            saw_move = true;
        }
    }
    saw_move && saw_later_clone
}

#[test]
fn reused_owned_string_into_owned_formal_must_clone_first_use() {
    let app = todo_app().unwrap_or_else(|| {
        panic!(
            "wj-todo-cli src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&app)
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
        .expect("wj build wj-todo-cli");
    assert!(
        build.status.success(),
        "P3.642 wj-todo-cli transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run_rs = fs::read_to_string(out.join("adapters").join("run.rs")).unwrap_or_default();
    let decode_lines: Vec<&str> = run_rs
        .lines()
        .filter(|l| l.contains("decode_store") && l.contains("snapshot"))
        .collect();
    eprintln!("P3.642 decode_store(snapshot) sites:\n{}", decode_lines.join("\n"));

    assert!(
        !first_decode_moves_snapshot(&run_rs),
        "P3.642 RED: first decode_store(snapshot) must clone (or demote formal) when \
         snapshot is reused — not move then later .clone():\n{}",
        decode_lines.join("\n")
    );
}
