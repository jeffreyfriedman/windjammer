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

//! WDB-181: owned `OptDatedBaseline` into demoted `&OptDatedBaseline` must auto-borrow.
//!
//! Product residual (`&T`←Other), e.g. sysbench/tpch:
//!   `opt_dated_baseline_is_set(b)` / `opt_dated_baseline_is_set(tpch_sf1_dated_baseline())`
//!   while formal is `&OptDatedBaseline` → E0308 expected `&`, found owned.
//!
//! Inverse of WDB-178 (demoted & into owned). Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod baseline
pub mod claim
"#;

const BASELINE: &str = r#"
pub struct DatedBaseline {
    pub label: string,
    pub set: bool,
}

/// Read-only probe — tip demotes to `&DatedBaseline` (product opt_dated_baseline_is_set).
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
    // Product: baseline_is_set(b) into demoted & — must &b
    baseline_is_set(b)
}

pub fn check_temp() -> bool {
    // Product: baseline_is_set(make_baseline()) into demoted & — must &make_baseline()
    baseline_is_set(make_baseline())
}
"#;

fn wdb181_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("baseline.wj", BASELINE);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn wdb181_module_file_owned_baseline_into_demoted_ref_must_borrow() {
    let test = wdb181_fixture();
    let map = test
        .compile()
        .expect("WDB-181 multipass compile should succeed");
    let base_rs = map.get("baseline.rs").expect("baseline.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("WDB-181 baseline.rs:\n{base_rs}\nclaim.rs:\n{claim_rs}");

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
            "WDB-181 RED: demoted &DatedBaseline received owned local/temp. \
             Product: opt_dated_baseline_is_set(b). Got:\n{claim_rs}\n{base_rs}"
        );
    }

    if demoted {
        assert!(
            good || !(bad_local || bad_temp),
            "WDB-181: demoted &Baseline must auto-borrow. Got:\n{claim_rs}"
        );
    }
}

#[test]
fn wdb181_product_sysbench_must_borrow_baseline_into_demoted_is_set() {
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let baseline = gen.join("stats/opt_dated_baseline_port.rs");
    let sysbench = if tip.join("sysbench_opt_port.rs").exists() {
        tip.join("sysbench_opt_port.rs")
    } else {
        gen.join("relational/sysbench_opt_port.rs")
    };
    if !baseline.exists() || !sysbench.exists() {
        eprintln!("WDB-181: skip product — baseline/sysbench missing");
        return;
    }
    let base_text = std::fs::read_to_string(&baseline).expect("baseline");
    let sb_text = std::fs::read_to_string(&sysbench).expect("sysbench");
    let demoted = base_text.contains("fn opt_dated_baseline_is_set(baseline: &OptDatedBaseline");
    let bad = demoted
        && (sb_text.contains("opt_dated_baseline_is_set(b)")
            || sb_text.contains("opt_dated_baseline_is_set(sysbench_dated_baseline())")
            || sb_text.contains("opt_dated_baseline_is_set(tpch_sf1_dated_baseline())"))
        && !sb_text.contains("opt_dated_baseline_is_set(&b)")
        && !sb_text.contains("opt_dated_baseline_is_set(&sysbench_dated_baseline())");
    eprintln!(
        "WDB-181 product demoted={} bad={} path={}",
        demoted,
        bad,
        sysbench.display()
    );
    assert!(
        !bad,
        "WDB-181 RED: product still passes owned baseline into demoted is_set. {}",
        sysbench.display()
    );
}
