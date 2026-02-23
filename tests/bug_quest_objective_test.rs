// BUG: Parser reports "Unexpected Type token" in quest/objective.wj
//
// DISCOVERED DURING: Dogfooding windjammer-game
//
// PROBLEM:
// Windjammer source: `pub type QuestObjective = Objective`
// Parser fails: "Unexpected token: Type (at token position N)"
//
// ROOT CAUSE:
// Parser's parse_item() doesn't handle top-level `pub type Name = Type` type alias.
// The `type` keyword is only recognized inside impl blocks (associated types).
//
// FIX:
// Add Token::Type handling in parse_item for module-level type aliases.
// Syntax: pub type Name = Type;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_compiler())
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| format!("Failed to read: {}", e))
}

#[test]
fn test_pub_type_alias_minimal() {
    // Minimal: pub type Alias = Struct (exact pattern from quest/objective.wj)
    let source = r#"
pub struct Objective {
    id: u32,
}

pub type QuestObjective = Objective
"#;

    let result = compile_to_rust(source);
    assert!(
        result.is_ok(),
        "pub type alias should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("pub type QuestObjective = Objective"),
        "Generated Rust should contain type alias: {}",
        rust_code
    );
}

#[test]
fn test_type_alias_without_pub() {
    // type alias without pub
    let source = r#"
pub struct Foo {
    x: i32,
}

type Bar = Foo
"#;

    let result = compile_to_rust(source);
    assert!(
        result.is_ok(),
        "type alias without pub should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("type Bar = Foo"),
        "Generated Rust should contain type alias"
    );
}

#[test]
fn test_type_alias_quest_objective_exact() {
    // Exact pattern from quest/objective.wj (simplified)
    let source = r#"
pub enum ObjectiveType {
    Kill,
    Collect,
}

pub struct Objective {
    id: u32,
    objective_type: ObjectiveType,
}

impl Objective {
    fn new(id: u32, objective_type: ObjectiveType) -> Objective {
        Objective { id, objective_type }
    }
}

pub type QuestObjective = Objective
"#;

    let result = compile_to_rust(source);
    assert!(
        result.is_ok(),
        "quest/objective.wj pattern should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("pub type QuestObjective = Objective"),
        "Generated Rust should contain pub type QuestObjective = Objective"
    );
}
