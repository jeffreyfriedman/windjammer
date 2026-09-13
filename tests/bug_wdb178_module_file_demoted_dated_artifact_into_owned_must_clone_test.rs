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

//! WDB-178: demoted `&OptDatedArtifact` into owned formal must clone.
//!
//! Product residual (~20× OptDated mismatches), e.g. sysbench_opt_port:
//!   `sysbench_*_claim_live_publishable(…, artifact: &OptDatedArtifact)`
//!   calls `opt_dated_quiet_run_live_publishable(…, artifact: OptDatedArtifact)`
//! → E0308 expected `OptDatedArtifact`, found `&OptDatedArtifact`.
//!
//! Same class as WDB-174/177 (demoted Custom → owned) for dated-artifact paths.
//! Signature-driven. Prefer tip greens over dogfood.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod dated
pub mod claim
"#;

const DATED: &str = r#"
pub struct DatedArtifact {
    pub label: string,
    pub ready: bool,
}

/// Owned artifact consumer (product opt_dated_quiet_run_live_publishable).
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
    // Product: live_publishable(…, artifact) while demoted — must clone.
    live_publishable(claim_ready, median, contended, artifact)
}
"#;

fn wdb178_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("dated.wj", DATED);
    test.add_file("claim.wj", CLAIM);
    test
}

#[test]
fn wdb178_module_file_demoted_dated_artifact_into_owned_must_clone() {
    let test = wdb178_fixture();
    let map = test
        .compile()
        .expect("WDB-178 multipass compile should succeed");
    let dated_rs = map.get("dated.rs").expect("dated.rs");
    let claim_rs = map.get("claim.rs").expect("claim.rs");

    eprintln!("WDB-178 dated.rs:\n{dated_rs}\nclaim.rs:\n{claim_rs}");

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
            "WDB-178 RED: demoted &DatedArtifact into owned live_publishable without clone. \
             Product: opt_dated_quiet_run_live_publishable(…, artifact). Got:\n{claim_rs}\n{dated_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-178: demoted &OptDated into owned must clone. Got:\n{claim_rs}"
        );
    }
}

#[test]
fn wdb178_product_sysbench_must_clone_demoted_artifact_into_owned_publishable() {
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let quiet = gen.join("stats/opt_dated_quiet_run_port.rs");
    let sysbench = gen.join("relational/sysbench_opt_port.rs");
    if !quiet.exists() || !sysbench.exists() {
        eprintln!("WDB-178: skip product gate — quiet/sysbench missing");
        return;
    }
    let quiet_text = std::fs::read_to_string(&quiet).expect("quiet");
    let sb_text = std::fs::read_to_string(&sysbench).expect("sysbench");
    let publish_owned = {
        let i = quiet_text
            .find("fn opt_dated_quiet_run_live_publishable")
            .unwrap_or(0);
        let sl = &quiet_text[i..quiet_text.len().min(i + 220)];
        sl.contains("artifact: OptDatedArtifact") && !sl.contains("artifact: &OptDatedArtifact")
    };
    let caller_demoted = sb_text.contains("artifact: &OptDatedArtifact");
    let bare = sb_text.contains("opt_dated_quiet_run_live_publishable(")
        && sb_text.contains(", artifact)")
        && !sb_text.contains(", artifact.clone())");
    eprintln!(
        "WDB-178 product publish_owned={} demoted={} bare={}",
        publish_owned, caller_demoted, bare
    );
    assert!(
        !(publish_owned && caller_demoted && bare),
        "WDB-178 RED: sysbench still passes &OptDatedArtifact into owned live_publishable."
    );
}
