/// TDD Test: Recursive trait object detection in trait derivation.
///
/// Verifies that the fixpoint loop in `collect_trait_object_types` correctly
/// propagates trait object containment through multiple levels of struct nesting.
///
/// Chain: StructA { field: trait MyTrait } → StructB { inner: StructA } → StructC { nested: StructB }
///
/// Expected: None of StructA, StructB, StructC should get #[derive(Debug, Clone)]
/// because they all transitively contain a trait object.
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_direct_trait_object_field_no_derive() {
    let source = r#"
trait Renderable {
    fn draw(self)
}

struct Widget {
    renderer: trait Renderable
}
"#;

    let generated = test_utils::compile_single(source);

    let widget_derive = extract_derive_for_struct(&generated, "Widget");
    assert!(
        !widget_derive.contains("Debug"),
        "Widget should NOT have #[derive(Debug, Clone)] because it has a trait object field.\n\
         Generated:\n{}",
        generated
    );

    let widget_section = extract_derive_for_struct(&generated, "Widget");
    assert!(
        !widget_section.contains("Debug"),
        "Widget should NOT derive Debug (has trait object field).\nWidget section:\n{}",
        widget_section
    );
}

#[test]
fn test_one_level_nested_trait_object_no_derive() {
    let source = r#"
trait EventHandler {
    fn handle(self)
}

struct Inner {
    handler: trait EventHandler
}

struct Outer {
    inner: Inner
}
"#;

    let generated = test_utils::compile_single(source);

    let inner_section = extract_derive_for_struct(&generated, "Inner");
    assert!(
        !inner_section.contains("Debug"),
        "Inner should NOT derive Debug (has trait object field).\nSection:\n{}",
        inner_section
    );

    let outer_section = extract_derive_for_struct(&generated, "Outer");
    assert!(
        !outer_section.contains("Debug"),
        "Outer should NOT derive Debug (transitively contains trait object via Inner).\nSection:\n{}",
        outer_section
    );
}

#[test]
fn test_two_level_nested_trait_object_no_derive() {
    let source = r#"
trait System {
    fn update(self, dt: f32)
}

struct SystemHolder {
    system: trait System
}

struct Manager {
    holder: SystemHolder
}

struct App {
    manager: Manager,
    name: String
}
"#;

    let generated = test_utils::compile_single(source);

    let holder_section = extract_derive_for_struct(&generated, "SystemHolder");
    assert!(
        !holder_section.contains("Debug"),
        "SystemHolder should NOT derive Debug.\nSection:\n{}",
        holder_section
    );

    let manager_section = extract_derive_for_struct(&generated, "Manager");
    assert!(
        !manager_section.contains("Debug"),
        "Manager should NOT derive Debug (transitively contains trait object).\nSection:\n{}",
        manager_section
    );

    let app_section = extract_derive_for_struct(&generated, "App");
    assert!(
        !app_section.contains("Debug"),
        "App should NOT derive Debug (2-level transitive trait object).\nSection:\n{}",
        app_section
    );
}

#[test]
fn test_vec_of_trait_object_struct_no_derive() {
    let source = r#"
trait Plugin {
    fn init(self)
}

struct PluginEntry {
    plugin: trait Plugin
}

struct PluginRegistry {
    plugins: Vec<PluginEntry>
}
"#;

    let generated = test_utils::compile_single(source);

    let registry_section = extract_derive_for_struct(&generated, "PluginRegistry");
    assert!(
        !registry_section.contains("Debug"),
        "PluginRegistry should NOT derive Debug (Vec<PluginEntry> where PluginEntry has trait object).\nSection:\n{}",
        registry_section
    );
}

#[test]
fn test_normal_struct_still_gets_derive() {
    let source = r#"
struct Point {
    x: f32,
    y: f32
}

struct Line {
    start: Point,
    end: Point
}
"#;

    let generated = test_utils::compile_single(source);

    let point_section = extract_derive_for_struct(&generated, "Point");
    assert!(
        point_section.contains("Debug") && point_section.contains("Clone"),
        "Point should derive Debug, Clone (no trait objects).\nSection:\n{}",
        point_section
    );

    let line_section = extract_derive_for_struct(&generated, "Line");
    assert!(
        line_section.contains("Debug") && line_section.contains("Clone"),
        "Line should derive Debug, Clone (all fields are normal structs).\nSection:\n{}",
        line_section
    );
}

fn extract_derive_for_struct(generated: &str, struct_name: &str) -> String {
    let lines: Vec<&str> = generated.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(&format!("struct {}", struct_name)) {
            let start = if i > 0 && lines[i - 1].starts_with("#[derive") {
                i - 1
            } else if i > 1 && lines[i - 2].starts_with("#[derive") {
                i - 2
            } else {
                i
            };
            let end = (i + 3).min(lines.len());
            return lines[start..end].join("\n");
        }
    }
    format!("(struct {} not found in output)", struct_name)
}
