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

//!
//!   while formal is `&OptDatedBaseline` → E0308 expected `&`, found owned.
//!

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod baseline
pub mod claim
"#;

const BASELINE: &str = r#"
pub struct DatedBaseline {
    pub label: string,
    pub set: bool,
}

pub fn baseline_is_set(baseline: DatedBaseline) -> bool {
    baseline.set && baseline.label.len() > 0
}

pub fn make_baseline() -> DatedBaseline {
    DatedBaseline { label: "cap", set: true }
}
"#;

const CLAIM: &str = r#"
use crate::baseline::DatedBaseline
use crate::baseline::baseline_is_set
use crate::baseline::make_baseline

pub fn check_local(b: DatedBaseline) -> bool {
    baseline_is_set(b)
}

pub fn check_temp() -> bool {
    baseline_is_set(make_baseline())
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("baseline.wj", BASELINE);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn module_file_owned_baseline_into_demoted_ref_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let base_rs = map.get("baseline.rs").expect("baseline.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("baseline.rs:\n{base_rs}\nclaim.rs:\n{claim_rs}");

    let demoted = {
        let i = base_rs.find("fn baseline_is_set").unwrap_or(0);
        let sl = &base_rs[i..base_rs.len().min(i + 120)];
        sl.contains("baseline: &DatedBaseline") || sl.contains("baseline:&DatedBaseline")
    };
    let bad_local = claim_rs.contains("baseline_is_set(b)")
        && !claim_rs.contains("baseline_is_set(&b)");
    let bad_temp = claim_rs.contains("baseline_is_set(make_baseline())")
        && !claim_rs.contains("baseline_is_set(&make_baseline())");
    let good = claim_rs.contains("baseline_is_set(&b")
        || claim_rs.contains("baseline_is_set(&make_baseline()");

    if demoted && (bad_local || bad_temp) && !good {
        panic!(
            "RED: demoted &DatedBaseline received owned local/temp. \
             Got:\n{claim_rs}\n{base_rs}"
        );
    }

    if demoted {
        assert!(
            good || !(bad_local || bad_temp),
            "demoted &Baseline must auto-borrow. Got:\n{claim_rs}"
        );
    }
}

