//! Multipass: `match Ok(markdown)` must move into cross-module owned `String`
//! formals (not `&markdown`) when the callee keeps owned emission via helpers.
//!
//! Ecosystem `wj-sitegen` `adapters/fs_site.wj` → `domain::generate_page`.

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

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multipass_match_ok_string_into_owned_cross_module_callee() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "domain/mod.wj",
        r#"
pub mod render
pub use render::generate_page
"#,
    );
    test.add_file(
        "domain/render.wj",
        r#"
use std::strings

fn take_owned(s: string) -> string {
    strings.trim(s)
}

pub fn generate_page(path: string, markdown: string) -> string {
    let name = take_owned(path)
    let body = take_owned(markdown)
    "${name}:${body}"
}
"#,
    );
    test.add_file(
        "adapters/mod.wj",
        r#"
pub mod fs_site
pub use fs_site::load_one
"#,
    );
    test.add_file(
        "adapters/fs_site.wj",
        r#"
use crate::domain::generate_page

fn read_md(path: string) -> Result<string, string> {
    Ok("${path}-body")
}

pub fn load_one(path: string) -> Result<string, string> {
    let source_path = "${path}"
    match read_md(path) {
        Ok(markdown) => Ok(generate_page(source_path, markdown)),
        Err(e) => Err(e),
    }
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );

    let map = test.compile().expect("compile");
    let domain = map.get("domain/render.rs").expect("domain/render.rs");
    let rs = map.get("adapters/fs_site.rs").expect("adapters/fs_site.rs");
    let owned = domain.contains("path: String") && domain.contains("markdown: String");
    let demoted = domain.contains("path: &str") && domain.contains("markdown: &str");
    assert!(
        owned || demoted,
        "generate_page must emit String or Phase-2 &str formals, got:\n{domain}"
    );
    if owned {
        assert!(
            !rs.contains("generate_page(source_path, &markdown)")
                && !rs.contains("generate_page(&source_path, markdown)")
                && !rs.contains("generate_page(&source_path, &markdown)"),
            "match Ok(markdown) must move into owned generate_page, got:\n{rs}"
        );
    } else {
        assert!(
            rs.contains("generate_page(") && !rs.contains("generate_page(source_path, markdown)"),
            "demoted generate_page must borrow owned match bindings, got:\n{rs}"
        );
    }
}
