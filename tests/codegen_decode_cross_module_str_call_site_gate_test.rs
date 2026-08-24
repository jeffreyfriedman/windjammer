#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! FAILING REPRO — cross-module call to decode helper with `&str` formal must auto-borrow.
//!
//! Dogfood (`adapters/inbound/routes.wj` → `login_decode.parse_login_request`):
//! owned temp `body + ""` passed to a demoted `&str` formal must emit `&_temp…`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn cross_module_decode_call_must_auto_borrow_owned_body() {
    let files = [
        (
            "login.wj",
            r#"
pub fn parse_login_request(body: string) -> int {
    strings.len(body + "")
}
"#,
        ),
        (
            "routes.wj",
            r#"
use login::parse_login_request

pub fn handle(body: string) -> int {
    parse_login_request(body + "")
}
"#,
        ),
        (
            "main.wj",
            r#"
use routes::handle

fn main() {
    let _ = handle("x" + "")
}
"#,
        ),
        ("mod.wj", "pub mod login\npub mod routes\n"),
    ];

    let map = test_utils::compile_project(&files);
    let routes = map.get("routes.rs").cloned().unwrap_or_default();
    let login = map.get("login.rs").cloned().unwrap_or_default();

    // If the formal demoted to `&str`, call site must borrow the owned concat temp.
    let demoted = login.contains("body: &str");
    if demoted {
        assert!(
            routes.contains("&_temp")
                || routes.contains("parse_login_request(&")
                || routes.contains("parse_login_request(&_"),
            "demoted &str formal requires auto-borrow at call site. login=\n{login}\nroutes=\n{routes}"
        );
    } else {
        // Owned String formal: bare owned temp is fine (no E0308).
        assert!(
            routes.contains("parse_login_request("),
            "must call parse_login_request. routes=\n{routes}"
        );
    }
}
