// TDD Test: Cross-module struct field literal typing
//
// This test captures the REAL-WORLD failure pattern:
// - Struct defined in module A with u32/usize fields
// - Usage in module B imports via `use super::*`
// - Literals in struct initializers should match field types
//
// Expected behavior:
//   DialogueChoice { id: 1, ... } should generate `1_u32` (NOT `1_i32`)
//
// Current behavior:
//   TDD tests with same-file structs work ✅
//   Real-world cross-module imports FAIL ❌
//
// This test SHOULD FAIL right now - that's the bug we're investigating!

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_cross_module_struct_u32_literal_via_types_module() {
    // Simulate real-world pattern: types/entity.wj + usage.wj
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&src.join("types")).unwrap();

    // types/mod.wj - Module declaration
    std::fs::write(
        src.join("types").join("mod.wj"),
        r#"
pub mod entity

pub use entity::Entity
"#,
    )
    .unwrap();

    // types/entity.wj - Struct with u32 fields
    std::fs::write(
        src.join("types").join("entity.wj"),
        r#"
pub struct Entity {
    pub id: u32,
    pub health: u32,
    pub level: u32
}

impl Entity {
    pub fn new(id: u32, health: u32, level: u32) -> Entity {
        Entity {
            id: id,
            health: health,
            level: level
        }
    }
}
"#,
    )
    .unwrap();

    // usage.wj - Imports and uses Entity
    std::fs::write(
        src.join("usage.wj"),
        r#"
use types::Entity

pub fn create_entities() -> Vec<Entity> {
    vec![
        Entity { id: 1, health: 100, level: 5 },
        Entity { id: 2, health: 150, level: 10 },
        Entity { id: 3, health: 200, level: 15 }
    ]
}

pub fn create_via_constructor() -> Entity {
    Entity::new(42, 300, 20)
}
"#,
    )
    .unwrap();

    // Build as library (multi-file mode)
    build_project_ext(
        &src,
        &build,
        CompilationTarget::Rust,
        false,
        true, // library mode
        &[],
    )
    .expect("Build should succeed");

    let usage_code = std::fs::read_to_string(build.join("usage.rs")).unwrap();

    // CRITICAL: These should be u32, NOT i32!
    // This is the bug: cross-module struct fields don't type literals correctly

    // Check struct literal initializers
    assert!(
        usage_code.contains("id: 1_u32"),
        "Expected 'id: 1_u32' but generated code has i32. Generated:\n{}",
        usage_code
    );
    assert!(
        usage_code.contains("health: 100_u32"),
        "Expected 'health: 100_u32' but generated code has i32. Generated:\n{}",
        usage_code
    );
    assert!(
        usage_code.contains("level: 5_u32"),
        "Expected 'level: 5_u32' but generated code has i32. Generated:\n{}",
        usage_code
    );

    // Check constructor calls (function parameters)
    assert!(
        usage_code.contains("Entity::new(42_u32, 300_u32, 20_u32)"),
        "Expected constructor args to be u32. Generated:\n{}",
        usage_code
    );

    // Verify NO i32 suffixes appear (common bug)
    assert!(
        !usage_code.contains("1_i32")
            && !usage_code.contains("100_i32")
            && !usage_code.contains("5_i32"),
        "Found i32 suffixes where u32 expected! Generated:\n{}",
        usage_code
    );
}

#[test]
fn test_cross_module_struct_usize_literal() {
    // Test usize fields (len(), capacity, etc.)
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // container.wj - Struct with usize fields
    std::fs::write(
        src.join("container.wj"),
        r#"
pub struct Container {
    pub capacity: usize,
    pub count: usize
}

impl Container {
    pub fn new(capacity: usize) -> Container {
        Container {
            capacity: capacity,
            count: 0
        }
    }
}
"#,
    )
    .unwrap();

    // usage.wj - Uses Container
    std::fs::write(
        src.join("usage.wj"),
        r#"
use crate::container::Container

pub fn create_containers() -> Vec<Container> {
    vec![
        Container { capacity: 10, count: 0 },
        Container { capacity: 100, count: 50 },
        Container { capacity: 1000, count: 999 }
    ]
}

pub fn create_default() -> Container {
    Container::new(256)
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Build should succeed");

    let usage_code = std::fs::read_to_string(build.join("usage.rs")).unwrap();

    // CRITICAL: These should be usize, NOT i32!
    assert!(
        usage_code.contains("capacity: 10_usize"),
        "Expected 'capacity: 10_usize'. Generated:\n{}",
        usage_code
    );
    assert!(
        usage_code.contains("capacity: 100_usize"),
        "Expected 'capacity: 100_usize'. Generated:\n{}",
        usage_code
    );
    assert!(
        usage_code.contains("count: 0_usize"),
        "Expected 'count: 0_usize'. Generated:\n{}",
        usage_code
    );
    assert!(
        usage_code.contains("Container::new(256_usize)"),
        "Expected constructor arg to be usize. Generated:\n{}",
        usage_code
    );

    // Verify NO i32 suffixes
    assert!(
        !usage_code.contains("10_i32")
            && !usage_code.contains("0_i32")
            && !usage_code.contains("256_i32"),
        "Found i32 suffixes where usize expected! Generated:\n{}",
        usage_code
    );
}

#[test]
fn test_cross_module_via_glob_import() {
    // Most common pattern in windjammer-game: `use super::*`
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src.join("parent")).unwrap();

    // parent/mod.wj - Re-exports types
    std::fs::write(
        src.join("parent").join("mod.wj"),
        r#"
pub mod types
pub mod child

pub use types::*
"#,
    )
    .unwrap();

    // parent/types.wj - Struct definition
    std::fs::write(
        src.join("parent").join("types.wj"),
        r#"
pub struct Item {
    pub id: u32,
    pub count: usize
}
"#,
    )
    .unwrap();

    // parent/child.wj - Uses glob import
    std::fs::write(
        src.join("parent").join("child.wj"),
        r#"
use super::*

pub fn create_items() -> Vec<Item> {
    vec![
        Item { id: 1, count: 10 },
        Item { id: 2, count: 20 },
        Item { id: 3, count: 30 }
    ]
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Build should succeed");

    let child_code = std::fs::read_to_string(build.join("parent/child.rs")).unwrap();

    // Via `use super::*`, Item should still type literals correctly
    assert!(
        child_code.contains("id: 1_u32"),
        "Expected 'id: 1_u32' via glob import. Generated:\n{}",
        child_code
    );
    assert!(
        child_code.contains("count: 10_usize"),
        "Expected 'count: 10_usize' via glob import. Generated:\n{}",
        child_code
    );
    assert!(
        child_code.contains("id: 2_u32"),
        "Expected 'id: 2_u32' via glob import. Generated:\n{}",
        child_code
    );
    assert!(
        child_code.contains("count: 20_usize"),
        "Expected 'count: 20_usize' via glob import. Generated:\n{}",
        child_code
    );

    // No i32
    assert!(
        !child_code.contains("1_i32") && !child_code.contains("10_i32"),
        "Found i32 suffixes with glob import! Generated:\n{}",
        child_code
    );
}
