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

//! WDB-091 large-crate dogfood: isolated multipass regen of session tests after
//! `ldbc_graph_loader.wj.meta` exists must keep Borrowed path literals bare for
//! `LdbcGraphLoader::datagen_defaults()` receivers (same as struct literals).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn wdb_layers_src() -> Option<PathBuf> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../windjammerdb/crates/wdb-layers/src");
    p.canonicalize().ok().filter(|q| q.join("mod.wj").exists())
}

fn wdb_layers_root(src: &PathBuf) -> PathBuf {
    src.parent().unwrap().to_path_buf()
}

fn assert_datagen_paths_bare(rs: &str, pass: &str) {
    assert!(
        rs.contains("LdbcGraphLoader::datagen_defaults()"),
        "{pass}: expected datagen_defaults in session tests. Got excerpt missing call"
    );
    assert!(
        !rs.contains("load_edge_file(\"fixtures/toy_triangle.e\".to_string())"),
        "{pass}: WDB-091 datagen_defaults must not force .to_string() on Borrowed path. Got:\n{rs}"
    );
    assert!(
        !rs.contains("load_edge_file(\"fixtures/toy_two_components.e\".to_string())"),
        "{pass}: WDB-091 datagen_defaults must not force .to_string() on Borrowed path. Got:\n{rs}"
    );
    assert!(
        rs.contains("load_edge_file(\"fixtures/toy_triangle.e\")")
            || rs.contains("load_edge_file(\"fixtures/toy_two_components.e\")"),
        "{pass}: expected bare path literals after datagen_defaults. Got:\n{rs}"
    );
}

#[test]
fn wdb_layers_datagen_defaults_path_literal_stays_bare() {
    let Some(src) = wdb_layers_src() else {
        eprintln!("skip: windjammerdb/crates/wdb-layers not checked out beside windjammer");
        return;
    };
    let root = wdb_layers_root(&src);
    let out = tempfile::TempDir::new().unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let cache = root.join(".wj-cache");
    let _ = fs::remove_dir_all(&cache);

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
    assert!(status.success(), "wdb-layers library build failed (pass 1)");

    let session_rs = out.path().join("graph/graph_analytics_session_test.rs");
    let rs1 = fs::read_to_string(&session_rs).expect("graph_analytics_session_test.rs");
    assert_datagen_paths_bare(&rs1, "pass1");

    // Isolated second pass: defining-module meta stays; only session_test regenerates.
    let _ = fs::remove_file(cache.join("graph/graph_analytics_session_test.wj.meta"));
    let _ = fs::remove_file(&session_rs);
    let session_wj = src.join("graph/graph_analytics_session_test.wj");
    let mut body = fs::read_to_string(&session_wj).expect("session_test.wj");
    body.push_str(&format!(
        "\n// touch {}\n",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(&session_wj, &body).expect("touch session_test.wj");

    let status2 = Command::new(wj)
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
        .expect("wj build pass2");

    // Revert touch so the sibling checkout stays clean for other agents.
    let original = body
        .lines()
        .filter(|l| !l.starts_with("// touch "))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(&session_wj, format!("{original}\n"));

    assert!(status2.success(), "wdb-layers library build failed (pass 2)");
    let rs2 = fs::read_to_string(&session_rs).expect("session_test.rs pass2");
    assert_datagen_paths_bare(&rs2, "pass2-isolated");
}
