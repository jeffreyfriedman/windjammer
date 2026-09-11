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

//! WDB-129: multipass demotes a caller formal to `&T` but a callee still takes owned `T`,
//! and the call site passes the bare demoted binding (no `.clone()`) → E0308
//! (`expected OptDatedArtifact, found &OptDatedArtifact`).
//!
//! WindjammerDB CQ-C5 (~12× OptDatedArtifact cleared by dogfood; QueryFeedback residual).
//! Inverse of WDB-125. Struct must be **non-Copy** (string field) so demotion is meaningful.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod dated
pub mod claim
"#;

const DATED: &str = r#"
pub struct DatedArtifact {
    pub median_nanos: u64,
    pub label: string,
}

pub fn touch(artifact: DatedArtifact) -> u64 {
    artifact.median_nanos
}

pub fn publishable(claim_ready: bool, artifact: DatedArtifact) -> bool {
    claim_ready && artifact.median_nanos > 0
}
"#;

const CLAIM: &str = r#"
use crate::dated::DatedArtifact
use crate::dated::touch
use crate::dated::publishable

/// Read-only reuse demotes non-Copy `artifact` to `&DatedArtifact`.
/// If `publishable` keeps owned formal, call must `.clone()`.
pub fn claim_ok(claim_ready: bool, artifact: DatedArtifact) -> bool {
    let _ = touch(artifact)
    let a = publishable(claim_ready, artifact)
    let b = publishable(claim_ready, artifact)
    a && b
}

pub fn cap() -> bool {
    claim_ok(true, DatedArtifact { median_nanos: 10, label: "cap" })
}
"#;

fn wdb129_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("dated.wj", DATED);
    test.add_file("claim.wj", CLAIM);
    test
}

fn publishable_sig_owned(dated_rs: &str) -> bool {
    let Some(i) = dated_rs.find("fn publishable") else {
        return false;
    };
    let slice = &dated_rs[i..dated_rs.len().min(i + 160)];
    slice.contains("artifact: DatedArtifact")
        && !slice.contains("artifact: &DatedArtifact")
        && !slice.contains("artifact: & DatedArtifact")
}

#[test]
fn wdb129_module_file_demoted_caller_must_clone_into_owned_callee_formal() {
    let test = wdb129_fixture();
    let map = test
        .compile()
        .expect("WDB-129 multipass compile should succeed (codegen may still be wrong)");
    let dated_rs = map.get("dated.rs").expect("dated.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    let caller_demoted = claim_rs.contains("artifact: &DatedArtifact")
        || claim_rs.contains("artifact: & DatedArtifact");
    let publishable_owned = publishable_sig_owned(dated_rs);
    let clones = claim_rs.contains("publishable(claim_ready, artifact.clone()");
    let bare = claim_rs.contains("publishable(claim_ready, artifact)")
        && !claim_rs.contains("publishable(claim_ready, artifact.clone()");
    let bad = caller_demoted && publishable_owned && bare;

    eprintln!("WDB-129 dated.rs:\n{dated_rs}\nclaim.rs:\n{claim_rs}");
    eprintln!(
        "caller_demoted={caller_demoted} publishable_owned={publishable_owned} clones={clones} bare={bare} bad={bad}"
    );

    if bad {
        panic!(
            "WDB-129 RED: demoted caller &DatedArtifact into owned publishable formal must .clone(). \
             Product: opt_dated_quiet_run_live_publishable / query_feedback_cache_*."
        );
    }

    test.cargo_check().expect(
        "WDB-129: demoted caller &T into owned callee formal must compile (clone or unify demotion).",
    );
}
