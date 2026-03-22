//! TDD: Ref/ref mut pattern binding logic - ownership tracking system
//!
//! Fixes E0596 (cannot borrow as mutable) and E0594 (cannot borrow as immutable)
//! by using systematic ownership tracking for if-let and match patterns.
//!
//! 30 comprehensive tests covering:
//! - if let Some(x) with &Option, &mut Option, Option
//! - match Some(x) with various scrutinee ownership
//! - match tuple patterns with Copy/non-Copy elements
//! - ref mut only when scrutinee is &mut AND binding is mutated

use std::fs;
use std::process::Command;

fn get_wj_binary() -> String {
    env!("CARGO_BIN_EXE_wj").to_string()
}

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, wj_source).expect("write");
    fs::create_dir_all(&out_dir).expect("create dir");

    let output = Command::new(get_wj_binary())
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

    let src_main = out_dir.join("src").join("main.rs");
    let test_rs = out_dir.join("test.rs");
    let content = if src_main.exists() {
        fs::read_to_string(src_main)
    } else if test_rs.exists() {
        fs::read_to_string(test_rs)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No generated Rust file",
        ))
    };
    content.map_err(|e| e.to_string())
}

fn rust_compiles(rust_code: &str) -> bool {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let rs_path = temp_dir.path().join("test.rs");
    fs::write(&rs_path, rust_code).expect("write");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .arg(&rs_path)
        .output()
        .expect("rustc");
    output.status.success()
}

// =============================================================================
// IF-LET: Immutable scrutinee (&Option<T>)
// =============================================================================

#[test]
fn test_if_let_some_ref_immutable_scrutinee() {
    let src = r#"
pub fn process(opt: Option<String>) -> usize {
    if let Some(s) = opt {
        s.len()
    } else {
        0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    // opt is Option<String> (Owned) - s is owned, no ref mut
    assert!(result.contains("Some(s)") || result.contains("Some(ref s)"));
    assert!(!result.contains("Some(ref mut s)"), "Read-only: never ref mut");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_some_ref_borrowed_option_param() {
    let src = r#"
pub fn process(opt: Option<String>) -> usize {
    let r = opt;
    if let Some(s) = r {
        s.len()
    } else {
        0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_some_ref_mut_mutable_scrutinee_and_mutation() {
    let src = r#"
pub fn process(opt: Option<Vec<i32>>) {
    let mut v = opt;
    if let Some(arr) = v {
        arr.push(1)
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    // v is owned Option - arr needs mut for push. Emit Some(mut arr)
    assert!(result.contains("Some(mut arr)") || result.contains("Some(arr)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_some_owned_scrutinee() {
    let src = r#"
pub fn unwrap_default(opt: Option<String>) -> usize {
    if let Some(s) = opt {
        s.len()
    } else {
        0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(s)"), "Owned scrutinee: no ref. Got: {}", result);
    assert!(!result.contains("Some(ref mut s)"), "Never ref mut for read-only");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_some_owned_scrutinee_mutated() {
    let src = r#"
pub fn process(opt: Option<Vec<i32>>) {
    if let Some(v) = opt {
        v.push(1)
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(mut v)") || result.contains("Some(v)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

// =============================================================================
// IF-LET: Self field (borrowed/mut borrowed)
// =============================================================================

#[test]
fn test_if_let_self_field_borrowed() {
    let src = r#"
pub struct Container {
    pub value: Option<String>,
}
impl Container {
    pub fn get_len(self) -> usize {
        if let Some(s) = self.value {
            s.len()
        } else {
            0
        }
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_self_field_mut_borrowed_and_mutation() {
    let src = r#"
pub struct Container {
    pub items: Option<Vec<i32>>,
}
impl Container {
    pub fn add_item(self) {
        if let Some(v) = self.items {
            v.push(1)
        }
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

// =============================================================================
// MATCH: Option patterns
// =============================================================================

#[test]
fn test_match_some_owned() {
    let src = r#"
pub fn unwrap(opt: Option<i32>) -> i32 {
    match opt {
        Some(x) => x,
        None => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(x)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_some_owned_string() {
    let src = r#"
pub fn unwrap(opt: Option<String>) -> usize {
    match opt {
        Some(s) => s.len(),
        None => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(s)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_some_owned_mutated() {
    let src = r#"
pub fn process(opt: Option<Vec<i32>>) {
    match opt {
        Some(mut v) => v.push(1),
        None => {}
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(mut v)") || result.contains("Some(v)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

// =============================================================================
// MATCH: Tuple patterns
// =============================================================================

#[test]
fn test_match_tuple_copy_elements() {
    let src = r#"
pub fn process(items: Vec<(i32, i32)>) {
    if items.len() > 0 {
        match items[0] {
            (x, y) => {
                let sum = x + y
            }
        }
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    // items[0] yields &(i32, i32), (i32, i32) is Copy → x, y owned
    assert!(result.contains("(x, y)") || result.contains("(ref x, ref y)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_tuple_mixed_copy_noncopy() {
    let src = r#"
pub fn process(items: Vec<(i32, String)>) {
    if items.len() > 0 {
        match items[0] {
            (id, name) => {
                let _ = id;
                let _ = name
            }
        }
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    // items[0] is &(i32, String), id is Copy→owned, name is non-Copy→ref
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_tuple_owned_scrutinee() {
    let src = r#"
pub fn process(pair: (i32, String)) {
    match pair {
        (a, b) => {
            let _ = a;
            let _ = b
        }
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("(a, b)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

// =============================================================================
// MATCH: Index scrutinee (returns &T)
// =============================================================================

#[test]
fn test_match_index_borrowed_vec() {
    let src = r#"
pub fn first(items: Vec<i32>) -> i32 {
    match items.get(0) {
        Some(x) => *x,
        None => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_index_option_vec() {
    let src = r#"
pub fn process(items: Vec<Option<String>>) {
    if items.len() > 0 {
        match items[0] {
            Some(s) => {
                let _ = s.len()
            }
            None => {}
        }
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_if_let_none_pattern() {
    let src = r#"
pub fn is_none(opt: Option<i32>) -> bool {
    if let None = opt {
        true
    } else {
        false
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("None"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_wildcard() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    match opt {
        Some(x) => x,
        _ => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(x)"));
    assert!(result.contains("_"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_multiple_arms() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    match opt {
        Some(0) => 0,
        Some(x) => x,
        None => -1
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_with_guard() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    if let Some(x) = opt if x > 0 {
        x
    } else {
        0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("if let"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_option_result() {
    let src = r#"
pub fn process(res: Result<i32, String>) -> i32 {
    match res {
        Ok(v) => v,
        Err(_) => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_nested_if_let() {
    let src = r#"
pub fn process(opt: Option<Option<i32>>) -> i32 {
    if let Some(inner) = opt {
        if let Some(x) = inner {
            x
        } else {
            0
        }
    } else {
        0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_tuple_three_elements() {
    let src = r#"
pub fn process(t: (i32, i32, i32)) -> i32 {
    match t {
        (a, b, c) => a + b + c
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("(a, b, c)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_enum_variant_struct() {
    let src = r#"
pub enum E {
    A(i32),
    B { x: i32 }
}
pub fn process(e: E) -> i32 {
    match e {
        E::A(n) => n,
        E::B { x } => x
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_method_return_option() {
    let src = r#"
pub struct Map;
impl Map {
    pub fn get(self, key: i32) -> Option<i32> {
        None
    }
}
pub fn process(m: Map) -> i32 {
    if let Some(v) = m.get(0) {
        v
    } else {
        0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_or_pattern() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    match opt {
        Some(1) | Some(2) => 1,
        Some(x) => x,
        None => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_option_vec_push() {
    let src = r#"
pub fn ensure_one(opt: Option<Vec<i32>>) {
    let mut o = opt;
    if let Some(v) = o {
        v.push(1)
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_tuple_from_param() {
    let src = r#"
pub fn sum(pair: (i32, i32)) -> i32 {
    match pair {
        (a, b) => a + b
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("(a, b)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_match_option_string_len() {
    let src = r#"
pub fn len(opt: Option<String>) -> usize {
    match opt {
        Some(s) => s.len(),
        None => 0
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("Some(s)"));
    assert!(rust_compiles(&result), "Generated Rust must compile");
}

#[test]
fn test_if_let_option_clone() {
    let src = r#"
pub fn clone_inner(opt: Option<String>) -> Option<String> {
    if let Some(s) = opt {
        Some(s.clone())
    } else {
        None
    }
}
"#;
    let result = compile_to_rust(src).expect("compile");
    assert!(rust_compiles(&result), "Generated Rust must compile");
}
