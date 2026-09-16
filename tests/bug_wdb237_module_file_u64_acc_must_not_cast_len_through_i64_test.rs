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

//! WDB-237: `u64` accumulator `+= len()` must not emit `len() as u64 as i64`.
//!
//! Product residual (~18× u64←i64), tip-out/gen lsqb_typed_graph:
//!   `let mut total = 0_u64; total += self.countries.len() as u64 as i64;`
//! Expected: `len() as u64` (or keep u64 width). Related to WDB-215/227 width
//! casts but distinct: compound-add into u64 via double cast to i64.

use std::path::PathBuf;

#[test]
fn wdb237_tip_out_lsqb_vertex_count_must_not_cast_len_through_i64() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("lsqb_typed_graph.rs"),
        gen.join("graph/lsqb_typed_graph.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("typed");
        let has_u64_acc = text.contains("let mut total = 0_u64")
            || text.contains("mut total = 0_u64")
            || text.contains("-> u64");
        let bad = text.contains("len() as u64 as i64")
            || text.contains("len() as u64 as i64;");
        eprintln!(
            "WDB-237 has_u64_acc={} bad={} path={}",
            has_u64_acc,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-237 RED: tip-out/product casts len() through u64→i64 into u64 acc. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-237: lsqb_typed_graph missing");
}
