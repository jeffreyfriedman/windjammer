#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD Test: Vec<String> indexing generates & (auto-borrow) for non-Copy types
///
/// Bug: `let line = lines[i]` where lines: Vec<String> generates move instead of borrow
/// Root cause: Rust doesn't allow moving out of Vec index (E0507)
/// Fix: Generate &vec[idx] (auto-borrow) for non-Copy - zero-cost, idiomatic
///      Generate vec[idx].clone() only when owned value needed (e.g. struct literal)
///
/// Discovered via dogfooding: breach-protocol save_manager.wj (split returns Vec<String>)
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_string_index_generates_borrow() {
    // Vec<String> indexing - need & (auto-borrow) because String is not Copy
    // Prefer & over .clone() - zero-cost, idiomatic
    let source = r#"
pub fn get_line(lines: Vec<string>, index: i32) -> string {
    let line = lines[index]
    return line
}

fn main() {
    let lines = vec!["a".to_string(), "b".to_string()]
    let x = get_line(lines, 0)
}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    // Should generate &lines[index] (auto-borrow) - NOT raw lines[index] (E0507)
    assert!(
        rust.contains("&lines[") || rust.contains("& lines["),
        "Vec<String> indexing should generate & (auto-borrow), got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_vec_int_index_remains_direct() {
    // Vec<i32> indexing - no .clone() needed (Copy type)
    let source = r#"
pub fn get_int(numbers: Vec<i32>, index: i32) -> i32 {
    let num = numbers[index]
    return num
}

fn main() {
    let nums = vec![1, 2, 3]
    let x = get_int(nums, 0)
}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    // i32 is Copy - get_int body should have numbers[index] without .clone()
    let get_int_body = rust
        .split("fn get_int")
        .nth(1)
        .unwrap_or("")
        .split("fn main")
        .next()
        .unwrap_or("");
    assert!(
        !get_int_body.contains("numbers[") || !get_int_body.contains("].clone()"),
        "Vec<i32> indexing should NOT add .clone() (Copy type), got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_local_var_vec_string_index_generates_borrow() {
    // Real-world case: local var from function returning Vec<String>
    // Simulates: let lines = split(...); let line = lines[i]
    let source = r#"
pub fn get_parts(text: string) -> Vec<string> {
    vec![text.to_string()]
}

pub fn parse_first(text: string) -> string {
    let parts = get_parts(text)
    let first = parts[0]
    return first
}

fn main() {}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    // parts[0] where parts: Vec<String> needs & (auto-borrow) to avoid E0507
    assert!(
        rust.contains("&parts[") || rust.contains("& parts["),
        "Vec<String> from local var indexing should generate & (auto-borrow), got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_vec_index_nonopy_field_access_compiles() {
    // Bug: `let text = items[i].name` where name is String generates a move out of Vec
    // The compiler suppresses borrow/clone on vec[i] when inside a FieldAccess,
    // but doesn't handle the case where the *field itself* is non-Copy (String).
    // Result: `items[i].name` tries to move String out of Vec element → E0507
    //
    // Fix: When accessing a non-Copy field through Vec indexing, the codegen must
    // add .clone() to the field access result (e.g., items[i].name.clone())
    let source = r#"
pub struct Choice {
    pub text: string,
    pub value: i32,
}

pub fn get_choice_text(choices: Vec<Choice>, idx: i32) -> string {
    let choice_text = choices[idx].text
    return choice_text
}

fn main() {
    let mut choices = Vec::new()
    choices.push(Choice { text: "Hello".to_string(), value: 1 })
    let t = get_choice_text(choices, 0)
}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    // The generated Rust MUST compile. The exact mechanism (clone or borrow) is
    // an implementation detail — what matters is no E0507 move-out-of-Vec error.
    assert!(
        compiles,
        "Vec[i].string_field must compile (no E0507 move). Generated Rust:\n{}",
        rust
    );
}
