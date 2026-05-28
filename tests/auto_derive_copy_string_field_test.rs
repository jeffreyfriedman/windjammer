#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
/// TDD: Verify that structs containing String fields do NOT auto-derive Copy.
///
/// Bug: InputAction has a `name: string` field, yet InputBinding (which contains
/// InputAction) was getting `#[derive(Debug, Clone, Copy)]`. String is not Copy,
/// so any struct transitively containing String must not derive Copy.
use std::process::Command;
use tempfile::TempDir;

fn find_rs_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                results.push(path.to_path_buf());
            }
            if path.is_dir() {
                results.extend(find_rs_files(&path));
            }
        }
    }
    results
}

fn compile_wj_library(files: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    for (name, code) in files {
        let file_path = src_dir.join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, code).unwrap();
    }

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .arg("build")
        .arg(&src_dir)
        .arg("--output")
        .arg(&out_dir)
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj library compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_files = find_rs_files(&out_dir);
    let mut results = std::collections::HashMap::new();
    for path in rs_files {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(&path).unwrap();
        results.insert(name, content);
    }
    results
}

#[test]
fn test_struct_with_string_field_does_not_derive_copy() {
    let code = r#"
pub struct Name {
    pub value: string,
}
"#;
    let rust = test_utils::compile_single(code);
    // The struct should derive Debug, Clone, but NOT Copy
    assert!(
        !rust.contains("Copy"),
        "Struct with String field should NOT derive Copy. Generated:\n{}",
        rust
    );
}

#[test]
fn test_struct_containing_string_struct_does_not_derive_copy() {
    let code = r#"
pub struct InputAction {
    pub name: string,
}

pub struct InputBinding {
    pub action: InputAction,
    pub key_code: i32,
}
"#;
    let rust = test_utils::compile_single(code);

    // Neither struct should derive Copy
    // InputAction has String, InputBinding has InputAction (which has String)
    let _binding_section = rust.split("pub struct InputBinding").nth(1).unwrap_or("");

    // The InputBinding derive should not contain Copy
    let action_derive_line = rust
        .lines()
        .take_while(|l| !l.contains("pub struct InputAction"))
        .filter(|l| l.contains("#[derive("))
        .last();

    if let Some(derive_line) = action_derive_line {
        assert!(
            !derive_line.contains("Copy"),
            "InputAction should NOT derive Copy (has String field). Derive: {}",
            derive_line
        );
    }

    // Check that InputBinding doesn't have Copy either
    // Find the derive line just before "pub struct InputBinding"
    let lines: Vec<&str> = rust.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("pub struct InputBinding") && i > 0 {
            let prev_line = lines[i - 1];
            if prev_line.contains("#[derive(") {
                assert!(
                    !prev_line.contains("Copy"),
                    "InputBinding should NOT derive Copy (contains InputAction with String). Derive: {}",
                    prev_line
                );
            }
        }
    }
}

#[test]
fn test_library_mode_string_struct_no_copy() {
    let files = vec![
        ("mod.wj", "pub mod input_action\npub mod input_binding\n"),
        (
            "input_action.wj",
            r#"
pub struct InputAction {
    pub name: string,
}

impl InputAction {
    pub fn new(name: string) -> InputAction {
        InputAction { name }
    }
}
"#,
        ),
        (
            "input_binding.wj",
            r#"
use crate::input_action::InputAction

pub struct InputBinding {
    pub action: InputAction,
    pub is_keyboard: bool,
    pub key_code: i32,
    pub gamepad_code: i32,
}
"#,
        ),
    ];

    let results = compile_wj_library(&files);

    if let Some(binding_rs) = results.get("input_binding") {
        let lines: Vec<&str> = binding_rs.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("pub struct InputBinding") && i > 0 {
                let derive_line = lines[i - 1];
                if derive_line.contains("#[derive(") {
                    assert!(
                        !derive_line.contains("Copy"),
                        "LIBRARY MODE: InputBinding should NOT derive Copy (contains InputAction with String). Derive: {}",
                        derive_line
                    );
                }
            }
        }
    }

    if let Some(action_rs) = results.get("input_action") {
        let lines: Vec<&str> = action_rs.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("pub struct InputAction") && i > 0 {
                let derive_line = lines[i - 1];
                if derive_line.contains("#[derive(") {
                    assert!(
                        !derive_line.contains("Copy"),
                        "LIBRARY MODE: InputAction should NOT derive Copy (has String field). Derive: {}",
                        derive_line
                    );
                }
            }
        }
    }
}

#[test]
fn test_library_mode_with_use_import() {
    let files = vec![
        (
            "mod.wj",
            "pub mod input_rebinding\n",
        ),
        (
            "input_rebinding/mod.wj",
            "pub mod input_action\npub mod key_code\npub mod gamepad_button\npub mod input_binding\n",
        ),
        (
            "input_rebinding/input_action.wj",
            r#"
pub struct InputAction {
    pub name: string,
}

impl InputAction {
    pub fn new(name: string) -> InputAction {
        InputAction { name }
    }

    pub fn name(self) -> string {
        self.name.clone()
    }
}
"#,
        ),
        (
            "input_rebinding/key_code.wj",
            r#"
pub struct KeyCode {
    pub code: i32,
}
"#,
        ),
        (
            "input_rebinding/gamepad_button.wj",
            r#"
pub struct GamepadButton {
    pub code: i32,
}
"#,
        ),
        (
            "input_rebinding/input_binding.wj",
            r#"
use crate::input_rebinding::InputAction
use crate::input_rebinding::KeyCode
use crate::input_rebinding::GamepadButton

pub struct InputBinding {
    pub action: InputAction,
    pub is_keyboard: bool,
    pub key_code: i32,
    pub gamepad_code: i32,
}

impl InputBinding {
    pub fn keyboard(action: InputAction, key_code: i32) -> InputBinding {
        InputBinding {
            action,
            is_keyboard: true,
            key_code,
            gamepad_code: -1,
        }
    }

    pub fn gamepad(action: InputAction, gamepad_code: i32) -> InputBinding {
        InputBinding {
            action,
            is_keyboard: false,
            key_code: -1,
            gamepad_code,
        }
    }

    pub fn is_keyboard_binding(self) -> bool {
        self.is_keyboard
    }

    pub fn is_gamepad_binding(self) -> bool {
        !self.is_keyboard
    }
}
"#,
        ),
    ];

    let results = compile_wj_library(&files);

    // Check input_binding output
    for (name, content) in &results {
        if name == "input_binding" || name.ends_with("/input_binding") {
            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.contains("pub struct InputBinding") && i > 0 {
                    let derive_line = lines[i - 1];
                    if derive_line.contains("#[derive(") {
                        assert!(
                            !derive_line.contains("Copy"),
                            "InputBinding should NOT derive Copy with nested imports. Derive: {}",
                            derive_line
                        );
                    }
                }
            }
        }
    }
}

/// TDD: Name collision between struct and enum with same name.
/// A unit enum `InputAction` is Copy, but a struct `InputAction` with a String
/// field is NOT. When both exist in the same project, the copy registry must
/// track them as distinct types (qualified by module path), not by bare name.
#[test]
fn test_name_collision_struct_and_enum_same_name() {
    let files = vec![
        ("mod.wj", "pub mod input\npub mod input_rebinding\n"),
        ("input/mod.wj", "pub mod input_port\n"),
        (
            "input/input_port.wj",
            r#"
pub enum InputAction {
    MoveForward,
    MoveBackward,
    Jump,
    Fire,
}
"#,
        ),
        (
            "input_rebinding/mod.wj",
            "pub mod input_action\npub mod input_binding\n",
        ),
        (
            "input_rebinding/input_action.wj",
            r#"
pub struct InputAction {
    pub name: string,
}
"#,
        ),
        (
            "input_rebinding/input_binding.wj",
            r#"
use crate::input_rebinding::InputAction

pub struct InputBinding {
    pub action: InputAction,
    pub is_keyboard: bool,
    pub key_code: i32,
}
"#,
        ),
    ];

    let results = compile_wj_library(&files);

    for (name, content) in &results {
        if name == "input_binding" || name.ends_with("/input_binding") {
            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.contains("pub struct InputBinding") && i > 0 {
                    let derive_line = lines[i - 1];
                    if derive_line.contains("#[derive(") {
                        assert!(
                            !derive_line.contains("Copy"),
                            "InputBinding should NOT derive Copy when InputAction struct has String \
                             (even though an enum InputAction exists). Derive: {}",
                            derive_line
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_struct_with_all_copy_fields_does_derive_copy() {
    let code = r#"
pub struct Point {
    pub x: f32,
    pub y: f32,
}
"#;
    let rust = test_utils::compile_single(code);
    // All fields are f32 (Copy), so struct should derive Copy
    assert!(
        rust.contains("Copy"),
        "Struct with all Copy fields should derive Copy. Generated:\n{}",
        rust
    );
}
