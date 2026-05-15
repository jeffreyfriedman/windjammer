#![cfg(not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
)))]

// BUG: String comparison codegen adds incorrect dereference operators
//
// PROBLEM: When comparing two borrowed strings (&String or &str), the compiler
// incorrectly adds a dereference operator (*) on one side, causing type mismatches.
//
// Expected: `&String == &String` or `&str == &str`
// Generated: `&String == *String` (WRONG!) or `&str == *str` (WRONG!)
//
// This causes E0277 errors: "can't compare `&String` with `String`"

#[path = "../common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;

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

    let (rust_code, success) = test_utils::compile_single_check(source);
    let stderr = if !success { &rust_code } else { "" };

    if !success {
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    // Verify no type mismatch errors
    assert!(
        !stderr.contains("E0277"),
        "Should not have E0277 type mismatch errors:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("can't compare"),
        "Should not have comparison errors:\n{}",
        stderr
    );

    // Check generated code doesn't have incorrect dereferences
    // Should NOT have `name == *target` or `*name == target`
    assert!(
        !rust_code.contains("name == *target"),
        "Should not add * dereference in comparison:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("*name == target"),
        "Should not add * dereference in comparison:\n{}",
        rust_code
    );

    // Should have correct comparison: `name == target`
    assert!(
        rust_code.contains("name == target") || rust_code.contains("(name == target)"),
        "Should have correct comparison without dereference:\n{}",
        rust_code
    );
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

    let (rust_code, success) = test_utils::compile_single_check(source);
    let stderr = if !success { &rust_code } else { "" };

    if !success {
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    // Should not have E0277 comparison errors
    assert!(
        !stderr.contains("E0277"),
        "Should not have E0277 errors:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("can't compare"),
        "Should not have comparison errors:\n{}",
        stderr
    );

    // Verify the generated Rust compiles (the real correctness check).
    // The codegen may or may not add * for &String iteration vars — either way,
    // the comparison must compile (String==&str, &String==&String both work).
    let _tmp2 = tempfile::tempdir().unwrap();
    let temp_dir = _tmp2.path();

    let rs_file = temp_dir.join(format!(
        "verify_deref_loop_{}_{}.rs",
        std::process::id(),
        std::sync::atomic::AtomicUsize::new(0).fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    fs::write(&rs_file, &rust_code).expect("write rs");
    let verify = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            rs_file.to_str().unwrap(),
        ])
        .output()
        .expect("run rustc");
    let _ = fs::remove_file(&rs_file);
    assert!(
        verify.status.success(),
        "Generated Rust must compile:\n{}\n\nCode:\n{}",
        String::from_utf8_lossy(&verify.stderr),
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_str_comparison() {
    let source = r#"
fn has_tag(tags: &Vec<String>, tag: &str) -> bool {
    for t in tags.iter() {
        if t == tag {
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

    let (rust_code, success) = test_utils::compile_single_check(source);
    let stderr = if !success { &rust_code } else { "" };

    if !success {
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    // Should not have E0277 comparison errors
    assert!(
        !stderr.contains("E0277"),
        "Should not have E0277 errors:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("can't compare `&str` with `str`"),
        "Should not have &str vs str comparison error:\n{}",
        stderr
    );

    // Check generated code
    // Should NOT add incorrect dereference
    assert!(
        !rust_code.contains("== *tag"),
        "Should not add * dereference on tag:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_owned_vs_borrowed_comparison() {
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
    let found = find_member(&members, &"a".to_string())
}
"#;

    let (rust_code, success) = test_utils::compile_single_check(source);
    let stderr = if !success { &rust_code } else { "" };

    if !success {
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    // Should not have E0277 comparison errors
    assert!(
        !stderr.contains("E0277"),
        "Should not have E0277 errors:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("can't compare `String` with `&String`"),
        "Should not have String vs &String error:\n{}",
        stderr
    );

    // Rust handles all String/&str/&String comparison combinations natively.
    // The codegen should NOT add * for text types — just emit the clean comparison.
    assert!(
        rust_code.contains("m.id == target_id"),
        "Should generate clean comparison without * for text types:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("*target_id"),
        "Should NOT add * dereference to text params:\n{}",
        rust_code
    );
}
