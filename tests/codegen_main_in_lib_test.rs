/// TDD Test: main.wj should not be included in lib.rs
/// 
/// Problem: When compiling a project with main.wj + other modules,
/// the generated lib.rs includes "pub mod main;" which is incorrect.
/// 
/// Binary projects should have:
/// - main.rs (contains fn main())
/// - lib.rs (contains pub mod for other modules, NOT main)
/// 
/// This test verifies the fix.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_project(sources: &[(&str, &str)]) -> (String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // Create source files
    for (filename, content) in sources {
        let file_path = project_dir.join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&file_path, content).expect("Failed to write source file");
    }

    // Compile with wj
    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output_dir = project_dir.join("out");
    
    let _output = Command::new(wj_binary)
        .current_dir(project_dir)
        .arg("build")
        .arg(project_dir.join("main.wj"))
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    // Read generated files
    let lib_rs = fs::read_to_string(output_dir.join("lib.rs"))
        .unwrap_or_else(|_| String::new());
    let main_rs = fs::read_to_string(output_dir.join("main.rs"))
        .unwrap_or_else(|_| String::new());

    (lib_rs, main_rs)
}

#[test]
fn test_main_not_in_lib_rs() {
    // TDD: Binary project with main.wj + helper module
    let sources = &[
        ("main.wj", r#"
use helper

fn main() {
    let result = helper::process()
    println!("Result: {}", result)
}
"#),
        ("helper.wj", r#"
pub fn process() -> int {
    42
}
"#),
    ];

    let (lib_rs, main_rs) = compile_project(sources);

    println!("Generated lib.rs:\n{}", lib_rs);
    println!("\nGenerated main.rs:\n{}", main_rs);

    // CRITICAL: lib.rs should NOT contain "pub mod main"
    assert!(
        !lib_rs.contains("pub mod main"),
        "lib.rs should not include 'pub mod main' for binary projects.\n\
         main() belongs in main.rs, not lib.rs.\n\
         Generated lib.rs:\n{}",
        lib_rs
    );

    // lib.rs SHOULD contain other modules
    assert!(
        lib_rs.contains("pub mod helper") || lib_rs.is_empty(),
        "lib.rs should contain other modules or be empty.\n\
         Generated lib.rs:\n{}",
        lib_rs
    );

    // main.rs SHOULD contain fn main()
    assert!(
        main_rs.contains("fn main()"),
        "main.rs should contain fn main().\n\
         Generated main.rs:\n{}",
        main_rs
    );
}

#[test]
fn test_library_project_no_main() {
    // TDD: Library project (no main, just regular modules)
    // The key test: even if a file is named "main.wj" but has no main() function,
    // it shouldn't be treated as the entry point
    let sources = &[
        ("helper.wj", r#"
pub fn library_function() -> int {
    100
}
"#),
    ];

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // Create source files
    for (filename, content) in sources {
        fs::write(project_dir.join(filename), content).expect("Failed to write source");
    }

    // Compile with wj
    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output_dir = project_dir.join("out");
    
    let _output = Command::new(wj_binary)
        .current_dir(project_dir)
        .arg("build")
        .arg(project_dir.join("helper.wj"))
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    // For a single module file without main(), it should generate a single .rs file
    let helper_rs = fs::read_to_string(output_dir.join("helper.rs"))
        .unwrap_or_else(|_| String::new());

    println!("Generated helper.rs:\n{}", helper_rs);

    // Should contain the library function
    assert!(
        helper_rs.contains("pub fn library_function"),
        "Generated Rust should contain library function"
    );

    // Should NOT contain "pub mod main" anywhere
    assert!(
        !helper_rs.contains("pub mod main"),
        "Library file should not reference main module"
    );

    // Should NOT contain fn main()
    assert!(
        !helper_rs.contains("fn main()"),
        "Library file should not contain main() function"
    );
}

#[test]
fn test_main_with_submodules() {
    // TDD: Binary with nested module structure
    let sources = &[
        ("main.wj", r#"
use game::player
use game::enemy

fn main() {
    player::spawn()
    enemy::spawn()
}
"#),
        ("game/player.wj", r#"
pub fn spawn() {
    println!("Player spawned")
}
"#),
        ("game/enemy.wj", r#"
pub fn spawn() {
    println!("Enemy spawned")
}
"#),
        ("game/mod.wj", r#"
pub mod player
pub mod enemy
"#),
    ];

    let (lib_rs, main_rs) = compile_project(sources);

    println!("Generated lib.rs (nested modules):\n{}", lib_rs);
    println!("\nGenerated main.rs (nested modules):\n{}", main_rs);

    // lib.rs should NOT contain "pub mod main"
    assert!(
        !lib_rs.contains("pub mod main"),
        "lib.rs should not include 'pub mod main' even with nested modules"
    );

    // lib.rs SHOULD contain game module
    assert!(
        lib_rs.contains("pub mod game") || lib_rs.is_empty(),
        "lib.rs should contain game module"
    );

    // main.rs SHOULD contain fn main()
    assert!(
        main_rs.contains("fn main()"),
        "main.rs should contain fn main()"
    );
}
