use anyhow::Result;
/// TDD Test: Sibling Module Imports (Nested Import Bug Part 2)
///
/// PROBLEM: When a module imports a sibling module (same parent directory),
/// the transpiler doesn't correctly prefix the import with `super::` or `crate::parent::`.
///
/// Example:
/// ```
/// // src_wj/rendering/sprite.wj
/// use texture_atlas::TextureAtlas  // Import sibling module
/// ```
///
/// Should generate:
/// ```rust
/// // Generated: rendering/sprite.rs
/// use super::texture_atlas::TextureAtlas;  // or crate::rendering::texture_atlas::TextureAtlas
/// ```
///
/// But currently generates:
/// ```rust
/// use texture_atlas::TextureAtlas;  // ❌ Bare import, doesn't exist!
/// ```
///
/// This is the HARDEST problem - nested module import resolution within subdirectories.
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
fn test_sibling_module_import() -> Result<()> {
    // Create temp directory with unique name
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_sibling_import_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    // Create src_wj/rendering directory
    let rendering_dir = temp_dir.join("src_wj").join("rendering");
    fs::create_dir_all(&rendering_dir)?;

    // Create texture.wj (sibling module 1)
    let texture_wj = rendering_dir.join("texture.wj");
    fs::write(
        &texture_wj,
        r#"
struct Texture {
    id: u32,
    width: i32,
    height: i32,
}

impl Texture {
    fn new(id: u32) -> Texture {
        Texture { id: id, width: 0, height: 0 }
    }
}
"#,
    )?;

    // Create sprite.wj (imports sibling module)
    let sprite_wj = rendering_dir.join("sprite.wj");
    fs::write(
        &sprite_wj,
        r#"
use texture::Texture

struct Sprite {
    texture: Texture,
    x: f32,
    y: f32,
}

impl Sprite {
    fn new(tex: Texture) -> Sprite {
        Sprite { texture: tex, x: 0.0, y: 0.0 }
    }
}
"#,
    )?;

    // Create mod.wj to declare both modules
    let mod_wj = rendering_dir.join("mod.wj");
    fs::write(
        &mod_wj,
        r#"
pub mod texture;
pub mod sprite;

pub use texture::Texture;
pub use sprite::Sprite;
"#,
    )?;

    // Create wj.toml
    let wj_toml = temp_dir.join("wj.toml");
    fs::write(
        &wj_toml,
        r#"
[package]
name = "sibling_import_test"
version = "0.1.0"

[dependencies]
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

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check the generated sprite.rs file
    let sprite_rs_path = lib_output.join("rendering").join("sprite.rs");
    assert!(
        sprite_rs_path.exists(),
        "sprite.rs should be generated at {:?}",
        sprite_rs_path
    );

    let sprite_rs_content = fs::read_to_string(&sprite_rs_path)?;

    // The import should be prefixed with super:: or crate::rendering::
    let has_super_import = sprite_rs_content.contains("use super::texture::Texture");
    let has_crate_import = sprite_rs_content.contains("use crate::rendering::texture::Texture");
    let has_bare_import = sprite_rs_content.contains("use texture::Texture")
        && !sprite_rs_content.contains("super::texture::Texture")
        && !sprite_rs_content.contains("crate::rendering::texture::Texture");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !has_bare_import,
        "Sibling module import should NOT be bare 'use texture::Texture'.\nGenerated sprite.rs:\n{}",
        sprite_rs_content
    );

    assert!(
        has_super_import || has_crate_import,
        "Sibling module import should use 'super::texture::Texture' or 'crate::rendering::texture::Texture'.\nGenerated sprite.rs:\n{}",
        sprite_rs_content
    );

    Ok(())
}
