//! Comprehensive Analyzer is_stored Detection Tests
//!
//! These tests verify that the Windjammer compiler correctly detects
//! when a parameter is "stored" (requiring owned ownership) vs "used"
//! (allowing borrowed). This is critical for automatic ownership inference.

#[path = "test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    // item should be owned because it's pushed
    assert!(success, "Push to vec should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Push to local vec should compile. Error: {}", err);
}

// ============================================================================
// INSERT TO HASHMAP (STORED)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "HashMap basic should compile. Error: {}", err);
}

// ============================================================================
// ASSIGN TO FIELD (STORED)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Assign to field should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    // item should be owned because it's returned
    assert!(success, "Return value should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Return in match should compile. Error: {}", err);
}

// ============================================================================
// READ-ONLY (NOT STORED)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    // Reading a Copy field doesn't require ownership of item
    assert!(success, "Read field should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Print debug should compile. Error: {}", err);
}

// ============================================================================
// CONDITIONAL STORAGE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    // Even if push is conditional, item must be owned
    assert!(success, "Conditional push should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Conditional return should compile. Error: {}", err);
}

// ============================================================================
// LOOP STORAGE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Push in loop should compile. Error: {}", err);
}

// ============================================================================
// PASS TO FUNCTION (DEPENDS ON FUNCTION SIGNATURE)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Consuming function should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Clone then push should compile. Error: {}", err);
}

// ============================================================================
// NESTED STRUCT STORAGE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Wrap in Some should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Wrap in Ok should compile. Error: {}", err);
}
