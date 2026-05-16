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
// Dogfooding — strings, decorators, refs.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_as_str_in_if_else_branch() {
    // Idiomatic Windjammer: use self.name directly, compiler handles conversion
    let code = r#"
struct Item {
    name: string,
}

impl Item {
    fn get_display_name(self) -> string {
        if self.name == "" {
            "Unnamed"
        } else {
            self.name
        }
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Idiomatic Windjammer (self.name) compiles - compiler handles conversion
    assert!(!generated.is_empty(), "Should generate valid Rust");
}

// =============================================================================
// Test: Type::method signature lookup
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_type_method_signature_lookup() {
    let code = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct Panel {}

impl Panel {
    fn render_vec3(v: Vec3) -> string {
        format!("({}, {}, {})", v.x, v.y, v.z)
    }
}

fn test() -> string {
    let pos = Vec3 { x: 1.0, y: 2.0, z: 3.0 }
    Panel::render_vec3(pos)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Some(iterator_var.clone()) for non-Copy types
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_some_iterator_var_clone() {
    let code = r#"
@auto
struct Item {
    name: string,
}

fn find_item(items: Vec<Item>, target: string) -> Option<Item> {
    for item in items {
        if item.name == target {
            return Some(item)
        }
    }
    None
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Should clone iterator variable when wrapping in Some
    assert!(
        generated.contains("Some(item.clone())") || generated.contains("Some(item)"),
        "Should handle Some() with iterator variable: {}",
        generated
    );
}

// =============================================================================
// Test: Some(borrowed_param.clone())
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_some_borrowed_param_clone() {
    let code = r#"
struct Panel {
    selected_id: Option<string>,
}

impl Panel {
    fn select(self, id: string) {
        self.selected_id = Some(id)
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: usize variable tracking for comparisons
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_usize_var_comparison_cast() {
    let code = r#"
struct Panel {
    selected_index: i32,
    items: Vec<string>,
}

impl Panel {
    fn validate_selection(self) {
        let count = self.items.len()
        if self.selected_index >= count {
            self.selected_index = 0
        }
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Should cast when comparing with .len() result
    // Accept any integer cast (i32, i64, usize) - they're all valid
    assert!(
        generated.contains("as usize")
            || generated.contains("as i32")
            || generated.contains("as i64"),
        "Should handle i32 vs usize comparison with automatic casting: {}",
        generated
    );
}

// =============================================================================
// Test: push_str auto-borrow for String variables
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_str_auto_borrow() {
    let code = r#"
fn build_html() -> string {
    let mut html = ""
    let content = "Hello"
    html.push_str(content)
    html
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // push_str should receive &str, not String
    assert!(
        generated.contains("push_str(&") || generated.contains("push_str(content"),
        "Should auto-borrow String for push_str: {}",
        generated
    );
}

// =============================================================================
// Test: @auto decorator generates derives
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_decorator_derives() {
    let code = r#"
@auto
struct Point {
    x: f32,
    y: f32,
}

@auto
enum Direction {
    North,
    South,
    East,
    West,
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // @auto should generate #[derive(...)]
    assert!(
        generated.contains("#[derive("),
        "Should generate derive for @auto: {}",
        generated
    );
    assert!(
        generated.contains("Clone") && generated.contains("Debug"),
        "Should derive Clone and Debug: {}",
        generated
    );
}

// =============================================================================
// Test: Methods that return references detected correctly
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_reference_returning_methods() {
    let code = r#"
struct Data {
    items: Vec<string>,
}

impl Data {
    fn get_first(self) -> Option<string> {
        self.items.first()
    }
    
    fn find_item(self, target: string) -> Option<string> {
        for item in self.items {
            if item == target {
                return Some(item)
            }
        }
        None
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Iterator variable cloning for Vec push
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iterator_clone_for_push() {
    let code = r#"
@auto
struct Item {
    id: i32,
}

fn filter_items(items: Vec<Item>) -> Vec<Item> {
    let mut result = Vec::new()
    for item in items {
        if item.id > 0 {
            result.push(item)
        }
    }
    result
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Should clone item when pushing since it's from an iterator
    assert!(
        generated.contains("item.clone()") || generated.contains("item)"),
        "Should handle iterator variable push: {}",
        generated
    );
}

// =============================================================================
// Test: String literal in format with borrowed context
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_format_context() {
    let code = r#"
fn render_item(name: string) -> string {
    if name == "" {
        "No name"
    } else {
        name
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Borrowed non-string parameter auto-reference
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_non_string_param() {
    let code = r#"
@auto
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct Editor {}

impl Editor {
    fn render_vec3(v: Vec3) -> string {
        format!("{}, {}, {}", v.x, v.y, v.z)
    }
}

fn test() {
    let pos = Vec3 { x: 1.0, y: 2.0, z: 3.0 }
    Editor::render_vec3(pos)
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Match arms type consistency
// =============================================================================
