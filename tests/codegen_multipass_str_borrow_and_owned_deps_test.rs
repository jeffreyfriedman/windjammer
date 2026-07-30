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

//! Multipass TDD for downstream-app bugs:
//! - Bug A: `string` formals demoted to `&str` must auto-borrow owned String / format temps at call sites.
//! - Bug B: forwarding to owned Custom callee must emit owned formal, not `&mut`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[path = "common/test_utils.rs"]
mod test_utils;

// --- Bug A: multipass &str formal + owned String / format concat call sites ---

#[test]
fn multipass_str_formal_borrows_owned_local_and_format_temp() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "http.wj",
        r#"
pub struct ServerResponse {
    pub status: int,
    pub body: string,
}

fn error_json(message: string) -> string {
    "{\"error\":\"" + message + "\"}"
}

pub fn json_cors_error(status: int, message: string) -> ServerResponse {
    ServerResponse {
        status: status,
        body: error_json(message),
    }
}
"#,
    );
    test.add_file(
        "routes.wj",
        r#"
use http::{json_cors_error, ServerResponse}

pub fn missing_auth() -> ServerResponse {
    json_cors_error(401, "missing Authorization Bearer token" + "")
}

pub fn with_msg(msg: string) -> ServerResponse {
    json_cors_error(400, msg)
}
"#,
    );
    test.add_file("mod.wj", "pub mod http;\npub mod routes;");

    let map = test.compile().expect("compile");
    let http = map.get("http.rs").expect("http.rs");
    let routes = map.get("routes.rs").expect("routes.rs");

    // Forwarding to owned `error_json(message)` should keep owned String; otherwise
    // demote to &str and borrow at every call site (dogfood).
    let owned_formal = http.contains("message: String");
    let str_formal = http.contains("message: &str") || http.contains("message: & String");
    assert!(
        owned_formal || str_formal,
        "http formal must be &str or String. Got:\n{http}"
    );

    if str_formal {
        assert!(
            routes.contains("json_cors_error(401, &")
                || routes.contains("json_cors_error(401,&")
                || (routes.contains("let _temp")
                    && routes.contains("&_temp")
                    && routes.contains("json_cors_error(401,")),
            "format concat to &str formal must pass &… Got:\n{routes}"
        );
        assert!(
            routes.contains("json_cors_error(400, &msg")
                || routes.contains("json_cors_error(400,&msg"),
            "owned String local to &str formal must pass &msg. Got:\n{routes}"
        );
    }
}

#[test]
fn same_file_str_formal_borrows_format_temp_and_owned_local() {
    let source = r#"
pub fn greet(name: string) -> string {
    "hello " + name
}

pub fn call_with_literal() -> string {
    greet("world" + "")
}

pub fn call_with_owned(msg: string) -> string {
    greet(msg)
}

fn main() {
    let _ = call_with_literal()
    let _ = call_with_owned("x" + "")
}
"#;
    let (generated, compiles) = test_utils::compile_single_check(source);

    let str_formal = generated.contains("name: &str") || generated.contains("name: String");
    assert!(str_formal, "greet formal must be &str or String. Got:\n{generated}");

    if generated.contains("name: &str") {
        assert!(
            generated.contains("greet(&")
                || (generated.contains("let _temp") && generated.contains("greet(&_temp")),
            "&str formal must borrow String / format temp at call site. Got:\n{generated}"
        );
    }
    assert!(compiles, "generated Rust must compile. Got:\n{generated}");
}

#[test]
fn multipass_append_opt_bool_str_formals_borrow_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "body.wj",
        r#"
pub fn append_opt_bool(body: string, key: string, value: bool, suffix: string) -> string {
    body + key + (if value { "1" + "" } else { "0" + "" }) + suffix
}
"#,
    );
    test.add_file(
        "routes.wj",
        r#"
use body::append_opt_bool

pub fn patch_flags(first: string, rest: string) -> string {
    append_opt_bool(first + "", "enabled" + "", true, rest + "")
}
"#,
    );
    test.add_file("mod.wj", "pub mod body;\npub mod routes;");

    let map = test.compile().expect("compile");
    let body = map.get("body.rs").expect("body.rs");
    let routes = map.get("routes.rs").expect("routes.rs");

    if body.contains("body: &str") || body.contains("key: &str") {
        assert!(
            routes.contains("append_opt_bool(&")
                || routes.contains("append_opt_bool( &")
                || (routes.contains("&_temp") && routes.contains("append_opt_bool(")),
            "append_opt_bool &str formals require borrowed args. Got:\n{routes}"
        );
        assert!(
            !routes.contains("append_opt_bool(_temp0,")
                || routes.contains("append_opt_bool(&_temp0"),
            "must not pass bare format temp to &str formal. Got:\n{routes}"
        );
    }
}

#[test]
fn multipass_opt_or_str_fallback_borrows_format_temp() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "http.wj",
        r#"
pub fn opt_or(value: Option<string>, fallback: string) -> string {
    match value {
        Some(v) => v,
        None => fallback + "",
    }
}
"#,
    );
    test.add_file(
        "routes.wj",
        r#"
use http::opt_or

pub fn defaults(payload_as_of: Option<string>) -> string {
    opt_or(payload_as_of, "2026-06-01" + "")
}
"#,
    );
    test.add_file("mod.wj", "pub mod http;\npub mod routes;");

    let map = test.compile().expect("compile");
    let http = map.get("http.rs").expect("http.rs");
    let routes = map.get("routes.rs").expect("routes.rs");

    if http.contains("fallback: &str") {
        assert!(
            routes.contains("opt_or(") && routes.contains("&_temp"),
            "format temp to &str fallback must pass &_temp. Got:\n{routes}\nhttp:\n{http}"
        );
        assert!(
            !routes.contains("opt_or(payload_as_of, _temp")
                || routes.contains("opt_or(payload_as_of, &_temp"),
            "must not pass bare _temp to &str formal. Got:\n{routes}"
        );
    }
}

// --- Bug B: multipass owned AppDeps forwarding ---

#[test]
fn multipass_create_forwards_owned_deps_to_mutating_callee() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "journal.wj",
        r#"
pub struct Writer {
    pub tag: string,
}

impl Writer {
    // ports: &self methods on fields of AppDeps — callee stays owned AppDeps.
    fn append(self, line: string) -> string {
        self.tag + ":" + line
    }
}

pub struct AppDeps {
    pub writer: Writer,
}

pub fn post_journal_entry(deps: AppDeps, tenant_slug: string, draft: string) -> string {
    deps.writer.append(tenant_slug + ":" + draft)
}
"#,
    );
    test.add_file(
        "routes.wj",
        r#"
use journal::{post_journal_entry, AppDeps, Writer}

pub fn create_journal_entry(deps: AppDeps, tenant_slug: string, draft: string) -> string {
    post_journal_entry(deps, tenant_slug + "", draft + "")
}
"#,
    );
    test.add_file("mod.wj", "pub mod journal;\npub mod routes;");

    let map = test.compile().expect("compile");
    let journal = map.get("journal.rs").expect("journal.rs");
    let routes = map.get("routes.rs").expect("routes.rs");

    assert!(
        journal.contains("fn post_journal_entry(deps: AppDeps")
            || journal.contains("fn post_journal_entry(mut deps: AppDeps"),
        "post_journal_entry must emit owned AppDeps. Got:\n{journal}"
    );
    assert!(
        !journal.contains("fn post_journal_entry(deps: &mut AppDeps")
            && !journal.contains("fn post_journal_entry(deps: &AppDeps"),
        "post must not demote to &/&mut AppDeps. Got:\n{journal}"
    );
    assert!(
        routes.contains("fn create_journal_entry(deps: AppDeps")
            || routes.contains("fn create_journal_entry(mut deps: AppDeps"),
        "create_journal_entry must emit owned AppDeps. Got:\n{routes}"
    );
    assert!(
        !routes.contains("fn create_journal_entry(deps: &mut AppDeps")
            && !routes.contains("fn create_journal_entry(deps: &AppDeps"),
        "create must not demote to &/&mut AppDeps. Got:\n{routes}"
    );
    assert!(
        !routes.contains("post_journal_entry(&deps")
            && !routes.contains("post_journal_entry(&mut deps"),
        "forwarding call must pass owned deps. Got:\n{routes}"
    );
}
