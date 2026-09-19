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

//! WDB-320: owned `String` formals must not receive bare `&dated_label` (wave1_publish_allows).
//!
//! Product tip-out/gen:
//!   `wave1_publish_allows(…, &dated_label, …)` with `dated_label: String` → E0308.
//! Distinct from WDB-312 (`&dated_label.clone()` into append_gate_rows).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

// Readonly `len`/`is_empty` demotes to `&str` (P3.264). Force owned String formals
// via struct store — mirrors product `wave1_publish_allows(..., dated_label: String, ...)`.
const SRC: &str = r#"
pub struct Gate {
    pub dated_label: string,
    pub live: bool,
}

pub fn publish_allows(live: bool, dated_label: string) -> Gate {
    Gate {
        dated_label: dated_label,
        live: live,
    }
}

pub fn write_report(dated_label: string, live: bool) -> Gate {
    let g = publish_allows(live, dated_label)
    let _g2 = publish_allows(live, dated_label)
    g
}
"#;

#[test]
fn wdb320_module_file_owned_string_must_not_receive_bare_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-320 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-320 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn publish_allows(live: bool, dated_label: String")
        || rs.contains("fn publish_allows(live: bool, mut dated_label: String");
    assert!(
        owned,
        "WDB-320: expected owned String formal on publish_allows:\n{rs}"
    );
    let bad = rs.contains("publish_allows(live, &dated_label")
        || rs.contains("publish_allows(live, &dated_label.clone()");
    assert!(
        !bad,
        "WDB-320 RED: owned publish_allows received &dated_label:\n{rs}"
    );
    test.cargo_check().expect("WDB-320 cargo-check");
}

#[test]
fn wdb320_tip_out_publish_allows_must_not_pass_bare_ref_dated_label() {
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
        let bad_line = text.lines().any(|line| {
            line.contains("wave1_publish_allows(") && line.contains("&dated_label")
        });
        if bad_line {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-320: tip-out/gen wave1 publish missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-320 RED: tip-out/product passes &dated_label into owned wave1_publish_allows in:\n  {}",
        bad_paths.join("\n  ")
    );
}
