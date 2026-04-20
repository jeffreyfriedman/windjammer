//! TDD: Duplicate struct names in different modules must not overwrite the struct field registry.

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_duplicate_struct_names_different_modules() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src.join("dialogue")).unwrap();

    std::fs::write(
        src.join("dialogue").join("legacy.wj"),
        r#"
pub struct DialogueChoice {
    pub text: String,
    pub next_node: String
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("dialogue").join("modern.wj"),
        r#"
pub struct DialogueChoice {
    pub id: u32,
    pub text: String,
    pub next_line: u32
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("dialogue").join("examples.wj"),
        r#"
use super::modern::DialogueChoice

pub fn create() -> DialogueChoice {
    DialogueChoice {
        id: 1,
        text: "Hello",
        next_line: 2
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
        "Expected 'id: 1_u32' (qualified lookup should find modern::DialogueChoice). Generated:\n{}",
        examples_code
    );
    assert!(
        examples_code.contains("next_line: 2_u32"),
        "Expected 'next_line: 2_u32'. Generated:\n{}",
        examples_code
    );
}

#[test]
fn test_qualified_struct_literal_typing() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src.join("types")).unwrap();

    std::fs::write(
        src.join("types").join("entity.wj"),
        r#"
pub struct Entity {
    pub id: u32,
    pub health: u32
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("main.wj"),
        r#"
use types::entity::Entity

pub fn create() -> Entity {
    types::entity::Entity {
        id: 42,
        health: 100
    }
}
"#,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("Build should succeed");

    let main_code = std::fs::read_to_string(build.join("main.rs")).unwrap();

    assert!(
        main_code.contains("id: 42_u32") && main_code.contains("health: 100_u32"),
        "Qualified struct literal should type correctly. Generated:\n{}",
        main_code
    );
}
