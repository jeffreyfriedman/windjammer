#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD: Struct with @derive(Debug, Clone) WITHOUT Copy must NOT get Copy derived.
///
/// Bug (E0204): The compiler auto-derives Copy on structs whose fields are all
/// Copy-looking types, even when those field types explicitly opted out of Copy
/// via `@derive(Debug, Clone)` (without Copy). This happens because:
///
/// 1. Local case: `collect_global_copy_structs_for_library` tracks `explicit_non_copy`
///    but this set is only used within the local fixpoint loop — it's not propagated
///    to the codegen's `non_copy_types_registry`.
///
/// 2. Cross-crate case: Dependency metadata infers Copy from field types alone
///    (`infer_copy_from_metadata_structs`), so a struct like `JoltWorld { handle: u64 }`
///    with `@derive(Debug, Clone)` gets falsely inferred as Copy.
///
/// Real-world trigger: `GamePhysics { world: JoltWorld }` where JoltWorld has a Drop
/// impl and explicitly derives only Debug+Clone. The compiler generates
/// `#[derive(Debug, Clone, Copy)]` on GamePhysics, causing Rust E0204.
///
/// Fix: Propagate explicit non-Copy structs into `non_copy_types_registry` so
/// `is_copy_type_with_registry` rejects them.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_derive_debug_clone_without_copy_blocks_copy_on_container() {
    let generated = test_utils::compile_single(
        r#"
@derive(Debug, Clone)
pub struct Handle {
    pub value: i64
}

pub struct Container {
    pub handle: Handle
    pub count: int
}
"#,
    );

    // Container should NOT have Copy because Handle is explicitly non-Copy
    let container_derive = find_struct_derive(&generated, "Container");
    assert!(
        !container_derive.contains("Copy"),
        "Container should NOT have Copy derived because Handle is non-Copy.\n\
         Container derive line: {}\nFull output:\n{}",
        container_derive,
        generated
    );

    // Container should also NOT have Copy because Handle is not Copy
    let container_section = find_struct_derive(&generated, "Container");
    assert!(
        !container_section.contains("Copy"),
        "Container should NOT have Copy derived because Handle is non-Copy.\n\
         Container derive line: {}\nFull output:\n{}",
        container_section,
        generated
    );
}

#[test]
fn test_explicit_derive_copy_still_works() {
    let generated = test_utils::compile_single(
        r#"
@derive(Debug, Clone, Copy)
pub struct Point {
    pub x: float
    pub y: float
}

pub struct Rect {
    pub origin: Point
    pub width: float
    pub height: float
}
"#,
    );

    // Point explicitly derives Copy
    let point_derive = find_struct_derive(&generated, "Point");
    assert!(
        point_derive.contains("Copy"),
        "Point should have Copy derived (explicit @derive).\nDerives: {}",
        point_derive
    );

    // Rect should also get Copy since all fields are Copy
    let rect_derive = find_struct_derive(&generated, "Rect");
    assert!(
        rect_derive.contains("Copy"),
        "Rect should have Copy derived since all fields are Copy.\nDerives: {}",
        rect_derive
    );
}

#[test]
fn test_no_derive_decorator_with_all_copy_fields_gets_copy() {
    let generated = test_utils::compile_single(
        r#"
pub struct SimplePoint {
    pub x: float
    pub y: float
}
"#,
    );

    // With no @derive decorator and all Copy fields, auto-derive should include Copy
    let derive = find_struct_derive(&generated, "SimplePoint");
    assert!(
        derive.contains("Copy"),
        "SimplePoint (no @derive, all Copy fields) should auto-derive Copy.\nDerives: {}",
        derive
    );
}

fn find_struct_derive(code: &str, struct_name: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(&format!("struct {}", struct_name)) {
            // Look backwards for #[derive(...)]
            if i > 0 {
                let prev = lines[i - 1].trim();
                if prev.starts_with("#[derive(") {
                    return prev.to_string();
                }
            }
            if i > 1 {
                let prev2 = lines[i - 2].trim();
                if prev2.starts_with("#[derive(") {
                    return prev2.to_string();
                }
            }
            return format!("(no derive found before {})", struct_name);
        }
    }
    format!("(struct {} not found)", struct_name)
}
