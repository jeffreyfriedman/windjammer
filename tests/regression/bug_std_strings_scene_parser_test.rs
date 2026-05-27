#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Scene-parser style code must compile with std::strings (no .as_bytes() leakage).
#[test]
fn test_std_strings_scene_parser_pattern_compiles() {
    let source = r##"
use std::strings

pub struct MiniScene {
    pub name: string,
    pub spawn_x: f32,
}

pub fn parse_scene_line(text: string) -> MiniScene {
    let mut scene = MiniScene { name: "Unnamed", spawn_x: 0.0 }
    let lines = strings.split_lines(text)
    let mut i: usize = 0
    while i < lines.len() {
        let line = strings.trim(lines[i])
        i = i + 1
        if strings.is_empty(line) || strings.starts_with(line, "//") {
            continue
        }
        let parts = strings.split_whitespace(line)
        if parts.len() >= 2 && parts[0] == "scene" {
            scene.name = parts[1]
        } else if parts.len() >= 2 && parts[0] == "spawn" {
            scene.spawn_x = strings.parse_f32(parts[1])
        }
    }
    scene
}
"##;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("strings::split_lines"),
        "runtime std module calls must use :: not . — got:\n{}",
        generated
    );
    assert!(
        generated.contains("strings::parse_f32"),
        "runtime std module calls must use :: not . — got:\n{}",
        generated
    );
    assert!(
        !generated.contains("strings.split_lines"),
        "must not emit strings.split_lines (module is not a value). Got:\n{}",
        generated
    );
    assert!(
        !generated.contains(".as_bytes()"),
        "must not emit .as_bytes() in scene parser pattern. Got:\n{}",
        generated
    );
}
