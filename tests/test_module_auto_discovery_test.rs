#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! TDD: `*_test.wj` modules auto-discover with `#[cfg(test)]` even when mod.wj lists explicit pub mod names.

use std::fs;
use std::process::Command;

fn compile_project(files: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_dir = temp_dir.path().join("src");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    for (path, content) in files {
        let full_path = src_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, content).unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    assert!(
        output.status.success(),
        "compile failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut result = std::collections::HashMap::new();
    fn read_dir_recursive(
        dir: &std::path::Path,
        base: &std::path::Path,
        result: &mut std::collections::HashMap<String, String>,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    read_dir_recursive(&path, base, result);
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    let rel = path
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    let content = fs::read_to_string(&path).unwrap();
                    result.insert(rel, content);
                }
            }
        }
    }
    read_dir_recursive(&out_dir, &out_dir, &mut result);
    result
}

#[test]
fn test_auto_discovers_test_module_without_mod_wj_entry() {
    let files = &[
        (
            "weather/mod.wj",
            "pub mod weather_type\npub mod weather_system\n",
        ),
        ("weather/weather_type.wj", "pub enum WeatherType { Clear }\n"),
        ("weather/weather_system.wj", "pub struct WeatherSystem {}\n"),
        (
            "weather/weather_system_test.wj",
            "pub fn test_weather_defaults() { assert(true, \"ok\") }\n",
        ),
    ];

    let output = compile_project(files);
    let mod_rs = output
        .get("weather/mod.rs")
        .expect("weather/mod.rs should exist");

    assert!(
        mod_rs.contains("#[cfg(test)]") && mod_rs.contains("pub mod weather_system_test;"),
        "Expected auto-discovered test module with cfg(test). Got:\n{}",
        mod_rs
    );
    assert!(
        !mod_rs.contains("pub use weather_system_test"),
        "Test modules must not be glob re-exported. Got:\n{}",
        mod_rs
    );
}
