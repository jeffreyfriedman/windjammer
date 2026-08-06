#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
    feature = "codegen_tests",
))]

//! FAILING REPRO — `mut` bindings in struct destructure patterns (Rust parity).
//!
//! Rust (stable) supports field-level mutability in struct patterns:
//!
//! ```rust
//! let VertexMap { mut inner } = map;
//! inner.insert(vertex, value);
//! ```
//!
//! Equivalent to moving `map.inner` into a mutable binding without cloning the map.
//! See: <https://doc.rust-lang.org/reference/patterns.html#binding-modes>
//!
//! Windjammer already supports irrefutable struct destructuring in `let`:
//!   `let Point { x, y } = point`
//!
//! but rejects `{ mut inner }` at parse time:
//!   `Expected field name in struct pattern`
//!
//! **Dogfooding impact:** `windjammerdb/crates/wdb-layers/src/graph/graph_vertex_map.wj`
//! needs in-place HashMap mutation. Without this syntax, codegen emits
//! `map.inner.clone()` when extracting via `let mut next = map.inner`, making
//! PageRank O(V×E) instead of O(E). Workaround: `let mut result = map; result.inner.insert(...)`.
//!
//! **Fix:** In struct pattern field lists, accept `mut ident` as a field pattern
//! (parser) and emit `mut inner` in Rust destructure (codegen). Also support
//! explicit form `field: mut binding` if shorthand differs.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture_source() -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/struct_destructure_mut_field.wj"),
    )
    .expect("struct_destructure_mut_field.wj fixture")
}

/// Gate 1: parser + full pipeline must accept Rust-style `{ mut field }` destructure.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_destructure_mut_field_compiles() {
    let source = fixture_source();
    match test_utils::compile_single_result(&source) {
        Ok(rust) => {
            println!("Generated Rust:\n{rust}");
        }
        Err(e) => panic!(
            "struct destructure with `mut` field should compile (Rust parity).\n\
             Bug: parser rejects `let Type {{ mut field }} = value`.\n\
             Error: {e}"
        ),
    }
}

/// Gate 2: codegen must move the HashMap, not clone it on each insert path.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_destructure_mut_field_hashmap_set_no_inner_clone() {
    let source = fixture_source();
    let rust = test_utils::compile_single(&source);

    assert!(
        rust.contains("mut inner") || rust.contains("let mut inner"),
        "expected mutable inner binding from struct destructure;\nGenerated:\n{rust}"
    );

    assert!(
        !rust.contains("inner.clone()"),
        "struct destructure should move HashMap, not clone inner on each update;\nGenerated:\n{rust}"
    );

    // rustc must accept emitted code
    let work = TempDir::new().expect("tempdir");
    let rs_path = work.path().join("struct_destructure_mut_field.rs");
    fs::write(&rs_path, &rust).expect("write rs");

    let output = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "-o",
            work.path()
                .join("libstruct_destructure_mut_field.rlib")
                .to_str()
                .unwrap(),
            rs_path.to_str().unwrap(),
        ])
        .output()
        .expect("rustc");

    if !output.status.success() {
        panic!(
            "generated Rust must compile with rustc:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Gate 3: contrast — today's workaround (`let mut result = map`) compiles and must not clone.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_destructure_mut_field_workaround_baseline_compiles() {
    let source = r#"
use std::collections::HashMap

pub struct VertexMap {
    pub inner: HashMap<i64, f64>,
}

pub fn vertex_map_set(map: VertexMap, vertex: i64, value: f64) -> VertexMap {
    let mut result = map
    result.inner.insert(vertex, value)
    result
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        !rust.contains("inner.clone()"),
        "workaround should move map and mutate in place;\nGenerated:\n{rust}"
    );
}
