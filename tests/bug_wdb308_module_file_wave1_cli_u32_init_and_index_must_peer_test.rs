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
//! Product WJ is untyped `let mut i = 0` with `-> u32` and `args.len()` / `args[i+1]`.
//! Adjacency/CDLP/LCC were synced (P3.390); wave1 CLI MultiFile+tip GREEN (P3.400).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn parse_samples(args: Vec<string>) -> u32 {
    let mut i = 0
    while i < args.len() {
        if args[i] == "--samples" {
            if i + 1 < args.len() {
                let _n = args[i + 1]
                return 1
            }
        }
        i = i + 1
    }
    0
}
"#;

#[test]
fn wdb308_module_file_u32_return_index_loop_must_not_emit_0_usize_or_bare_i_plus_1() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-308 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-308 MultiFile lib.rs:\n{rs}");
    let bad_init = rs.contains("u32 = 0_usize") || rs.contains(": u32 = 0_usize");
    let bad_index = rs.contains("args[i + 1]") || rs.contains("args[i+1]");
    assert!(
        !bad_init && !bad_index,
        "WDB-308 RED: MultiFile u32 return + len() loop emitted init={bad_init} index={bad_index}:\n{rs}"
    );
    test.cargo_check().expect("WDB-308 cargo-check");
}

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
