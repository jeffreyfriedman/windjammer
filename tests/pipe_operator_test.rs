#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! Pipe operator `|>` (see docs/design/pipe-operator.md): parser desugaring,
//! precedence, diagnostics, and "same Rust as the direct call" equivalence.

#[path = "common/test_utils.rs"]
mod test_utils;

use test_utils::{compile_single, compile_single_result};

/// Strip whitespace so formatting differences do not matter.
fn norm(s: &str) -> String {
    s.replace("_i64", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Body of `pub fn <name>` as normalized text.
fn fn_body(generated: &str, name: &str) -> String {
    let start = generated
        .find(&format!("fn {}(", name))
        .unwrap_or_else(|| panic!("fn {} not found in:\n{}", name, generated));
    let rest = &generated[start..];
    let open = rest.find('{').unwrap();
    let mut depth = 0;
    let mut end = open;
    for (i, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    norm(&rest[open + 1..end])
}

fn err_of(code: &str) -> String {
    match compile_single_result(code) {
        Ok(g) => panic!("expected compile error, got:\n{}", g),
        Err(e) => e,
    }
}

#[test]
fn pipe_bare_function() {
    let g = compile_single("fn inc(a: int) -> int { a + 1 }\npub fn t(x: int) -> int { x |> inc }");
    assert_eq!(fn_body(&g, "t"), "inc(x)");
}

#[test]
fn pipe_call_with_args_and_turbofish() {
    let g = compile_single(
        "fn add(a: int, b: int) -> int { a + b }\npub fn t(x: int) -> int { x |> add(2) }",
    );
    assert_eq!(fn_body(&g, "t"), "add(x, 2)");
}

#[test]
fn pipe_chain_is_left_associative() {
    let g = compile_single(
        "fn f(a: int) -> int { a }\nfn g(a: int, b: int) -> int { a + b }\npub fn t(x: int) -> int { x |> f |> g(1) |> f }",
    );
    assert_eq!(fn_body(&g, "t"), "f(g(f(x), 1))");
}

#[test]
fn pipe_has_lowest_precedence() {
    let g = compile_single(
        "fn f(a: int) -> int { a }\npub fn t(a: int) -> int { a + a * 2 |> f }\npub fn u(a: bool, b: bool) -> bool { a || b |> id }\nfn id(v: bool) -> bool { v }",
    );
    assert_eq!(fn_body(&g, "t"), "f(a + a * 2)");
    assert_eq!(fn_body(&g, "u"), "id(a || b)");
}

#[test]
fn pipe_method_form() {
    let g = compile_single(
        "struct S { v: int }\nimpl S { fn get(self) -> int { self.v } fn plus(self, n: int) -> int { self.v + n } }\npub fn t(s: S) -> int { s |> .plus(4) }",
    );
    assert_eq!(fn_body(&g, "t"), "s.plus(4)");
}

#[test]
fn pipe_assoc_path_call() {
    let g = compile_single(
        "struct S { v: int }\nimpl S { fn make(n: int, m: int) -> S { S { v: n + m } } }\npub fn t(x: int) -> S { x |> S::make(1) }",
    );
    assert_eq!(fn_body(&g, "t"), "S::make(x, 1)");
}

#[test]
fn pipe_closure_rhs_is_applied() {
    let g = compile_single("pub fn t(x: int) -> int { x |> |v| v + 1 }");
    let b = fn_body(&g, "t");
    assert_eq!(b, "(|v| v + 1)(x)");
}

#[test]
fn pipe_placeholder_position() {
    let g = compile_single(
        "fn f(a: int, b: int, c: int) -> int { a + b + c }\npub fn t(x: int) -> int { x |> f(1, _, 3) }",
    );
    assert_eq!(fn_body(&g, "t"), "f(1, x, 3)");
}

#[test]
fn pipe_multiline_with_comments() {
    let g = compile_single(
        "fn f(a: int) -> int { a }\nfn g(a: int, b: int) -> int { a + b }\npub fn t(x: int) -> int {\n    x\n        |> f   // first\n        // second\n        |> g(2)\n}",
    );
    assert_eq!(fn_body(&g, "t"), "g(f(x), 2)");
}

#[test]
fn pipe_try_applies_to_stage() {
    let g = compile_single(
        "fn parse(s: string) -> Result<int, string> { Ok(1) }\nfn check(n: int) -> Result<int, string> { Ok(n) }\npub fn t(s: string) -> Result<int, string> { s |> parse()? |> check }",
    );
    assert_eq!(fn_body(&g, "t"), "check(parse(s)?)");
}

#[test]
fn pipe_in_match_arm_and_args() {
    let g = compile_single(
        "fn f(a: int) -> int { a }\nfn h(a: int) -> int { a }\npub fn t(x: int) -> int { match x { 0 => 1 |> f, _ => h(x |> f) } }",
    );
    let b = fn_body(&g, "t");
    assert!(b.contains("f(1)") && b.contains("h(f(x))"), "got {}", b);
}

// ---- ownership: identical Rust to the direct call ----

fn same_rust(helpers: &str, piped: &str, direct: &str) {
    let a = compile_single(&format!(
        "{}\npub fn t(x: Probe) -> int {{ {} }}",
        helpers, piped
    ));
    let b = compile_single(&format!(
        "{}\npub fn t(x: Probe) -> int {{ {} }}",
        helpers, direct
    ));
    assert_eq!(fn_body(&a, "t"), fn_body(&b, "t"));
    assert_eq!(norm(&a), norm(&b));
}

const PROBE: &str = "struct Probe { v: int }\nimpl Probe { fn peek(self) -> int { self.v } fn eat(self) -> int { self.v } fn bump(self, n: int) -> int { self.v + n } }\nfn borrow(p: Probe) -> int { p.v }\nfn borrow2(p: Probe, n: int) -> int { p.v + n }\n";

#[test]
fn pipe_equals_direct_call_borrowing_callee() {
    same_rust(PROBE, "x |> borrow", "borrow(x)");
    same_rust(PROBE, "x |> borrow2(3)", "borrow2(x, 3)");
}

#[test]
fn pipe_equals_direct_call_moving_callee() {
    let helpers = "struct Probe { v: int }\nfn consume(p: Probe) -> Probe { p }\nfn val(p: Probe) -> int { p.v }\n";
    same_rust(helpers, "x |> consume |> val", "val(consume(x))");
}

#[test]
fn pipe_equals_direct_method_call() {
    same_rust(PROBE, "x |> .peek()", "x.peek()");
    same_rust(PROBE, "x |> .bump(2)", "x.bump(2)");
}

#[test]
fn pipe_equals_direct_call_copy_and_widths() {
    let a = compile_single(
        "fn w(a: u8, b: i64) -> i64 { b + 1 }\nfn s(a: string) -> int { a.len() }\npub fn t(x: u8, y: string) -> int { let _a = x |> w(5); y |> s }",
    );
    let b = compile_single(
        "fn w(a: u8, b: i64) -> i64 { b + 1 }\nfn s(a: string) -> int { a.len() }\npub fn t(x: u8, y: string) -> int { let _a = w(x, 5); s(y) }",
    );
    assert_eq!(norm(&a), norm(&b));
}

// ---- diagnostics ----

#[test]
fn pipe_error_rhs_not_callable() {
    let e = err_of("pub fn t(x: int) -> int { x |> 5 }");
    assert!(e.contains("right-hand side of `|>`"), "got {}", e);
}

#[test]
fn pipe_error_trailing() {
    let e = err_of("fn f(a: int) -> int { a }\npub fn t(x: int) -> int { x |> }");
    assert!(e.contains("expected a stage after `|>`"), "got {}", e);
}

#[test]
fn pipe_error_at_statement_start() {
    let e = err_of("fn f(a: int) -> int { a }\npub fn t(x: int) -> int {\n    |> f\n}");
    assert!(e.contains("`|>` needs a left-hand value"), "got {}", e);
}

#[test]
fn pipe_error_two_placeholders() {
    let e = err_of("fn f(a: int, b: int) -> int { a }\npub fn t(x: int) -> int { x |> f(_, _) }");
    assert!(e.contains("at most one `_` placeholder"), "got {}", e);
}

#[test]
fn pipe_error_nested_placeholder() {
    let e = err_of("fn f(a: int) -> int { a }\nfn g(a: int, b: int) -> int { a }\npub fn t(x: int) -> int { x |> g(1, f(_)) }");
    assert!(
        e.contains("placeholder `_` must be a direct argument"),
        "got {}",
        e
    );
}

#[test]
fn placeholder_outside_pipe_is_error() {
    let e = err_of("fn f(a: int) -> int { a }\npub fn t(x: int) -> int { f(_) }");
    assert!(!e.is_empty());
}
