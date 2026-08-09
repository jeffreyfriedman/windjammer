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

//! WDB-091 large-crate dogfood: `LdbcGraphLoader::datagen_defaults()` then
//! `load_edge_file("…")` must keep the path literal bare under full multipass.
//!
//! Mini multipass gates pass; this builds the real `wdb-layers` graph sources when
//! the sibling checkout exists (optional otherwise).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn wdb_layers_src() -> Option<PathBuf> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../windjammerdb/crates/wdb-layers/src");
    p.canonicalize().ok().filter(|q| q.join("mod.wj").exists())
}

#[test]
fn wdb_layers_datagen_defaults_path_literal_stays_bare() {
    let Some(src) = wdb_layers_src() else {
        eprintln!("skip: windjammerdb/crates/wdb-layers not checked out beside windjammer");
        return;
    };
    let out = tempfile::TempDir::new().unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let status = Command::new(wj)
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--library",
            "--module-file",
            "-o",
            out.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .status()
        .expect("wj build");
    assert!(status.success(), "wdb-layers library build failed");

    let rs = fs::read_to_string(
        out.path()
            .join("graph/graph_analytics_session_test.rs"),
    )
    .expect("graph_analytics_session_test.rs");

    assert!(
        rs.contains("LdbcGraphLoader::datagen_defaults()"),
        "expected datagen_defaults in session tests (remove struct-literal workaround). Got excerpt missing call"
    );
    assert!(
        !rs.contains("load_edge_file(\"fixtures/toy_triangle.e\".to_string())"),
        "WDB-091: datagen_defaults receiver must not force .to_string() on Borrowed path. Got:\n{rs}"
    );
    assert!(
        !rs.contains("load_edge_file(\"fixtures/toy_two_components.e\".to_string())"),
        "WDB-091: datagen_defaults receiver must not force .to_string() on Borrowed path. Got:\n{rs}"
    );
    assert!(
        rs.contains("load_edge_file(\"fixtures/toy_triangle.e\")")
            || rs.contains("load_edge_file(\"fixtures/toy_two_components.e\")"),
        "expected bare path literals after datagen_defaults. Got:\n{rs}"
    );
}
