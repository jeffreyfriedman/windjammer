// TDD Test: Generated Rust should prefix unused variables with `_` to suppress warnings
//
// Bug: When a Windjammer function parameter or let binding is declared but never used
// in the function body, the generated Rust code triggers "unused variable" warnings.
// The Windjammer philosophy says "compiler does the hard work" — the compiler should
// automatically prefix unused variables with `_` in the generated Rust code.
//
// Root Cause: The codegen emits variable names verbatim from the Windjammer source.
// It doesn't analyze whether variables are actually used in the function body.
//
// Fix: During code generation, check if each function parameter and let binding is
// referenced anywhere in the function body. If not, prefix the name with `_`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_check_warnings(code: &str) -> (bool, String, Vec<String>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return (
            false,
            format!(
                "Compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            vec![],
        );
    }

    let generated_path = out_dir.join("test.rs");
    let generated =
        fs::read_to_string(&generated_path).unwrap_or_else(|e| format!("Read error: {}", e));

    let rustc = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc {
        Ok(rustc_output) => {
            let stderr = String::from_utf8_lossy(&rustc_output.stderr).to_string();
            let unused_warnings: Vec<String> = stderr
                .lines()
                .filter(|l| l.contains("unused variable"))
                .map(|l| l.to_string())
                .collect();
            (rustc_output.status.success(), generated, unused_warnings)
        }
        Err(e) => (
            false,
            generated,
            vec![format!("Failed to run rustc: {}", e)],
        ),
    }
}

#[test]
fn test_unused_function_param_gets_underscore_prefix() {
    // THE BUG: When a function parameter is never used in the body,
    // the generated Rust triggers "unused variable: `dt`" warnings.
    // This is the exact pattern from windjammer-game's update functions:
    //   fn update(self, dt: f64, game_state: GameState) { ... only uses self ... }
    let (ok, generated, warnings) = compile_and_check_warnings(
        r#"
struct GameState {
    score: i64,
}

struct Enemy {
    health: i64,
    x: f64,
    y: f64,
}

impl Enemy {
    fn update(self, dt: f64, game_state: GameState) {
        self.health = self.health - 1
    }

    fn render(self, ctx: i64, camera_x: f64, camera_y: f64) {
        println("{}", self.health)
    }
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !warnings.is_empty() {
        println!("Unused variable warnings:\n{}", warnings.join("\n"));
    }

    assert!(ok, "Generated Rust should compile");
    assert!(
        warnings.is_empty(),
        "Should have no 'unused variable' warnings. Unused params should be prefixed with `_`.\nWarnings:\n{}\nGenerated:\n{}",
        warnings.join("\n"),
        generated
    );
}

#[test]
fn test_used_params_keep_original_name() {
    // IMPORTANT: Used parameters must NOT be prefixed with `_`
    // because `_x` in Rust means "I know this is unused" and also
    // changes drop timing for non-Copy types.
    let (ok, generated, warnings) = compile_and_check_warnings(
        r#"
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn greet(name: String) {
    println("Hello, {}", name)
}
"#,
    );

    println!("Generated:\n{}", generated);

    assert!(ok, "Generated Rust should compile");
    assert!(
        warnings.is_empty(),
        "Should have no warnings for used params.\nWarnings:\n{}\nGenerated:\n{}",
        warnings.join("\n"),
        generated
    );
    // Verify the names are NOT prefixed
    assert!(
        !generated.contains("_a:") && !generated.contains("_b:"),
        "Used parameters should NOT be prefixed with `_`.\nGenerated:\n{}",
        generated
    );
}
