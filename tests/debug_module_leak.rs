/// Debug test to trace exactly where inline modules are being created
///
/// This test will help us pinpoint where Item::Mod entries are getting
/// their items populated during compilation.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_trace_module_item_creation() {
    // Create simple test structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    // Create audio module with just mod declaration
    let audio_dir = src_dir.join("audio");
    fs::create_dir(&audio_dir).expect("Failed to create audio dir");

    fs::write(
        audio_dir.join("mod.wj"),
        "// Audio module\npub mod sound;\npub use sound::Sound;\n", // With pub use!
    )
    .expect("Failed to write audio/mod.wj");

    fs::write(
        audio_dir.join("sound.wj"),
        "pub struct Sound { pub volume: f32 }\n",
    )
    .expect("Failed to write audio/sound.wj");

    // Create ecs module (SECOND module - this triggers the bug!)
    let ecs_dir = src_dir.join("ecs");
    fs::create_dir(&ecs_dir).expect("Failed to create ecs dir");

    fs::write(
        ecs_dir.join("mod.wj"),
        "// ECS module\npub mod entity;\npub use entity::Entity;\n",
    )
    .expect("Failed to write ecs/mod.wj");

    fs::write(
        ecs_dir.join("entity.wj"),
        "pub struct Entity { pub id: i64 }\n",
    )
    .expect("Failed to write ecs/entity.wj");

    // Create root mod.wj with BOTH modules
    fs::write(src_dir.join("mod.wj"), "pub mod audio;\npub mod ecs;\n")
        .expect("Failed to write mod.wj");

    // Compile with debug output
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output_dir = temp_dir.path().join("build");

    let compile_output = Command::new(&wj_binary)
        .args([
            "build",
            src_dir.join("mod.wj").to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    // Don't print full stdout/stderr, too verbose
    // println!("Compiler stdout:\n{}", String::from_utf8_lossy(&compile_output.stdout));
    // println!("Compiler stderr:\n{}", String::from_utf8_lossy(&compile_output.stderr));

    assert!(
        compile_output.status.success(),
        "Compilation failed:\n{}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    // Read the generated audio/mod.rs
    let audio_mod_rs = output_dir.join("audio/mod.rs");
    let audio_mod_content = fs::read_to_string(&audio_mod_rs).expect("Failed to read audio/mod.rs");

    println!("\n=== Generated audio/mod.rs ===");
    println!("{}", audio_mod_content);
    println!("=== End ===\n");

    // Read the generated ecs/mod.rs
    let ecs_mod_rs = output_dir.join("ecs/mod.rs");
    let ecs_mod_content = fs::read_to_string(&ecs_mod_rs).expect("Failed to read ecs/mod.rs");

    println!("\n=== Generated ecs/mod.rs ===");
    println!("{}", ecs_mod_content);
    println!("=== End ===\n");

    // ANALYSIS: What do we see in audio/mod.rs?
    let audio_has_inline_sound = audio_mod_content.contains("pub mod sound {");
    let audio_has_declaration_sound = audio_mod_content.contains("pub mod sound;");
    let audio_has_inline_entity = audio_mod_content.contains("pub mod entity {");

    println!("Audio module analysis:");
    println!(
        "  Has inline 'pub mod sound {{ ... }}': {}",
        audio_has_inline_sound
    );
    println!(
        "  Has declaration 'pub mod sound;': {}",
        audio_has_declaration_sound
    );
    println!(
        "  Has inline 'pub mod entity {{ ... }}' (WRONG!): {}",
        audio_has_inline_entity
    );

    // ANALYSIS: What do we see in ecs/mod.rs?
    let ecs_has_inline_entity = ecs_mod_content.contains("pub mod entity {");
    let ecs_has_declaration_entity = ecs_mod_content.contains("pub mod entity;");
    let ecs_has_inline_sound = ecs_mod_content.contains("pub mod sound {");

    println!("\nECS module analysis:");
    println!(
        "  Has inline 'pub mod entity {{ ... }}': {}",
        ecs_has_inline_entity
    );
    println!(
        "  Has declaration 'pub mod entity;': {}",
        ecs_has_declaration_entity
    );
    println!(
        "  Has inline 'pub mod sound {{ ... }}' (WRONG!): {}",
        ecs_has_inline_sound
    );

    // The bug: We expect ONLY local declarations
    if audio_has_inline_entity {
        panic!(
            "BUG CONFIRMED: audio/mod.rs contains inline definition of entity!\n\
             Entity belongs in ecs/, not audio/.\n\
             This means ALL items from ALL files are being merged during generation."
        );
    }

    if ecs_has_inline_sound {
        panic!(
            "BUG CONFIRMED: ecs/mod.rs contains inline definition of sound!\n\
             Sound belongs in audio/, not ecs/.\n\
             This means ALL items from ALL files are being merged during generation."
        );
    }
}
