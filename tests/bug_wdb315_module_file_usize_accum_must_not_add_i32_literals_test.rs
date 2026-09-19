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

//! WDB-315: `usize` accumulator must not add bare `_i32` literals (pg_wire field walk).
//!
//! Product tip-out/gen relational_pg_wire_port:
//!   `let mut pos = frame_offset + 7_usize;`
//!   `pos = pos + 4_i32 + 2_i32 + …` → E0277 (usize + i32).
//! Prefer `4_usize` / `4` with usize peer.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn skip_fields(frame_offset: usize, field_index: usize, count: usize) -> usize {
    let mut pos = frame_offset + 7
    let mut i = 0
    while i < count {
        if i == field_index {
            return pos
        }
        pos = pos + 4 + 2 + 4 + 2 + 4 + 2
        i = i + 1
    }
    pos
}
"#;

#[test]
fn wdb315_module_file_usize_accum_must_not_add_i32_literals() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-315 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-315 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_i32")
        && (rs.contains("pos = pos +") || rs.contains("pos += "));
    assert!(
        !bad,
        "WDB-315 RED: MultiFile mixed usize pos with _i32 literals:\n{rs}"
    );
    test.cargo_check().expect("WDB-315 cargo-check");
}

#[test]
fn wdb315_tip_out_pg_wire_must_not_add_i32_to_usize_pos() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("relational_pg_wire_port.rs"),
        tip.join("relational/relational_pg_wire_port.rs"),
        gen.join("relational/relational_pg_wire_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pg_wire");
        // Product bug shape: usize pos advanced by i32-suffixed literals.
        if text.contains("pos = pos + 4_i32") || text.contains("pos += 4_i32") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-315: tip-out/gen pg_wire missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-315 RED: tip-out/product adds _i32 literals to usize pos in:\n  {}",
        bad_paths.join("\n  ")
    );
}
