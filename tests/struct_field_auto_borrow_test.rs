// TDD Test: Struct field (owned String) auto-borrow when passed to methods expecting &String
// Reproduces E0308 error in dialog.wj line 238 (line 212 in generated code)

use std::fs;
use std::process::Command;
use tempfile::tempdir;
use windjammer::{build_project_ext, CompilationTarget};

fn compile_single_file(source: &str) -> String {
    let src = tempdir().expect("tempdir for src");
    let out = tempdir().expect("tempdir for out");
    fs::write(src.path().join("test.wj"), source).expect("write test.wj");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");
    let raw = fs::read_to_string(out.path().join("test.rs")).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.contains("use super::"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn verify_with_rustc(rust_code: &str) {
    let dir = tempdir().expect("tempdir for rustc");
    let rs_file = dir.path().join("verify.rs");
    fs::write(&rs_file, rust_code).expect("write .rs for rustc");
    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(dir.path().join("verify.rmeta"))
        .arg(&rs_file)
        .output()
        .expect("failed to run rustc");
    assert!(
        output.status.success(),
        "Generated code should compile. Error:\n{}\n\nGenerated:\n{}",
        String::from_utf8_lossy(&output.stderr),
        rust_code
    );
}

#[test]
fn test_struct_field_string_auto_borrow() {
    let code = r#"
struct Player {
    name: string,
}

impl Player {
    // Avoid str == literal in body: current codegen can emit `*attr_name == "strength"`
    // which is invalid Rust; this test targets auto-borrow at the call site only.
    pub fn get_attribute(self, attr_name: string) -> i32 {
        if attr_name.len() > 0 {
            10
        } else {
            0
        }
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

    let generated = compile_single_file(code);

    assert!(
        generated.contains("player.get_attribute(&self.stat_name)")
            || generated.contains("player.get_attribute(&*self.stat_name)")
            || generated.contains("player.get_attribute(self.stat_name)"),
        "Should pass stat_name to get_attribute. Generated:\n{}",
        generated
    );

    verify_with_rustc(&generated);
}

#[test]
fn test_struct_field_in_comparison() {
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

    let generated = compile_single_file(code);

    assert!(
        generated.contains("health <= self.max_health"),
        "Should generate comparison with struct field. Generated:\n{}",
        generated
    );

    verify_with_rustc(&generated);
}
