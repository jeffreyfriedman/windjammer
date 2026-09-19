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

//! WDB-303: `u32` index into `[T]` / `Vec` must cast to `usize` (not index by `u32`).
//!
//! Product tip-out/gen adjacency:
//!   `view.offsets[i + 1] - view.offsets[i as usize]` with `i: u32`
//! → E0277 `[u32]` cannot be indexed by `u32`. Prefer `(i + 1) as usize` / keep one width.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn degree_at(offsets: Vec<u32>, i: u32) -> u32 {
    offsets[i + 1] - offsets[i]
}
"#;

#[test]
fn wdb303_module_file_u32_index_into_vec_must_cast_to_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-303 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-303 MultiFile lib.rs:\n{rs}");
    // Bad: offsets[i + 1] or offsets[i] when i is u32 without as usize
    let has_u32_i = rs.contains("i: u32") || rs.contains("i:u32");
    let bad = has_u32_i
        && (rs.contains("offsets[i + 1]")
            || rs.contains("offsets[i+1]")
            || (rs.contains("offsets[i]") && !rs.contains("offsets[i as usize]")));
    assert!(
        !bad,
        "WDB-303 RED: MultiFile indexes Vec with bare u32 (need as usize):\n{rs}"
    );
    test.cargo_check().expect("WDB-303 cargo-check");
}

#[test]
fn wdb303_tip_out_adjacency_must_not_index_by_u32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_adjacency_port.rs"),
        tip.join("graph/graph_adjacency_port.rs"),
        gen.join("graph/graph_adjacency_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("adj");
        // Tip may use `let mut i = 0_usize` for offset walks (bare `i + 1` is fine).
        // Only flag when a u32-typed `i` indexes with bare `i + 1` (no cast).
        if u32_i_indexes_offsets_with_bare_plus_one(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-303: tip-out/gen adjacency port missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-303 RED: tip-out/product indexes offsets with bare u32 `i + 1` in:\n  {}",
        bad_paths.join("\n  ")
    );
}

/// True when some `let mut i: u32` / `i: u32` window also has bare `offsets[i + 1]`.
fn u32_i_indexes_offsets_with_bare_plus_one(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let bare = line.contains("offsets[i + 1]") || line.contains("offsets[i+1]");
        if !bare || line.contains("(i + 1) as usize") || line.contains("((i + 1) as usize)") {
            continue;
        }
        // Look backward for nearest `let mut i` / `i:` binding.
        for prev in lines[..idx].iter().rev().take(40) {
            let t = prev.trim();
            if t.starts_with("let mut i:") || t.starts_with("let mut i =") || t.contains("i: u32")
            {
                if t.contains("u32") {
                    return true;
                }
                if t.contains("usize") || t.contains("i32") || t.contains("i64") {
                    break;
                }
            }
            if t.starts_with("fn ") || t.starts_with("pub fn ") {
                break;
            }
        }
    }
    false
}
