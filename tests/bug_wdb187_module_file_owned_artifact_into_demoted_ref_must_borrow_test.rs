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

//! WDB-187: owned `OptDatedArtifact` into demoted `&OptDatedArtifact` must borrow.
//!
//! Product residual (~6× &OptDated←OptDated), tip-out/gen bakeoff:
//!   `wave1_opt_live_row_publishable(…, artifact: &OptDatedArtifact)`
//!   call `wave1_opt_live_row_publishable(…, a1)` with owned temp → E0308.
//! Inverse of WDB-178; same class as WDB-181 (baseline) for Live artifacts.
//!
//! Also: demoted live_row body forwards `&artifact` into owned
//! `opt_dated_quiet_run_live_publishable` without clone (WDB-178) — covered
//! there; this gate focuses on the bakeoff call-site borrow.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod live
pub mod bakeoff
"#;

const LIVE: &str = r#"
pub struct DatedArtifact {
    pub label: string,
    pub ready: bool,
}

/// Read-only live gate — tip demotes to `&DatedArtifact` (product wave1_opt_live_row_publishable).
pub fn live_row_publishable(claim_ready: bool, median: int, contended: bool, artifact: DatedArtifact) -> bool {
    claim_ready && !contended && artifact.ready && artifact.label.len() > 0 && median >= 0
}

pub fn make_artifact() -> DatedArtifact {
    DatedArtifact { label: "live", ready: true }
}
"#;

const BAKEOFF: &str = r#"
use crate::live::DatedArtifact
use crate::live::live_row_publishable
use crate::live::make_artifact

pub fn suite_count(claim_ready: bool, median: int, contended: bool) -> int {
    let a1 = make_artifact()
    // Product: live_row_publishable(…, a1) into demoted & — must &a1
    if live_row_publishable(claim_ready, median, contended, a1) {
        return 1
    }
    0
}
"#;

fn wdb187_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("live.wj", LIVE);
    test.add_file("bakeoff.wj", BAKEOFF);
    test
}

#[test]
fn wdb187_module_file_owned_artifact_into_demoted_ref_must_borrow() {
    let test = wdb187_fixture();
    let map = test
        .compile()
        .expect("WDB-187 multipass compile should succeed");
    let live_rs = map.get("live.rs").expect("live.rs");
    let bake_rs = map.get("bakeoff.rs").expect("bakeoff.rs");

    eprintln!("WDB-187 live.rs:\n{live_rs}\nbakeoff.rs:\n{bake_rs}");

    let demoted = {
        let i = live_rs.find("fn live_row_publishable").unwrap_or(0);
        let sl = &live_rs[i..live_rs.len().min(i + 180)];
        sl.contains("artifact: &DatedArtifact") || sl.contains("artifact:&DatedArtifact")
    };
    let bad = bake_rs.contains("live_row_publishable(claim_ready, median, contended, a1)")
        && !bake_rs.contains("live_row_publishable(claim_ready, median, contended, &a1)");
    let good = bake_rs.contains("live_row_publishable(claim_ready, median, contended, &a1")
        || bake_rs.contains("live_row_publishable(claim_ready, median, contended, &make_artifact()");

    if demoted && bad && !good {
        panic!(
            "WDB-187 RED: demoted &DatedArtifact received owned temp. \
             Product: wave1_opt_live_row_publishable(…, a1). Got:\n{bake_rs}\n{live_rs}"
        );
    }

    if demoted {
        assert!(
            good || !bad,
            "WDB-187: demoted &Artifact must auto-borrow. Got:\n{bake_rs}"
        );
    }
}

#[test]
fn wdb187_product_bakeoff_must_borrow_artifact_into_demoted_live_row() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let bakeoff = if tip.join("wave1_opt_bakeoff_port.rs").exists() {
        tip.join("wave1_opt_bakeoff_port.rs")
    } else {
        gen.join("relational/wave1_opt_bakeoff_port.rs")
    };
    let live = gen.join("relational/wave1_opt_live_port.rs");
    if !bakeoff.exists() || !live.exists() {
        eprintln!("WDB-187: skip product — bakeoff/live missing");
        return;
    }
    let bake_text = std::fs::read_to_string(&bakeoff).expect("bakeoff");
    let live_text = std::fs::read_to_string(&live).expect("live");
    let demoted = live_text
        .contains("fn wave1_opt_live_row_publishable(claim_ready: bool, wdb_median_nanos: u64, contended: bool, artifact: &OptDatedArtifact");
    let bad = demoted
        && bake_text.contains("wave1_opt_live_row_publishable(")
        && (bake_text.contains(", a1)")
            || bake_text.contains(", a6)")
            || bake_text.contains(", a12)")
            || bake_text.contains(", ap)")
            || bake_text.contains(", asec)")
            || bake_text.contains(", au)"))
        && !bake_text.contains(", &a1)")
        && !bake_text.contains(", &a6)");
    eprintln!(
        "WDB-187 product demoted={} bad={} path={}",
        demoted,
        bad,
        bakeoff.display()
    );
    assert!(
        !bad,
        "WDB-187 RED: product still passes owned artifact into demoted live_row. {}",
        bakeoff.display()
    );
}
