//! TDD for Bug #2: Detect test files and generate [[test]] targets
//! 
//! Problem: Compiler generates Cargo.toml without [[bin]] or [[test]] targets,
//!          causing test files to not be recognized properly.
//! 
//! Solution: Detect test files (containing #[test] functions) and generate
//!           appropriate [[test]] targets. Generate [[bin]] for executables.

use std::path::PathBuf;
use std::fs;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_file_with_test_functions_generates_test_target() {
    // GREEN: Test that .rs files with #[test] generate [[test]] targets
    // Note: We test with pre-generated .rs files since Windjammer parser
    // doesn't support #[test] attributes yet (future feature)
    
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_target_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    let build_dir = test_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap();

    // Manually create a .rs test file (simulating generated output)
    let rust_test_code = r#"
#[test]
fn test_addition() {
    assert_eq!(1 + 1, 2);
}

#[test]
fn test_subtraction() {
    assert_eq!(5 - 3, 2);
}
"#;
    
    fs::write(build_dir.join("my_tests.rs"), rust_test_code).unwrap();
    
    // Manually trigger Cargo.toml generation by calling wj build with a dummy file
    let dummy_wj = r#"
fn helper() {
    let x = 42;
}
"#;
    fs::write(test_dir.join("dummy.wj"), dummy_wj).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("dummy.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(),
            "wj build should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr));

    let cargo_toml = fs::read_to_string(build_dir.join("Cargo.toml"))
        .expect("Should have generated Cargo.toml");

    println!("Generated Cargo.toml:\n{}", cargo_toml);

    // Should generate [[test]] target for test files
    assert!(
        cargo_toml.contains("[[test]]"),
        "Should generate [[test]] target for files with #[test] functions, got:\n{}",
        cargo_toml
    );
    assert!(
        cargo_toml.contains("name = \"my_tests\""),
        "Should set test name to file name, got:\n{}",
        cargo_toml
    );
    assert!(
        cargo_toml.contains("path = \"my_tests.rs\""),
        "Should set test path, got:\n{}",
        cargo_toml
    );

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_executable_file_generates_bin_target() {
    // RED: This test should fail until we implement bin target generation
    
    let test_dir = std::env::temp_dir().join(format!(
        "wj_bin_target_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    // Executable file with main() function
    let windjammer_code = r#"
fn main() {
    println("Hello, World!");
}
"#;
    
    fs::write(test_dir.join("my_game.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("my_game.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(),
            "wj build should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr));

    let cargo_toml = fs::read_to_string(test_dir.join("build/Cargo.toml"))
        .expect("Should have generated Cargo.toml");

    println!("Generated Cargo.toml:\n{}", cargo_toml);

    // Should generate [[bin]] target for executables
    assert!(
        cargo_toml.contains("[[bin]]"),
        "Should generate [[bin]] target for files with main(), got:\n{}",
        cargo_toml
    );
    assert!(
        cargo_toml.contains("name = \"my_game\""),
        "Should set bin name to file name, got:\n{}",
        cargo_toml
    );
    assert!(
        cargo_toml.contains("path = \"my_game.rs\""),
        "Should set bin path, got:\n{}",
        cargo_toml
    );
    
    // Should NOT generate [[test]] for executables
    assert!(
        !cargo_toml.contains("[[test]]"),
        "Should NOT generate [[test]] for executable files, got:\n{}",
        cargo_toml
    );

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mixed_file_with_main_and_tests_generates_bin_target() {
    // GREEN: Mixed files (main + tests) should be treated as executables
    // Tests can live alongside main() in the same file
    
    let test_dir = std::env::temp_dir().join(format!(
        "wj_mixed_target_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    let build_dir = test_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap();

    // Manually create a .rs file with both main() and tests
    let rust_mixed_code = r#"
fn main() {
    println!("Hello, World!");
}

#[test]
fn test_something() {
    assert!(true);
}
"#;
    
    fs::write(build_dir.join("my_app.rs"), rust_mixed_code).unwrap();
    
    // Manually trigger Cargo.toml generation
    let dummy_wj = r#"fn helper() {}"#;
    fs::write(test_dir.join("dummy.wj"), dummy_wj).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("dummy.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(),
            "wj build should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr));

    let cargo_toml = fs::read_to_string(build_dir.join("Cargo.toml"))
        .expect("Should have generated Cargo.toml");

    println!("Generated Cargo.toml:\n{}", cargo_toml);

    // Mixed files should generate [[bin]] (main takes precedence)
    assert!(
        cargo_toml.contains("[[bin]]"),
        "Should generate [[bin]] for files with main() (even with tests), got:\n{}",
        cargo_toml
    );
    assert!(
        cargo_toml.contains("name = \"my_app\""),
        "Should set bin name, got:\n{}",
        cargo_toml
    );

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_library_file_generates_no_target() {
    // Library files (no main, no tests) should not generate [[bin]] or [[test]]
    
    let test_dir = std::env::temp_dir().join(format!(
        "wj_lib_target_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    // Library code (no main, no tests)
    let windjammer_code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

struct Point {
    x: f32,
    y: f32,
}
"#;
    
    fs::write(test_dir.join("math.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("math.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(),
            "wj build should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr));

    let cargo_toml = fs::read_to_string(test_dir.join("build/Cargo.toml"))
        .expect("Should have generated Cargo.toml");

    println!("Generated Cargo.toml:\n{}", cargo_toml);

    // Library files should NOT generate targets (they're just modules)
    assert!(
        !cargo_toml.contains("[[bin]]"),
        "Should NOT generate [[bin]] for library files, got:\n{}",
        cargo_toml
    );
    assert!(
        !cargo_toml.contains("[[test]]"),
        "Should NOT generate [[test]] for library files, got:\n{}",
        cargo_toml
    );

    fs::remove_dir_all(test_dir).ok();
}
