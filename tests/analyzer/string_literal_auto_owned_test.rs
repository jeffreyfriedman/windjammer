//! TDD: String literals in String-typed contexts must generate `"".to_string()` in Rust, not bare `&str` literals.
//!
//! After removing `.to_string()` from Windjammer source, codegen must still emit owned `String` where Rust expects it.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_empty_string_in_match_arm() {
    let source = r#"
pub fn pick_name(m: Option<string>) -> string {
    match m {
        Some(name) => name,
        None => ""
    }
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("None => \"\".to_string()"),
        "None arm with empty literal should emit .to_string() for String. Got:\n{}",
        rust
    );
}

#[test]
fn test_empty_string_in_if_else() {
    let source = r#"
pub fn branch(use_default: bool, s: string) -> string {
    if use_default {
        ""
    } else {
        s
    }
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("\"\".to_string()"),
        "if branch returning empty string should emit .to_string(). Got:\n{}",
        rust
    );
}

#[test]
fn test_string_literal_return() {
    let source = r#"
pub fn empty_str() -> string {
    return ""
}
"#;
    let rust = test_utils::compile_single(source);
    // Compiler may elide `return` and emit implicit tail expression
    let ok_tail = rust.contains("\"\".to_string()") && rust.contains("empty_str");
    let ok_explicit = rust.contains("return \"\".to_string()");
    assert!(
        ok_tail || ok_explicit,
        "Function returning string literal should emit .to_string() (tail or return). Got:\n{}",
        rust
    );
}
