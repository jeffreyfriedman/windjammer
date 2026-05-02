#[path = "test_utils.rs"]
mod test_utils;

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

    let (output, success) = test_utils::compile_single_check(source);

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

    let (output, success) = test_utils::compile_single_check(source);

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

    let (output, success) = test_utils::compile_single_check(source);

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

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile without errors");
}

// Helper function
