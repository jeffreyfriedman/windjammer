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

//! owned `OptDatedArtifact` into demoted `&OptDatedArtifact` must borrow.
//!
//! Also: demoted live_row body forwards `&artifact` into owned
//! there; this gate focuses on the bakeoff call-site borrow.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod live
pub mod bakeoff
"#;

const LIVE: &str = r#"
pub struct DatedArtifact {
    pub label: string,
    pub ready: bool,
}

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
    if live_row_publishable(claim_ready, median, contended, a1) {
        return 1
    }
    0
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("live.wj", LIVE);
    test.add_file("bakeoff.wj", BAKEOFF);
    test
}

#[test]
fn module_file_owned_artifact_into_demoted_ref_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let live_rs = map.get("live.rs").expect("live.rs");
    let bake_rs = map.get("bakeoff.rs").expect("bakeoff.rs");

    eprintln!("live.rs:\n{live_rs}\nbakeoff.rs:\n{bake_rs}");

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
            "RED demoted &DatedArtifact received owned temp. \
Got:\n{bake_rs}\n{live_rs}"
        );
    }

    if demoted {
        assert!(
            good || !bad,
            "demoted &Artifact must auto-borrow. Got:\n{bake_rs}"
        );
    }
}
