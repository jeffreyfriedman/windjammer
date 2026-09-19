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
    feature = "codegen_tests",
))]

//! WDB-321: owned `String` formals must not receive bare `&content` (LSQB edge CSV load).
//!
//! Product tip-out/gen:
//!   `lsqb_load_edge_csv_content(graph.clone(), &filename.clone(), &content)`
//!   with `content: String` → E0308.
//! Distinct from WDB-310 (`&content.clone()` on the vertex path).

use std::path::PathBuf;

#[test]
fn wdb321_tip_out_lsqb_edge_must_not_pass_bare_ref_content_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("lsqb_csv_loader.rs"),
        tip.join("graph/lsqb_csv_loader.rs"),
        gen.join("graph/lsqb_csv_loader.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lsqb");
        // Only the call site into owned formals — not split_lines(&content).
        let bad = text.lines().any(|line| {
            line.contains("lsqb_load_edge_csv_content(")
                && (line.contains("&content") || line.contains("&filename"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-321: tip-out/gen lsqb_csv_loader missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-321 RED: tip-out/product passes &content into owned String in:\n  {}",
        bad_paths.join("\n  ")
    );
}
