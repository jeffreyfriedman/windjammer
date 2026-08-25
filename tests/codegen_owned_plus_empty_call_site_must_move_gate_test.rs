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

//! FAILING REPRO — when a cross-module formal stays owned `String` (not demoted),
//! inline `slug + ""` / move of local must NOT be auto-borrowed.
//!
//! Dogfood (`handlers.wj` owned formals + `routes.wj`):
//! - `fetch_accounts_api_json(deps, tenant_slug: String)` still owns
//! - tip must emit `fetch_accounts_api_json(deps, format!(…))` not `&format!(…)`
//!
//! Product may keep `clone_tenant_slug` only for those owned formals until green.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn owned_string_formal_must_not_receive_borrowed_plus_empty() {
    let mut test = MultiFileTest::new();
    // Force owned formal: consume by value into a struct field (no demotion to &str).
    test.add_file(
        "composition/handlers.wj",
        r#"
pub struct SlugBox {
    pub slug: string,
}

pub fn box_slug(tenant_slug: string) -> SlugBox {
    SlugBox { slug: tenant_slug }
}
"#,
    );
    test.add_file(
        "adapters/routes.wj",
        r#"
use crate::composition::handlers::box_slug
use crate::composition::handlers::SlugBox

pub fn route_arm(tenant_slug: string) -> SlugBox {
    box_slug(tenant_slug + "")
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map
        .get("adapters/routes.rs")
        .or_else(|| map.get("routes.rs"))
        .expect("routes.rs output");

    assert!(
        !rs.contains("box_slug(&")
            && !rs.contains("box_slug(&_temp"),
        "owned String formal must not receive borrow of inline concat temp. Got:\n{rs}"
    );

    let handlers = map
        .get("composition/handlers.rs")
        .or_else(|| map.get("handlers.rs"))
        .expect("handlers.rs output");
    assert!(
        handlers.contains("tenant_slug: String") || handlers.contains("tenant_slug: string"),
        "repro requires owned formal (not demoted &str). Got:\n{handlers}"
    );

    test.cargo_check()
        .expect("cargo check owned call site must succeed");
}
