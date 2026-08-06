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

//! Gate — classic Vec field swap behind `&mut self` must not clone (WDB-088).
//!
//! Pattern (PageRank double-buffer):
//! ```text
//! let tmp = self.scores
//! self.scores = self.next
//! self.next = tmp
//! ```
//!
//! Expected: `std::mem::swap(&mut self.scores, &mut self.next)`
//! Forbidden: `self.scores.clone()` / `self.next.clone()` on the hot path.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn read_emitted_rs(out_dir: &Path, stem: &str) -> String {
    fs::read_to_string(out_dir.join(format!("{stem}.rs"))).unwrap_or_else(|_| {
        fs::read_dir(out_dir)
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
    })
}

fn build_source(source: &str, stem: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let src_path = tmp.path().join(format!("{stem}.wj"));
    fs::write(&src_path, source).unwrap();
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .args([
            "build",
            src_path.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    read_emitted_rs(&out, stem)
}

const SWAP_SOURCE: &str = r#"
pub struct PageRankBuffers {
    pub scores: Vec<f64>,
    pub next: Vec<f64>,
}

impl PageRankBuffers {
    pub fn swap(self) {
        let tmp = self.scores
        self.scores = self.next
        self.next = tmp
    }
}

fn main() {
    let mut buf = PageRankBuffers {
        scores: Vec::new(),
        next: Vec::new(),
    }
    buf.swap()
}
"#;

#[test]
fn test_self_field_vec_swap_must_use_mem_swap_not_clone() {
    let rust = build_source(SWAP_SOURCE, "pr_swap");
    assert!(
        rust.contains("std::mem::swap(&mut self.scores, &mut self.next)")
            || rust.contains("std::mem::swap(&mut self.next, &mut self.scores)"),
        "Vec field swap must emit std::mem::swap. Got:\n{rust}"
    );
    assert!(
        !rust.contains("self.scores.clone()") && !rust.contains("self.next.clone()"),
        "must not clone Vec fields during swap. Got:\n{rust}"
    );
}
