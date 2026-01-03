/// TDD Test: Module Declaration Bug
///
/// Bug: Generated mod.rs files declare modules from other directories
///
/// Example: audio/mod.rs incorrectly declares `pub mod entity;`
/// when entity.wj is actually in ecs/ directory, not audio/.
///
/// THE WINDJAMMER WAY: Fix the root cause, not the symptom.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_module_declarations_only_include_local_modules() {
    // Create a test project structure:
    // src/
    //   mod.wj
    //   audio/
    //     mod.wj
    //     sound.wj
    //   ecs/
    //     mod.wj
    //     entity.wj

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    // Create audio module
    let audio_dir = src_dir.join("audio");
    fs::create_dir(&audio_dir).expect("Failed to create audio dir");

    fs::write(
        audio_dir.join("mod.wj"),
        "// Audio module\npub mod sound;\npub use sound::Sound;\n",
    )
    .expect("Failed to write audio/mod.wj");

    fs::write(
        audio_dir.join("sound.wj"),
        "pub struct Sound { pub volume: f32 }\n",
    )
    .expect("Failed to write audio/sound.wj");

    // Create ecs module
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

    // Create root mod.wj
    fs::write(src_dir.join("mod.wj"), "pub mod audio;\npub mod ecs;\n")
        .expect("Failed to write mod.wj");

    // Compile the project
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

    assert!(
        compile_output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );

    // Debug: List generated files
    println!("\nGenerated files:");
    fn list_dir(dir: &std::path::Path, prefix: &str) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy();
                if path.is_dir() {
                    println!("  {}{}/", prefix, name);
                    list_dir(&path, &format!("{}  ", prefix));
                } else {
                    println!("  {}{}", prefix, name);
                }
            }
        }
    }
    list_dir(&output_dir, "");
    println!();

    // Read the generated audio/mod.rs
    let audio_mod_rs = output_dir.join("audio/mod.rs");
    let audio_mod_content = fs::read_to_string(&audio_mod_rs).expect("Failed to read audio/mod.rs");

    println!("Generated audio/mod.rs:\n{}", audio_mod_content);

    // Check: audio/mod.rs should declare `pub mod sound;`
    assert!(
        audio_mod_content.contains("pub mod sound;"),
        "audio/mod.rs should declare sound module"
    );

    // Check: audio/mod.rs should NOT declare OR define entity module
    // (neither `pub mod entity;` nor `pub mod entity { ... }`)
    assert!(
        !audio_mod_content.contains("pub mod entity"),
        "audio/mod.rs should NOT declare or define entity module (it's in ecs/, not audio/)\n\nGenerated content:\n{}",
        audio_mod_content
    );

    // Read the generated ecs/mod.rs
    let ecs_mod_rs = output_dir.join("ecs/mod.rs");
    let ecs_mod_content = fs::read_to_string(&ecs_mod_rs).expect("Failed to read ecs/mod.rs");

    println!("Generated ecs/mod.rs:\n{}", ecs_mod_content);

    // Check: ecs/mod.rs should declare `pub mod entity;`
    assert!(
        ecs_mod_content.contains("pub mod entity;"),
        "ecs/mod.rs should declare entity module"
    );

    // Check: ecs/mod.rs should NOT declare OR define sound module
    // (neither `pub mod sound;` nor `pub mod sound { ... }`)
    assert!(
        !ecs_mod_content.contains("pub mod sound"),
        "ecs/mod.rs should NOT declare or define sound module (it's in audio/, not ecs/)\n\nGenerated content:\n{}",
        ecs_mod_content
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_lib_rs_declares_only_top_level_modules() {
    // Create a test project structure
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    // Create audio module with submodule
    let audio_dir = src_dir.join("audio");
    fs::create_dir(&audio_dir).expect("Failed to create audio dir");

    fs::write(
        audio_dir.join("mod.wj"),
        "pub mod sound;\npub use sound::Sound;\n",
    )
    .expect("Failed to write audio/mod.wj");

    fs::write(audio_dir.join("sound.wj"), "pub struct Sound {}\n")
        .expect("Failed to write audio/sound.wj");

    // Create root mod.wj
    fs::write(
        src_dir.join("mod.wj"),
        "pub mod audio;\npub use audio::Sound;\n",
    )
    .expect("Failed to write mod.wj");

    // Compile the project
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

    assert!(
        compile_output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );

    // Read the generated lib.rs
    let lib_rs = output_dir.join("lib.rs");
    let lib_content = fs::read_to_string(&lib_rs).expect("Failed to read lib.rs");

    println!("Generated lib.rs:\n{}", lib_content);

    // Check: lib.rs should declare `pub mod audio;`
    assert!(
        lib_content.contains("pub mod audio;"),
        "lib.rs should declare audio module"
    );

    // Check: lib.rs should NOT declare OR define `pub mod sound`
    // (sound is a submodule of audio, not a top-level module)
    assert!(
        !lib_content.contains("pub mod sound"),
        "lib.rs should NOT declare or define sound module (it's in audio/, not top-level)\n\nGenerated content:\n{}",
        lib_content
    );
}
