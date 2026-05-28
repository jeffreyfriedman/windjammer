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

use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let input = dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test.wj");
    let output = dir.path().join("output");
    std::fs::create_dir_all(&output).expect("create output dir");

    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", input.to_str().unwrap(), "--no-cargo", "-o"])
        .arg(output.to_str().unwrap())
        .output()
        .expect("run wj");

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let generated_path = output.join("test.rs");
    let generated = if generated_path.exists() {
        std::fs::read_to_string(&generated_path).unwrap_or_default()
    } else {
        String::new()
    };

    (result.status.success(), generated, combined)
}

/// When iterating over a field of a loop variable that is itself from a borrowed
/// iterator, the inner for-loop should borrow the field to avoid E0507.
///
/// Pattern:
///   for pass in self.passes {       // self is &self, so pass is &Pass
///       for binding in pass.bindings { // pass.bindings must be &pass.bindings
///           ...
///       }
///   }
#[test]
fn test_nested_for_loop_borrows_inner_field() {
    let source = r#"
pub struct Binding {
    pub slot: i32,
    pub buffer_id: i32,
}

pub struct Pass {
    pub name: String,
    pub bindings: Vec<Binding>,
}

pub struct Graph {
    pub passes: Vec<Pass>,
}

impl Graph {
    pub fn print_all(self) {
        for pass in self.passes {
            for binding in pass.bindings {
                eprintln!("{}: slot {}", pass.name, binding.slot)
            }
        }
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // The method should be &self (read-only)
    assert!(
        generated.contains("fn print_all(&self)"),
        "Method should be &self.\nGenerated:\n{}",
        generated
    );

    // The inner for-loop must borrow pass.bindings since pass is &Pass
    assert!(
        generated.contains("for binding in &pass.bindings"),
        "Inner for-loop should borrow pass.bindings (use &pass.bindings).\nGenerated:\n{}",
        generated
    );
}

/// When a method param is inferred as borrowed (&), inner for-loops
/// on fields of loop variables should also borrow.
#[test]
fn test_for_loop_borrows_field_of_borrowed_param() {
    let source = r#"
pub struct Item {
    pub name: String,
    pub value: i32,
}

pub struct Container {
    pub items: Vec<Item>,
    pub label: String,
}

pub struct Registry {
    pub containers: Vec<Container>,
}

impl Registry {
    pub fn print_all(self) {
        for c in self.containers {
            eprintln!("Container: {}", c.label)
            for item in c.items {
                eprintln!("  {}: {}", item.name, item.value)
            }
        }
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // self is &self (read-only), so containers are borrowed,
    // loop vars are borrowed, and c.items must also be borrowed
    assert!(
        generated.contains("fn print_all(&self)"),
        "Method should be &self.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("for item in &c.items"),
        "Inner for-loop should borrow c.items.\nGenerated:\n{}",
        generated
    );
}

/// A local variable from if-let that's borrowed should propagate borrow to its fields
#[test]
fn test_for_loop_borrows_field_of_option_binding() {
    let source = r#"
pub struct Member {
    pub name: String,
}

pub struct Squad {
    pub members: Vec<Member>,
}

pub struct Game {
    pub squad: Option<Squad>,
}

impl Game {
    pub fn list_members(self) {
        if let Some(squad) = self.squad {
            for member in squad.members {
                eprintln!("{}", member.name)
            }
        }
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // Method should be &self (only reads)
    assert!(
        generated.contains("fn list_members(&self)"),
        "Method should be &self.\nGenerated:\n{}",
        generated
    );
}
