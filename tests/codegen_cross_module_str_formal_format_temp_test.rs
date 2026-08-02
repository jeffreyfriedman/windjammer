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

//! Gate (dogfood):
//!
//! Handlers demote `tenant_slug: string` → `tenant_slug: &str`, while call sites
//! build `let _temp0 = format!(…); fetch_*(deps.clone(), _temp0, …)` and pass
//! owned `String` → expected `&str`, found `String`.
//!
//! Formal + call site must agree (prefer WJ owned `string` throughout).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn demoted_str_formal_must_borrow_format_temp_or_stay_owned() {
    let files = [
        (
            "handlers.wj",
            r#"
pub struct AppDeps {
    pub tag: string,
}

pub fn fetch_accounts(deps: AppDeps, tenant_slug: string) -> string {
    deps.tag + ":" + tenant_slug
}

pub fn fetch_report(deps: AppDeps, tenant_slug: string, as_of: string) -> string {
    deps.tag + ":" + tenant_slug + ":" + as_of
}
"#,
        ),
        (
            "tool_invoke.wj",
            r#"
use handlers::{AppDeps, fetch_accounts, fetch_report}

pub fn invoke(deps: AppDeps, slug: string, as_of: string, which: int) -> string {
    if which == 1 {
        return fetch_accounts(deps, slug + "")
    }
    fetch_report(deps, slug + "", as_of + "")
}
"#,
        ),
        (
            "main.wj",
            r#"
use handlers::AppDeps
use tool_invoke::invoke

fn main() {
    let deps = AppDeps { tag: "x" + "" }
    let _ = invoke(deps, "demo" + "", "2026-01-01" + "", 1)
}
"#,
        ),
        ("mod.wj", "pub mod handlers\npub mod tool_invoke\n"),
    ];

    let map = test_utils::compile_project(&files);
    let handlers = map.get("handlers.rs").cloned().unwrap_or_default();
    let invoke = map.get("tool_invoke.rs").cloned().unwrap_or_default();

    let demoted = handlers.contains("tenant_slug: &str");
    if demoted {
        // Call sites must borrow format temps: fetch_accounts(..., &_temp0)
        let accounts_ok = invoke.contains("fetch_accounts(")
            && (invoke.contains("&_temp")
                || invoke.contains("fetch_accounts(deps.clone(), &")
                || invoke.contains("fetch_accounts(deps, &"));
        let report_ok = !invoke.contains("fetch_report(")
            || invoke.contains("&_temp")
            || invoke.contains("fetch_report(deps.clone(), &")
            || invoke.contains("fetch_report(deps, &");
        assert!(
            accounts_ok && report_ok,
            "&str formals require &_temp at call sites (tool_invoke). handlers:\n{handlers}\ninvoke:\n{invoke}"
        );
    } else {
        assert!(
            handlers.contains("tenant_slug: String"),
            "prefer owned String formals matching WJ. Got:\n{handlers}"
        );
    }
}
