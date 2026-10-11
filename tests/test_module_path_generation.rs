#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// TDD: Test module path generation in hierarchical structures
//
// Bug: Generated Rust code uses `use self::a game engine project::...`
// but a game engine project is a crate-level alias, not in `self`
//
// Fix: Use `crate::a game engine project::...` or just `a game engine project::...`

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_external_crate_import_in_submodule() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("test")).unwrap();

    // lib.rs would have: pub use windjammer_app as a game engine project;

    // Submodule file that imports from external crate
    std::fs::write(
        src.join("test/mod.wj"),
        r#"
use game_core::math::vec3::Vec3

pub fn test_create_vector() {
    let v = Vec3::new(1.0, 2.0, 3.0)
    assert_eq(v.x, 1.0)
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod test
"#,
    )
    .unwrap();

    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true, // library mode
        &[],
    )
    .expect("Build should succeed");

    // mod.wj generates _mod_items.rs, not mod.rs
    let rust_code = std::fs::read_to_string(build.join("test/_mod_items.rs"))
        .expect("Failed to read generated test/_mod_items.rs");

    // ASSERT: Should NOT use `self::a game engine project`
    assert!(
        !rust_code.contains("use self::game_core"),
        "Should not use 'use self::game_core' (game_core is crate-level). Found in:\n{}",
        rust_code.lines().take(20).collect::<Vec<_>>().join("\n")
    );

    // ASSERT: Should use either `crate::` or direct path
    assert!(
        rust_code.contains("use crate::game_core")
            || rust_code.contains("use game_core")
            || rust_code.contains("use super::game_core"),
        "Should use 'use crate::game_core' or direct path. Found:\n{}",
        rust_code.lines().take(20).collect::<Vec<_>>().join("\n")
    );
}
