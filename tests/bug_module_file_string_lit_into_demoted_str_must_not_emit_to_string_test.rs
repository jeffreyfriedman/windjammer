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

//! Multipass: string literal into demoted `&str` method formal must stay bare
//! (not `.to_string()`).
//!
//! Ecosystem `wj-auth-api` (before handle_http split):
//! ```
//! app.handle("GET", path, …)  // method: string demoted to &str
//! ```
//! Tip emitted `handle("GET".to_string(), …)` → E0308 expected `&str`, found `String`.
//! Related to WDB-168 (`String::from`) but this gate targets `.to_string()` on method
//! formals across domain/adapters modules.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod domain
pub mod adapters
"#;

const DOMAIN: &str = r#"
use std::strings

pub fn handle(method: string, path: string) -> int {
    if method == "GET" {
        return strings.len(path)
    }
    0
}
"#;

const ADAPTERS: &str = r#"
use crate::domain::handle

pub fn dispatch(path: string) -> int {
    handle("GET", path)
}
"#;

fn auth_shape_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("domain.wj", DOMAIN);
    test.add_file("adapters.wj", ADAPTERS);
    test
}

#[test]
fn module_file_string_lit_into_demoted_str_must_not_emit_to_string() {
    let test = auth_shape_fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let domain_rs = map.get("domain.rs").expect("domain.rs");
    let adapters_rs = map.get("adapters.rs").expect("adapters.rs");

    eprintln!("domain.rs:\n{domain_rs}\nadapters.rs:\n{adapters_rs}");

    let demoted = {
        let i = domain_rs.find("fn handle").unwrap_or(0);
        let sl = &domain_rs[i..domain_rs.len().min(i + 120)];
        sl.contains("method: &str") || sl.contains("method:&str")
    };

    if demoted {
        let bad = adapters_rs.contains("\"GET\".to_string()")
            || adapters_rs.contains("String::from(\"GET\")");
        assert!(
            !bad,
            "demoted &str method formal must not receive .to_string()/String::from. Got:\n{adapters_rs}\n{domain_rs}"
        );
        assert!(
            adapters_rs.contains("handle(\"GET\"")
                || adapters_rs.contains("handle(\"GET\",")
                || adapters_rs.contains("handle(&"),
            "expected bare lit or borrow into demoted &str. Got:\n{adapters_rs}"
        );
    }
}
