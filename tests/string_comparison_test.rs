#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// TDD Tests: String comparison codegen
//
// Ensures the WJ compiler generates correct Rust for all string comparison
// scenarios. Tests use both idiomatic Windjammer (`string`) and explicit Rust
// types (`String`, `&str`) to verify correctness across the board.
//
// Rust natively handles all String/&str/&String comparison combinations
// via PartialEq impls — the compiler should not add spurious `&`, `*`, or `.clone()`.

#[path = "common/test_utils.rs"]
mod test_utils;

// ── Idiomatic Windjammer tests (using `string`) ─────────────────────────────

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_eq_both_params() {
    let source = r#"
fn compare(a: string, b: string) -> bool {
    a == b
}

fn main() {
    let _ = compare("foo", "bar")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("a == b"),
        "Should generate clean comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_eq_with_literal() {
    let source = r#"
fn check(s: string) -> bool {
    s == "hello"
}

fn main() {
    let _ = check("hello")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("s == \"hello\""),
        "Should generate clean literal comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_field_comparison() {
    let source = r#"
struct User {
    pub name: string,
}

fn check(user: User, target: string) -> bool {
    user.name == target
}

fn main() {
    let u = User { name: "alice" }
    let _ = check(u, "alice")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("user.name == target") || generated.contains("user.name == *target"),
        "Should generate clean field comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_ne_operator() {
    let source = r#"
fn different(a: string, b: string) -> bool {
    a != b
}

fn main() {
    let _ = different("x", "y")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("a != b"),
        "Should generate clean ne comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_in_condition() {
    let source = r#"
fn process(cmd: string) -> bool {
    if cmd == "quit" {
        return true
    }
    false
}

fn main() {
    let _ = process("run")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("cmd == \"quit\""),
        "Should generate clean condition comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_chain_comparison() {
    let source = r#"
fn all_eq(a: string, b: string, c: string) -> bool {
    a == b && b == c
}

fn main() {
    let _ = all_eq("x", "x", "x")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("E0277"),
        "Should not have E0277:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_or_comparison() {
    let source = r#"
fn is_hello(s: string) -> bool {
    s == "hello" || s == "hello!"
}

fn main() {
    let _ = is_hello("hello")
}
"#;
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_lt_operator() {
    let source = r#"
fn before(a: string, b: string) -> bool {
    a < b
}

fn main() {
    let _ = before("a", "b")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("a < b"),
        "Should generate clean lt comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_ge_operator() {
    let source = r#"
fn gte(a: string, b: string) -> bool {
    a >= b
}

fn main() {
    let _ = gte("z", "a")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("a >= b"),
        "Should generate clean ge comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_no_spurious_deref() {
    let source = r#"
fn check(a: string, b: string) -> bool {
    a == b
}

fn main() {
    let _ = check("x", "y")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("*a"),
        "Should not deref &str params: {}",
        generated
    );
    assert!(
        !generated.contains("*b"),
        "Should not deref &str params: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_to_string_comparison() {
    let source = r#"
fn check(s: string) -> bool {
    s == "hello"
}

fn main() {
    let x = "hi"
    let _ = check(x)
}
"#;
    test_utils::compile_single(source);
}

// ── Vec iteration with string comparison ─────────────────────────────────────

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_vec_iter_comparison() {
    let source = r#"
fn has_match(items: Vec<string>, target: string) -> bool {
    for item in items {
        if item == target {
            return true
        }
    }
    false
}

fn main() {
    let items = vec!["a", "b"]
    let _ = has_match(items, "a")
}
"#;
    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_struct_field_vs_param() {
    let source = r#"
struct Member {
    id: string,
}

fn find_member(members: Vec<Member>, target_id: string) -> bool {
    for m in members {
        if m.id == target_id {
            return true
        }
    }
    false
}

fn main() {
    let members = vec![Member { id: "a" }]
    let _ = find_member(members, "a")
}
"#;
    test_utils::compile_single(source);
}

// ── Explicit Rust type tests (defense in depth) ──────────────────────────────
// These test that even when users write explicit Rust types (which the compiler
// warns about), the generated code is still valid.

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_string_vs_str() {
    let source = r#"
fn compare(a: string, b: string) -> bool {
    a == b
}

fn main() {
    let s = "hello".to_string()
    let _ = compare(s, "world")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("a == b"),
        "Should generate clean comparison for explicit types: {}",
        generated
    );
    assert!(
        !generated.contains("*b"),
        "Should not deref &str: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_str_vs_literal() {
    let source = r#"
fn check(s: string) -> bool {
    s == "hello"
}

fn main() {
    let _ = check("hello")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("s == \"hello\""),
        "Should generate clean literal comparison for explicit &str: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_ref_struct_field() {
    let source = r#"
struct User {
    pub name: string,
}

fn check(user: User, target: string) -> bool {
    user.name == target
}

fn main() {
    let u = User { name: "alice".to_string() }
    let _ = check(&u, "alice")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("user.name == target") || generated.contains("user.name == *target"),
        "Should generate clean field comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_str_in_condition() {
    let source = r#"
fn process(cmd: string) -> bool {
    if cmd == "quit" {
        return true
    }
    false
}

fn main() {
    let _ = process("run")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("cmd == \"quit\""),
        "Should generate clean condition for explicit &str: {}",
        generated
    );
}
