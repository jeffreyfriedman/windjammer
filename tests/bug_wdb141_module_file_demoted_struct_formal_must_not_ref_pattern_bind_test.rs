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
))]

//! WDB-141: demoted owned struct formal must emit `baseline: &T`, never `&baseline: &T`.
//!
//! WindjammerDB CQ-C5 (`opt_dated_baseline_port.rs`):
//!   `pub fn opt_dated_baseline_is_set(&baseline: &OptDatedBaseline)`
//! which moves out of a shared reference (E0507).
//!
//! Source is owned `baseline: OptDatedBaseline` with field reads only — tip may demote
//! to `&OptDatedBaseline`, but the binding must be a plain name, not a ref-pattern.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod baseline
pub mod cap
"#;

const BASELINE: &str = r#"
pub struct DatedBaseline {
    pub label: string,
    pub nanos: u64,
}

pub fn is_set(baseline: DatedBaseline) -> bool {
    if baseline.nanos == 0 {
        return false
    }
    if baseline.label.contains("UNSET") {
        return false
    }
    true
}
"#;

const CAP: &str = r#"
use crate::baseline::DatedBaseline
use crate::baseline::is_set

pub fn cap_unset() -> bool {
    is_set(DatedBaseline { label: "UNSET — x", nanos: 0 })
}

pub fn cap_set() -> bool {
    is_set(DatedBaseline { label: "2026-09-09 quiet", nanos: 42 })
}
"#;

fn wdb141_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("baseline.wj", BASELINE);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb141_module_file_demoted_struct_formal_must_not_ref_pattern_bind() {
    let test = wdb141_fixture();
    let map = test
        .compile()
        .expect("WDB-141 multipass compile should succeed (codegen may still be wrong)");
    let baseline_rs = map.get("baseline.rs").expect("baseline.rs");

    let bad_ref_pattern = baseline_rs.contains("&baseline:")
        || baseline_rs.contains("&b:")
        || regex_like_ref_pattern(baseline_rs);

    eprintln!("WDB-141 baseline.rs:\n{baseline_rs}");
    eprintln!("bad_ref_pattern={bad_ref_pattern}");

    if bad_ref_pattern {
        panic!(
            "WDB-141 RED: demoted struct formal must be `name: &T`, not `&name: &T`. \
             Product: opt_dated_baseline_is_set(&baseline: &OptDatedBaseline) → E0507."
        );
    }

    test.cargo_check().expect(
        "WDB-141: demoted struct formal must cargo-check without ref-pattern bind.",
    );
}

fn regex_like_ref_pattern(rs: &str) -> bool {
    // fn is_set(&baseline: &DatedBaseline) or similar
    rs.lines().any(|line| {
        let line = line.trim();
        line.contains("fn is_set")
            && line.contains("&")
            && line.contains(": &")
            && line.contains("&baseline")
    })
}
