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
//! Ecosystem `wj-auth-api` (string-label adapter path):
//! ```
//! app.handle("GET", path, …)  // method: string demoted to &str
//! ```
//! Tip emitted `handle("GET".to_string(), …)` → E0308 expected `&str`, found `String`.
//! Related to WDB-168 (`String::from`); this gate targets `.to_string()` on impl
//! method formals across domain/adapters modules (auth dogfood shape).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod domain
pub mod adapters
"#;

const DOMAIN: &str = r#"
use std::strings

pub struct App {
    pub hits: int,
}

impl App {
    pub fn new() -> App {
        App { hits: 0 }
    }

    /// Read-only label probes (parity with auth parse_method) tend to demote
    /// `method: string` → `&str` under multipass.
    pub fn handle(self, method: string, path: string) -> int {
        if method == "GET" {
            return strings.len(path)
        }
        if method == "POST" {
            return strings.len(path) + 1
        }
        if method == "PUT" {
            return strings.len(path) + 2
        }
        if method == "DELETE" {
            return strings.len(path) + 3
        }
        if method == "PATCH" {
            return strings.len(path) + 4
        }
        if method == "HEAD" {
            return strings.len(path) + 5
        }
        if method == "OPTIONS" {
            return strings.len(path) + 6
        }
        0
    }
}
"#;

const ADAPTERS: &str = r#"
use crate::domain::App

pub fn dispatch_get(path: string) -> int {
    let mut app = App::new()
    app.handle("GET", path)
}

pub fn dispatch_options(path: string) -> int {
    let mut app = App::new()
    app.handle("OPTIONS", path)
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
        let sl = &domain_rs[i..domain_rs.len().min(i + 160)];
        sl.contains("method: &str") || sl.contains("method:&str")
    };

    // Always assert cargo-check shape: if demoted, lit must not own; if owned,
    // .to_string() is fine. Force RED when we see the auth dogfood bug pattern.
    let owns_lit = adapters_rs.contains("\"GET\".to_string()")
        || adapters_rs.contains("\"OPTIONS\".to_string()")
        || adapters_rs.contains("String::from(\"GET\")")
        || adapters_rs.contains("String::from(\"OPTIONS\")");

    if demoted && owns_lit {
        panic!(
            "demoted &str method formal must not receive .to_string()/String::from. Got:\n{adapters_rs}\n{domain_rs}"
        );
    }

    if demoted {
        assert!(
            adapters_rs.contains("handle(\"GET\"")
                || adapters_rs.contains("handle(\"OPTIONS\"")
                || adapters_rs.contains(".handle(\"GET\"")
                || adapters_rs.contains(".handle(\"OPTIONS\"")
                || adapters_rs.contains("handle(&"),
            "expected bare lit or borrow into demoted &str. Got:\n{adapters_rs}"
        );
    }

    // Soft signal for compiler agent: fixture should demote like product.
    // When tip keeps owned String, gate stays GREEN (no false RED).
    if !demoted {
        eprintln!("note: method formal stayed owned String (product auth demoted to &str)");
    }
}
