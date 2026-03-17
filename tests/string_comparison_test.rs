//! TDD Tests: String comparison codegen (E0277 fix)
//!
//! Ensures string comparisons always work regardless of ownership:
//! - &str == &str, String == &str, &String == &String, etc.
//! - Never generate * on &str (produces invalid bare `str`)
//! - Add & for owned String when needed for coercion to &str

use std::fs;
use std::process::Command;

fn compile_wj_test(source: &str) -> (bool, String, String) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let unique_id = format!("test_{}_{}", std::process::id(), test_id);

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("{}.wj", unique_id));
    fs::write(&test_file, source).expect("Failed to write temp file");

    let output_dir = temp_dir.join(format!("output_{}", unique_id));
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            test_file.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let rs_file = output_dir.join(format!("{}.rs", unique_id));
    let rust_code = fs::read_to_string(&rs_file).unwrap_or_default();

    // Cleanup
    let _ = fs::remove_file(&test_file);
    let _ = fs::remove_dir_all(&output_dir);

    (success, rust_code, stderr)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_eq_both_borrowed() {
    let source = r#"
fn compare(a: &str, b: &str) -> bool {
    a == b
}

fn main() {
    let _ = compare("foo", "bar")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);

    // Both &str → no spurious & or *
    assert!(rust_code.contains("a == b") || rust_code.contains("(a == b)"));
    assert!(!rust_code.contains("&a == &b"), "No double-ref");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_eq_owned_vs_borrowed() {
    let source = r#"
fn compare(a: String, b: &str) -> bool {
    a == b
}

fn main() {
    let s = "hello".to_string()
    let _ = compare(s, "world")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);

    // a is String (owned) → &a for coercion; or Rust coerces a directly
    assert!(
        rust_code.contains("&a == b") || rust_code.contains("a == b"),
        "Expected valid comparison, got:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_field_comparison() {
    let source = r#"
struct User {
    pub name: String,
}

fn check(user: &User, target: &str) -> bool {
    user.name == target
}

fn main() {
    let u = User { name: "alice".to_string() }
    let _ = check(&u, "alice")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);

    // user.name is String (owned via field) → &user.name or user.name; target is &str
    assert!(
        rust_code.contains("user.name == target") || rust_code.contains("&user.name == target"),
        "Expected valid comparison, got:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_comparison() {
    let source = r#"
fn check(s: &str) -> bool {
    s == "hello"
}

fn main() {
    let _ = check("hello")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);

    assert!(rust_code.contains("s == \"hello\"") || rust_code.contains("(\"hello\" == s)"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_ne_operator() {
    let source = r#"
fn different(a: &str, b: &str) -> bool {
    a != b
}

fn main() {
    let _ = different("x", "y")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(rust_code.contains("a != b") || rust_code.contains("(a != b)"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_method_result_comparison() {
    let source = r#"
fn check_prefix(s: &str) -> bool {
    s.trim() == "test"
}

fn main() {
    let _ = check_prefix("  test  ")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);

    // trim() returns &str
    assert!(rust_code.contains("s.trim() == \"test\""));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_lt_operator() {
    let source = r#"
fn before(a: &str, b: &str) -> bool {
    a < b
}

fn main() {
    let _ = before("a", "b")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(rust_code.contains("a < b"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_ge_operator() {
    let source = r#"
fn gte(a: &str, b: &str) -> bool {
    a >= b
}

fn main() {
    let _ = gte("z", "a")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(rust_code.contains("a >= b"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_vec_iter_comparison() {
    let source = r#"
fn has_match(items: &Vec<String>, target: &str) -> bool {
    for item in items.iter() {
        if item == target {
            return true
        }
    }
    false
}

fn main() {
    let items = vec!["a".to_string(), "b".to_string()]
    let _ = has_match(&items, "a")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);
    assert!(!rust_code.contains("item == *target"), "No * on &str");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_owned_field_vs_borrowed_param() {
    let source = r#"
struct Member {
    id: String,
}

fn find_member(members: &Vec<Member>, target_id: &String) -> bool {
    for m in members.iter() {
        if m.id == target_id {
            return true
        }
    }
    false
}

fn main() {
    let members = vec![Member { id: "a".to_string() }]
    let _ = find_member(&members, &"a".to_string())
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);

    // m.id (String) vs target_id (&String) - both string types, ensure valid comparison
    assert!(
        rust_code.contains("m.id == target_id")
            || rust_code.contains("&m.id == target_id")
            || rust_code.contains("m.id == *target_id")
            || rust_code.contains("*target_id == m.id"),
        "Expected valid comparison, got:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_no_deref_on_str() {
    let source = r#"
fn check(a: &str, b: &str) -> bool {
    a == b
}

fn main() {
    let _ = check("x", "y")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);

    // Never add * to &str (would produce invalid bare str)
    assert!(!rust_code.contains("*a =="), "No * on &str");
    assert!(!rust_code.contains("== *b"), "No * on &str");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_to_string_comparison() {
    let source = r#"
fn check(s: String) -> bool {
    s == "hello"
}

fn main() {
    let x = "hi".to_string()
    let _ = check(x)
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_chain_comparison() {
    let source = r#"
fn all_eq(a: &str, b: &str, c: &str) -> bool {
    a == b && b == c
}

fn main() {
    let _ = all_eq("x", "x", "x")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_in_condition() {
    let source = r#"
fn process(cmd: &str) -> bool {
    if cmd == "quit" {
        return true
    }
    false
}

fn main() {
    let _ = process("run")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(rust_code.contains("cmd == \"quit\"") || rust_code.contains("(\"quit\" == cmd)"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_starts_with_pattern() {
    let source = r#"
fn is_hello(s: &str) -> bool {
    s == "hello" || s == "hello!"
}

fn main() {
    let _ = is_hello("hello")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(success, "Compilation failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);
    assert!(!stderr.contains("E0277"), "Should not have E0277:\n{}", stderr);
}
