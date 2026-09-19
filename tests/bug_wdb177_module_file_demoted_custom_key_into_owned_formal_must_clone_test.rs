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

//! WDB-177: demoted `&Custom` (non-Store) into owned Custom formal must clone.
//!
//! Product residual after tip-out sync — QueryFeedbackKey / OptDatedArtifact:
//!   `query_session_df_stats(…, key, …)` while `key: &QueryFeedbackKey` and formal owned
//!   `opt_dated_quiet_run_live_publishable(…, artifact)` while `artifact: &OptDatedArtifact`
//! → E0308 expected Custom, found &Custom.
//!
//! Same class as WDB-174 (Store) but covers non-Copy Custom with string fields used
//! in opt/feedback paths. Signature-driven. Prefer tip greens over dogfood.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod feedback
pub mod host
"#;

const FEEDBACK: &str = r#"
pub struct FeedbackKey {
    pub sql: string,
    pub n: int,
}

/// Owned key consumer (product query_session_df_stats / cache_put keep owned).
pub fn stats_with_key(row_count: int, key: FeedbackKey) -> int {
    row_count + key.n + key.sql.len()
}

pub fn cache_put(cache_n: int, key: FeedbackKey) -> int {
    cache_n + key.n
}
"#;

const HOST: &str = r#"
use crate::feedback::FeedbackKey
use crate::feedback::stats_with_key
use crate::feedback::cache_put

/// Multi-use read-only probe demotes toward `&FeedbackKey`.
pub fn key_label(key: FeedbackKey) -> int {
    key.sql.len() + key.n
}

pub fn run_stats(row_count: int, key: FeedbackKey) -> int {
    let _l = key_label(key)
    // Product: stats_with_key(…, key) while demoted — must clone into owned.
    stats_with_key(row_count, key)
}

pub fn run_put(cache_n: int, key: FeedbackKey) -> int {
    let _l = key_label(key)
    cache_put(cache_n, key)
}
"#;

fn wdb177_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("feedback.wj", FEEDBACK);
    test.add_file("host.wj", HOST);
    test
}

#[test]
fn wdb177_module_file_demoted_custom_key_into_owned_formal_must_clone() {
    let test = wdb177_fixture();
    let map = test
        .compile()
        .expect("WDB-177 multipass compile should succeed");
    let fb_rs = map.get("feedback.rs").expect("feedback.rs");
    let host_rs = map.get("host.rs").expect("host.rs");

    eprintln!("WDB-177 feedback.rs:\n{fb_rs}\nhost.rs:\n{host_rs}");

    let callee_owned = {
        let i = fb_rs.find("fn stats_with_key").unwrap_or(0);
        let sl = &fb_rs[i..fb_rs.len().min(i + 140)];
        (sl.contains("key: FeedbackKey") || sl.contains("key:FeedbackKey"))
            && !(sl.contains("key: &FeedbackKey") || sl.contains("key:&FeedbackKey"))
    };
    let caller_demoted = {
        let i = host_rs.find("fn run_stats").unwrap_or(0);
        let sl = &host_rs[i..host_rs.len().min(i + 120)];
        sl.contains("key: &FeedbackKey") || sl.contains("key:&FeedbackKey")
    };
    let call_ok = host_rs.contains("stats_with_key(row_count, key.clone()")
        || host_rs.contains("stats_with_key(row_count, (*key).clone()");
    let call_bad = host_rs.contains("stats_with_key(row_count, key)")
        && !host_rs.contains("stats_with_key(row_count, key.clone()");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "WDB-177 RED: demoted &FeedbackKey into owned stats_with_key without clone. \
             Product: query_session_df_stats(…, key) / OptDatedArtifact. Got:\n{host_rs}\n{fb_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-177: demoted &Custom into owned must clone. Got:\n{host_rs}"
        );
    }
}

#[test]
fn wdb177_tip_out_feedback_key_must_clone_into_owned_stats() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational");
    let provider = if tip.join("relational_df_provider_port.rs").exists() {
        tip.join("relational_df_provider_port.rs")
    } else {
        gen.join("relational_df_provider_port.rs")
    };
    let row = if tip.join("relational_row_port.rs").exists() {
        tip.join("relational_row_port.rs")
    } else {
        gen.join("relational_row_port.rs")
    };
    if !provider.exists() && !row.exists() {
        eprintln!("WDB-177: skip tip-out — provider/row missing");
        return;
    }
    let mut bad = Vec::new();
    for path in [provider, row] {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("rs");
        // Bare `key` into owned QueryFeedbackKey sites (demoted caller locals).
        if text.contains("query_session_df_stats(")
            && text.contains(", key,")
            && !text.contains(", key.clone(),")
            && (text.contains("key: &QueryFeedbackKey")
                || text.contains("let key =")
                || text.contains("&key"))
        {
            // Stronger: look for expected E0308 shape — call with bare key after demoted
            if text.contains("query_session_df_stats(row_count, histogram_sel, key,")
                && !text.contains("query_session_df_stats(row_count, histogram_sel, key.clone(),")
            {
                bad.push(format!("{}: bare key into session_df_stats", path.display()));
            }
        }
        if text.contains("query_feedback_cache_put(")
            && text.contains("query_feedback_cache_put(out.feedback_cache.clone(), key,")
            && text.contains("key: &QueryFeedbackKey")
        {
            // Only RED when a demoted `&QueryFeedbackKey` local/formal is passed bare into owned put.
            bad.push(format!("{}: demoted &key into owned cache_put", path.display()));
        }
    }
    eprintln!("WDB-177 tip-out bad={}", bad.len());
    assert!(
        bad.is_empty(),
        "WDB-177 RED: tip-out/gen still passes &QueryFeedbackKey into owned formals:\n{}",
        bad.join("\n")
    );
}
