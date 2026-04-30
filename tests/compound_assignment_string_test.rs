/// TDD Test: Compound Assignment with String-returning expressions
///
/// Bug: User writes `result += func()` where func() -> String
/// Problem: Generated `result += func()` doesn't compile (String += String invalid)
/// Solution: Either add & prefix OR convert to regular assignment
///
/// This is different from the binary expression case (result = result + func())
/// because it's parsed as a CompoundAssignment statement, not Binary expression.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_compound_assignment_function_call() {
    let source = r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}

pub fn build_greetings() -> string {
    let mut result = ""
    result += greet("Alice")
    result += greet("Bob")
    result
}
"#;

    let (success, output) = compile_and_verify_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile without errors");

    // Should either use & prefix OR convert to regular assignment
    let has_borrow = output.contains("result += &greet");
    let has_assignment = output.contains("result = result + ");

    assert!(
        has_borrow || has_assignment,
        "Should either add & prefix or use regular assignment: {}",
        output
    );
}

#[test]
fn test_compound_assignment_method_call() {
    let source = r#"
struct Renderer {
    prefix: string,
}

impl Renderer {
    pub fn render(self, text: string) -> string {
        format!("{}: {}", self.prefix, text)
    }
}

pub fn render_all(r: Renderer) -> string {
    let mut html = ""
    html += r.render("line1")
    html += r.render("line2")
    html
}
"#;

    let (success, output) = compile_and_verify_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile without errors");
}

#[test]
fn test_compound_assignment_format_macro() {
    let source = r#"
pub fn build_report(name: string, score: i32) -> string {
    let mut output = ""
    output += format!("Name: {}", name)
    output += format!("Score: {}", score)
    output
}
"#;

    let (success, output) = compile_and_verify_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile without errors");
}

#[test]
fn test_compound_assignment_mixed() {
    let source = r#"
pub fn format_value(v: i32) -> string {
    format!("{}", v)
}

pub fn build_mixed() -> string {
    let mut result = ""
    result += "Prefix: "      // String literal (already &str) - should work
    result += format_value(42) // Function returning String - needs fix
    result += " - Suffix"     // String literal - should work
    result
}
"#;

    let (success, output) = compile_and_verify_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile without errors");
}

// Helper function
fn compile_and_verify_rust(source: &str) -> (bool, String) {
    let _ = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let source_file = tmp.path().join("test.wj");
    std::fs::write(&source_file, source).unwrap();

    if let Err(e) = windjammer::build_project(
        &source_file,
        tmp.path(),
        windjammer::CompilationTarget::Rust,
        false,
    ) {
        return (false, format!("Compilation failed: {}", e));
    }

    let rust_file = tmp.path().join("test.rs");
    let rust_code =
        std::fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    let rmeta = tmp.path().join("verify.rmeta");
    let rustc = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(rmeta.to_str().unwrap())
        .arg(rust_file.to_str().unwrap())
        .output()
        .expect("Failed to run rustc");

    (rustc.status.success(), rust_code)
}
