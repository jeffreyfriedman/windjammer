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

//! WDB-206: demoted `&QueryFeedbackKey` into owned FeedbackKey formals must clone.
//!
//! Product residual (~8×), tip-out/gen row_port + df_provider:
//!   `key: &QueryFeedbackKey` forwarded bare into
//!   `query_session_df_stats(..., key: QueryFeedbackKey, ...)` /
//!   `query_feedback_cache_put/get(..., key: QueryFeedbackKey, ...)`.
//! Signature-driven demoted→owned clone (WDB-178/183 class).

use std::path::PathBuf;

fn product_paths() -> Vec<PathBuf> {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    vec![
        tip.join("relational_row_port.rs"),
        tip.join("relational_df_provider_port.rs"),
        gen.join("relational/relational_row_port.rs"),
        gen.join("relational/relational_df_provider_port.rs"),
    ]
}

#[test]
fn wdb206_tip_out_feedback_key_demoted_into_owned_must_clone() {
    let mut saw = false;
    for path in product_paths() {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(&path).expect("read");
        let demoted = text.contains("key: &QueryFeedbackKey");
        let bare_into_owned = text.contains("query_session_df_stats(row_count, histogram_sel, key,")
            || text.contains("query_feedback_cache_put(out.feedback_cache.clone(), key,")
            || text.contains("query_feedback_cache_get(host.feedback_cache, key)");
        let clones = text.contains("key.clone()")
            || text.contains("(*key).clone()")
            || text.contains(", key.clone(),");
        let bad = demoted && bare_into_owned && !clones;
        eprintln!(
            "WDB-206 demoted={} bare={} clones={} bad={} path={}",
            demoted,
            bare_into_owned,
            clones,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-206 RED: demoted &QueryFeedbackKey forwarded into owned FeedbackKey slot without clone. {}",
            path.display()
        );
    }
    if !saw {
        eprintln!("WDB-206: skip — row/df_provider missing");
    }
}
