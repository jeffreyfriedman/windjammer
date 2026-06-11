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

/// TDD Test: Auto-Self Inference
///
/// Bug: Methods that use `self` in the body but don't declare it in parameters
/// cause E0424 errors in generated Rust code.
///
/// Expected: Compiler should automatically detect `self` usage and add it to parameters
/// Actual: E0424 "expected value, found module `self`"
///
/// THE WINDJAMMER WAY: The compiler infers mechanical details like self parameters,
/// letting users focus on logic.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_infer_immutable_self() {
    let source = r#"
struct Avatar {
    src: string,
    alt: string,
}

impl Avatar {
    fn new(src: string) -> Avatar {
        Avatar {
            src,
            alt: "Avatar".to_string(),
        }
    }
    
    // Uses self but doesn't declare it - should auto-infer self
    fn alt(alt: string) -> Avatar {
        self.alt = alt
        self
    }
    
    // Uses self immutably - should infer self
    fn get_src() -> string {
        self.src
    }
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: Compiler should auto-detect self usage
    // For methods that mutate self and return self, infer &mut self
    assert!(
        rust_code.contains("fn alt(&mut self, alt: String) -> Avatar")
            || rust_code.contains("fn alt(mut self, alt: String) -> Avatar"),
        "Should auto-infer self parameter (either &mut self or mut self).\nGenerated:\n{}",
        rust_code
    );

    // For immutable access, infer &self
    assert!(
        rust_code.contains("fn get_src(&self) -> String"),
        "Should auto-infer &self for immutable access.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_infer_mutable_self() {
    let source = r#"
struct Counter {
    count: int,
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }
    
    // Mutates self - should infer self
    fn increment() {
        self.count += 1
    }
    
    // Returns self after mutation - should infer mut self (owned)
    fn add(amount: int) -> Counter {
        self.count += amount
        self
    }
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // Should infer &mut self for methods that mutate but don't return self
    assert!(
        rust_code.contains("fn increment(&mut self)"),
        "Should auto-infer &mut self for mutation.\nGenerated:\n{}",
        rust_code
    );

    // Should infer mut self for methods that mutate and return self
    assert!(
        rust_code.contains("fn add(mut self, amount: i64) -> Counter")
            || rust_code.contains("fn add(&mut self, amount: i64) -> Counter"),
        "Should auto-infer mutable self (owned or borrowed).\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_infer_owned_self() {
    let source = r#"
struct Builder {
    value: string,
}

impl Builder {
    fn new() -> Builder {
        Builder { value: "" }
    }
    
    // Builder pattern: consumes self, returns self
    fn with_value(value: string) -> Builder {
        self.value = value
        self
    }
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // Builder pattern should infer mut self (owned)
    assert!(
        rust_code.contains("fn with_value(mut self, value: String) -> Builder"),
        "Should auto-infer mut self for builder pattern.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_self_when_not_used() {
    let source = r#"
struct Math {}

impl Math {
    // Static method - no self usage
    fn add(a: int, b: int) -> int {
        a + b
    }
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // Should NOT add self when it's not used
    assert!(
        rust_code.contains("fn add(a: i64, b: i64) -> i64"),
        "Should NOT add self to static methods.\nGenerated:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("&self") && !rust_code.contains("mut self"),
        "Should have no self parameter.\nGenerated:\n{}",
        rust_code
    );
}
