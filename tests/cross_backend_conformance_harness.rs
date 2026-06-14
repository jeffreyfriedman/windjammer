#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "conformance_tests",
))]

//! Shared harness for cross-backend conformance crates in this directory.

#![allow(dead_code)] // Helpers used only when Go/JS backends are exercised in CI
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[allow(dead_code)]
pub struct BackendResult {
    pub backend: String,
    pub stdout: String,
    pub success: bool,
    pub error: String,
}

pub fn compile_and_run_rust(source: &str) -> BackendResult {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    if !wj.status.success() {
        return BackendResult {
            backend: "rust".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "wj compilation failed: {}",
                String::from_utf8_lossy(&wj.stderr)
            ),
        };
    }

    let rs_file = output_dir.join("test.rs");
    if !rs_file.exists() {
        let entries: Vec<String> = fs::read_dir(&output_dir)
            .map(|dir| {
                dir.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        return BackendResult {
            backend: "rust".into(),
            stdout: String::new(),
            success: false,
            error: format!("No test.rs found. Files in build/: {:?}", entries),
        };
    }

    let bin_path = temp_dir.path().join("test_bin");
    let rustc = Command::new("rustc")
        .arg(&rs_file)
        .arg("-o")
        .arg(&bin_path)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to execute rustc");

    if !rustc.status.success() {
        let rs_code = fs::read_to_string(&rs_file).unwrap_or_default();
        return BackendResult {
            backend: "rust".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "rustc failed: {}\n\nGenerated Rust:\n{}",
                String::from_utf8_lossy(&rustc.stderr),
                rs_code
            ),
        };
    }

    let run = Command::new(&bin_path)
        .output()
        .expect("Failed to run binary");

    BackendResult {
        backend: "rust".into(),
        stdout: String::from_utf8(run.stdout).unwrap_or_default(),
        success: run.status.success(),
        error: String::from_utf8(run.stderr).unwrap_or_default(),
    }
}

fn go_is_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn compile_and_run_go(source: &str) -> Option<BackendResult> {
    if !go_is_available() {
        return None;
    }

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("go")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    if !wj.status.success() {
        return Some(BackendResult {
            backend: "go".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "wj compilation failed: {}",
                String::from_utf8_lossy(&wj.stderr)
            ),
        });
    }

    let go_file = output_dir.join("main.go");
    let run = Command::new("go")
        .arg("run")
        .arg(&go_file)
        .current_dir(&output_dir)
        .output()
        .expect("Failed to execute go run");

    Some(BackendResult {
        backend: "go".into(),
        stdout: String::from_utf8(run.stdout).unwrap_or_default(),
        success: run.status.success(),
        error: String::from_utf8(run.stderr).unwrap_or_default(),
    })
}

pub fn compile_and_run_js(source: &str) -> BackendResult {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let wj = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("javascript")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    if !wj.status.success() {
        return BackendResult {
            backend: "js".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "wj compilation failed: {}",
                String::from_utf8_lossy(&wj.stderr)
            ),
        };
    }

    let js_file = output_dir.join("output.js");
    if !js_file.exists() {
        return BackendResult {
            backend: "js".into(),
            stdout: String::new(),
            success: false,
            error: "No output.js generated".into(),
        };
    }

    let js_code = fs::read_to_string(&js_file).unwrap();
    let runner = output_dir.join("_run.mjs");

    let stripped = js_code
        .replace("export function", "function")
        .replace("export class", "class")
        .replace("export const", "const")
        .replace("export let", "let");

    let stripped = if let Some(idx) = stripped.find("// Auto-run main") {
        stripped[..idx].to_string()
    } else {
        stripped
    };

    let runner_code = format!("{}\nif (typeof main === 'function') main();\n", stripped);
    fs::write(&runner, &runner_code).unwrap();

    let run = Command::new("node")
        .arg(&runner)
        .current_dir(&output_dir)
        .output()
        .expect("Failed to execute node");

    BackendResult {
        backend: "js".into(),
        stdout: String::from_utf8(run.stdout).unwrap_or_default(),
        success: run.status.success(),
        error: String::from_utf8(run.stderr).unwrap_or_default(),
    }
}

pub fn interpret(source: &str) -> BackendResult {
    let mut lex = windjammer::lexer::Lexer::new(source);
    let tokens = lex.tokenize_with_locations();
    let mut parse = windjammer::parser::Parser::new_with_source(
        tokens,
        "conformance_test.wj".to_string(),
        source.to_string(),
    );
    let program = match parse.parse() {
        Ok(p) => p,
        Err(e) => {
            return BackendResult {
                backend: "interpreter".into(),
                stdout: String::new(),
                success: false,
                error: format!("Parse error: {}", e),
            };
        }
    };

    let mut interp = windjammer::interpreter::Interpreter::new_capturing();
    match interp.run(&program) {
        Ok(_) => BackendResult {
            backend: "interpreter".into(),
            stdout: interp.get_output(),
            success: true,
            error: String::new(),
        },
        Err(e) => BackendResult {
            backend: "interpreter".into(),
            stdout: interp.get_output(),
            success: false,
            error: e,
        },
    }
}

#[allow(dead_code)] // Used by opt-in conformance modules
pub fn assert_rust_and_interpreter_agree(test_name: &str, source: &str, expected_contains: &str) {
    let rust_result = compile_and_run_rust(source);
    let interp_result = interpret(source);

    assert!(
        rust_result.success,
        "[{}] Rust backend failed: {}",
        test_name, rust_result.error
    );
    assert!(
        interp_result.success,
        "[{}] Interpreter failed: {}",
        test_name, interp_result.error
    );

    assert!(
        rust_result.stdout.contains(expected_contains),
        "[{}] Rust output missing '{}'. Got:\n{}",
        test_name,
        expected_contains,
        rust_result.stdout
    );

    assert_eq!(
        rust_result.stdout, interp_result.stdout,
        "[{}] Rust vs Interpreter output mismatch!\nRust:\n{}\nInterpreter:\n{}",
        test_name, rust_result.stdout, interp_result.stdout
    );
}

pub fn assert_backends_agree(test_name: &str, source: &str, expected_contains: &str) {
    let rust_result = compile_and_run_rust(source);
    let go_result = compile_and_run_go(source);
    let js_result = compile_and_run_js(source);
    let interp_result = interpret(source);

    assert!(
        rust_result.success,
        "[{}] Rust backend failed: {}",
        test_name, rust_result.error
    );
    if let Some(ref go) = go_result {
        assert!(
            go.success,
            "[{}] Go backend failed: {}",
            test_name, go.error
        );
    }
    assert!(
        js_result.success,
        "[{}] JS backend failed: {}",
        test_name, js_result.error
    );
    assert!(
        interp_result.success,
        "[{}] Interpreter failed: {}",
        test_name, interp_result.error
    );

    assert!(
        rust_result.stdout.contains(expected_contains),
        "[{}] Rust output missing '{}'. Got:\n{}",
        test_name,
        expected_contains,
        rust_result.stdout
    );

    if let Some(ref go) = go_result {
        assert_eq!(
            rust_result.stdout, go.stdout,
            "[{}] Rust vs Go output mismatch!\nRust:\n{}\nGo:\n{}",
            test_name, rust_result.stdout, go.stdout
        );
    }

    assert_eq!(
        rust_result.stdout, js_result.stdout,
        "[{}] Rust vs JS output mismatch!\nRust:\n{}\nJS:\n{}",
        test_name, rust_result.stdout, js_result.stdout
    );

    assert_eq!(
        rust_result.stdout, interp_result.stdout,
        "[{}] Rust vs Interpreter output mismatch!\nRust:\n{}\nInterpreter:\n{}",
        test_name, rust_result.stdout, interp_result.stdout
    );
}
