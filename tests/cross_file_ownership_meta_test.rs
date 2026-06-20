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

//! TDD: Cross-file ownership via signature registry (same as loading from `.wj.meta` at compile time).
//! When file A defines a function with a borrowed parameter, file B's codegen should insert `&`
//! at the call site when that signature is merged before analysis.

use windjammer::analyzer::{Analyzer, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_with_external_sigs(source: &str, external_sigs: &SignatureRegistry) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_fns, registry, _) = analyzer
        .analyze_program_with_global_signatures(&program, external_sigs)
        .unwrap();
    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed_fns)
}

#[test]
fn test_cross_file_borrowed_param_gets_ampersand() {
    // File A: defines a function with a read-only Vec parameter (inferred as borrowed for Rust)
    let file_a_source = r#"
struct AABB {
    min_x: f32,
    max_x: f32,
}

fn check_collisions(walls: Vec<AABB>) -> bool {
    let mut i = 0
    while i < walls.len() {
        if walls[i].min_x > 0.0 {
            return true
        }
        i = i + 1
    }
    false
}
"#;

    let mut lexer_a = Lexer::new(file_a_source);
    let tokens_a = lexer_a.tokenize_with_locations();
    let parser_a = Box::leak(Box::new(Parser::new(tokens_a)));
    let program_a = parser_a.parse().unwrap();
    let mut analyzer_a = Analyzer::new();
    let (_, registry_a, _) = analyzer_a.analyze_program(&program_a).unwrap();

    let sig = registry_a.get_signature("check_collisions").unwrap();
    assert_eq!(
        sig.param_ownership[0],
        OwnershipMode::Borrowed,
        "walls should be inferred as Borrowed"
    );

    // File B: calls check_collisions only (callee lives in external registry)
    let file_b_source = r#"
struct AABB {
    min_x: f32,
    max_x: f32,
}

fn get_walls() -> Vec<AABB> {
    Vec::new()
}

fn game_update() {
    let walls = get_walls()
    let result = check_collisions(walls)
}
"#;

    let code = compile_with_external_sigs(file_b_source, &registry_a);

    assert!(
        code.contains("check_collisions(&walls)"),
        "Cross-file call should insert & for borrowed parameter. Got:\n{}",
        code
    );
}

/// Single-file library `metadata.json` must export non-empty `param_ownership` for static
/// methods with owned formal parameters (skeleton defaults + post-codegen refresh).
#[test]
fn test_single_file_library_metadata_static_method_owned_param_ownership() {
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).expect("mkdir");

    fs::write(
        src.join("mesh.wj"),
        r#"
pub struct MannequinConfig { pub torso_height: f32 }

pub struct MannequinMesh { tag: i32 }

impl MannequinMesh {
    pub fn generate(config: MannequinConfig) -> MannequinMesh {
        MannequinMesh { tag: 1 }
    }
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("gen");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mesh.wj").to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata_path = out.join("metadata.json");
    let metadata = fs::read_to_string(&metadata_path).unwrap_or_else(|_| {
        panic!(
            "metadata.json missing at {}. stderr:\n{}",
            metadata_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    assert!(
        metadata.contains("MannequinMesh::generate"),
        "metadata should list MannequinMesh::generate. Got:\n{}",
        metadata
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&metadata).expect("metadata.json should be valid JSON");
    let sig = parsed["functions"]["MannequinMesh::generate"]
        .as_object()
        .expect("MannequinMesh::generate entry");
    let ownership = sig["param_ownership"]
        .as_array()
        .expect("param_ownership array");
    assert!(
        !ownership.is_empty(),
        "param_ownership must not be empty for static method with owned formal. Got:\n{}",
        metadata
    );
    assert_eq!(
        ownership[0].as_str(),
        Some("Owned"),
        "owned MannequinConfig formal should export Owned param_ownership. Got:\n{}",
        metadata
    );
}
