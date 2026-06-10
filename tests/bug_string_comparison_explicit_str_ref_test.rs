#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD test for BUG: string parameters getting unnecessary * derefs on the parameter itself
///
/// PROBLEM: When a function has a read-only `tag: string` parameter (generates &str),
/// comparing it with Vec<String> elements must not deref the parameter: `*tag`
///
/// EXPECTED: Parameter used directly (`tag`), iterator may use `*t == tag` for &String vs &str
///
/// ROOT CAUSE: balance_eq_operands_for_rust must not add * to read-only string params
#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_str_ref_param_no_deref() {
    let source = r#"
fn has_tag(tags: Vec<string>, tag: string) -> bool {
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

    // Iterator over &String may need *t when comparing with &str param; both sides must not deref tag
    assert!(
        rust_code.contains("t == tag")
            || rust_code.contains("tag == t")
            || rust_code.contains("*t == tag")
            || rust_code.contains("tag == *t"),
        "Should generate valid String/&str comparison involving tag:\n{}",
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
    id: string,
}

fn find_member(members: &Vec<Member>, target_id: string) -> bool {
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
