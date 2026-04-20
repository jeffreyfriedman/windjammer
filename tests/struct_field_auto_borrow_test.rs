// TDD Test: Struct field (owned String) auto-borrow when passed to methods expecting &String
// Reproduces E0308 error in dialog.wj line 238 (line 212 in generated code)
//
// PROBLEM:
// self.stat_name (String field) passed to get_attribute(name: &String)
// Compiler should auto-add & to make it &self.stat_name
//
// EXAMPLE:
// game_state.player.get_attribute(self.stat_name) >= value
//                                  ^^^^^^^^^^^^^^
// ERROR: expected `&String`, found `String`
//
// SOLUTION:
// Auto-borrow struct fields when methods expect borrowed parameters

use std::fs;
use std::process::Command;

#[test]
fn test_struct_field_string_auto_borrow() {
    let code = r#"
struct Player {
    name: string,
}

impl Player {
    pub fn get_attribute(self, attr_name: string) -> i32 {
        if attr_name == "strength" {
            return 10
        }
        0
    }
}

struct StatCheck {
    stat_name: string,
    required_value: i32,
}

impl StatCheck {
    pub fn passes(self, player: Player) -> bool {
        player.get_attribute(self.stat_name) >= self.required_value
    }
}
"#;

    // Compile with wj compiler
    let temp_dir = "/tmp/windjammer_struct_field_autoborrow";
    let _ = std::fs::remove_dir_all(temp_dir);
    std::fs::create_dir_all(temp_dir).unwrap();

    let wj_file = format!("{}/test.wj", temp_dir);
    std::fs::write(&wj_file, code).unwrap();

    let wj_path = "/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj";
    let output = Command::new(wj_path)
        .arg("build")
        .arg(&wj_file)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "wj compilation failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    // Read generated Rust code
    let rust_file = format!("{}/build/test.rs", temp_dir);
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust");

    // ASSERT: Should auto-borrow self.stat_name when passing to get_attribute
    assert!(
        generated.contains("player.get_attribute(&self.stat_name)")
            || generated.contains("player.get_attribute(&*self.stat_name)"),
        "Should auto-borrow self.stat_name when passing to method expecting &String. Generated:\n{}",
        generated
    );

    // Verify rustc compilation
    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(&rust_file)
        .output()
        .unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Generated code should compile. Error:\n{}\n\nGenerated:\n{}",
            stderr, generated
        );
    }
}

#[test]
fn test_struct_field_in_comparison() {
    // TDD: Struct field used in comparison with method call result
    let code = r#"
struct Config {
    max_health: i32,
}

impl Config {
    pub fn is_valid(self, health: i32) -> bool {
        health <= self.max_health
    }
}
"#;

    // Compile with wj compiler
    let temp_dir = "/tmp/windjammer_struct_field_comparison";
    let _ = std::fs::remove_dir_all(temp_dir);
    std::fs::create_dir_all(temp_dir).unwrap();

    let wj_file = format!("{}/test.wj", temp_dir);
    std::fs::write(&wj_file, code).unwrap();

    let wj_path = "/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj";
    let output = Command::new(wj_path)
        .arg("build")
        .arg(&wj_file)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "wj compilation failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    // Read generated Rust code
    let rust_file = format!("{}/build/test.rs", temp_dir);
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust");

    // Verify rustc compilation (i32 is Copy, should work fine)
    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(&rust_file)
        .output()
        .unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Generated code should compile. Error:\n{}\n\nGenerated:\n{}",
            stderr, generated
        );
    }
}
