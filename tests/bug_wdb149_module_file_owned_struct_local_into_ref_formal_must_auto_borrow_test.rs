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

//! WDB-149: owned struct local into demoted `&T` formal must auto-borrow at call site.
//!
//! WindjammerDB CQ-C5: after tip-sync of
//! `observability_otlp_http_post_body_trace_complete_ops_cross_signal_search_host_port`,
//! tip emits `cross_signal_search_host_run(req)` while the formal is
//! `req: &CrossSignalSearchRequest` → rustc E0308. Product dogfood inserts `&req`.
//!
//! Expected: call site emits `&req` (or formal stays owned). Idiomatic `.wj` has no `&`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod host
pub mod cap
"#;

const HOST: &str = r#"
pub struct SearchRequest {
    pub query_label: string
    pub k: u32
}

pub struct SearchResponse {
    pub hit_count: u32
}

pub fn search_host_run(req: SearchRequest) -> SearchResponse {
    SearchResponse {
        hit_count: req.k
    }
}
"#;

const CAP: &str = r#"
use crate::host::SearchRequest
use crate::host::search_host_run

pub fn cap_run() -> u32 {
    let req = SearchRequest {
        query_label: "otlp_trace_complete_ops",
        k: 2,
    }
    let resp = search_host_run(req)
    resp.hit_count
}
"#;

fn wdb149_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("host.wj", HOST);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb149_module_file_owned_struct_local_into_ref_formal_must_auto_borrow() {
    let test = wdb149_fixture();
    let map = test
        .compile()
        .expect("WDB-149 multipass compile should succeed (codegen may still be wrong)");
    let host_rs = map.get("host.rs").expect("host.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    let demoted_ref = {
        let i = host_rs.find("fn search_host_run").unwrap_or(0);
        let sl = &host_rs[i..host_rs.len().min(i + 120)];
        sl.contains("req: &SearchRequest")
    };
    let bad = demoted_ref
        && cap_rs.contains("search_host_run(req)")
        && !cap_rs.contains("search_host_run(&req)");

    eprintln!("WDB-149 host.rs:\n{host_rs}\ncap.rs:\n{cap_rs}");
    eprintln!("demoted_ref={demoted_ref} bad={bad}");

    if bad {
        panic!(
            "WDB-149 RED: demoted &SearchRequest formal must auto-borrow owned local at call site. \
             Product: complete_ops cross_signal_search_host_run(req) → E0308."
        );
    }

    test.cargo_check().expect(
        "WDB-149: owned struct local into &T formal must cargo-check (auto-borrow or owned formal).",
    );
}
