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

//! WDB-316: owned `String` formals must not receive `&out` (wave1 scale status).
//!
//! Product tip-out/gen:
//!   `out = wave1_scale_status_append_rows(&out, …)` with `out: String` → E0308.
//! Twin of WDB-314 (hardware report `append_bool(&out)`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn append_rows(out: string, ready: bool) -> string {
    if ready {
        return out + "|ready"
    }
    out + "|not"
}

pub fn format_status(ready: bool) -> string {
    let mut out = "hdr"
    out = append_rows(out, ready)
    out
}
"#;

#[test]
fn wdb316_module_file_owned_string_must_not_receive_ref_out() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-316 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-316 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_rows(out: String")
        || rs.contains("fn append_rows(mut out: String");
    assert!(
        owned,
        "WDB-316: expected owned String formal on append_rows:\n{rs}"
    );
    let bad = rs.contains("append_rows(&out") || rs.contains("append_rows(&mut out");
    assert!(
        !bad,
        "WDB-316 RED: owned append_rows received &out:\n{rs}"
    );
    test.cargo_check().expect("WDB-316 cargo-check");
}

#[test]
fn wdb316_tip_out_scale_status_must_not_pass_ref_out_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_scale_status_port.rs"),
        tip.join("relational/wave1_scale_status_port.rs"),
        gen.join("relational/wave1_scale_status_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("scale");
        if text.contains("wave1_scale_status_append_rows(&out") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-316: tip-out/gen scale_status missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-316 RED: tip-out/product passes wave1_scale_status_append_rows(&out) in:\n  {}",
        bad_paths.join("\n  ")
    );
}
