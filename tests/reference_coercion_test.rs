//! TDD: Automatic Reference Coercion
//!
//! Rust auto-coerces references. So should Windjammer.
//!
//! Patterns implemented:
//! 1. Auto-borrow: fn foo(x: &Vec<i32>) { }  foo(v)  → foo(&v)
//! 2. Auto-deref:  fn foo(x: i32) { }  foo(&r)  → foo(*r)
//! 3. Auto-ref for method calls: r.process() when process takes owned self → (*r).process()
//! 4. Auto-deref for binary ops: let x = &5; x + y → *x + y

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc_output {
        Ok(output) => {
            let rustc_success = output.status.success();
            let rustc_err = String::from_utf8_lossy(&output.stderr).to_string();
            (rustc_success, generated, rustc_err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

// =============================================================================
// Pattern 1: Auto-borrow for reference parameters
// =============================================================================

/// fn foo(x: &Vec<i32>) { }  let v = vec![1,2,3];  foo(v)  → foo(&v)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_borrow_owned_vec_to_ref_param() {
    let code = r#"
pub fn process_items(items: &Vec<i32>) -> i32 {
    let mut sum = 0
    for i in items {
        sum = sum + *i
    }
    sum
}

pub fn main() -> i32 {
    let v = vec![1, 2, 3]
    process_items(v)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("process_items(&v)"),
        "Should auto-borrow: process_items(v) → process_items(&v). Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// fn foo(x: &String) { }  foo(s)  → foo(&s)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_borrow_owned_string_to_ref_param() {
    let code = r#"
pub fn print_len(s: &string) -> usize {
    s.len()
}

pub fn main() -> usize {
    let text = "hello".to_string()
    print_len(text)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("print_len(&text)") || generated.contains("print_len(text"),
        "Should auto-borrow owned String. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// Nested: foo(&vec[i]) - vec[i] is T, param is &T → &vec[i]
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_borrow_index_result_to_ref_param() {
    let code = r#"
pub struct Point { x: f32, y: f32 }

pub fn distance(p: &Point) -> f32 {
    (p.x * p.x + p.y * p.y).sqrt()
}

pub fn main() -> f32 {
    let points = vec![Point { x: 1.0, y: 0.0 }, Point { x: 0.0, y: 1.0 }]
    distance(points[0])
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    // Should generate distance(&points[0]) or distance(points[0].clone()) depending on Copy
    // Point has all Copy fields, so points[0] may be auto-cloned. For &Point param we need &.
    assert!(
        generated.contains("distance(&points[0])") || generated.contains("distance(points[0]"),
        "Should handle index to ref param. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Pattern 2: Auto-deref for Copy types
// =============================================================================

/// fn foo(x: i32) { }  let r = &42;  foo(r)  → foo(*r)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_deref_ref_copy_to_value_param() {
    let code = r#"
pub fn double(x: i32) -> i32 {
    x * 2
}

pub fn main() -> i32 {
    let n = 42
    let r = &n
    double(r)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("double(*r)") || generated.contains("double(r)"),
        "Should auto-deref &i32 when param expects i32. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// fn foo(x: f32) { }  foo(&f)  → foo(*f)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_deref_ref_f32_to_value_param() {
    let code = r#"
pub fn scale(v: f32, factor: f32) -> f32 {
    v * factor
}

pub fn main() -> f32 {
    let x = 1.5
    let r = &x
    scale(r, 2.0)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("scale(*r") || generated.contains("scale(r"),
        "Should auto-deref &f32. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// fn foo(x: u32) { }  match returns &u32  → foo(*v)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_deref_match_binding_to_value_param() {
    let code = r#"
pub fn add_one(x: u32) -> u32 {
    x + 1
}

pub fn main() -> u32 {
    let nums = vec![1u32, 2u32, 3u32]
    match nums.get(0) {
        Some(v) => add_one(v),
        None => 0
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    // Vec::get returns Option<&T>, so v is &u32. add_one expects u32.
    assert!(
        generated.contains("add_one(*v)") || generated.contains("add_one(v)"),
        "Should auto-deref match binding &u32. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Pattern 3: Auto-ref for method calls (r.process() when process takes owned self)
// =============================================================================

/// r.process() when process takes owned self → (*r).process()
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_deref_method_receiver_ref_to_owned_self() {
    let code = r#"
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn get(self) -> i32 {
        self.value
    }
}

pub fn main() -> i32 {
    let c = Counter { value: 42 }
    let r = &c
    r.get()
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    // r.get() where get takes self - need (*r).get() to deref the ref
    assert!(
        generated.contains("(*r).get()") || generated.contains("r.clone().get()"),
        "Should deref ref receiver when method takes owned self. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Pattern 4: Auto-deref for binary operations
// =============================================================================

/// let x = &5; x + y  → *x + y
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_deref_binary_op_ref_plus_value() {
    let code = r#"
pub fn main() -> i32 {
    let x = 5
    let r = &x
    let y = 3
    r + y
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("*r + y") || generated.contains("r + y"),
        "Should auto-deref in binary op. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// let x = &5; x * 2  → *x * 2
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_deref_binary_op_ref_times_literal() {
    let code = r#"
pub fn main() -> i32 {
    let n = 10
    let r = &n
    r * 2
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("*r * 2") || generated.contains("r * 2"),
        "Should auto-deref ref in multiplication. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// Comparison: &x == y
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_deref_binary_op_comparison() {
    let code = r#"
pub fn main() -> bool {
    let x = 42
    let r = &x
    r == 42
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("*r == 42") || generated.contains("r == 42"),
        "Should handle ref in comparison. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

// =============================================================================
// Combined / Edge cases
// =============================================================================

/// Multiple args: some need borrow, some need deref
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mixed_coercion_multiple_args() {
    let code = r#"
pub fn compute(items: &Vec<i32>, index: usize) -> i32 {
    items[index]
}

pub fn main() -> i32 {
    let v = vec![1, 2, 3]
    let i = 1
    let ri = &i
    compute(v, ri)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    // compute expects (&Vec<i32>, usize). We pass (Vec<i32>, &usize).
    // Need: compute(&v, *ri)
    assert!(
        (generated.contains("compute(&v") || generated.contains("compute(v")) &&
        (generated.contains(", *ri)") || generated.contains(", ri)")),
        "Should coerce both args. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// Method with &self - no coercion needed for receiver
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_ref_self_no_receiver_coercion() {
    let code = r#"
pub struct Data { value: i32 }

impl Data {
    pub fn get(self) -> i32 {
        self.value
    }
}

pub fn main() -> i32 {
    let d = Data { value: 10 }
    d.get()
}
"#;

    let (success, _, err) = compile_and_verify(code);

    assert!(success, "d.get() with owned self should compile. Error:\n{}", err);
}

/// Vec::contains with owned value - needs &
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_contains_auto_borrow() {
    let code = r#"
pub fn has_item(items: Vec<i32>, search: i32) -> bool {
    items.contains(search)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    // Vec::contains expects &T, we pass T. Need &search
    assert!(
        generated.contains("contains(&search)") || generated.contains("contains(search)"),
        "Vec::contains should get correct arg. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// String literal to &str param - already correct, no double &
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_no_double_ref() {
    let code = r#"
pub fn check(s: &str) -> bool {
    s.len() > 0
}

pub fn main() -> bool {
    check("hello")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        !generated.contains("check(&\"hello\")"),
        "String literal is already &str, no extra &. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// Custom struct with &param
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_custom_struct_auto_borrow() {
    let code = r#"
pub struct Config { name: string }

pub fn process(c: &Config) -> string {
    c.name.clone()
}

pub fn main() -> string {
    let cfg = Config { name: "test".to_string() }
    process(cfg)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("process(&cfg)") || generated.contains("process(cfg"),
        "Should auto-borrow Config. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// Option::unwrap with &Option - needs deref or as_ref
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_ref_unwrap() {
    let code = r#"
pub fn get_value(opt: &Option<i32>) -> i32 {
    opt.unwrap()
}
"#;

    let (success, _, err) = compile_and_verify(code);

    // Option::unwrap takes self. &Option<T> has .unwrap() that consumes - actually
    // Option impl has unwrap(&self) that returns T when T: Copy. So this might work.
    assert!(success, "Option::unwrap with &Option should compile. Error:\n{}", err);
}
