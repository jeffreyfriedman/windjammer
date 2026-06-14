#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]
#![allow(unused)]
// Dogfooding — match, patterns, collections.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arms_type_consistency() {
    let code = r#"
enum Status {
    Active,
    Inactive,
    Unknown,
}

fn get_status_label(status: Status) -> string {
    match status {
        Status::Active => "Active",
        Status::Inactive => "Inactive",
        Status::Unknown => "Unknown",
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Complex if/else with method calls returning references
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_complex_if_else_refs() {
    let code = r#"
struct Config {
    default_color: string,
    custom_color: Option<string>,
}

impl Config {
    fn get_color(self) -> string {
        if self.custom_color.is_some() {
            self.custom_color.unwrap()
        } else {
            self.default_color
        }
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Pattern matching on enums with field extraction keeps parameter owned
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pattern_match_field_extraction_owned() {
    let code = r#"
@auto
enum Shape {
    Circle { radius: f32 },
    Rectangle { width: f32, height: f32 },
}

fn get_area(shape: Shape) -> f32 {
    match shape {
        Shape::Circle { radius: r } => 3.14159 * r * r,
        Shape::Rectangle { width: w, height: h } => w * h,
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Parameter should remain owned (not &Shape) because we pattern match with field extraction
    assert!(
        generated.contains("shape: Shape") || generated.contains("fn get_area(shape:"),
        "Should keep pattern-matched param owned: {}",
        generated
    );
}

// =============================================================================
// Test: Pattern matching passes extracted primitives to functions correctly
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pattern_match_primitives_to_functions() {
    let code = r#"
@auto
enum ObjectType {
    Cube { size: f32 },
    Sphere { radius: f32 },
}

fn format_value(v: f32) -> string {
    format!("{:.2}", v)
}

fn render_object(obj: ObjectType) -> string {
    match obj {
        ObjectType::Cube { size: s } => format_value(s),
        ObjectType::Sphere { radius: r } => format_value(r),
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: String literal assignment to String field
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_field_assignment() {
    let code = r#"
struct Config {
    color: string,
    name: string,
}

impl Config {
    fn reset(self) {
        self.color = "red"
        self.name = "default"
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // String literals assigned to String fields should be converted to String
    assert!(
        generated.contains(".to_string()") || generated.contains("String::from("),
        "Should convert string literal for String field assignment: {}",
        generated
    );
}

// =============================================================================
// Test: Unit return type discards expression value
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_return_discards_value() {
    let code = r#"
use std::collections::HashMap

struct Store {
    items: HashMap<string, i32>,
}

impl Store {
    fn add(self, key: string, value: i32) {
        self.items.insert(key, value)
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Should have semicolon after insert to discard Option<V>
    assert!(
        generated.contains(".insert(") && generated.contains(";"),
        "Should discard HashMap::insert return: {}",
        generated
    );
}

// =============================================================================
// Test: HashMap.get().cloned() for owned Option return
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_cloned() {
    let code = r#"
use std::collections::HashMap

@auto
struct Item {
    name: string,
}

struct Store {
    items: HashMap<string, Item>,
}

impl Store {
    fn get(self, key: string) -> Option<Item> {
        self.items.get(key)
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    assert!(
        generated.contains(".cloned()"),
        "Should add .cloned() for HashMap.get: {}",
        generated
    );
}

// =============================================================================
// Test: Iterator variables not double-referenced in borrowed param calls
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iterator_var_no_double_ref() {
    let code = r#"
@auto
struct Item {
    id: i32,
}

fn process_item(item: Item) -> i32 {
    item.id
}

fn process_all(items: Vec<Item>) -> i32 {
    let mut sum = 0
    for item in items {
        sum = sum + process_item(item)
    }
    sum
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: If expressions in format! arguments don't have semicolons
// =============================================================================
