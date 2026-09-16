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

//! WDB-232: `u32` index `i + 1` must not emit `1_u32 as usize` into u32 compare.
//!
//! Product residual (~6× u32←usize), tip-out/gen wave1_*_cli:
//!   `if (i + 1_u32 as usize) < args.len()` with `i: u32` → E0308 expected u32, found usize.
//! Prefer `i + 1 < args.len() as u32` (or keep both sides usize).

use std::path::PathBuf;

#[test]
fn wdb232_tip_out_wave1_cli_must_not_cast_u32_one_to_usize_in_compare() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_artifact_cli.rs"),
        tip.join("wave1_bench_cli.rs"),
        tip.join("wave1_publish_cli.rs"),
        gen.join("relational/wave1_artifact_cli.rs"),
        gen.join("relational/wave1_bench_cli.rs"),
        gen.join("relational/wave1_publish_cli.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("cli");
        let bad = text.contains("i + 1_u32 as usize");
        eprintln!("WDB-232 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-232 RED: tip-out/product casts u32 +1 to usize in compare. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-232: wave1 CLI tip-out/gen missing");
}
