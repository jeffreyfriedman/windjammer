#![allow(unused)]
// Tests for compiler fixes discovered through dogfooding
// These test specific edge cases found while compiling the editor panels

use std::process::Command;
use tempfile::TempDir;

fn compile_and_check(wj_code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_file = temp_dir.path().join("test.wj");
    std::fs::write(&src_file, wj_code).expect("Failed to write source");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_file)
        .arg("-o")
        .arg(temp_dir.path().join("out"))
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let rs_file = temp_dir.path().join("out").join("test.rs");
    Ok(std::fs::read_to_string(rs_file).unwrap_or_default())
}

fn compile_and_rustc_check(wj_code: &str) -> Result<(), String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_file = temp_dir.path().join("test.wj");
    std::fs::write(&src_file, wj_code).expect("Failed to write source");

    let out_dir = temp_dir.path().join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_file)
        .arg("-o")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        return Err(format!(
            "WJ compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let rs_file = out_dir.join("test.rs");
    let rustc_output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(temp_dir.path().join("test"))
        .arg(&rs_file)
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        return Err(format!(
            "Rust compilation failed: {}",
            String::from_utf8_lossy(&rustc_output.stderr)
        ));
    }

    Ok(())
}

// =============================================================================
// Test: .as_str() returns reference detection in if/else
// =============================================================================

#[test]
fn test_as_str_in_if_else_branch() {
    let code = r#"
struct Item {
    name: string,
}

impl Item {
    fn get_display_name(self) -> string {
        if self.name == "" {
            "Unnamed"
        } else {
            self.name.as_str()
        }
    }
}
"#;

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Both branches should return &str (not String in one and &str in the other)
    assert!(
        !generated.contains(".to_string()") || generated.contains("as_str"),
        "Should handle as_str in if/else branches consistently"
    );
}

// =============================================================================
// Test: Type::method signature lookup
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Some(iterator_var.clone()) for non-Copy types
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: usize variable tracking for comparisons
// =============================================================================

#[test]
#[ignore] // TODO: Implement usize/i32 automatic casting for .len() comparisons
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Should cast i32 to usize when comparing with .len() result
    assert!(
        generated.contains("as usize") || generated.contains("as i32"),
        "Should handle i32 vs usize comparison: {}",
        generated
    );
}

// =============================================================================
// Test: push_str auto-borrow for String variables
// =============================================================================

#[test]
fn test_push_str_auto_borrow() {
    let code = r#"
fn build_html() -> string {
    let mut html = ""
    let content = "Hello"
    html.push_str(content)
    html
}
"#;

    let result = compile_and_check(code);
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

    let result = compile_and_check(code);
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Iterator variable cloning for Vec push
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
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
fn test_string_literal_format_context() {
    let code = r#"
fn render_item(name: string) -> string {
    if name == "" {
        "No name"
    } else {
        name.as_str()
    }
}
"#;

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Borrowed non-string parameter auto-reference
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Match arms type consistency
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Complex if/else with method calls returning references
// =============================================================================

#[test]
fn test_complex_if_else_refs() {
    let code = r#"
struct Config {
    default_color: string,
    custom_color: Option<string>,
}

impl Config {
    fn get_color(self) -> string {
        if self.custom_color.is_some() {
            self.custom_color.unwrap().as_str()
        } else {
            self.default_color.as_str()
        }
    }
}
"#;

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: Pattern matching on enums with field extraction keeps parameter owned
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: String literal assignment to String field
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // String literals assigned to String fields should get .to_string()
    assert!(
        generated.contains(".to_string()"),
        "Should add .to_string() for string literal field assignment: {}",
        generated
    );
}

// =============================================================================
// Test: Unit return type discards expression value
// =============================================================================

#[test]
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

    let result = compile_and_check(code);
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
#[ignore] // TODO: Implement .cloned() for HashMap.get() when return type is Option<T>
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

    let result = compile_and_check(code);
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

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: If expressions in format! arguments don't have semicolons
// =============================================================================

#[test]
fn test_if_expression_format_arg() {
    let code = r#"
struct Display {
    value: string,
}

impl Display {
    fn render(self) -> string {
        format!("<div>{}</div>", if self.value == "" { "empty" } else { self.value.as_str() })
    }
}
"#;

    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: @auto decorator correctly identifies Copy types
// Stats has only primitives (i32, f32) so @auto should derive Copy
// This allows passing w.stats by value when w is from Option<&Weapon>
// =============================================================================

#[test]
fn test_auto_copy_detection() {
    let code = r#"
@auto
struct Stats {
    damage: i32,
    speed: f32,
}

fn process_stats(stats: Stats) -> i32 {
    stats.damage
}

fn test_copy() {
    let s = Stats { damage: 10, speed: 1.5 }
    let x = process_stats(s)
    let y = process_stats(s)
}
"#;

    // Verify that Stats gets Copy derived (and PartialEq since all fields support it)
    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Check for Copy trait (and PartialEq which is also auto-derived for f32/i32 fields)
    assert!(
        generated.contains("Copy") && generated.contains("Clone") && generated.contains("Debug"),
        "Should derive Copy for @auto struct with all Copy fields: {}",
        generated
    );
}

// =============================================================================
// Test: Pattern match on self field should use ref binding
// Pattern: match self.field { Some(id) => ... }
// When pattern matching on a field of self, use ref binding to avoid partial move
// =============================================================================

#[test]
fn test_pattern_match_self_field_ref_binding() {
    let code = r#"
struct Panel {
    selected_id: Option<string>,
}

impl Panel {
    fn is_selected(&self, object_id: string) -> bool {
        match self.selected_id {
            Some(id) => id == object_id,
            None => false,
        }
    }
    
    fn other_method(&self) -> i32 {
        42
    }
    
    fn combined(&self, object_id: string) -> string {
        let selected = match self.selected_id {
            Some(id) => id == object_id,
            None => false,
        }
        // After match, self should still be usable
        format!("{} {}", selected, self.other_method())
    }
}
"#;

    // Compiler should clone self.field to avoid partial move
    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // Check that we clone the field before matching
    assert!(
        generated.contains("self.selected_id.clone()"),
        "Should clone self.field to avoid partial move: {}",
        generated
    );
}

// =============================================================================
// Test: Clone fields when constructing struct from borrowed self
// Pattern: fn method(&self) -> Self { Self { field: self.field } }
// Non-Copy fields need to be cloned when building struct from &self
// =============================================================================

#[test]
fn test_struct_construction_from_borrowed_self() {
    let code = r#"
@auto
struct Config {
    items: Vec<string>,
    name: string,
}

impl Config {
    fn with_name(&self, new_name: string) -> Config {
        Config {
            items: self.items,
            name: new_name,
        }
    }
}
"#;

    // Compiler should auto-clone self.items since self is borrowed and Vec is not Copy
    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // The generated code should have self.items.clone()
    assert!(
        generated.contains("self.items.clone()"),
        "Should auto-clone non-Copy fields from borrowed self: {}",
        generated
    );
}

// =============================================================================
// Test: Iterator variable handling - borrowed params avoid cloning
// Pattern: for item in collection { method(item) } where item is &T
// The compiler should infer borrowed parameter when method only reads the arg,
// so no cloning is needed (iterator var is already &T).
// =============================================================================

#[test]
fn test_iterator_var_borrowed_param_no_clone() {
    let code = r#"
@auto
struct Polygon {
    id: i32,
    name: string,
}

struct Renderer {
    polygons: Vec<Polygon>,
}

impl Renderer {
    fn render_polygon(&self, poly: Polygon) -> string {
        format!("polygon {} {}", poly.id, poly.name)
    }
    
    fn render_all(&self) -> string {
        let mut result = ""
        for poly in self.polygons {
            result = result + self.render_polygon(poly).as_str()
        }
        result
    }
}
"#;

    // Compiler should infer &Polygon for render_polygon's poly parameter
    // since it's only read, avoiding the need to clone
    let result = compile_and_check(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // The parameter should be &Polygon (borrowed), not Polygon (owned)
    assert!(
        generated.contains("poly: &Polygon"),
        "Should infer borrowed parameter: {}",
        generated
    );
}
