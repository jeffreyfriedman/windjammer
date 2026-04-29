/// TDD Test: .clone() on borrowed strings should generate .to_string() when needed
///
/// Bug: When a string parameter is inferred as &str and .clone() is called on it,
/// and the result is passed to a function expecting String, the codegen generates
/// .clone() which returns &str, not String, causing E0308 type mismatch.
///
/// Fix: Detect when .clone() result needs to be String and generate .to_string() instead.

use std::fs;
use tempfile::tempdir;
use windjammer::{build_project_ext, CompilationTarget};

fn compile_single_file(source: &str) -> String {
    let src = tempdir().expect("tempdir for src");
    let out = tempdir().expect("tempdir for out");

    fs::write(src.path().join("test.wj"), source).expect("write test.wj");

    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    fs::read_to_string(out.path().join("test.rs")).unwrap_or_default()
}

#[test]
fn test_string_clone_generates_to_string() {
    let source = r#"
struct DialogTree {
    id: string,
}

impl DialogTree {
    pub fn new(id: string) -> DialogTree {
        DialogTree { id }
    }
}

pub fn create_dialog(id: string) -> DialogTree {
    DialogTree::new(id.clone())
}
"#;

    let generated = compile_single_file(source);
    println!("Generated Rust:\n{}", generated);

    if generated.contains("id: &str") {
        assert!(
            generated.contains(".to_string()") || generated.contains(".to_owned()"),
            "Should convert &str to String with .to_string() or .to_owned(), not .clone()"
        );
    }
}

#[test]
fn test_owned_string_can_use_clone() {
    let source = r#"
struct DialogTree {
    id: string,
}

impl DialogTree {
    pub fn new(id: string) -> DialogTree {
        DialogTree { id }
    }
}

pub fn create_dialog(id: string, suffix: string) -> DialogTree {
    let full_id = format!("{}_{}", id, suffix)
    DialogTree::new(full_id.clone())
}
"#;

    let generated = compile_single_file(source);
    println!("Generated Rust:\n{}", generated);

    assert!(
        !generated.is_empty(),
        "Should generate valid Rust code"
    );
}
