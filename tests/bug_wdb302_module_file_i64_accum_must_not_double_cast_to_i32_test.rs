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

//! WDB-302: i64/u64 triangle accumulator must not emit double casts / wrong divisor width.
//!
//! Product tip-out/gen LCC:
//!   WJ: `total = total + triangles as i64` then `(total / 3) as u64`
//!   tip: `total += triangles as i64 as i32` and `total / 3_u64`
//! MultiFile isolate may emit `as i64 as u64` + `/ 3_u64` (same class).
//! → E0277/E0308 width mismatches.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

// Product LCC: untyped `total = 0` under Custom return (not annotated i64).
const SRC: &str = r#"
pub struct LccEngine {
    pub total_triangles: u64,
}

pub fn sum_triangles(counts: Vec<u32>) -> LccEngine {
    let mut total = 0
    let mut i = 0
    while i < counts.len() {
        let triangles = counts[i]
        total = total + triangles as i64
        i = i + 1
    }
    LccEngine {
        total_triangles: (total / 3) as u64,
    }
}
"#;

#[test]
fn wdb302_module_file_i64_accum_must_not_double_cast_to_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-302 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-302 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as i64 as i32")
        || rs.contains("as i64 as u64")
        || rs.contains("/ 3_u64");
    assert!(
        !bad,
        "WDB-302 RED: MultiFile emitted double-cast / u64 divisor into i64 accum:\n{rs}"
    );
    test.cargo_check().expect("WDB-302 cargo-check");
}

#[test]
fn wdb302_tip_out_lcc_must_not_emit_i64_as_i32_double_cast() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_lcc_engine.rs"),
        tip.join("graph/graph_lcc_engine.rs"),
        gen.join("graph/graph_lcc_engine.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lcc");
        if text.contains("as i64 as i32") || text.contains("total / 3_u64") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-302: tip-out/gen LCC engine missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-302 RED: tip-out/product emits `as i64 as i32` or `total / 3_u64` in:\n  {}",
        bad_paths.join("\n  ")
    );
}
