/// TDD: Trait Auto-Borrow for E0277 Elimination
///
/// Root cause: Not auto-borrowing for trait operations (PartialEq, Add, Sub, etc).
/// Examples: str == &str, f32 * i32, &Vec3 + Vec3
///
/// Fix: Automatically insert borrows/derefs to satisfy trait bounds, like Rust does.
/// Philosophy: "Safety Without Ceremony" - trait operations should "just work".

use std::fs;
use std::process::Command;

fn compile_wj_to_rust(source: &str) -> (bool, String, String) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let unique_id = format!("trait_auto_borrow_{}_{}", std::process::id(), test_id);

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
            "--features",
            "cli",
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
    let _ = std::fs::remove_dir_all(&output_dir);

    (success, rust_code, stderr)
}

fn run_rustc(rs_code: &str) -> (bool, String) {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "trait_auto_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    std::fs::create_dir_all(&test_dir).unwrap();

    let rs_file = test_dir.join("test.rs");
    std::fs::write(&rs_file, rs_code).unwrap();

    let output = Command::new("rustc")
        .arg(&rs_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let _ = std::fs::remove_dir_all(&test_dir);

    (output.status.success(), stderr)
}

/// String/str comparison: member.field == param where field: String, param: &str
/// Must NOT produce *member.field (str) - need &member.field for &str coercion
#[test]
fn test_string_field_vs_str_param_comparison() {
    let source = r#"
struct SquadMember {
    pub npc_id: string,
}

pub fn remove_member(members: Vec<SquadMember>, npc_id: string) {
    let mut i = 0
    while i < members.len() {
        if members[i as usize].npc_id == npc_id {
            return
        }
        i = i + 1
    }
}

pub fn main() {
    let members = vec![]
    remove_member(members, "test")
}
"#;

    let (compile_ok, rust_code, stderr) = compile_wj_to_rust(source);
    assert!(compile_ok, "Windjammer compile failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);

    // Must NOT have * before .npc_id - that produces str which can't compare with &str
    assert!(
        !rust_code.contains("*.npc_id") && !rust_code.contains("*members"),
        "Should not deref String to str in comparison. Got:\n{}",
        rust_code
    );

    let (rustc_ok, rustc_stderr) = run_rustc(&rust_code);
    assert!(
        rustc_ok,
        "rustc failed (E0277 trait):\n{}\n\nGenerated:\n{}",
        rustc_stderr,
        rust_code
    );
    assert!(
        !rustc_stderr.contains("can't compare") && !rustc_stderr.contains("PartialEq"),
        "Should not have comparison trait errors:\n{}",
        rustc_stderr
    );
}

/// &str == &str - both borrowed, no spurious *
#[test]
fn test_str_vs_str_comparison() {
    let source = r#"
pub fn has_tag(tags: Vec<string>, tag: string) -> bool {
    let mut i = 0
    while i < tags.len() {
        if tags[i as usize] == tag {
            return true
        }
        i = i + 1
    }
    false
}

pub fn main() {
    let tags = vec!["a".to_string(), "b".to_string()]
    let _ = has_tag(tags, "a")
}
"#;

    let (compile_ok, rust_code, stderr) = compile_wj_to_rust(source);
    assert!(compile_ok, "Windjammer compile failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);

    let (rustc_ok, rustc_stderr) = run_rustc(&rust_code);
    assert!(
        rustc_ok,
        "rustc failed:\n{}\n\nGenerated:\n{}",
        rustc_stderr,
        rust_code
    );
    assert!(
        !rustc_stderr.contains("can't compare"),
        "Should not have str/&str comparison error:\n{}",
        rustc_stderr
    );
}

/// f32 * i32 - mixed numeric arithmetic needs auto-cast
#[test]
fn test_f32_times_i32_arithmetic() {
    let source = r#"
pub fn scale(base: f32, count: i32) -> f32 {
    base * count
}

pub fn main() {
    let _ = scale(1.0, 10)
}
"#;

    let (compile_ok, rust_code, stderr) = compile_wj_to_rust(source);
    assert!(compile_ok, "Windjammer compile failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);

    // Should have cast for mixed types
    let has_cast = rust_code.contains("as f32") || rust_code.contains("_f32");
    assert!(
        has_cast,
        "f32 * i32 should have cast. Got:\n{}",
        rust_code
    );

    let (rustc_ok, rustc_stderr) = run_rustc(&rust_code);
    assert!(
        rustc_ok,
        "rustc failed (E0277 Mul):\n{}\n\nGenerated:\n{}",
        rustc_stderr,
        rust_code
    );
}

/// f32 - i32 subtraction - mixed numeric
#[test]
fn test_f32_minus_i32_arithmetic() {
    let source = r#"
pub fn adjust(value: f32, delta: i32) -> f32 {
    value - delta
}

pub fn main() {
    let _ = adjust(5.0, 2)
}
"#;

    let (compile_ok, rust_code, stderr) = compile_wj_to_rust(source);
    assert!(compile_ok, "Windjammer compile failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);

    let (rustc_ok, rustc_stderr) = run_rustc(&rust_code);
    assert!(
        rustc_ok,
        "rustc failed (E0277 Sub):\n{}\n\nGenerated:\n{}",
        rustc_stderr,
        rust_code
    );
}

/// for t in tags { if t == tag } - &String == &str, never *tag
#[test]
fn test_for_loop_string_comparison() {
    let source = r#"
pub fn has_tag(tags: Vec<string>, tag: string) -> bool {
    for t in tags {
        if t == tag {
            return true
        }
    }
    false
}

pub fn main() {
    let tags = vec!["a".to_string(), "b".to_string()]
    let _ = has_tag(tags, "a")
}
"#;

    let (compile_ok, rust_code, stderr) = compile_wj_to_rust(source);
    assert!(compile_ok, "Windjammer compile failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);

    // Must NOT have t == *tag - that produces &String == str (E0277)
    assert!(
        !rust_code.contains("== *tag"),
        "Should not deref tag in comparison. Got:\n{}",
        rust_code
    );

    let (rustc_ok, rustc_stderr) = run_rustc(&rust_code);
    assert!(
        rustc_ok,
        "rustc failed (E0277 PartialEq):\n{}\n\nGenerated:\n{}",
        rustc_stderr,
        rust_code
    );
}

/// i32 + f32 - mixed int/float needs auto-cast
#[test]
fn test_i32_plus_f32_arithmetic() {
    let source = r#"
pub fn add_scaled(count: i32, scale: f32) -> f32 {
    count + scale
}

pub fn main() {
    let _ = add_scaled(5, 2.0)
}
"#;

    let (compile_ok, rust_code, stderr) = compile_wj_to_rust(source);
    assert!(compile_ok, "Windjammer compile failed:\n{}\n\nGenerated:\n{}", stderr, rust_code);

    let (rustc_ok, rustc_stderr) = run_rustc(&rust_code);
    assert!(
        rustc_ok,
        "rustc failed (E0277 Add):\n{}\n\nGenerated:\n{}",
        rustc_stderr,
        rust_code
    );
}
