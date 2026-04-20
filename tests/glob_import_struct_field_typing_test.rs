//! TDD: Glob imports (`use super::*`, `use crate::...::*`) must register struct field registry keys.

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_use_super_star_resolves_struct_fields() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src.join("dialogue")).unwrap();

    std::fs::write(
        src.join("dialogue").join("mod.wj"),
        r#"
pub mod system
pub mod examples

pub use system::DialogueChoice
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("dialogue").join("system.wj"),
        r#"
pub struct DialogueChoice {
    pub id: u32,
    pub text: String
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("dialogue").join("examples.wj"),
        r#"
use super::*

pub fn create() -> DialogueChoice {
    DialogueChoice {
        id: 1,
        text: "Hello".to_string()
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Build should succeed");

    let examples_code = std::fs::read_to_string(build.join("dialogue/examples.rs")).unwrap();

    assert!(
        examples_code.contains("id: 1_u32"),
        "Expected 'id: 1_u32' via glob import. Generated:\n{}",
        examples_code
    );
}

#[test]
fn test_use_crate_star_resolves_types() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src.join("types")).unwrap();

    std::fs::write(
        src.join("types").join("entity.wj"),
        r#"
pub struct Entity {
    pub id: u32
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("usage.wj"),
        r#"
use crate::types::entity::*

pub fn create() -> Entity {
    Entity { id: 42 }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Build should succeed");

    let usage_code = std::fs::read_to_string(build.join("usage.rs")).unwrap();

    assert!(
        usage_code.contains("id: 42_u32"),
        "Expected 'id: 42_u32' via crate glob. Generated:\n{}",
        usage_code
    );
}

/// Regression: `pub use system::T` in `parent/mod.wj` must resolve relative to `parent`, not crate-root `system::T`.
#[test]
fn test_pub_use_child_module_path_resolves_under_parent_module() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src.join("dialogue")).unwrap();

    std::fs::write(
        src.join("dialogue").join("mod.wj"),
        r#"
pub mod system
pub mod examples

pub use system::DialogueChoice
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("dialogue").join("system.wj"),
        r#"
pub struct DialogueChoice {
    pub id: u32,
    pub text: String
}

pub struct OtherDup {
    pub id: u32
}
"#,
    )
    .unwrap();

    // Second module with same struct name (disambiguation relies on re-export, not wrong `system::` key)
    std::fs::write(
        src.join("dialogue").join("legacy.wj"),
        r#"
pub struct DialogueChoice {
    pub text: String
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("dialogue").join("examples.wj"),
        r#"
use super::*

pub fn create() -> DialogueChoice {
    DialogueChoice {
        id: 7,
        text: "Hi".to_string()
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Build should succeed");

    let examples_code = std::fs::read_to_string(build.join("dialogue/examples.rs")).unwrap();
    assert!(
        examples_code.contains("id: 7_u32"),
        "Expected modern DialogueChoice (u32 id) via pub use + glob under dialogue::. Got:\n{}",
        examples_code
    );
}
