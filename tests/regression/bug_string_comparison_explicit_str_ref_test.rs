/// TDD test for BUG: Explicit &str type parameters getting unnecessary * derefs in comparisons
///
/// PROBLEM: When a function has an explicit `tag: &str` parameter, comparing it
/// with other strings adds incorrect * derefs: `*t == *tag`
///
/// EXPECTED: Clean comparison without derefs: `t == tag`
/// Rust's PartialEq handles &String == &str natively
///
/// ROOT CAUSE: balance_eq_operands_for_rust doesn't distinguish between:
/// 1. Explicit &str type (Type::Reference(Custom("str"))) - NO deref needed
/// 2. Inferred borrowed string (Type::String with inferred borrow) - might need deref
#[path = "../common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_str_ref_param_no_deref() {
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

    println!("Generated code:\n{}", rust_code);

    // CRITICAL: Explicit &str parameter should NOT get * deref
    assert!(
        !rust_code.contains("*tag"),
        "Should NOT add * deref to explicit &str parameter 'tag':\n{}",
        rust_code
    );

    // CRITICAL: Iterator variable should NOT get * deref when comparing with &str
    assert!(
        !rust_code.contains("*t ==") && !rust_code.contains("== *t"),
        "Should NOT add * deref to iterator variable when comparing with &str:\n{}",
        rust_code
    );

    // Expected: Clean comparison
    assert!(
        rust_code.contains("t == tag") || rust_code.contains("tag == t"),
        "Should generate clean comparison 't == tag':\n{}",
        rust_code
    );

    let rustc_dir = tempdir().expect("tempdir for rustc");
    let rs_file = rustc_dir.path().join("verify.rs");
    fs::write(&rs_file, &rust_code).expect("Failed to write Rust file");

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "--edition",
            "2021",
            "-o",
        ])
        .arg(rustc_dir.path().join("verify.rmeta"))
        .arg(&rs_file)
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Rustc compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_str_ref_param_with_struct_field() {
    let source = r#"
struct Member {
    id: String,
}

fn find_member(members: &Vec<Member>, target_id: &str) -> bool {
    for m in members.iter() {
        if m.id == target_id {
            return true
        }
    }
    false
}

fn main() {
    let members = vec![Member { id: "a".to_string() }]
    let found = find_member(&members, "a")
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

    println!("Generated code:\n{}", rust_code);

    // CRITICAL: Explicit &str parameter should NOT get * deref
    assert!(
        !rust_code.contains("*target_id"),
        "Should NOT add * deref to explicit &str parameter 'target_id':\n{}",
        rust_code
    );

    // Expected: Clean comparison (String field naturally compares with &str)
    assert!(
        rust_code.contains("m.id == target_id") || rust_code.contains("target_id == m.id"),
        "Should generate clean comparison 'm.id == target_id':\n{}",
        rust_code
    );

    let rustc_dir = tempdir().expect("tempdir for rustc");
    let rs_file = rustc_dir.path().join("verify.rs");
    fs::write(&rs_file, &rust_code).expect("Failed to write Rust file");

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "--edition",
            "2021",
            "-o",
        ])
        .arg(rustc_dir.path().join("verify.rmeta"))
        .arg(&rs_file)
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Rustc compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }
}
