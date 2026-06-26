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

//! Owned `string` trait method params must receive owned `String` at call sites when the
//! caller passes an owned `string` local — no `tenant_slug + ""` concat workaround.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn trait_method_owned_string_param_passes_identifier_by_value() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ports/readers.wj",
        r#"
trait AccountReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<int>
}
"#,
    );
    test.add_file(
        "adapters/seed.wj",
        r#"
use ports::readers::AccountReader

pub struct SeedReader {}

impl AccountReader for SeedReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<int> {
        if tenant_slug == "demo" {
            vec![1, 2, 3]
        } else {
            vec![]
        }
    }
}
"#,
    );
    test.add_file(
        "composition/handlers.wj",
        r#"
use ports::readers::AccountReader
use adapters::seed::SeedReader

pub struct AppDeps {
    account_reader: SeedReader,
}

pub fn fetch_accounts(deps: AppDeps, tenant_slug: string) -> Vec<int> {
    deps.account_reader.list_accounts(tenant_slug)
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod ports {
    pub mod readers
}
pub mod adapters {
    pub mod seed
}
pub mod composition {
    pub mod handlers
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map
        .get("composition/handlers.rs")
        .expect("composition/handlers.rs");

    assert!(
        rs.contains("list_accounts(tenant_slug") || rs.contains("list_accounts( tenant_slug"),
        "owned string local must pass by value to owned string param. Got:\n{rs}"
    );
    assert!(
        !rs.contains("list_accounts(&tenant_slug"),
        "must not borrow owned string for owned string trait param. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}
