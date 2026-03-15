//! TDD: lib.rs auto-generation from actual module exports
//!
//! Problem: lib.rs has manual pub use statements that don't match module contents.
//! Solution: Scan modules, extract actual exports, generate only valid re-exports.

use std::fs;
use std::path::Path;

fn create_test_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().unwrap();
    for (path, content) in files {
        let full_path = temp_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
    }
    temp_dir
}

#[test]
fn test_extract_exports_from_rs_file() {
    // Module exports: pub struct X, pub enum X, pub fn x, pub type X
    let exports = windjammer::lib_rs_generator::extract_pub_items_from_rust(
        r#"
        pub struct Vec2 { x: f32, y: f32 }
        pub struct Vec3 { x: f32, y: f32, z: f32 }
        pub enum Color { Red, Green, Blue }
        pub fn create_default() -> Vec2 { todo!() }
        pub type Result = std::result::Result<(), ()>;
        fn private_fn() {}
        "#,
    );
    assert!(exports.contains("Vec2"), "Should extract pub struct: {:?}", exports);
    assert!(exports.contains("Vec3"), "Should extract pub struct: {:?}", exports);
    assert!(exports.contains("Color"), "Should extract pub enum: {:?}", exports);
    assert!(exports.contains("create_default"), "Should extract pub fn: {:?}", exports);
    assert!(exports.contains("Result"), "Should extract pub type: {:?}", exports);
    assert!(!exports.contains("private_fn"), "Should NOT extract private: {:?}", exports);
}

#[test]
fn test_extract_pub_use_from_mod_rs() {
    // mod.rs can have: pub use self::x::Item, pub use crate::x::Item
    let exports = windjammer::lib_rs_generator::extract_pub_use_items_from_mod_rs(
        r#"
        pub use self::vec2::Vec2;
        pub use self::vec3::Vec3;
        pub use crate::math::Mat4;
        pub mod vec2;
        pub mod vec3;
        "#,
    );
    assert!(exports.contains(&"Vec2".to_string()), "Should extract: {:?}", exports);
    assert!(exports.contains(&"Vec3".to_string()), "Should extract: {:?}", exports);
    assert!(exports.contains(&"Mat4".to_string()), "Should extract: {:?}", exports);
}

#[test]
fn test_module_exports_with_submodules() {
    // Module with submodule: game_loop has game_loop.rs (GameLoop, GameLoopConfig)
    let temp_dir = create_test_dir(&[
        ("game_loop/mod.rs", "pub mod game_loop;"),
        (
            "game_loop/game_loop.rs",
            "pub struct GameLoopConfig {} pub trait GameLoop {}",
        ),
    ]);

    let exports = windjammer::lib_rs_generator::get_module_exports(temp_dir.path().join("game_loop"));
    assert!(exports.contains("GameLoopConfig"), "Should get from submodule: {:?}", exports);
    assert!(exports.contains("GameLoop"), "Should get trait: {:?}", exports);
}

#[test]
fn test_generate_lib_rs_only_valid_exports() {
    // lib.rs has pub use ai::SoundSource but ai doesn't export SoundSource
    // Should generate lib.rs with ONLY valid re-exports
    let temp_dir = create_test_dir(&[
        ("lib.rs", "pub mod ai; pub mod game_loop; pub use ai::SoundSource; pub use game_loop::GameLoop;"),
        ("ai/mod.rs", "pub use self::tilemap::Tilemap; pub mod tilemap;"),
        ("ai/tilemap.rs", "pub struct Tilemap {} pub struct Tile {}"),
        ("game_loop/mod.rs", "pub mod game_loop;"),
        ("game_loop/game_loop.rs", "pub struct GameLoopConfig {} pub trait GameLoop {}"),
    ]);

    let generated = windjammer::lib_rs_generator::regenerate_lib_rs(temp_dir.path()).unwrap();

    // Should have pub mod for each module
    assert!(generated.contains("pub mod ai;"));
    assert!(generated.contains("pub mod game_loop;"));

    // Should have valid re-exports only (submodule items use full path: game_loop::game_loop::X)
    assert!(generated.contains("pub use ai::Tilemap;"), "ai exports Tilemap: {}", generated);
    assert!(
        generated.contains("pub use game_loop::game_loop::GameLoop;") || generated.contains("pub use game_loop::GameLoop;"),
        "game_loop exports GameLoop: {}",
        generated
    );
    assert!(
        generated.contains("pub use game_loop::game_loop::GameLoopConfig;") || generated.contains("pub use game_loop::GameLoopConfig;"),
        "game_loop exports GameLoopConfig: {}",
        generated
    );

    // Should NOT have invalid re-exports
    assert!(
        !generated.contains("pub use ai::SoundSource;"),
        "ai does NOT export SoundSource - should be removed: {}",
        generated
    );
}

#[test]
fn test_generate_lib_rs_modules_without_exports() {
    // Module with no public items - should still have pub mod, no pub use
    let temp_dir = create_test_dir(&[
        ("lib.rs", "pub mod empty;"),
        ("empty/mod.rs", "fn internal() {}"),
    ]);

    let generated = windjammer::lib_rs_generator::regenerate_lib_rs(temp_dir.path()).unwrap();

    assert!(generated.contains("pub mod empty;"));
    assert!(!generated.contains("pub use empty::"), "Empty module should have no re-exports");
}

#[test]
fn test_generate_lib_rs_preserves_allow_attributes() {
    let temp_dir = create_test_dir(&[
        (
            "lib.rs",
            "#![allow(unused_imports)]\npub mod foo;",
        ),
        ("foo/mod.rs", "pub mod bar;"),
        ("foo/bar.rs", "pub struct Bar {}"),
    ]);

    let generated = windjammer::lib_rs_generator::regenerate_lib_rs(temp_dir.path()).unwrap();

    assert!(generated.contains("#![allow(unused_imports)"));
    assert!(generated.contains("pub mod foo;"));
    // Bar is in foo::bar, so full path is foo::bar::Bar
    assert!(generated.contains("pub use foo::bar::Bar;"));
}
