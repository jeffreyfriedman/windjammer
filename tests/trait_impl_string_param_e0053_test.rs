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

//! E0053: trait method uses `string` (owned) but impl codegen emitted `&str` when the body only reads the param.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn trait_impl_string_param_matches_trait_signature() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "reader.wj",
        r#"
trait AccountReader {
    fn list_accounts(self, tenant_slug: string) -> Vec<int>
}
"#,
    );
    test.add_file(
        "seed.wj",
        r#"
use reader::AccountReader

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
        "mod.wj",
        r#"
pub mod reader
pub mod seed
"#,
    );

    test.assert_contains("reader.rs", "tenant_slug: String");
    test.assert_contains("seed.rs", "tenant_slug: String");
    test.assert_compiles_without_error();
}
