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

//! WDB-304: `return Some(x)` must not emit `Some(x.clone()).cloned()`.
//!
//! Product tip-out/gen semantic:
//!   WJ: `return Some(m)` / `return Some(rel)`
//!   tip: `return Some(m.clone()).cloned()` → E0599 (Option<T> has no `.cloned()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Metric {
    pub name: string,
}

pub struct Model {
    pub metrics: Vec<Metric>,
}

pub fn find_metric(model: Model, tool_name: string) -> Option<Metric> {
    for m in model.metrics {
        if m.name == tool_name {
            return Some(m)
        }
    }
    None
}
"#;

#[test]
fn wdb304_module_file_some_must_not_emit_cloned_chain() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-304 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-304 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(").cloned()"),
        "WDB-304 RED: MultiFile emitted Some(...).cloned() (illegal on Option<T>):\n{rs}"
    );
    test.cargo_check().expect("WDB-304 cargo-check");
}

#[test]
fn wdb304_tip_out_semantic_must_not_emit_some_cloned() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("semantic_mcp_port.rs"),
        tip.join("semantic/semantic_mcp_port.rs"),
        tip.join("semantic_model_port.rs"),
        tip.join("semantic/semantic_model_port.rs"),
        gen.join("semantic/semantic_mcp_port.rs"),
        gen.join("semantic/semantic_model_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("semantic");
        if text.contains(").cloned()") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-304: tip-out/gen semantic ports missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-304 RED: tip-out/product emits Some(...).cloned() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
