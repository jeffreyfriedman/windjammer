/// TDD test: Dependency Copy struct should NOT override local non-Copy struct
///
/// Bug: When a dependency crate's .wj.meta lists a struct as Copy
/// (e.g., `GameState { window: u64, ... }` in the engine), but the
/// current crate defines its own struct with the same name that is NOT
/// Copy (e.g., `GameState { items: Vec<...> }`), the dep Copy status
/// should NOT leak into the current crate.
///
/// Root cause: `dep_copy_structs` from `.wj.meta` files were
/// unconditionally added to `global_copy_structs` without checking if
/// the current crate has a non-Copy definition with the same name.
///
/// Impact: The analyzer thinks `GameState` is Copy, so it infers `Owned`
/// instead of `Borrowed` for parameters, causing E0382 (use of moved
/// value) when the parameter is used multiple times in a loop.
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(&wj_path, source).unwrap();

    let wj_binary = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(wj_binary)
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    std::fs::read_to_string(rs_path).unwrap_or_else(|_| {
        panic!(
            "No output generated. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn test_non_copy_struct_with_vec_gets_borrowed_param() {
    let source = r#"
struct State {
    values: Vec<i32>,
    name: string,
}

impl State {
    pub fn get_value(self) -> i32 {
        42
    }
}

pub enum Condition {
    Check(i32),
    Named(string),
}

impl Condition {
    pub fn evaluate(self, state: State) -> bool {
        match self {
            Condition::Check(v) => {
                state.get_value() >= v
            },
            Condition::Named(name) => {
                state.name == name
            },
        }
    }
}

struct Validator {
    conditions: Vec<Condition>,
}

impl Validator {
    pub fn all_pass(self, state: State) -> bool {
        for cond in self.conditions {
            if !cond.evaluate(state) {
                return false
            }
        }
        true
    }
}
"#;

    let generated = compile_wj(source);

    assert!(
        generated.contains("state: &State"),
        "evaluate should take state: &State (borrowed), but generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("state: State"),
        "state should NOT be owned (would cause E0382 in loop), but generated:\n{}",
        generated
    );
}
