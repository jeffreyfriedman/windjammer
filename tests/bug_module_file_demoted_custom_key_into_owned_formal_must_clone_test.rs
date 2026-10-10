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
//!   `query_session_df_stats(…, key, …)` while `key: &QueryFeedbackKey` and formal owned
//!   `opt_dated_quiet_run_live_publishable(…, artifact)` while `artifact: &OptDatedArtifact`
//! → E0308 expected Custom, found &Custom.
//!

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod feedback
pub mod host
"#;

const FEEDBACK: &str = r#"
pub struct FeedbackKey {
    pub sql: string,
    pub n: int,
}

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
    stats_with_key(row_count, key)
}

pub fn run_put(cache_n: int, key: FeedbackKey) -> int {
    let _l = key_label(key)
    cache_put(cache_n, key)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("feedback.wj", FEEDBACK);
    test.add_file("host.wj", HOST);
    test
}

#[test]
fn module_file_demoted_custom_key_into_owned_formal_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let fb_rs = map.get("feedback.rs").expect("feedback.rs");
    let host_rs = map.get("host.rs").expect("host.rs");

    eprintln!("feedback.rs:\n{fb_rs}\nhost.rs:\n{host_rs}");

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
            "RED: demoted &FeedbackKey into owned stats_with_key without clone. \
             Got:\n{host_rs}\n{fb_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "demoted &Custom into owned must clone. Got:\n{host_rs}"
        );
    }
}

