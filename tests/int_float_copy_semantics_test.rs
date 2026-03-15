/// Integration tests: Int/float fix + Copy semantics (ownership)
///
/// Verifies that the `both_int` check in expression_generation.rs is preserved
/// and works correctly with ownership inference. These systems are INDEPENDENT:
/// - Int/float: TYPE compatibility (i32 vs f32) in binary expressions
/// - Ownership: BORROW semantics (&T vs T) across all expressions
///
/// Uses wj CLI (integration) rather than compile_and_get_rust (unit) to ensure
/// full pipeline correctness.

use std::fs;
use std::process::Command;

fn compile_wj_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("write");
    fs::create_dir_all(&out_dir).expect("create dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let src_dir = out_dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        out_dir.join("test.rs")
    };

    fs::read_to_string(&main_rs).map_err(|e| e.to_string())
}

// 1. Int division then cast: ((len as i32) / 2) as f32 - 2 should NOT be cast
#[test]
fn test_int_division_then_cast() {
    let source = r#"
pub fn compute(len: usize) -> f32 {
    ((len as i32) / 2) as f32
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        generated.contains("/ 2") || generated.contains("/ (2)"),
        "The 2 should NOT be cast to f32 (both_int=true). Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("/ (2) as f32"),
        "Should NOT have / (2) as f32. Got:\n{}",
        generated
    );
}

// 2. Owned param int division: (count / 2) as f32 - both_int, no spurious cast
#[test]
fn test_owned_param_int_division() {
    let source = r#"
pub fn compute(count: i32) -> f32 {
    (count / 2) as f32
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        generated.contains("count / 2") || generated.contains("count / (2)"),
        "Should have integer division. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("/ (2) as f32"),
        "Should NOT cast 2 to f32. Got:\n{}",
        generated
    );
}

// 3. f32 + i32 (mixed) - y should be cast to f32
#[test]
fn test_f32_plus_int_literal() {
    let source = r#"
pub fn compute(x: f32, y: i32) -> f32 {
    x + y
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        generated.contains("(y) as f32") || generated.contains("y as f32"),
        "y should be cast to f32 (mixed int/float). Got:\n{}",
        generated
    );
}

// 4. Compound assignment: price += 1 (f32 + int)
#[test]
fn test_compound_assignment_int_float() {
    let source = r#"
pub fn accumulate() -> f32 {
    let mut price = 0.0
    price += 1
    price
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        generated.contains("as f32"),
        "price += 1 should cast 1 to f32. Got:\n{}",
        generated
    );
}

// 5. Nested: (a + b / 2) as f32 - inner b/2 stays int
#[test]
fn test_nested_int_in_float_context() {
    let source = r#"
pub fn compute(a: i32, b: i32) -> f32 {
    (a + b / 2) as f32
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        !generated.contains(") as f32 / 2") && !generated.contains("as f32 / 2"),
        "b/2 should stay int. Got:\n{}",
        generated
    );
}

// 6. Struct field: self.count * 2 (both int)
#[test]
fn test_struct_field_int_division() {
    let source = r#"
pub struct Stats { count: i32 }
impl Stats {
    pub fn double(self) -> i32 {
        self.count * 2
    }
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        !generated.contains(" as f32") && !generated.contains(" as f64"),
        "self.count * 2 should stay int. Got:\n{}",
        generated
    );
}

// 7. i32 % 2 (both int)
#[test]
fn test_i32_mod_int_literal() {
    let source = r#"
pub fn is_even(x: i32) -> i32 {
    x % 2
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        !generated.contains(" as f32") && !generated.contains(" as f64"),
        "i32 % 2 should stay int. Got:\n{}",
        generated
    );
}

// 8. usize + f32 (mixed - should cast)
#[test]
fn test_usize_plus_f32_mixed() {
    let source = r#"
pub fn compute(x: usize, y: f32) -> f32 {
    x + y
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        generated.contains("as f32"),
        "usize + f32 should cast x. Got:\n{}",
        generated
    );
}

// 9. Const: 1.0 + 1 (immut context)
#[test]
fn test_const_int_float() {
    let source = r#"
pub const SCALE: f32 = 1.0 + 1
pub fn get_scale() -> f32 { SCALE }
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        generated.contains("as f32"),
        "1.0 + 1 in const should cast. Got:\n{}",
        generated
    );
}

// 10. Vec len: items.len() - 1 (both int)
#[test]
fn test_vec_len_minus_one() {
    let source = r#"
pub fn last_index(items: Vec<u32>) -> i32 {
    items.len() as i32 - 1
}
pub fn main() {}
"#;
    let generated = compile_wj_to_rust(source).expect("compile");
    assert!(
        !generated.contains(" as f32") && !generated.contains(" as f64"),
        "Int - int should stay int. Got:\n{}",
        generated
    );
}
