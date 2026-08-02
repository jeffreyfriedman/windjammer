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
//! Platform emits `opt_or(payload.as_of, _temp0)` while formal is `fallback: &str`.
//! Single-file often borrows (`&_temp0`); full multipass still drops the `&`
//! (dominant class of ~129 WIP errors / subset on clean tip).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multipass_opt_or_format_temps_must_borrow_for_str_formal() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mcp_args_decode.wj",
        r#"
pub struct McpReadArgs {
    pub as_of: string,
    pub account_code: string,
}

struct McpReadArgsPayload {
    as_of: Option<string>,
    account_code: Option<string>,
}

fn opt_or(value: Option<string>, fallback: string) -> string {
    match value {
        Some(v) => v + "",
        None => fallback + "",
    }
}

pub fn parse_mcp_read_args(payload: McpReadArgsPayload) -> McpReadArgs {
    McpReadArgs {
        as_of: opt_or(payload.as_of, "2026-06-01" + ""),
        account_code: opt_or(payload.account_code, "1000" + ""),
    }
}
"#,
    );
    test.add_file(
        "routes.wj",
        r#"
use mcp_args_decode::{parse_mcp_read_args, McpReadArgs}

pub fn defaults() -> McpReadArgs {
    // Force another module to consume decode so multipass keeps signatures live.
    parse_mcp_read_args(mcp_args_decode::McpReadArgsPayload {
        as_of: None,
        account_code: None,
    })
}
"#,
    );
    // Payload is private — keep decode self-contained; routes only needs a public entry.
    // Rewrite routes to call a public helper instead.
    test = MultiFileTest::new();
    test.add_file(
        "mcp_args_decode.wj",
        r#"
pub struct McpReadArgs {
    pub as_of: string,
    pub account_code: string,
}

fn opt_or(value: Option<string>, fallback: string) -> string {
    match value {
        Some(v) => v + "",
        None => fallback + "",
    }
}

pub fn parse_defaults(as_of: Option<string>, account_code: Option<string>) -> McpReadArgs {
    McpReadArgs {
        as_of: opt_or(as_of, "2026-06-01" + ""),
        account_code: opt_or(account_code, "1000" + ""),
    }
}
"#,
    );
    test.add_file(
        "routes.wj",
        r#"
use mcp_args_decode::{parse_defaults, McpReadArgs}

pub fn defaults() -> McpReadArgs {
    parse_defaults(None, None)
}
"#,
    );
    test.add_file("mod.wj", "pub mod mcp_args_decode;\npub mod routes;");

    let map = test.compile().expect("compile");
    let decode = map.get("mcp_args_decode.rs").expect("mcp_args_decode.rs");

    let demoted = decode.contains("fallback: &str");
    if demoted {
        // Detect bare `_temp` arg: `opt_or(..., _temp` without `&_temp`.
        let mut unborrowed = false;
        for part in decode.split("opt_or(").skip(1) {
            let args = part.split(')').next().unwrap_or("");
            // second arg region after first comma
            if let Some((_, rest)) = args.split_once(',') {
                let rest = rest.trim();
                if rest.starts_with("_temp") {
                    unborrowed = true;
                    break;
                }
            }
        }
        assert!(
            !unborrowed,
            "multipass: &str fallback requires &_temp (dogfood). Got:\n{decode}"
        );
    } else {
        assert!(
            decode.contains("fallback: String"),
            "owned fallback formal is also fine. Got:\n{decode}"
        );
    }
}
