//! Backend Integration Conformance Tests
//!
//! Verifies that Windjammer programs produce identical output across all backends:
//! - Rust (compiled via wj build → rustc)
//! - Go (compiled via wj build → go run)
//! - JavaScript (compiled via wj build → node)
//! - Interpreter (Windjammerscript tree-walking)
//!
//! WGSL is excluded: shader-only target, no main()/println.
//!
//! Test cases live in test_cases/*.wj and cover:
//! - basic: variables, functions, structs, enums, control flow
//! - ownership: copy semantics, mutation, parameter inference
//! - patterns: match, match guards, if let
//! - traits: trait definitions, implementations, generics
//! - strings: literals, concatenation, interpolation
//! - collections: Vec push, len, indexing, iteration

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Result of compiling and running a .wj file on a specific backend
struct BackendResult {
    backend: String,
    stdout: String,
    success: bool,
    error: String,
}

fn compile_and_run_rust(source: &str, test_name: &str) -> BackendResult {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join(format!("{}.wj", test_name));
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

    let expected_rs = output_dir.join(format!("{}.rs", test_name));
    let rs_file = if expected_rs.exists() {
        expected_rs
    } else {
        fs::read_dir(&output_dir)
            .ok()
            .and_then(|dir| {
                dir.filter_map(|e| e.ok())
                    .find(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
                    .map(|e| e.path())
            })
            .unwrap_or_else(|| {
                let entries: Vec<String> = fs::read_dir(&output_dir)
                    .map(|d| {
                        d.filter_map(|e| e.ok())
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                panic!("No .rs file in {:?}. Files: {:?}", output_dir, entries);
            })
    };

    let bin_path = temp_dir.path().join("bin");
    let rustc = Command::new("rustc")
        .arg(&rs_file)
        .arg("-o")
        .arg(&bin_path)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to execute rustc");

    if !rustc.status.success() {
        return BackendResult {
            backend: "rust".into(),
            stdout: String::new(),
            success: false,
            error: format!(
                "rustc failed: {}",
                String::from_utf8_lossy(&rustc.stderr)
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

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn compile_and_run_go(source: &str, test_name: &str) -> Option<BackendResult> {
    if !go_available() {
        return None;
    }

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join(format!("{}.wj", test_name));
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
    if !go_file.exists() {
        let entries: Vec<String> = fs::read_dir(&output_dir)
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        return Some(BackendResult {
            backend: "go".into(),
            stdout: String::new(),
            success: false,
            error: format!("No main.go found. Files: {:?}", entries),
        });
    }

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

fn compile_and_run_js(source: &str, test_name: &str) -> BackendResult {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join(format!("{}.wj", test_name));
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

    let runner = output_dir.join("_run.mjs");
    fs::write(&runner, format!("{}\nif (typeof main === 'function') main();\n", stripped)).unwrap();

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

fn interpret(source: &str, test_name: &str) -> BackendResult {
    let mut lex = windjammer::lexer::Lexer::new(source);
    let tokens = lex.tokenize_with_locations();
    let mut parser = windjammer::parser::Parser::new_with_source(
        tokens,
        format!("{}.wj", test_name),
        source.to_string(),
    );
    let program = match parser.parse() {
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

/// Run a test case across all backends, assert identical output
fn run_integration_test(test_name: &str, source: &str) {
    let rust_result = compile_and_run_rust(source, test_name);
    let go_result = compile_and_run_go(source, test_name);
    let js_result = compile_and_run_js(source, test_name);
    let interp_result = interpret(source, test_name);

    assert!(
        rust_result.success,
        "[{}] Rust failed: {}",
        test_name,
        rust_result.error
    );
    assert!(
        rust_result.stdout.contains("PASSED"),
        "[{}] Rust output missing PASSED: {}",
        test_name,
        rust_result.stdout
    );

    if let Some(ref go) = go_result {
        assert!(
            go.success,
            "[{}] Go failed: {}",
            test_name,
            go.error
        );
        assert_eq!(
            rust_result.stdout,
            go.stdout,
            "[{}] Rust vs Go mismatch",
            test_name
        );
    }

    assert!(
        js_result.success,
        "[{}] JS failed: {}",
        test_name,
        js_result.error
    );
    assert_eq!(
        rust_result.stdout,
        js_result.stdout,
        "[{}] Rust vs JS mismatch",
        test_name
    );

    assert!(
        interp_result.success,
        "[{}] Interpreter failed: {}",
        test_name,
        interp_result.error
    );
    assert_eq!(
        rust_result.stdout,
        interp_result.stdout,
        "[{}] Rust vs Interpreter mismatch",
        test_name
    );
}

fn load_test_case(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("test_cases")
        .join(format!("{}.wj", name));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
}

// ========== Integration Tests ==========

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_integration_basic() {
    run_integration_test("basic", &load_test_case("basic"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_integration_ownership() {
    run_integration_test("ownership", &load_test_case("ownership"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_integration_patterns() {
    run_integration_test("patterns", &load_test_case("patterns"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_integration_strings() {
    run_integration_test("strings", &load_test_case("strings"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_integration_collections() {
    run_integration_test("collections", &load_test_case("collections"));
}

/// Traits test: Rust + Interpreter only (Go/JS may not support full trait system yet)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_integration_traits() {
    let source = load_test_case("traits");
    let rust_result = compile_and_run_rust(&source, "traits");
    let interp_result = interpret(&source, "traits");

    assert!(
        rust_result.success,
        "[traits] Rust failed: {}",
        rust_result.error
    );
    assert!(
        interp_result.success,
        "[traits] Interpreter failed: {}",
        interp_result.error
    );
    assert_eq!(
        rust_result.stdout,
        interp_result.stdout,
        "[traits] Rust vs Interpreter mismatch"
    );
    assert!(
        rust_result.stdout.contains("PASSED"),
        "[traits] Output missing PASSED"
    );
}
