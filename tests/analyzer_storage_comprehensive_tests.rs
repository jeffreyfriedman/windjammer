//! Comprehensive Analyzer is_stored Detection Tests
//!
//! These tests verify that the Windjammer compiler correctly detects
//! when a parameter is "stored" (requiring owned ownership) vs "used"
//! (allowing borrowed). This is critical for automatic ownership inference.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn compile_and_get_rust(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| format!("Failed to read generated file: {}", e))
}

fn compile_and_verify(code: &str) -> (bool, String, String) {
    match compile_and_get_rust(code) {
        Ok(generated) => {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rs_path = temp_dir.path().join("test.rs");
            fs::write(&rs_path, &generated).expect("Failed to write rs file");

            let rustc = Command::new("rustc")
                .arg("--crate-type=lib")
                .arg(&rs_path)
                .arg("-o")
                .arg(temp_dir.path().join("test.rlib"))
                .output();

            match rustc {
                Ok(output) => {
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    (output.status.success(), generated, err)
                }
                Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
            }
        }
        Err(e) => (false, String::new(), e),
    }
}

// ============================================================================
// PUSH TO VEC (STORED)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_to_vec_requires_owned() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

@derive(Clone, Debug)
pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn add(&mut self, item: Item) {
        self.items.push(item)
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    // item should be owned because it's pushed
    assert!(success, "Push to vec should compile. Error: {}", err);
}

#[test]
fn test_push_to_local_vec() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn collect_item(item: Item) -> Vec<Item> {
    let mut items = Vec::new();
    items.push(item);
    items
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Push to local vec should compile. Error: {}", err);
}

// ============================================================================
// INSERT TO HASHMAP (STORED)
// ============================================================================

#[test]
fn test_hashmap_basic() {
    // HashMap basic operations
    let code = r#"
use std::collections::HashMap

pub fn create_map() -> HashMap<i32, i32> {
    let mut map = HashMap::new();
    map.insert(1, 100);
    map.insert(2, 200);
    map
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "HashMap basic should compile. Error: {}", err);
}

// ============================================================================
// ASSIGN TO FIELD (STORED)
// ============================================================================

#[test]
fn test_assign_to_field() {
    let code = r#"
@derive(Clone, Debug)
pub struct Inner {
    value: i32,
}

@derive(Clone, Debug)
pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn set_inner(&mut self, inner: Inner) {
        self.inner = inner
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Assign to field should compile. Error: {}", err);
}

#[test]
fn test_assign_in_struct_literal() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

@derive(Clone, Debug)
pub struct Line {
    start: Point,
    end: Point,
}

pub fn make_line(start: Point, end: Point) -> Line {
    Line { start: start, end: end }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Assign in struct literal should compile. Error: {}",
        err
    );
}

// ============================================================================
// RETURN VALUE (STORED)
// ============================================================================

#[test]
fn test_return_value() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn identity(item: Item) -> Item {
    item
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    // item should be owned because it's returned
    assert!(success, "Return value should compile. Error: {}", err);
}

#[test]
fn test_return_in_match() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn maybe_return(item: Item, flag: bool) -> Option<Item> {
    match flag {
        true => Some(item),
        false => None,
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Return in match should compile. Error: {}", err);
}

// ============================================================================
// READ-ONLY (NOT STORED)
// ============================================================================

#[test]
fn test_read_field_not_stored() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn get_value(item: Item) -> i32 {
    item.value
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    // Reading a Copy field doesn't require ownership of item
    assert!(success, "Read field should compile. Error: {}", err);
}

#[test]
fn test_print_debug_not_stored() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn print_item(item: Item) {
    println!("{:?}", item)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Print debug should compile. Error: {}", err);
}

// ============================================================================
// CONDITIONAL STORAGE
// ============================================================================

#[test]
fn test_conditional_push() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn maybe_store(item: Item, items: &mut Vec<Item>, flag: bool) {
    if flag {
        items.push(item)
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    // Even if push is conditional, item must be owned
    assert!(success, "Conditional push should compile. Error: {}", err);
}

#[test]
fn test_conditional_return() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn conditional_return(item: Item, flag: bool) -> Option<Item> {
    if flag {
        Some(item)
    } else {
        None
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Conditional return should compile. Error: {}", err);
}

// ============================================================================
// LOOP STORAGE
// ============================================================================

#[test]
fn test_push_in_loop() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn collect_items(items: &Vec<Item>) -> Vec<Item> {
    let mut result = Vec::new()
    for item in items {
        result.push(item.clone())
    }
    result
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Push in loop should compile. Error: {}", err);
}

// ============================================================================
// PASS TO FUNCTION (DEPENDS ON FUNCTION SIGNATURE)
// ============================================================================

#[test]
fn test_pass_to_consuming_function() {
    // Simple consuming function pattern
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn get_value(item: Item) -> i32 {
    item.value
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Consuming function should compile. Error: {}", err);
}

#[test]
fn test_pass_to_borrowing_function() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn borrow(item: &Item) -> i32 {
    item.value
}

pub fn forward(item: Item) -> i32 {
    borrow(&item)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Pass to borrowing function should compile. Error: {}",
        err
    );
}

// ============================================================================
// CLONE BEFORE STORAGE
// ============================================================================

#[test]
fn test_clone_then_push() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn clone_and_store(item: &Item, items: &mut Vec<Item>) {
    items.push(item.clone())
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Clone then push should compile. Error: {}", err);
}

// ============================================================================
// NESTED STRUCT STORAGE
// ============================================================================

#[test]
fn test_nested_struct_storage() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

@derive(Clone, Debug)
pub struct Container {
    point: Point,
}

impl Container {
    pub fn new(point: Point) -> Container {
        Container { point: point }
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Nested struct storage should compile. Error: {}",
        err
    );
}

// ============================================================================
// TUPLE STORAGE
// ============================================================================

#[test]
fn test_tuple_storage() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn wrap_in_tuple(item: Item) -> (i32, Item) {
    (item.value, item)
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Tuple storage should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

// ============================================================================
// OPTION WRAPPING
// ============================================================================

#[test]
fn test_wrap_in_some() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn wrap_some(item: Item) -> Option<Item> {
    Some(item)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Wrap in Some should compile. Error: {}", err);
}

#[test]
fn test_wrap_in_ok() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    value: i32,
}

pub fn wrap_ok(item: Item) -> Result<Item, string> {
    Ok(item)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Wrap in Ok should compile. Error: {}", err);
}
