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

//! WDB-306: owned `String` formals must not receive `&owned.clone()` (wave1 bakeoff).
//!
//! Product tip-out/gen (~6× fill_bundle):
//!   `bakeoff_run_to_fill(&hw.clone(), &tpch_scale.clone(), &label.clone(), …)`
//!   with `hardware: String, scale: String, dated_label: String` → E0308.
//! Twin of WDB-301 (`&path.clone()` into owned). Prefer `hw.clone()` without `&`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Fill {
    pub hardware: string,
    pub scale: string,
    pub label: string,
}

pub fn bakeoff_run_to_fill(hardware: string, scale: string, dated_label: string) -> Fill {
    Fill {
        hardware: hardware,
        scale: scale,
        label: dated_label,
    }
}

pub fn fill_bundle(hw: string, tpch_scale: string, label: string) -> Fill {
    let f1 = bakeoff_run_to_fill(hw, tpch_scale, label)
    let _f2 = bakeoff_run_to_fill(hw, tpch_scale, label)
    f1
}
"#;

#[test]
fn wdb306_module_file_owned_string_must_not_receive_ref_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-306 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-306 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn bakeoff_run_to_fill(hardware: String")
        || rs.contains("fn bakeoff_run_to_fill(mut hardware: String");
    assert!(
        owned,
        "WDB-306: expected owned String formals on bakeoff_run_to_fill:\n{rs}"
    );
    let bad = rs.contains("&hw.clone()")
        || rs.contains("&tpch_scale.clone()")
        || rs.contains("&label.clone()")
        || rs.contains("bakeoff_run_to_fill(&");
    assert!(
        !bad,
        "WDB-306 RED: owned bakeoff_run_to_fill received &owned.clone() / &args:\n{rs}"
    );
    test.cargo_check().expect("WDB-306 cargo-check");
}

#[test]
fn wdb306_tip_out_bakeoff_must_not_pass_ref_clone_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_opt_bakeoff_port.rs"),
        tip.join("relational/wave1_opt_bakeoff_port.rs"),
        gen.join("relational/wave1_opt_bakeoff_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("bakeoff");
        if text.contains("bakeoff_run_to_fill(&hw.clone()") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-306: tip-out/gen bakeoff port missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-306 RED: tip-out/product passes &hw.clone() into owned String formals in:\n  {}",
        bad_paths.join("\n  ")
    );
}
