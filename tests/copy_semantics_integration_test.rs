//! Copy Semantics Integration Tests
//!
//! Verifies that CopySemantics layer correctly integrates with OwnershipTracker,
//! producing effective ownership for code generation. Key rule: &Copy → Owned
//! (Rust auto-copies, so no explicit * needed).

use std::fs;
use std::process::Command;

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, wj_source).expect("write");
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

// 1. Borrowed Copy param: x + 1 not *x + 1 when x is &i32
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_copy_param_effective_owned() {
    let src = r#"
pub fn process(x: i32) -> i32 {
    x + 1
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    // When x is inferred borrowed (&i32), i32 is Copy so effective: Owned
    // Should use x + 1, not *x + 1
    assert!(
        result.contains("x + 1") || result.contains("x + (1)"),
        "Should use x + 1 for Copy param. Got:\n{}",
        result
    );
    assert!(
        !result.contains("*x + 1"),
        "Should NOT add * for Copy type. Got:\n{}",
        result
    );
}

// 2. Borrowed non-Copy param: s.len() works (auto-deref)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_noncopy_param_effective_borrowed() {
    let src = r#"
pub fn process(s: string) -> usize {
    s.len()
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    // s is string (may be borrowed), .len() auto-derefs
    assert!(
        result.contains("s.len()"),
        "Should use s.len() for string. Got:\n{}",
        result
    );
}

// 3. Tuple destructure: (x, y) from items[0] when (i32, i32) is Copy
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_tuple_destructure_copy_elements_owned() {
    let src = r#"
pub fn process(items: Vec<(i32, i32)>) -> i32 {
    let (x, y) = items[0]
    x + y
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    // items[0] yields &(i32, i32), Copy so destructured x, y are Owned
    assert!(
        result.contains("x + y") || result.contains("x + (y)"),
        "Should use x + y directly. Got:\n{}",
        result
    );
    assert!(
        !result.contains("*x + *y"),
        "Should NOT add * for Copy tuple elements. Got:\n{}",
        result
    );
}

// 4. Custom Copy type from registry
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_custom_copy_type_from_registry() {
    let src = r#"
@derive(Copy, Clone)
pub struct Entity {
    pub id: i32
}

pub fn process(e: Entity) -> i32 {
    e.id
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    // e may be &Entity (borrowed), Entity is Copy so effective: Owned
    assert!(
        result.contains("e.id"),
        "Should use e.id for Copy struct. Got:\n{}",
        result
    );
}

// 5. Field access on Copy field from borrowed struct
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_field_access_copy_field_from_borrowed_struct() {
    let src = r#"
pub struct Point {
    pub x: i32,
    pub y: i32
}

pub fn process(p: Point) -> i32 {
    p.x + p.y
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    // p.x yields i32 (Copy), effective: Owned
    assert!(
        result.contains("p.x") && result.contains("p.y"),
        "Should use p.x + p.y. Got:\n{}",
        result
    );
}

// 6. Literal always owned
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_literal_always_owned() {
    let src = r#"
pub fn process() -> i32 {
    42
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(result.contains("42"), "Should have literal. Got:\n{}", result);
}

// 7. Binary op with two Copy operands
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_op_copy_operands() {
    let src = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("a + b") || result.contains("a + (b)"),
        "Should use a + b. Got:\n{}",
        result
    );
}

// 8. Method call on Copy receiver
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_copy_receiver() {
    let src = r#"
@derive(Copy, Clone)
pub struct Id(i32)

impl Id {
    pub fn value(self) -> i32 {
        self.0
    }
}

pub fn process(id: Id) -> i32 {
    id.value()
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("id.value()"),
        "Should use id.value(). Got:\n{}",
        result
    );
}

// 9. Option<Copy> - Option<i32> is Copy
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_copy_type() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    match opt {
        Some(x) => x,
        None => 0
    }
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("Some(x)") || result.contains("Some(ref x)"),
        "Should handle Option. Got:\n{}",
        result
    );
}

// 10. For-loop over Vec<Copy>
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_copy_elements() {
    let src = r#"
pub fn sum(items: Vec<i32>) -> i32 {
    let mut total = 0
    for item in items {
        total += item
    }
    total
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("total += item") || result.contains("total = total + item"),
        "Should use item in loop. Got:\n{}",
        result
    );
}

// 11. Nested struct with Copy fields
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_struct_copy_fields() {
    let src = r#"
pub struct Inner {
    pub val: i32
}

pub struct Outer {
    pub inner: Inner
}

pub fn process(o: Outer) -> i32 {
    o.inner.val
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("o.inner.val") || result.contains("o.inner .val"),
        "Should access nested field. Got:\n{}",
        result
    );
}

// 12. Comparison of Copy values
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_comparison_copy_values() {
    let src = r#"
pub fn eq(a: i32, b: i32) -> bool {
    a == b
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("a == b"),
        "Should compare a == b. Got:\n{}",
        result
    );
}

// 13. Array index yields Copy element
#[test]
fn test_array_index_copy_element() {
    let src = r#"
pub fn first(arr: Vec<i32>) -> i32 {
    arr[0]
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("arr[0]") || result.contains("arr [0]"),
        "Should index array. Got:\n{}",
        result
    );
}

// 14. Float Copy type
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_float_copy_type() {
    let src = r#"
pub fn add_f32(a: f32, b: f32) -> f32 {
    a + b
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("a + b") || result.contains("a + (b)"),
        "Should add f32. Got:\n{}",
        result
    );
}

// 15. Bool Copy type
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_bool_copy_type() {
    let src = r#"
pub fn and(a: bool, b: bool) -> bool {
    a && b
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("a && b"),
        "Should use a && b. Got:\n{}",
        result
    );
}

// 16. Match arm with Copy binding
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_copy_binding() {
    let src = r#"
pub fn get_val(opt: Option<i32>) -> i32 {
    match opt {
        Some(v) => v,
        None => 0
    }
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("Some") && result.contains("=>"),
        "Should match Option. Got:\n{}",
        result
    );
}

// 17. Struct literal with Copy fields
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_literal_copy_fields() {
    let src = r#"
pub struct Point {
    pub x: i32,
    pub y: i32
}

pub fn origin() -> Point {
    Point { x: 0, y: 0 }
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("Point {") && result.contains("x: 0") && result.contains("y: 0"),
        "Should create struct literal. Got:\n{}",
        result
    );
}

// 18. Tuple (i32, i32) Copy
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_tuple_copy() {
    let src = r#"
pub fn swap(p: (i32, i32)) -> (i32, i32) {
    let (a, b) = p
    (b, a)
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("(b, a)") || result.contains("(b , a)"),
        "Should return swapped tuple. Got:\n{}",
        result
    );
}

// 19. usize Copy type
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_usize_copy_type() {
    let src = r#"
pub fn double(n: usize) -> usize {
    n + n
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("n + n") || result.contains("n + (n)"),
        "Should use n + n. Got:\n{}",
        result
    );
}

// 20. Copy type in if-else
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_copy_type_in_if_else() {
    let src = r#"
pub fn max(a: i32, b: i32) -> i32 {
    if a > b {
        a
    } else {
        b
    }
}
pub fn main() {}
"#;

    let result = compile_to_rust(src).expect("compile");
    assert!(
        result.contains("a > b") && result.contains("a") && result.contains("b"),
        "Should use a and b in branches. Got:\n{}",
        result
    );
}
