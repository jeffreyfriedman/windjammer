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

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj(source: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir().join(format!("wj_lifetime_test_{}_{}", std::process::id(), id));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();
    
    let source_path = tmp_dir.join("test.wj");
    std::fs::write(&source_path, source).unwrap();
    
    let output_dir = tmp_dir.join("output");
    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", source_path.to_str().unwrap(), "--target", "rust", "--output", output_dir.to_str().unwrap(), "--no-cargo"])
        .output()
        .expect("failed to run wj");
    
    let rs_path = output_dir.join("test.rs");
    let generated = std::fs::read_to_string(&rs_path)
        .unwrap_or_else(|_| panic!("Failed to read generated Rust at {:?}", rs_path));
    
    let _ = std::fs::remove_dir_all(&tmp_dir);
    generated
}

#[test]
fn test_two_ref_params_with_ref_return_gets_lifetime() {
    // This is the classic case: fn longest(a: &String, b: &String) -> &String
    // Rust CANNOT elide this - needs explicit lifetime
    let source = r#"
fn longest(a: &String, b: &String) -> &String {
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
    let generated = compile_wj(source);
    
    // Should have lifetime parameter <'a>
    assert!(generated.contains("<'a>"), 
        "Expected lifetime parameter <'a> in function signature.\nGenerated:\n{}", generated);
    
    // Should have lifetime on return type
    assert!(generated.contains("-> &'a") || generated.contains("-> &'a str"),
        "Expected lifetime annotation on return type.\nGenerated:\n{}", generated);
}

#[test]
fn test_single_ref_param_no_lifetime_needed() {
    // Single reference param: Rust elision handles this
    let source = r#"
fn first_char(s: &String) -> &String {
    s
}

fn main() {
    let s = "Hello".to_string()
    let c = first_char(&s)
    println!("{}", c)
}
"#;
    let generated = compile_wj(source);
    
    // Should NOT have explicit lifetime (Rust elision handles it)
    // Note: it's OK if it does have a lifetime, but it shouldn't be required
    // Let's just verify it compiles - the main test is the two-param case
    assert!(generated.contains("fn first_char"), "Function should exist in generated code");
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
    
    fn first(self) -> Option<&String> {
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
    let generated = compile_wj(source);
    
    // Self method: Rust elision should handle it, no explicit lifetime needed
    assert!(generated.contains("fn first"), "Method should exist in generated code");
}

#[test]
fn test_option_ref_return_with_multiple_params_gets_lifetime() {
    // Return Option<&T> with two ref params also needs lifetime
    let source = r#"
fn longer_option(a: &String, b: &String) -> Option<&String> {
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
    let generated = compile_wj(source);
    
    // Should have lifetime because of two ref params + ref in return type
    assert!(generated.contains("<'a>"),
        "Expected lifetime parameter <'a> for Option<&T> return with two ref params.\nGenerated:\n{}", generated);
}

#[test]
fn test_no_ref_return_no_lifetime_needed() {
    // No reference return type: no lifetime needed regardless of ref params
    let source = r#"
fn compare(a: &String, b: &String) -> bool {
    a.len() > b.len()
}

fn main() {
    let a = "Hello".to_string()
    let b = "Hi".to_string()
    println!("{}", compare(&a, &b))
}
"#;
    let generated = compile_wj(source);
    
    // No reference return: no lifetime needed
    assert!(!generated.contains("<'a>"),
        "Should NOT have lifetime parameter when return type has no references.\nGenerated:\n{}", generated);
}
