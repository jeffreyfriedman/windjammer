//! TDD Tests for Windjammer Linter
//!
//! Tests compiler lint warnings (performance, style, correctness)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to compile Windjammer source and capture output
fn compile_wj(source: &str) -> (String, String) {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let wj_file = tmp.path().join("test.wj");
    fs::write(&wj_file, source).expect("Failed to write test file");

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&wj_file)
        .current_dir(tmp.path())
        .output()
        .expect("Failed to run wj");

    let build_dir = tmp.path().join("build");
    let rust_file = build_dir.join("test.rs");

    let generated = if rust_file.exists() {
        fs::read_to_string(&rust_file).unwrap_or_else(|_| String::from("FILE NOT FOUND"))
    } else {
        String::from("BUILD DIR NOT CREATED")
    };

    let stderr = String::from_utf8_lossy(&wj_output.stderr).to_string();
    (generated, stderr)
}

// =============================================================================
// LINT: owned-but-not-returned
// =============================================================================

#[test]
fn test_lint_owned_but_not_returned_warns() {
    // THE WINDJAMMER WAY: Owned parameter mutated but not returned → suggest &mut
    let source = r#"
pub struct ResourcePool {
    items: Vec<string>,
    count: i32,
}

impl ResourcePool {
    pub fn add(self, item: string) {
        self.items.push(item)
        self.count = self.count + 1
    }
}

/// This should trigger lint: owned param mutated but not returned
pub fn fill_pool(pool: ResourcePool) {
    pool.add("water")
    pool.add("food")
}
"#;

    let (_generated, stderr) = compile_wj(source);

    // Check for lint warning
    assert!(
        stderr.contains("owned-but-not-returned")
            || stderr.contains("mutated but not returned")
            || stderr.contains("Consider using `&mut"),
        "Expected lint warning for owned-but-not-returned. Stderr:\n{}",
        stderr
    );
}

#[test]
fn test_lint_owned_and_returned_no_warning() {
    // THE WINDJAMMER WAY: Owned parameter mutated AND returned → no warning
    let source = r#"
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn increment(self) {
        self.value = self.value + 1
    }
}

/// This should NOT trigger lint: owned param mutated and returned
pub fn increment_counter(counter: Counter) -> Counter {
    counter.increment()
    counter
}
"#;

    let (_generated, stderr) = compile_wj(source);

    // Should NOT have warning
    assert!(
        !stderr.contains("owned-but-not-returned") && !stderr.contains("mutated but not returned"),
        "Should NOT warn for owned param that is returned. Stderr:\n{}",
        stderr
    );
}

#[test]
fn test_lint_owned_read_only_no_warning() {
    // THE WINDJAMMER WAY: Owned parameter only read → no warning (might be for ownership transfer)
    let source = r#"
pub struct Data {
    value: i32,
}

impl Data {
    pub fn get_value(self) -> i32 {
        self.value
    }
}

/// This should NOT trigger lint: owned param only read (might need ownership)
pub fn process_data(data: Data) -> i32 {
    data.get_value()
}
"#;

    let (_generated, stderr) = compile_wj(source);

    // Should NOT have warning (owned read-only is fine)
    assert!(
        !stderr.contains("owned-but-not-returned"),
        "Should NOT warn for owned param that is only read. Stderr:\n{}",
        stderr
    );
}

// =============================================================================
// LINT: explicit-to-string
// =============================================================================

// NOTE: explicit-to-string lint tests removed
// The compiler already normalizes "text".to_string() → "text" automatically
// This happens at parse/codegen time, so no lint is needed
// This is BETTER than a lint - it's automatic boilerplate elimination!

// =============================================================================
// LANGUAGE CONSISTENCY TESTS
// =============================================================================

#[test]
fn test_consistency_explicit_type_respected() {
    // User writes explicit type annotation → must be preserved
    let source = r#"
pub fn test() {
    let x: i32 = 42  // Explicit i32, even if usize would work
    let y: string = "test"  // Explicit String, even if &str would work
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Note: Compiler adds _ prefix to unused variables (Rust best practice)
    assert!(
        generated.contains(": i32 = 42"),
        "Expected explicit i32 type to be preserved. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains(": String = "),
        "Expected explicit String type to be preserved. Generated:\n{}",
        generated
    );
}

#[test]
fn test_consistency_explicit_mut_respected() {
    // User writes explicit mut → must be preserved (even if unnecessary)
    let source = r#"
pub fn test() {
    let mut x = 42  // Explicit mut, even if never mutated
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Note: Compiler adds _ prefix to unused variables
    assert!(
        generated.contains("mut") && generated.contains("= 42"),
        "Expected explicit mut to be preserved. Generated:\n{}",
        generated
    );
}

#[test]
fn test_consistency_explicit_ownership_respected() {
    // User writes explicit ownership → must be preserved (our recent fix!)
    // This test uses the EXACT same example as dogfooding_ownership_inference_test
    let source = r#"
pub struct ResourcePool {
    items: Vec<string>,
    count: i32,
}

impl ResourcePool {
    pub fn new() -> ResourcePool {
        ResourcePool { items: Vec::new(), count: 0 }
    }

    pub fn add(self, item: string) {
        self.items.push(item)
        self.count = self.count + 1
    }
}

pub fn fill_pool(pool: ResourcePool) {
    pool.add("water")
    pool.add("food")
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Should be: mut pool: ResourcePool (not pool: &mut ResourcePool)
    // This is the core fix: owned + mutated → mut binding, not &mut parameter
    assert!(
        generated.contains("pub fn fill_pool(mut pool: ResourcePool)"),
        "Expected explicit owned type to be preserved as 'mut pool: ResourcePool'. Generated:\n{}",
        generated
    );
}

#[test]
fn test_consistency_user_closure_preserved() {
    // User writes closure explicitly → must be preserved (our recent fix!)
    let source = r#"
pub struct Item {
    active: bool,
}

pub struct Inventory {
    items: Vec<Item>,
}

impl Inventory {
    pub fn count_inactive(self) -> usize {
        self.items.iter().filter(|e| !e.active).count()
    }
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Should preserve user-written closure: |e| !e.active (no move, no &e)
    assert!(
        generated.contains("filter(|e| !e.active)"),
        "Expected user-written closure to be preserved. Generated:\n{}",
        generated
    );
}
