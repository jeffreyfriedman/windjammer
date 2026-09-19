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

//! WDB-308: wave1 CLI residual — `u32 = 0_usize` and `args[i + 1]` index by u32.
//!
//! Twin of WDB-298/303 for remaining tip-out/gen sites (artifact/bench/publish CLI):
//!   `let mut i: u32 = 0_usize;` + `args[i + 1]` → E0308 / E0277.
//! Adjacency/CDLP/LCC were synced (P3.390); wave1 CLI still RED.

use std::path::PathBuf;

#[test]
fn wdb308_tip_out_wave1_cli_must_not_emit_u32_eq_0_usize_or_index_by_u32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_artifact_cli.rs"),
        tip.join("wave1_bench_cli.rs"),
        tip.join("wave1_publish_cli.rs"),
        tip.join("relational/wave1_artifact_cli.rs"),
        tip.join("relational/wave1_bench_cli.rs"),
        tip.join("relational/wave1_publish_cli.rs"),
        gen.join("relational/wave1_artifact_cli.rs"),
        gen.join("relational/wave1_bench_cli.rs"),
        gen.join("relational/wave1_publish_cli.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("cli");
        let bad_init = text.contains("u32 = 0_usize");
        let bad_index = text.contains("args[i + 1]") || text.contains("args[i+1]");
        if bad_init || bad_index {
            bad_paths.push(format!(
                "{} (init={} index={})",
                path.display(),
                bad_init,
                bad_index
            ));
        }
    }
    assert!(saw, "WDB-308: tip-out/gen wave1 CLI missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-308 RED: tip-out/product wave1 CLI still has u32=0_usize and/or args[i+1] in:\n  {}",
        bad_paths.join("\n  ")
    );
}
