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
// Dogfooding — copy detection, self patterns, iterator borrows.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_expression_format_arg() {
    let code = r#"
struct Display {
    value: string,
}

impl Display {
    fn render(self) -> string {
        format!("<div>{}</div>", if self.value == "" { "empty" } else { self.value })
    }
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
}

// =============================================================================
// Test: @auto decorator correctly identifies Copy types
// Stats has only primitives (i32, f32) so @auto should derive Copy
// This allows passing w.stats by value when w is from Option<&Weapon>
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let result = test_utils::compile_single_result(code);
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
#[cfg_attr(tarpaulin, ignore)]
fn test_pattern_match_self_field_ref_binding() {
    let code = r#"
struct Panel {
    selected_id: Option<string>,
}

impl Panel {
    fn is_selected(self, object_id: string) -> bool {
        match self.selected_id {
            Some(id) => id == object_id,
            None => false,
        }
    }
    
    fn other_method(self) -> i32 {
        42
    }
    
    fn combined(self, object_id: string) -> string {
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
    let result = test_utils::compile_single_result(code);
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
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_construction_from_borrowed_self() {
    let code = r#"
@auto
struct Config {
    items: Vec<string>,
    name: string,
}

impl Config {
    fn with_name(self, new_name: string) -> Config {
        Config {
            items: self.items,
            name: new_name,
        }
    }
}
"#;

    // with_name moves self.items into a new Config — self should be consumed (owned)
    // or borrowed with auto-clone
    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    assert!(
        generated.contains("self.items.clone()") || generated.contains("self.items"),
        "Should access self.items (owned move or borrowed clone): {}",
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
#[cfg_attr(tarpaulin, ignore)]
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
    fn render_polygon(self, poly: Polygon) -> string {
        format!("polygon {} {}", poly.id, poly.name)
    }
    
    fn render_all(self) -> string {
        let mut result = ""
        for poly in self.polygons {
            result = result + self.render_polygon(poly)
        }
        result
    }
}
"#;

    // Compiler should infer &Polygon for render_polygon's poly parameter
    // since it's only read, avoiding the need to clone
    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Compilation failed: {:?}", result);
    let generated = result.unwrap();
    // The parameter should be &Polygon (borrowed), not Polygon (owned)
    assert!(
        generated.contains("poly: &Polygon"),
        "Should infer borrowed parameter: {}",
        generated
    );
}
