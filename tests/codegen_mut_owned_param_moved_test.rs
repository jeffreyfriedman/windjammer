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

//! FAILING REPRO (LedgerKit E3.9.3 MCP reverse / tool_invoke):
//!
//! Owned composition deps params (`deps: AppDeps`) are sometimes codegen'd as
//! `deps: &mut AppDeps` when the body:
//!   - calls a consuming field method after a temporary / `.len()` check, or
//!   - is declared `mut deps: AppDeps` and later moved into an owned-param helper.
//!
//! Observed in LedgerKit:
//!   - `void_journal_entry(deps: AppDeps)` stays owned
//!   - `reverse_journal_entry(deps: AppDeps)` becomes `&mut AppDeps`
//!   - `post_journal_entry(deps: AppDeps)` becomes `&mut AppDeps`
//!   - `invoke_mcp_tool(mut deps: AppDeps)` became `&mut AppDeps`, then broke
//!     call sites that still pass owned `AppDeps` and helpers that still take owned.
//!
//! Platform workaround: drop `mut` on invoke and rebind locals before by-value calls.
//! Expected (green): preserve owned `AppDeps` in the signature (emit `mut deps: AppDeps`
//! binding only when needed); do not flip to `&mut AppDeps`.

#[path = "common/test_utils.rs"]
mod test_utils;

fn deps_fixture_source() -> &'static str {
    r#"
pub struct Writer {}

impl Writer {
    fn void(self, id: string) -> string {
        id + ""
    }

    fn reverse(self, id: string) -> string {
        id + ""
    }
}

pub struct Gateway {}

impl Gateway {
    fn evaluate(self, name: string) -> string {
        name + ""
    }
}

pub struct AppDeps {
    pub writer: Writer,
    pub gateway: Gateway,
}
"#
}

#[test]
fn reverse_after_len_check_must_keep_owned_deps_param() {
    let source = format!(
        r#"
{fixture}

pub fn void_entry(deps: AppDeps, id: string) -> string {{
    deps.writer.void(id + "")
}}

pub fn reverse_entry(deps: AppDeps, id: string) -> string {{
    let date = "2026-06-15" + ""
    if date.len() == 0 {{
        return "err" + ""
    }}
    deps.writer.reverse(id + "")
}}

fn main() {{
    let deps = AppDeps {{ writer: Writer {{}}, gateway: Gateway {{}} }}
    let _ = void_entry(deps, "seed-void" + "")
    let deps2 = AppDeps {{ writer: Writer {{}}, gateway: Gateway {{}} }}
    let _ = reverse_entry(deps2, "seed-rev" + "")
}}
"#,
        fixture = deps_fixture_source()
    );

    let (generated, ok) = test_utils::compile_single_check(&source);

    assert!(
        generated.contains("fn reverse_entry(deps: AppDeps")
            || generated.contains("fn reverse_entry(mut deps: AppDeps"),
        "reverse_entry must keep owned AppDeps (void_entry does). Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn reverse_entry(deps: &mut AppDeps")
            && !generated.contains("fn reverse_entry(mut deps: &mut AppDeps"),
        "owned AppDeps must not flip to &mut after date.len() + field.reverse. Got:\n{}",
        generated
    );
    // Call sites must pass owned AppDeps, not &mut (WIP bug: signature owned, call still &mut).
    assert!(
        !generated.contains("reverse_entry(&mut "),
        "call site must not auto-borrow &mut into owned AppDeps param. Got:\n{}",
        generated
    );
    assert!(
        ok,
        "generated Rust should compile with owned AppDeps call sites. Got:\n{}",
        generated
    );
}

#[test]
fn mut_owned_deps_moved_into_owned_helper_must_not_become_ref_mut() {
    let source = format!(
        r#"
{fixture}

pub fn void_entry(deps: AppDeps, id: string) -> string {{
    deps.writer.void(id + "")
}}

pub fn invoke_tool(mut deps: AppDeps, tool_name: string) -> string {{
    let name = tool_name + ""
    let _decision = deps.gateway.evaluate(name + "")
    let date = "2026-06-15" + ""
    if date.len() == 0 {{
        return "err" + ""
    }}
    let move_deps = deps
    void_entry(move_deps, "seed" + "")
}}

fn main() {{
    let deps = AppDeps {{ writer: Writer {{}}, gateway: Gateway {{}} }}
    let _ = invoke_tool(deps, "reverse_transaction" + "")
}}
"#,
        fixture = deps_fixture_source()
    );

    let (generated, ok) = test_utils::compile_single_check(&source);

    assert!(
        generated.contains("fn invoke_tool(mut deps: AppDeps")
            || generated.contains("fn invoke_tool(deps: AppDeps"),
        "invoke_tool must keep owned AppDeps. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn invoke_tool(deps: &mut AppDeps")
            && !generated.contains("fn invoke_tool(mut deps: &mut AppDeps"),
        "mut owned AppDeps must not flip to &mut when later moved into owned helper. Got:\n{}",
        generated
    );
    assert!(
        ok,
        "generated Rust should compile (main passes owned AppDeps). Got:\n{}",
        generated
    );
}
