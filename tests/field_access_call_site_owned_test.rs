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

//! Regression: `reader.report_lines(demo_tenant().slug)` must not become
//! `&demo_tenant().slug.clone()` when the trait method expects owned `String`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn trait_method_owned_string_param_accepts_field_access_without_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "domain/tenant.wj",
        r#"
pub struct Tenant {
    pub slug: string,
}

pub fn demo_tenant() -> Tenant {
    Tenant { slug: "demo" }
}
"#,
    );
    test.add_file(
        "ports/readers.wj",
        r#"
trait ReportReader {
    fn report_lines(self, tenant_slug: string) -> Vec<int>
}
"#,
    );
    test.add_file(
        "adapters/seed.wj",
        r#"
use ports::readers::ReportReader

pub struct SeedReader {}

impl ReportReader for SeedReader {
    fn report_lines(self, tenant_slug: string) -> Vec<int> {
        if tenant_slug == "demo" {
            vec![1]
        } else {
            vec![]
        }
    }
}
"#,
    );
    test.add_file(
        "tests/stub_test.wj",
        r#"
use std::test
use domain::tenant::demo_tenant
use adapters::seed::SeedReader
use ports::readers::ReportReader

@test
fn seed_report_reader_returns_empty_for_unknown_slug() {
    let reader = SeedReader {}
    assert_eq(reader.report_lines(demo_tenant().slug).len(), 0)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod domain {
    pub mod tenant
}
pub mod ports {
    pub mod readers
}
pub mod adapters {
    pub mod seed
}
pub mod tests {
    pub mod stub_test
}
"#,
    );

    test.assert_compiles_without_error();

    let map = test.compile().expect("compile map");
    let rs = map
        .get("tests/stub_test.rs")
        .expect("tests/stub_test.rs");
    assert!(
        !rs.contains("report_lines(&"),
        "owned String trait param must not get borrow prefix at call site. Got:\n{rs}"
    );
}
