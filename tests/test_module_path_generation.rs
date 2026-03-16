// TDD: Test module path generation in hierarchical structures
//
// Bug: Generated Rust code uses `use self::windjammer_game_core::...` 
// but `windjammer_game_core` is a crate-level alias, not in `self`
//
// Fix: Use `crate::windjammer_game_core::...` or just `windjammer_game_core::...`

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_external_crate_import_in_submodule() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("test")).unwrap();
    
    // lib.rs would have: pub use windjammer_app as windjammer_game_core;
    
    // Submodule file that imports from external crate
    std::fs::write(
        src.join("test/mod.wj"),
        r#"
use windjammer_game_core::math::vec3::Vec3

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
    
    let rust_code = std::fs::read_to_string(build.join("test/mod.rs")).unwrap();
    
    // ASSERT: Should NOT use `self::windjammer_game_core`
    assert!(
        !rust_code.contains("use self::windjammer_game_core"),
        "Should not use 'use self::windjammer_game_core' (windjammer_game_core is crate-level). Found in:\n{}",
        rust_code.lines().take(20).collect::<Vec<_>>().join("\n")
    );
    
    // ASSERT: Should use either `crate::` or direct path
    assert!(
        rust_code.contains("use crate::windjammer_game_core")
            || rust_code.contains("use windjammer_game_core")
            || rust_code.contains("use super::windjammer_game_core"),
        "Should use 'use crate::windjammer_game_core' or direct path. Found:\n{}",
        rust_code.lines().take(20).collect::<Vec<_>>().join("\n")
    );
}
