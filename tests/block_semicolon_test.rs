use anyhow::Result;
/// TDD Test: Block Expression Semicolons — Intermediate vs Last Statement
///
/// PROBLEM 1: In a block expression used as a match arm body (e.g., `let _ = match ... { Arm => { expr1; expr2 } }`),
/// intermediate expression statements lose their semicolons when `in_expression_context` is true.
/// This causes Rust to try to use the intermediate expression as a value, producing:
/// "expected `;`, found `identifier`"
///
/// PROBLEM 2: When fixing Problem 1 by clearing in_expression_context for ALL non-last statements,
/// it also affects the LAST statement of if-else branches used as expressions. The last expression
/// in `if cond { None } else { Some(x) }` would get a semicolon, turning `None` into `None;`
/// which returns () instead of Option<T>.
///
/// FIX: Only clear in_expression_context for truly non-last statements. The last statement
/// (even if not an Expression type, e.g., Statement::If) should preserve expression context
/// so inner branches retain correct semicolon behavior.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_intermediate_statements_get_semicolons_in_match_arms() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_semicolon_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with multiple statements in match arms
    // The intermediate statements MUST get semicolons even in expression context
    fs::write(
        src_dir.join("main.wj"),
        r#"
pub struct Panel {
    pub value: i32,
}

pub enum Mode {
    A,
    B,
}

impl Panel {
    pub fn render(self, mode: Mode) {
        match mode {
            Mode::A => {
                self.do_thing_a()
                self.do_thing_b()
            }
            Mode::B => {
                self.do_thing_b()
            }
        }
    }

    pub fn do_thing_a(self) {
        // placeholder
    }

    pub fn do_thing_b(self) {
        // placeholder
    }
}

fn main() {
    let panel = Panel { value: 42 }
    panel.render(Mode::A)
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "semicolon-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile with cargo to verify the generated Rust compiles
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Build with intermediate statements in match arms should succeed.\nstdout: {}\nstderr: {}",
        stdout, stderr
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_expression_preserves_return_value() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_ifelse_expr_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    // Write WJ code with if-else used as expression (the regression case)
    // The LAST expression in if/else branches must NOT get semicolons
    fs::write(
        src_dir.join("main.wj"),
        r#"
pub struct Editor {
    pub selected: Vec<i64>,
    pub last_selected: Option<i64>,
}

impl Editor {
    pub fn update_selection(mut self) {
        self.last_selected = if self.selected.is_empty() {
            None
        } else {
            Some(self.selected[0])
        }
    }
}

fn main() {
    let mut editor = Editor {
        selected: vec![1, 2, 3],
        last_selected: None,
    }
    editor.update_selection()
}
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "ifelse-expr-test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Compile with cargo to verify it compiles
    let output_dir = temp_dir.join("src");
    let output = Command::new(get_wj_compiler())
        .args(["build"])
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Build with if-else expression should succeed (no semicolons on None/Some).\nstdout: {}\nstderr: {}",
        stdout, stderr
    );

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
