use anyhow::Result;
/// TDD Test: Parent Module Re-export Imports
///
/// PROBLEM: When in `rendering/sprite.wj` and using `use rendering::Texture`,
/// this should resolve to parent module's re-export, not `super::rendering::Texture`
///
/// Example structure:
/// ```
/// rendering/
///   mod.wj  -> pub use texture::Texture;
///   sprite.wj -> use rendering::Texture  (means parent's re-export!)
///   texture.wj -> struct Texture { ... }
/// ```
///
/// Currently generates:
/// ```rust
/// use super::rendering::Texture;  // ❌ Wrong! Goes up then down to rendering
/// ```
///
/// Should generate:
/// ```rust
/// use super::Texture;  // ✅ Correct! Uses parent's re-export
/// ```
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
fn test_parent_module_reexport_import() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_parent_reexport_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    // Create src_wj/rendering directory
    let rendering_dir = temp_dir.join("src_wj").join("rendering");
    fs::create_dir_all(&rendering_dir)?;

    // Create texture.wj (base module)
    let texture_wj = rendering_dir.join("texture.wj");
    fs::write(
        &texture_wj,
        r#"
struct Texture {
    id: u32,
}

impl Texture {
    fn new() -> Texture {
        Texture { id: 0 }
    }
}
"#,
    )?;

    // Create mod.wj that re-exports Texture
    let mod_wj = rendering_dir.join("mod.wj");
    fs::write(
        &mod_wj,
        r#"
pub mod texture;
pub mod sprite;

pub use texture::Texture;
"#,
    )?;

    // Create sprite.wj that imports from parent's re-export
    let sprite_wj = rendering_dir.join("sprite.wj");
    fs::write(
        &sprite_wj,
        r#"
use rendering::Texture

struct Sprite {
    tex: Texture,
}
"#,
    )?;

    // Build the library
    let wj_compiler = get_wj_compiler();
    let lib_output = temp_dir.join("lib");
    fs::create_dir_all(&lib_output)?;

    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(temp_dir.join("src_wj"))
        .arg("-o")
        .arg(&lib_output)
        .arg("--library")
        .output()?;

    let _stderr = String::from_utf8_lossy(&output.stderr);
    let _stdout = String::from_utf8_lossy(&output.stdout);

    // Check the generated sprite.rs
    let sprite_rs_path = lib_output.join("rendering").join("sprite.rs");
    assert!(sprite_rs_path.exists(), "sprite.rs should be generated");

    let sprite_rs_content = fs::read_to_string(&sprite_rs_path)?;

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    // Should NOT have super::rendering::Texture (invalid path)
    assert!(
        !sprite_rs_content.contains("super::rendering::Texture"),
        "Should NOT use 'super::rendering::Texture' (invalid path from within rendering/).\nGenerated:\n{}",
        sprite_rs_content
    );

    // Should use super::Texture (parent's re-export) OR super::texture::Texture (direct sibling)
    let has_super_texture = sprite_rs_content.contains("use super::Texture");
    let has_super_texture_module = sprite_rs_content.contains("use super::texture::Texture");

    assert!(
        has_super_texture || has_super_texture_module,
        "Should use 'super::Texture' or 'super::texture::Texture'.\nGenerated:\n{}",
        sprite_rs_content
    );

    Ok(())
}
