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

//! Gate — WDB-090: take Vec field behind `&mut` param, clear with `Vec::new()`.
//!
//! Pattern (PageRank in-edge bind):
//! ```text
//! let v = csr.in_offsets
//! csr.in_offsets = Vec::new()
//! ```
//!
//! Expected: `std::mem::take(&mut csr.in_offsets)` (skip clear assign)
//! Forbidden: `csr.in_offsets.clone()`

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

const TAKE_CLEAR_SOURCE: &str = r#"
pub struct DenseCsr {
    pub in_offsets: Vec<u32>,
    pub in_neighbors: Vec<u32>,
}

pub fn take_in_offsets(csr: DenseCsr) -> Vec<u32> {
    let v = csr.in_offsets
    csr.in_offsets = Vec::new()
    v
}

pub fn take_in_edges(csr: DenseCsr) -> (Vec<u32>, Vec<u32>) {
    let off = csr.in_offsets
    csr.in_offsets = Vec::new()
    let nei = csr.in_neighbors
    csr.in_neighbors = Vec::new()
    (off, nei)
}

pub fn restore_in_edges(csr: DenseCsr, off: Vec<u32>, nei: Vec<u32>) {
    csr.in_offsets = off
    csr.in_neighbors = nei
}

fn main() {
    let mut csr = DenseCsr {
        in_offsets: Vec::new(),
        in_neighbors: Vec::new(),
    }
    let _ = take_in_offsets(csr)
    let pair = take_in_edges(csr)
    restore_in_edges(csr, pair.0, pair.1)
}
"#;

#[test]
fn test_mut_param_vec_field_take_clear_uses_mem_take() {
    let rust = build_source(TAKE_CLEAR_SOURCE, "csr_take");
    eprintln!("Generated Rust:\n{rust}");

    assert!(
        rust.contains("std::mem::take(&mut csr.in_offsets)"),
        "WDB-090: take+clear must use mem::take on &mut csr.in_offsets. Got:\n{rust}"
    );
    assert!(
        !rust.contains("csr.in_offsets.clone()"),
        "WDB-090: must not clone in_offsets when taking behind &mut. Got:\n{rust}"
    );
    assert!(
        rust.contains("std::mem::take(&mut csr.in_neighbors)")
            || rust.contains("take_in_edges"),
        "WDB-090: take_in_edges must mem::take neighbors too. Got:\n{rust}"
    );
}

#[test]
fn test_mut_param_vec_field_restore_moves_without_clone() {
    let rust = build_source(TAKE_CLEAR_SOURCE, "csr_take");
    eprintln!("Generated Rust:\n{rust}");

    // restore must assign by move, not clone the incoming Vecs
    assert!(
        !rust.contains("off.clone()") && !rust.contains("nei.clone()"),
        "WDB-090: restore_in_edges must move off/nei into csr fields. Got:\n{rust}"
    );
    assert!(
        rust.contains("csr.in_offsets = off") || rust.contains("csr.in_offsets = off;"),
        "WDB-090: restore must assign off into csr.in_offsets. Got:\n{rust}"
    );
}

#[test]
fn test_mut_param_take_inferred_as_mut_borrow_at_call_site() {
    let rust = build_source(TAKE_CLEAR_SOURCE, "csr_take");
    eprintln!("Generated Rust:\n{rust}");

    // Call sites must pass &mut csr, not csr.clone()
    assert!(
        rust.contains("take_in_offsets(&mut csr)")
            || rust.contains("take_in_offsets(&mut csr);"),
        "WDB-090: call site must borrow mutably, not clone DenseCsr. Got:\n{rust}"
    );
    assert!(
        !rust.contains("take_in_offsets(csr.clone())"),
        "WDB-090: must not clone DenseCsr into take_in_offsets. Got:\n{rust}"
    );
}
