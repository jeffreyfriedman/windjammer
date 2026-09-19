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

//! WDB-318: owned `String` first formal must not receive `&out` (wave1 publish).
//!
//! Product tip-out/gen (distinct from WDB-312 dated_label):
//!   `out = wave1_publish_append_gate_rows(&out, …, …)` with `out: String` → E0308.
//! Twin of WDB-314/316.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn append_gate_rows(out: string, label: string) -> string {
    out + "|" + label
}

pub fn format_check(label: string) -> string {
    let mut out = "hdr"
    out = append_gate_rows(out, label)
    out
}
"#;

#[test]
fn wdb318_module_file_owned_string_first_formal_must_not_receive_ref_out() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-318 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-318 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_gate_rows(out: String")
        || rs.contains("fn append_gate_rows(mut out: String");
    assert!(
        owned,
        "WDB-318: expected owned String first formal on append_gate_rows:\n{rs}"
    );
    let bad = rs.contains("append_gate_rows(&out") || rs.contains("append_gate_rows(&mut out");
    assert!(
        !bad,
        "WDB-318 RED: owned append_gate_rows received &out as first arg:\n{rs}"
    );
    test.cargo_check().expect("WDB-318 cargo-check");
}

#[test]
fn wdb318_tip_out_publish_must_not_pass_ref_out_as_first_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_publish_port.rs"),
        tip.join("relational/wave1_publish_port.rs"),
        gen.join("relational/wave1_publish_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("publish");
        if text.contains("wave1_publish_append_gate_rows(&out") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-318: tip-out/gen wave1 publish missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-318 RED: tip-out/product passes wave1_publish_append_gate_rows(&out) in:\n  {}",
        bad_paths.join("\n  ")
    );
}
