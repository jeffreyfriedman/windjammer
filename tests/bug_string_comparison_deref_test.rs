// BUG: String comparison codegen adds incorrect dereference operators
//
// PROBLEM: When comparing two borrowed strings (&String or &str), the compiler
// incorrectly adds a dereference operator (*) on one side, causing type mismatches.
//
// Expected: `&String == &String` or `&str == &str`
// Generated: `&String == *String` (WRONG!) or `&str == *str` (WRONG!)
//
// This causes E0277 errors: "can't compare `&String` with `String`"

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
    
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "wj",
            "--",
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
fn test_string_comparison_no_extra_deref() {
    let source = r#"
fn check_name(name: &String, target: &String) -> bool {
    name == target
}

fn check_str(a: &str, b: &str) -> bool {
    a == b
}

fn main() {
    let s1 = String::from("hello")
    let s2 = String::from("world")
    let result = check_name(&s1, &s2)
    
    let result2 = check_str("foo", "bar")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Verify no type mismatch errors
    assert!(!stderr.contains("E0277"), 
            "Should not have E0277 type mismatch errors:\n{}", stderr);
    assert!(!stderr.contains("can't compare"), 
            "Should not have comparison errors:\n{}", stderr);
    
    // Check generated code doesn't have incorrect dereferences
    // Should NOT have `name == *target` or `*name == target`
    assert!(!rust_code.contains("name == *target"), 
            "Should not add * dereference in comparison:\n{}", rust_code);
    assert!(!rust_code.contains("*name == target"), 
            "Should not add * dereference in comparison:\n{}", rust_code);
    
    // Should have correct comparison: `name == target`
    assert!(rust_code.contains("name == target") || rust_code.contains("(name == target)"),
            "Should have correct comparison without dereference:\n{}", rust_code);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_comparison_in_loop() {
    let source = r#"
fn find_match(items: &Vec<String>, target: &String) -> bool {
    for item in items.iter() {
        if item == target {
            return true
        }
    }
    false
}

fn main() {
    let items = vec!["a".to_string(), "b".to_string()]
    let target = "a".to_string()
    let found = find_match(&items, &target)
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Should not have E0277 comparison errors
    assert!(!stderr.contains("E0277"), 
            "Should not have E0277 errors:\n{}", stderr);
    assert!(!stderr.contains("can't compare"), 
            "Should not have comparison errors:\n{}", stderr);
    
    // Check generated code
    // Should NOT add incorrect dereference
    assert!(!rust_code.contains("item == *target"), 
            "Should not add * dereference:\n{}", rust_code);
    assert!(!rust_code.contains("*item == target"), 
            "Should not add * dereference:\n{}", rust_code);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_str_comparison() {
    let source = r#"
fn has_tag(tags: &Vec<String>, tag: &str) -> bool {
    for t in tags.iter() {
        if t.as_str() == tag {
            return true
        }
    }
    false
}

fn main() {
    let tags = vec!["player".to_string(), "enemy".to_string()]
    let found = has_tag(&tags, "player")
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Should not have E0277 comparison errors
    assert!(!stderr.contains("E0277"), 
            "Should not have E0277 errors:\n{}", stderr);
    assert!(!stderr.contains("can't compare `&str` with `str`"), 
            "Should not have &str vs str comparison error:\n{}", stderr);
    
    // Check generated code
    // Should NOT add incorrect dereference
    assert!(!rust_code.contains("== *tag"), 
            "Should not add * dereference on tag:\n{}", rust_code);
}
