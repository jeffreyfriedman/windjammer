/// TDD Test: Field access in comparison operations
///
/// Bug: self.current_wait > 0.0 generates 0.0_f64 instead of 0.0_f32
/// Root Cause: Field type not propagating through comparison operators
/// Expected: Comparison operands should match field type
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_field_in_comparison_gt() {
    let source = r#"
struct Timer {
    current: f32,
}

impl Timer {
    pub fn is_active() -> bool {
        self.current > 0.0
    }
}
"#;

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The 0.0 literal should be f32 (matches field type)
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32' (to match self.current: f32), got: {}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_field_in_comparison_lt() {
    let source = r#"
struct Position {
    x: f32,
    y: f32,
}

impl Position {
    pub fn is_left_of_origin() -> bool {
        self.x < 0.0
    }
}
"#;

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
}

#[test]
fn test_field_in_comparison_eq() {
    let source = r#"
struct Health {
    value: f32,
}

impl Health {
    pub fn is_zero() -> bool {
        self.value == 0.0
    }
}
"#;

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
}

#[test]
fn test_field_in_comparison_with_param() {
    let source = r#"
struct Cooldown {
    remaining: f32,
}

impl Cooldown {
    pub fn update(dt: f32) -> bool {
        if self.remaining > 0.0 {
            self.remaining = self.remaining - dt
            return false
        }
        return true
    }
}
"#;

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Both 0.0 should be f32
    assert!(
        output.contains("0.0_f32"),
        "Expected '0.0_f32', got: {}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let _ = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let source_file = tmp.path().join("test.wj");
    std::fs::write(&source_file, source).unwrap();

    windjammer::build_project(
        &source_file,
        tmp.path(),
        windjammer::CompilationTarget::Rust,
        false,
    )
    .expect("Failed to run wj compiler");

    std::fs::read_to_string(tmp.path().join("test.rs")).expect("Failed to read generated Rust file")
}
