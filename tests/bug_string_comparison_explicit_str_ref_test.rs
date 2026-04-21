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

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
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

    let (success, rust_code, stderr) = compile_wj_test(source);

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

    // Verify it compiles with rustc
    let temp_dir = std::env::temp_dir();
    let rs_file = temp_dir.join(format!("test_str_ref_{}.rs", std::process::id()));
    fs::write(&rs_file, &rust_code).expect("Failed to write Rust file");

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "bin",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            temp_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let _ = fs::remove_file(&rs_file);

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

    let (success, rust_code, stderr) = compile_wj_test(source);

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

    // Verify it compiles with rustc
    let temp_dir = std::env::temp_dir();
    let rs_file = temp_dir.join(format!("test_str_ref_struct_{}.rs", std::process::id()));
    fs::write(&rs_file, &rust_code).expect("Failed to write Rust file");

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "bin",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            temp_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let _ = fs::remove_file(&rs_file);

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Rustc compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }
}
