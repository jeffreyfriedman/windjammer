// TDD Tests: String comparison codegen
//
// Ensures the WJ compiler generates correct Rust for all string comparison
// scenarios. Tests use both idiomatic Windjammer (`string`) and explicit Rust
// types (`String`, `&str`) to verify correctness across the board.
//
// Rust natively handles all String/&str/&String comparison combinations
// via PartialEq impls — the compiler should not add spurious `&`, `*`, or `.clone()`.

use std::fs;
use std::process::Command;

fn compile_wj_test(source: &str) -> (bool, String, String) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let unique_id = format!("test_{}_{}", std::process::id(), test_id);

    let _tmp = tempfile::tempdir().unwrap();

    let temp_dir = _tmp.path();

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

    let _ = fs::remove_file(&test_file);

    (success, rust_code, stderr)
}

fn compile_and_verify_rustc(source: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static VERIFY_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let (success, rust_code, stderr) = compile_wj_test(source);
    assert!(
        success,
        "WJ compilation failed:\n{}\n\nGenerated:\n{}",
        stderr, rust_code
    );

    let verify_id = VERIFY_COUNTER.fetch_add(1, Ordering::SeqCst);
    let _tmp2 = tempfile::tempdir().unwrap();
    let temp_dir = _tmp2.path();

    let rs_file = temp_dir.join(format!(
        "verify_str_cmp_{}_{}.rs",
        std::process::id(),
        verify_id
    ));
    fs::write(&rs_file, &rust_code).expect("Failed to write rs file");

    let verify = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            rs_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let _ = fs::remove_file(&rs_file);

    if !verify.status.success() {
        let verify_stderr = String::from_utf8_lossy(&verify.stderr);
        panic!(
            "Generated Rust doesn't compile (E0277 or similar):\n{}\n\nGenerated code:\n{}",
            verify_stderr, rust_code
        );
    }

    rust_code
}

// ── Idiomatic Windjammer tests (using `string`) ─────────────────────────────

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_eq_both_params() {
    let source = r#"
fn compare(a: string, b: string) -> bool {
    a == b
}

fn main() {
    let _ = compare("foo", "bar")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("a == b"),
        "Should generate clean comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_eq_with_literal() {
    let source = r#"
fn check(s: string) -> bool {
    s == "hello"
}

fn main() {
    let _ = check("hello")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("s == \"hello\""),
        "Should generate clean literal comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_field_comparison() {
    let source = r#"
struct User {
    pub name: string,
}

fn check(user: User, target: string) -> bool {
    user.name == target
}

fn main() {
    let u = User { name: "alice" }
    let _ = check(u, "alice")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("user.name == target") || generated.contains("user.name == *target"),
        "Should generate clean field comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_ne_operator() {
    let source = r#"
fn different(a: string, b: string) -> bool {
    a != b
}

fn main() {
    let _ = different("x", "y")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("a != b"),
        "Should generate clean ne comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_in_condition() {
    let source = r#"
fn process(cmd: string) -> bool {
    if cmd == "quit" {
        return true
    }
    false
}

fn main() {
    let _ = process("run")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("cmd == \"quit\""),
        "Should generate clean condition comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_chain_comparison() {
    let source = r#"
fn all_eq(a: string, b: string, c: string) -> bool {
    a == b && b == c
}

fn main() {
    let _ = all_eq("x", "x", "x")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        !generated.contains("E0277"),
        "Should not have E0277:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_or_comparison() {
    let source = r#"
fn is_hello(s: string) -> bool {
    s == "hello" || s == "hello!"
}

fn main() {
    let _ = is_hello("hello")
}
"#;
    compile_and_verify_rustc(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_lt_operator() {
    let source = r#"
fn before(a: string, b: string) -> bool {
    a < b
}

fn main() {
    let _ = before("a", "b")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("a < b"),
        "Should generate clean lt comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_ge_operator() {
    let source = r#"
fn gte(a: string, b: string) -> bool {
    a >= b
}

fn main() {
    let _ = gte("z", "a")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("a >= b"),
        "Should generate clean ge comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_no_spurious_deref() {
    let source = r#"
fn check(a: string, b: string) -> bool {
    a == b
}

fn main() {
    let _ = check("x", "y")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        !generated.contains("*a"),
        "Should not deref &str params: {}",
        generated
    );
    assert!(
        !generated.contains("*b"),
        "Should not deref &str params: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_to_string_comparison() {
    let source = r#"
fn check(s: string) -> bool {
    s == "hello"
}

fn main() {
    let x = "hi"
    let _ = check(x)
}
"#;
    compile_and_verify_rustc(source);
}

// ── Vec iteration with string comparison ─────────────────────────────────────

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_vec_iter_comparison() {
    let source = r#"
fn has_match(items: Vec<string>, target: string) -> bool {
    for item in items {
        if item == target {
            return true
        }
    }
    false
}

fn main() {
    let items = vec!["a", "b"]
    let _ = has_match(items, "a")
}
"#;
    compile_and_verify_rustc(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_struct_field_vs_param() {
    let source = r#"
struct Member {
    id: string,
}

fn find_member(members: Vec<Member>, target_id: string) -> bool {
    for m in members {
        if m.id == target_id {
            return true
        }
    }
    false
}

fn main() {
    let members = vec![Member { id: "a" }]
    let _ = find_member(members, "a")
}
"#;
    compile_and_verify_rustc(source);
}

// ── Explicit Rust type tests (defense in depth) ──────────────────────────────
// These test that even when users write explicit Rust types (which the compiler
// warns about), the generated code is still valid.

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_string_vs_str() {
    let source = r#"
fn compare(a: String, b: &str) -> bool {
    a == b
}

fn main() {
    let s = "hello".to_string()
    let _ = compare(s, "world")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("a == b"),
        "Should generate clean comparison for explicit types: {}",
        generated
    );
    assert!(
        !generated.contains("*b"),
        "Should not deref &str: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_str_vs_literal() {
    let source = r#"
fn check(s: &str) -> bool {
    s == "hello"
}

fn main() {
    let _ = check("hello")
}
"#;
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("s == \"hello\""),
        "Should generate clean literal comparison for explicit &str: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_ref_struct_field() {
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
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("user.name == target") || generated.contains("user.name == *target"),
        "Should generate clean field comparison: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_str_in_condition() {
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
    let generated = compile_and_verify_rustc(source);
    assert!(
        generated.contains("cmd == \"quit\""),
        "Should generate clean condition for explicit &str: {}",
        generated
    );
}
