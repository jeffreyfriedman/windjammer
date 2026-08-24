#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! WDB-101: cross-module map getters — borrowed `&GraphVertexI64Map` formals must
//! auto-borrow owned locals at call sites (Phase 148 graph regen dogfood).
//!
//! WindjammerDB pattern:
//!   `graph_vertex_i64_get(map: GraphVertexI64Map, …)` in `.wj`
//!   → Rust formal `map: &GraphVertexI64Map`
//!   → call site still `graph_vertex_i64_get(distances, v)` → E0308
//!
//! Gate A: main `wj` multipass must emit `&map` (or equivalent borrow).
//! Gate B: PRE dogfood binary (`#[ignore]`) — same assertion when present.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn wdb101_sources() -> (&'static str, &'static str, &'static str) {
    (
        r#"
pub mod vertex_map
pub mod consumer
"#,
        r#"
use std::collections::HashMap

pub struct GraphVertexI64Map {
    pub inner: HashMap<i64, i64>,
}

pub fn graph_vertex_i64_get(map: GraphVertexI64Map, vertex: i64) -> i64 {
    if map.inner.contains_key(vertex) {
        if let Some(value) = map.inner.get(vertex) {
            return value
        }
    }
    -1
}
"#,
        r#"
use crate::vertex_map::GraphVertexI64Map
use crate::vertex_map::graph_vertex_i64_get

pub fn read_vertex(map: GraphVertexI64Map, vertex: i64) -> i64 {
    graph_vertex_i64_get(map, vertex)
}
"#,
    )
}

fn assert_consumer_borrows_map(rs: &str) {
    assert!(
        rs.contains("graph_vertex_i64_get(&map") || rs.contains("graph_vertex_i64_get(& map"),
        "WDB-101: borrowed map getter must auto-borrow owned local. Got:\n{rs}"
    );
    assert!(
        !rs.contains("graph_vertex_i64_get(map,"),
        "WDB-101: must not pass owned GraphVertexI64Map to & formal. Got:\n{rs}"
    );
}

#[test]
fn wdb101_borrowed_vertex_map_getter_must_auto_borrow_at_call_site() {
    let (mod_wj, vertex_map, consumer) = wdb101_sources();
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", mod_wj);
    test.add_file("vertex_map.wj", vertex_map);
    test.add_file("consumer.wj", consumer);

    let map = test
        .compile()
        .expect("WDB-101 multipass compile should succeed");
    let rs = map.get("consumer.rs").expect("consumer.rs generated");
    assert_consumer_borrows_map(rs);
}

#[test]
fn wdb101_pre_ir_dogfood_must_auto_borrow_vertex_map_getter() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pre = manifest
        .join("..")
        .join(".worktrees")
        .join("wj-pre-ir")
        .join("target")
        .join("release")
        .join("wj");
    if !pre.exists() {
        eprintln!("skip WDB-101 PRE gate: {}", pre.display());
        return;
    }

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    let (mod_wj, vertex_map, consumer) = wdb101_sources();
    fs::write(src.join("mod.wj"), mod_wj).unwrap();
    fs::write(src.join("vertex_map.wj"), vertex_map).unwrap();
    fs::write(src.join("consumer.wj"), consumer).unwrap();

    let build = Command::new(&pre)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
            "--library",
            "--no-generate-cargo-toml",
        ])
        .output()
        .expect("run PRE wj");
    assert!(
        build.status.success(),
        "PRE wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("consumer.rs")).expect("consumer.rs");
    assert_consumer_borrows_map(&rs);
}
