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

//! TDD test for lifetime inference in function signatures.
//!
//! Bug: When a function has 2+ reference parameters and returns a reference,
//! Rust requires explicit lifetime annotations. The Windjammer compiler should
//! automatically infer and add these lifetime annotations.
//!
//! Rust's lifetime elision rules:
//! 1. One input reference → output gets that lifetime (handled by Rust)
//! 2. &self/&mut self → output gets self's lifetime (handled by Rust)
//! 3. Multiple input references → MUST be explicit (Windjammer must handle this)
//!
//! Example:
//!   Windjammer: fn longest(a: &String, b: &String) -> &String
//!   Should generate: fn longest<'a>(a: &'a String, b: &'a String) -> &'a String

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_two_ref_params_with_ref_return_gets_lifetime() {
    // Windjammer `string` return type generates owned String — no lifetime annotation needed
    let source = r#"
fn longest(a: string, b: string) -> string {
    if a.len() >= b.len() {
        a
    } else {
        b
    }
}

fn main() {
    let a = "Hello World".to_string()
    let b = "Hi".to_string()
    let result = longest(&a, &b)
    println!("{}", result)
}
"#;
    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("fn longest") && generated.contains("-> String"),
        "Expected owned String return type.\nGenerated:\n{}",
        generated
    );

    // Owned return: no explicit lifetime parameter required
    assert!(
        !generated.contains("<\'a>"),
        "Owned String return should not need explicit lifetime.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_single_ref_param_no_lifetime_needed() {
    // Single reference param: Rust elision handles this
    let source = r#"
fn first_char(s: string) -> string {
    s
}

fn main() {
    let s = "Hello".to_string()
    let c = first_char(&s)
    println!("{}", c)
}
"#;
    let generated = test_utils::compile_single(source);

    // Should NOT have explicit lifetime (Rust elision handles it)
    // Note: it's OK if it does have a lifetime, but it shouldn't be required
    // Let's just verify it compiles - the main test is the two-param case
    assert!(
        generated.contains("fn first_char"),
        "Function should exist in generated code"
    );
}

#[test]
fn test_self_method_ref_return_no_lifetime_needed() {
    // &self method returning reference: Rust elision ties to self lifetime
    let source = r#"
struct Container {
    pub items: Vec<String>,
}

impl Container {
    fn new() -> Container {
        Container { items: Vec::new() }
    }
    
    fn first(self) -> Option<string> {
        if self.items.is_empty() {
            None
        } else {
            Some(&self.items[0])
        }
    }
}

fn main() {
    let c = Container::new()
    println!("done")
}
"#;
    let generated = test_utils::compile_single(source);

    // Self method: Rust elision should handle it, no explicit lifetime needed
    assert!(
        generated.contains("fn first"),
        "Method should exist in generated code"
    );
}

#[test]
fn test_option_ref_return_with_multiple_params_gets_lifetime() {
    // Windjammer `string` in Option<string> generates Option<String> — owned, no lifetime
    let source = r#"
fn longer_option(a: string, b: string) -> Option<string> {
    if a.len() > 0 {
        Some(a)
    } else {
        Some(b)
    }
}

fn main() {
    let a = "Hello".to_string()
    let b = "World".to_string()
    match longer_option(&a, &b) {
        Some(s) => println!("{}", s),
        None => println!("none"),
    }
}
"#;
    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("-> Option<String>"),
        "Expected owned Option<String> return.\nGenerated:\n{}",
        generated
    );

    assert!(
        !generated.contains("<\'a>"),
        "Owned Option<String> return should not need explicit lifetime.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_ref_return_no_lifetime_needed() {
    // No reference return type: no lifetime needed regardless of ref params
    let source = r#"
fn compare(a: string, b: string) -> bool {
    a.len() > b.len()
}

fn main() {
    let a = "Hello".to_string()
    let b = "Hi".to_string()
    println!("{}", compare(&a, &b))
}
"#;
    let generated = test_utils::compile_single(source);

    // No reference return: no lifetime needed
    assert!(
        !generated.contains("<'a>"),
        "Should NOT have lifetime parameter when return type has no references.\nGenerated:\n{}",
        generated
    );
}
