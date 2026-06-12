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

/// TDD: E0308 when passing owned variable to cross-module function expecting String
///
/// Bug: The compiler generates `&id` when calling a function from another module
/// that expects `id: String` (owned). It should generate `id.clone()` when the
/// variable is used in multiple branches, or `id` when it's the last use.
///
/// Dogfooding source: playtest-mcp (main.wj calling mcp_protocol functions)
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_owned_arg_across_branches() {
    // Single-file: the compiler sees both definition and call site.
    // Even if it borrows `id`, the function signature should match.
    let source = r#"
pub fn make_response(id: string) -> string {
    let mut out = ""
    out = out + "{\"id\":"
    out = out + id
    out = out + "}"
    out
}

pub fn make_error(id: string, msg: string) -> string {
    let mut out = ""
    out = out + "{\"error\":\"" + msg + "\",\"id\":"
    out = out + id
    out = out + "}"
    out
}

pub fn handle(line: string) -> string {
    let id = "42"
    let method = "test"

    if method == "init" {
        return make_response(id)
    }
    if method == "list" {
        return make_response(id)
    }
    make_error(id, "unknown")
}

pub fn main() {
    let r = handle("test")
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // The key invariant: if a function signature says `id: String`,
    // the call site must NOT pass `&id` (E0308 in Rust).
    // Either:
    //   (a) function has `id: &str` and call site has `&id` — consistent, compiles
    //   (b) function has `id: String` and call site has `id` or `id.clone()` — consistent, compiles
    // What we check: the generated Rust is internally consistent.
    for fn_name in &["make_response", "make_error"] {
        // Find the function signature
        let sig_has_owned_string = rust.contains(&format!("fn {}(id: String", fn_name))
            || rust.contains(&format!("fn {}(id: string", fn_name));
        let sig_has_ref_str = rust.contains(&format!("fn {}(id: &str", fn_name));

        if sig_has_owned_string {
            // If the function takes owned String, call sites must NOT pass &id
            let bad_call = format!("{}(&id", fn_name);
            assert!(
                !rust.contains(&bad_call),
                "Function {}(id: String) but call site passes &id — E0308!\nGenerated:\n{}",
                fn_name, rust
            );
        }
        // If sig_has_ref_str, passing &id is fine
        if sig_has_ref_str {
            // This is the expected pattern: function takes &str, call site passes &id
            // Both compile correctly.
        }
    }
}
