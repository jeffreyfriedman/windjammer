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

//! Regression: module file demoted dated artifact into owned must clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod dated
pub mod claim
"#;

const DATED: &str = r#"
pub struct DatedArtifact {
    pub label: string,
    pub ready: bool,
}

/// Owned artifact consumer.
pub fn live_publishable(claim_ready: bool, median: int, contended: bool, artifact: DatedArtifact) -> bool {
    claim_ready && !contended && artifact.ready && artifact.label.len() > 0 && median >= 0
}
"#;

const CLAIM: &str = r#"
use crate::dated::DatedArtifact
use crate::dated::live_publishable

/// Multi-use read-only probe demotes toward `&DatedArtifact`.
pub fn artifact_ok(artifact: DatedArtifact) -> bool {
    artifact.ready && artifact.label.len() > 0
}

pub fn claim_live_publishable(claim_ready: bool, median: int, contended: bool, artifact: DatedArtifact) -> bool {
    let _ok = artifact_ok(artifact)
    // live_publishable(…, artifact) while demoted — must clone.
    live_publishable(claim_ready, median, contended, artifact)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("dated.wj", DATED);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn module_file_demoted_dated_artifact_into_owned_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let dated_rs = map.get("dated.rs").expect("dated.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("dated.rs:\n{dated_rs}\nclaim.rs:\n{claim_rs}");

    let callee_owned = {
        let i = dated_rs.find("fn live_publishable").unwrap_or(0);
        let sl = &dated_rs[i..dated_rs.len().min(i + 200)];
        (sl.contains("artifact: DatedArtifact") || sl.contains("artifact:DatedArtifact"))
            && !(sl.contains("artifact: &DatedArtifact") || sl.contains("artifact:&DatedArtifact"))
    };
    let caller_demoted = {
        let i = claim_rs.find("fn claim_live_publishable").unwrap_or(0);
        let sl = &claim_rs[i..claim_rs.len().min(i + 180)];
        sl.contains("artifact: &DatedArtifact") || sl.contains("artifact:&DatedArtifact")
    };
    let call_ok = claim_rs.contains("live_publishable(claim_ready, median, contended, artifact.clone()")
        || claim_rs.contains("live_publishable(claim_ready, median, contended, (*artifact).clone()");
    let call_bad = claim_rs.contains("live_publishable(claim_ready, median, contended, artifact)")
        && !claim_rs.contains("artifact.clone()");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "RED: demoted &DatedArtifact into owned live_publishable without clone. \
             Got:\n{claim_rs}\n{dated_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "demoted &OptDated into owned must clone. Got:\n{claim_rs}"
        );
    }
}
