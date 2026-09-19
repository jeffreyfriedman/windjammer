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

//! WDB-322: owned `String` key formal must not receive `&out_key` (LSQB push_adj).
//!
//! Product tip-out/gen:
//!   `lsqb_push_adj(map.clone(), &out_key, dst)` with `key: String` → E0308.
//! Twin of WDB-310/321 (LSQB string ownership).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

// `.len()`-only helpers demote to `&str`. Force owned String via struct store.
const SRC: &str = r#"
pub struct Adj {
    pub key: string,
    pub neighbor: i64,
}

pub fn push_adj(key: string, neighbor: i64) -> Adj {
    Adj {
        key: key,
        neighbor: neighbor,
    }
}

pub fn add_edge(edge_kind: string, src: i64, dst: i64) -> Adj {
    let out_key = edge_kind + "_out"
    let in_key = edge_kind + "_in"
    let a = push_adj(out_key, dst)
    let _b = push_adj(in_key, src)
    a
}
"#;

#[test]
fn wdb322_module_file_owned_string_key_must_not_receive_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-322 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-322 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn push_adj(key: String")
        || rs.contains("fn push_adj(mut key: String");
    assert!(
        owned,
        "WDB-322: expected owned String key formal on push_adj:\n{rs}"
    );
    let bad = rs.contains("&out_key") || rs.contains("&in_key") || rs.contains("push_adj(&");
    assert!(
        !bad,
        "WDB-322 RED: owned push_adj received &out_key/&in_key:\n{rs}"
    );
    test.cargo_check().expect("WDB-322 cargo-check");
}

#[test]
fn wdb322_tip_out_lsqb_push_adj_must_not_pass_ref_key_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("lsqb_typed_graph.rs"),
        tip.join("graph/lsqb_typed_graph.rs"),
        gen.join("graph/lsqb_typed_graph.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("lsqb");
        let bad = text.lines().any(|line| {
            line.contains("lsqb_push_adj(") && (line.contains("&out_key") || line.contains("&in_key"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-322: tip-out/gen lsqb_typed_graph missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-322 RED: tip-out/product passes &out_key/&in_key into owned String in:\n  {}",
        bad_paths.join("\n  ")
    );
}
