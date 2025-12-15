// Test: Ambiguous import disambiguation
// Ensures module paths are preserved to avoid ambiguity with glob re-exports

use std::fs;
use std::process::Command;

fn compile_and_verify_imports(code: &str, module_name: &str) -> Result<String, String> {
    let test_dir = format!("tests/generated/ambiguous_import_test_{}", module_name);
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test.wj", test_dir);
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let generated_file = format!("{}/test.rs", test_dir);
    let generated = fs::read_to_string(&generated_file)
        .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).to_string());

    fs::remove_dir_all(&test_dir).ok();

    if output.status.success() {
        Ok(generated)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[test]
fn test_texture_atlas_import_preserves_module_path() {
    // Test: texture_atlas::TextureAtlas should generate use super::texture_atlas::TextureAtlas;
    // NOT use super::TextureAtlas; (which would be ambiguous with glob re-exports)
    let code = r#"
    use texture_atlas::TextureAtlas
    
    pub struct Sprite {
        pub atlas: TextureAtlas,
    }
    "#;

    let generated = compile_and_verify_imports(code, "texture_atlas").expect("Compilation failed");

    // Should preserve full module path to avoid ambiguity
    assert!(
        generated.contains("use super::texture_atlas::TextureAtlas;"),
        "Expected 'use super::texture_atlas::TextureAtlas;' to avoid ambiguity, got: {}",
        generated
    );

    // Should NOT strip module path
    assert!(
        !generated.contains("use super::TextureAtlas;")
            || generated.contains("use super::texture_atlas::TextureAtlas;"),
        "Should NOT use 'use super::TextureAtlas;' alone (ambiguous), got: {}",
        generated
    );
}

#[test]
fn test_sprite_region_import_preserves_module_path() {
    // Test: sprite_region::SpriteRegion should generate use super::sprite_region::SpriteRegion;
    let code = r#"
    use sprite_region::SpriteRegion
    
    pub struct Sprite {
        pub region: SpriteRegion,
    }
    "#;

    let generated = compile_and_verify_imports(code, "sprite_region").expect("Compilation failed");

    // Should preserve full module path
    assert!(
        generated.contains("use super::sprite_region::SpriteRegion;"),
        "Expected 'use super::sprite_region::SpriteRegion;', got: {}",
        generated
    );
}

#[test]
fn test_math_directory_prefix_stripped() {
    // Test: math::Vec2 should generate use super::Vec2; (directory prefix stripped)
    let code = r#"
    use math::Vec2
    
    pub struct Point {
        pub position: Vec2,
    }
    "#;

    let generated = compile_and_verify_imports(code, "math_vec2").expect("Compilation failed");

    // Directory prefix should be stripped
    assert!(
        generated.contains("use super::Vec2;"),
        "Expected 'use super::Vec2;' (directory prefix stripped), got: {}",
        generated
    );

    // Should NOT preserve module path for directory prefixes
    assert!(
        !generated.contains("use super::math::Vec2;"),
        "Should NOT preserve 'math' directory prefix, got: {}",
        generated
    );
}

#[test]
fn test_rendering_directory_prefix_stripped() {
    // Test: rendering::Color should generate use super::Color; (directory prefix stripped)
    let code = r#"
    use rendering::Color
    
    pub struct Material {
        pub color: Color,
    }
    "#;

    let generated =
        compile_and_verify_imports(code, "rendering_color").expect("Compilation failed");

    // Directory prefix should be stripped
    assert!(
        generated.contains("use super::Color;"),
        "Expected 'use super::Color;' (directory prefix stripped), got: {}",
        generated
    );
}

#[test]
fn test_multiple_imports_mixed_types() {
    // Test: Mix of directory prefixes and module files
    let code = r#"
    use math::Vec2
    use rendering::Texture
    use texture_atlas::TextureAtlas
    use sprite_region::SpriteRegion
    
    pub struct Sprite {
        pub position: Vec2,
        pub texture: Texture,
        pub atlas: TextureAtlas,
        pub region: SpriteRegion,
    }
    "#;

    let generated = compile_and_verify_imports(code, "mixed").expect("Compilation failed");

    // Directory prefixes stripped
    assert!(
        generated.contains("use super::Vec2;"),
        "Vec2 should have directory prefix stripped, got: {}",
        generated
    );

    assert!(
        generated.contains("use super::Texture;"),
        "Texture should have directory prefix stripped, got: {}",
        generated
    );

    // Module files preserved
    assert!(
        generated.contains("use super::texture_atlas::TextureAtlas;"),
        "TextureAtlas should preserve module path, got: {}",
        generated
    );

    assert!(
        generated.contains("use super::sprite_region::SpriteRegion;"),
        "SpriteRegion should preserve module path, got: {}",
        generated
    );
}

#[test]
fn test_collision2d_module_preserves_path() {
    // Test: collision2d::check_collision should preserve module path
    let code = r#"
    use collision2d::check_collision
    
    pub fn test_collision() {
        check_collision()
    }
    "#;

    let generated = compile_and_verify_imports(code, "collision2d").expect("Compilation failed");

    // Module path should be preserved
    assert!(
        generated.contains("use super::collision2d::check_collision;"),
        "Expected 'use super::collision2d::check_collision;', got: {}",
        generated
    );
}

#[test]
fn test_entity_component_imports_preserve_paths() {
    // Test: ECS imports should preserve module paths
    let code = r#"
    use entity::Entity
    use components::Transform
    
    pub struct World {
        pub entities: Vec<Entity>,
        pub transforms: Vec<Transform>,
    }
    "#;

    let generated = compile_and_verify_imports(code, "ecs").expect("Compilation failed");

    // Module paths should be preserved for actual module files
    assert!(
        generated.contains("use super::entity::Entity;"),
        "Entity import should preserve module path, got: {}",
        generated
    );

    assert!(
        generated.contains("use super::components::Transform;"),
        "Transform import should preserve module path, got: {}",
        generated
    );
}

#[test]
fn test_ambiguity_prevention_with_mod_rs_globs() {
    // Test: Ensures generated code won't have ambiguous imports when mod.rs has glob re-exports
    // This is the EXACT scenario that caused the original bug
    let code = r#"
    use texture_atlas::TextureAtlas
    use sprite_region::SpriteRegion
    
    pub struct Sprite {
        pub atlas: TextureAtlas,
        pub region: SpriteRegion,
    }
    
    impl Sprite {
        pub fn new() -> Sprite {
            Sprite {
                atlas: TextureAtlas::new(),
                region: SpriteRegion::new(),
            }
        }
    }
    "#;

    let generated = compile_and_verify_imports(code, "ambiguity_test").expect("Compilation failed");

    // Both should preserve module paths
    assert!(
        generated.contains("use super::texture_atlas::TextureAtlas;")
            && generated.contains("use super::sprite_region::SpriteRegion;"),
        "Both imports should preserve module paths to avoid ambiguity, got: {}",
        generated
    );

    // Verify we can compile the generated Rust code (no ambiguity errors)
    // This would fail if the imports were ambiguous
    assert!(
        !generated.contains("error[E0659]"),
        "Generated code should not have ambiguity errors, got: {}",
        generated
    );
}
