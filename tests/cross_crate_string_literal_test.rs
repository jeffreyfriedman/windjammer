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

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_dir() -> std::path::PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("wj_xcrate_str_{pid}_{id}"))
}

fn wj_bin() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/wj")
}

/// Simulate cross-crate metadata scenario:
/// An external crate defines SystemCoverage::register_function(self, name: String) -> usize.
/// The game code calls sys.register_function("fire") - should get .to_string().
///
/// We do this by:
/// 1. Creating a metadata.json with the signature
/// 2. Compiling a .wj file that calls the method with --metadata flag
#[test]
fn test_cross_crate_string_literal_with_metadata() {
    let dir = unique_dir();
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Create a Cargo.toml so the compiler recognizes the project root
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();

    // Create metadata.json simulating an external crate with SystemCoverage
    let metadata = r#"{
        "structs": {
            "SystemCoverage": {
                "name": "String"
            }
        },
        "functions": {
            "SystemCoverage::new": {
                "params": ["String"],
                "return_type": "Custom(\"SystemCoverage\")",
                "is_associated": true,
                "parent_type": "SystemCoverage",
                "param_ownership": ["Owned"],
                "has_self_receiver": false,
                "is_extern": false
            },
            "SystemCoverage::register_function": {
                "params": ["Custom(\"Self\")", "String"],
                "return_type": "Custom(\"usize\")",
                "is_associated": true,
                "parent_type": "SystemCoverage",
                "param_ownership": ["MutBorrowed", "Owned"],
                "has_self_receiver": true,
                "is_extern": false
            }
        },
        "version": "0.46.2"
    }"#;

    let engine_dir = dir.join("engine");
    std::fs::create_dir_all(&engine_dir).unwrap();
    let meta_file = engine_dir.join("metadata.json");
    std::fs::write(&meta_file, metadata).unwrap();

    // Game code that uses SystemCoverage from the "engine" crate
    let source = r#"
use engine::testing::coverage::SystemCoverage

pub fn validate_combat() {
    let mut sys = SystemCoverage::new("combat")
    let fire = sys.register_function("fire")
    let reload = sys.register_function("reload")
}
"#;
    std::fs::write(src_dir.join("test.wj"), source).unwrap();

    // Build with --metadata pointing to our fake engine metadata
    let output_dir = dir.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let output = Command::new(wj_bin())
        .arg("build")
        .arg(src_dir.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg("--library")
        .arg("--no-cargo")
        .arg("--metadata")
        .arg(format!("engine={}", meta_file.display()))
        .current_dir(&dir)
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("STDERR: {}", stderr);
    eprintln!("STDOUT: {}", stdout);

    // Find the generated .rs file
    let rs_file = output_dir.join("test.rs");
    let generated = if rs_file.exists() {
        std::fs::read_to_string(&rs_file).unwrap_or_default()
    } else {
        // Try other common locations
        let alt = dir.join("build/test.rs");
        if alt.exists() {
            std::fs::read_to_string(&alt).unwrap_or_default()
        } else {
            panic!(
                "Generated .rs file not found at {:?} or {:?}\nFiles in output: {:?}",
                rs_file,
                alt,
                std::fs::read_dir(&output_dir)
                    .map(|d| d
                        .filter_map(|e| e.ok().map(|e| e.path()))
                        .collect::<Vec<_>>())
                    .unwrap_or_default()
            );
        }
    };

    eprintln!("GENERATED:\n{}", generated);

    // The string literal "fire" must be converted to "fire".to_string()
    // because SystemCoverage::register_function expects Owned String
    assert!(
        generated.contains(r#""fire".to_string()"#),
        "String literal \"fire\" should be converted to .to_string() for cross-crate method expecting owned String.\nGenerated:\n{}",
        generated
    );

    assert!(
        generated.contains(r#""reload".to_string()"#),
        "String literal \"reload\" should also be converted.\nGenerated:\n{}",
        generated
    );

    let _ = std::fs::remove_dir_all(&dir);
}
