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

//! WDB-312: owned `String` formals must not receive `&dated_label.clone()` (wave1 publish).
//!
//! Product tip-out/gen:
//!   `wave1_publish_append_gate_rows(&out, …, &dated_label.clone(), …)`
//!   with `dated_label: String` → E0308.
//! Twin of WDB-306/310; prefer `dated_label.clone()` without `&` (and owned `out` move/clone).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn append_gate_rows(out: string, dated_label: string) -> string {
    out + "|" + dated_label
}

pub fn format_check(dated_label: string) -> string {
    let mut out = "hdr"
    out = append_gate_rows(out, dated_label)
    out
}
"#;

#[test]
fn wdb312_module_file_owned_string_must_not_receive_ref_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-312 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-312 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_gate_rows(out: String")
        || rs.contains("fn append_gate_rows(mut out: String");
    assert!(
        owned,
        "WDB-312: expected owned String formals on append_gate_rows:\n{rs}"
    );
    let bad = rs.contains("&dated_label.clone()")
        || rs.contains("&out.clone()")
        || rs.contains("append_gate_rows(&");
    assert!(
        !bad,
        "WDB-312 RED: owned append_gate_rows received &owned.clone() / &args:\n{rs}"
    );
    test.cargo_check().expect("WDB-312 cargo-check");
}

#[test]
fn wdb312_tip_out_publish_must_not_pass_ref_clone_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_publish_port.rs"),
        tip.join("relational/wave1_publish_port.rs"),
        tip.join("wave1_publish_cli.rs"),
        tip.join("relational/wave1_publish_cli.rs"),
        gen.join("relational/wave1_publish_port.rs"),
        gen.join("relational/wave1_publish_cli.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("publish");
        if text.contains("&dated_label.clone()") || text.contains("&dated.clone()") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-312: tip-out/gen wave1 publish missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-312 RED: tip-out/product passes &dated_label.clone()/&dated.clone() into owned String in:\n  {}",
        bad_paths.join("\n  ")
    );
}
