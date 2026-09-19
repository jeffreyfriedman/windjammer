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

//! WDB-299: demoted `&Vec` into owned `Key::from_components` must clone/move.
//!
//! Product tip-out/gen (~8× layers): `Key::from_components(&parts)` while formal is
//! `components: Vec<Value>` → E0308. WJ source passes bare `parts`. Twin of WDB-241.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Value {
    pub n: int,
}

pub struct Key {
    pub parts: Vec<Value>,
}

pub fn from_components(components: Vec<Value>) -> Key {
    Key { parts: components }
}

pub fn key_from_parts(parts: Vec<Value>) -> Key {
    from_components(parts)
}
"#;

#[test]
fn wdb299_module_file_owned_from_components_must_not_receive_ref_parts() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-299 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-299 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn from_components(components: Vec<")
        || rs.contains("fn from_components(mut components: Vec<");
    assert!(
        owned,
        "WDB-299: expected owned Vec formal for from_components:\n{rs}"
    );
    let bad = rs.contains("from_components(&parts)");
    assert!(
        !bad,
        "WDB-299 RED: owned from_components received &parts (must clone/move):\n{rs}"
    );
    test.cargo_check().expect("WDB-299 cargo-check");
}

#[test]
fn wdb299_tip_out_key_from_components_must_not_receive_ref_parts() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_layer.rs"),
        tip.join("graph/graph_layer.rs"),
        tip.join("document_layer.rs"),
        tip.join("document/document_layer.rs"),
        tip.join("document_dremel_port.rs"),
        tip.join("document/document_dremel_port.rs"),
        tip.join("relational_layer.rs"),
        tip.join("relational/relational_layer.rs"),
        tip.join("vector_layer.rs"),
        tip.join("vector/vector_layer.rs"),
        tip.join("timeseries_layer.rs"),
        tip.join("timeseries/timeseries_layer.rs"),
        gen.join("graph/graph_layer.rs"),
        gen.join("document/document_layer.rs"),
        gen.join("document/document_dremel_port.rs"),
        gen.join("relational/relational_layer.rs"),
        gen.join("vector/vector_layer.rs"),
        gen.join("timeseries/timeseries_layer.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("layer");
        if text.contains("Key::from_components(&parts)") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-299: tip-out/gen layer ports missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-299 RED: tip-out/product passes &parts into owned Key::from_components in:\n  {}",
        bad_paths.join("\n  ")
    );
}
