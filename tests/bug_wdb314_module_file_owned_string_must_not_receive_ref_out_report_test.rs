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

//! WDB-314: owned `String` formals must not receive `&out` (wave1 hardware report).
//!
//! Product tip-out/gen:
//!   `out = append_bool(&out, live)` with `append_bool(out: String, value: bool)` → E0308.
//! Twin of WDB-306/312; prefer `out` move/`out.clone()` without leading `&`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn append_bool(out: string, value: bool) -> string {
    if value {
        return out + "|true"
    }
    out + "|false"
}

pub fn format_flags(live: bool, competitive: bool) -> string {
    let mut out = "hdr"
    out = append_bool(out, live)
    out = append_bool(out, competitive)
    out
}
"#;

#[test]
fn wdb314_module_file_owned_string_must_not_receive_ref_out() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-314 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-314 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_bool(out: String")
        || rs.contains("fn append_bool(mut out: String");
    assert!(
        owned,
        "WDB-314: expected owned String formal on append_bool:\n{rs}"
    );
    let bad = rs.contains("append_bool(&out")
        || rs.contains("append_bool(&mut out");
    assert!(
        !bad,
        "WDB-314 RED: owned append_bool received &out:\n{rs}"
    );
    test.cargo_check().expect("WDB-314 cargo-check");
}

#[test]
fn wdb314_tip_out_hardware_report_must_not_pass_ref_out_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_opt_hardware_report_port.rs"),
        tip.join("relational/wave1_opt_hardware_report_port.rs"),
        gen.join("relational/wave1_opt_hardware_report_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("report");
        if text.contains("append_bool(&out") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-314: tip-out/gen hardware report missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-314 RED: tip-out/product passes append_bool(&out) into owned String in:\n  {}",
        bad_paths.join("\n  ")
    );
}
