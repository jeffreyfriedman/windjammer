#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_import_with_as_alias() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_import_alias");

    fs::create_dir_all(&test_dir).unwrap();

    // Test that imports with "as" aliases are preserved.
    // NOTE: Avoid aliasing to "Map" since Windjammer stdlib maps Map → HashMap.
    let test_content = r#"
use std::collections::HashMap as HMap;

fn main() {
    let _m: HMap<i32, i32> = HMap::new();
}
"#;

    let test_file = test_dir.join("import_alias.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join("import_alias.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should preserve the alias
    assert!(
        rust_code.contains("use std::collections::HashMap as HMap;"),
        "Expected import alias to be preserved.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let build_dir = test_dir.join("build");
    let compile_output = Command::new("rustc")
        .current_dir(&build_dir)
        .arg("--crate-type")
        .arg("bin")
        .arg("--out-dir")
        .arg(&build_dir)
        .arg("-o")
        .arg(build_dir.join("import_alias_bin"))
        .arg("import_alias.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile.\nRustc errors:\n{}",
        compile_stderr
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_import_alias_overrides_stdlib_mapping() {
    // When user writes `use std::collections::HashMap as Map`, the codegen must
    // preserve `Map` in the body, NOT apply the Windjammer stdlib Map → HashMap mapping.
    use windjammer::analyzer::Analyzer;
    use windjammer::codegen::rust::CodeGenerator;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;
    use windjammer::CompilationTarget;

    let source = r#"
use std::collections::HashMap as Map

fn test() {
    let m: Map<i32, i32> = Map::new()
}
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, _structs, _trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let registry = windjammer::analyzer::SignatureRegistry::new();
    let mut generator = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    let output = generator.generate_program(&program, &analyzed_functions);

    assert!(
        output.contains("Map<i32, i32>"),
        "Import alias should override stdlib Map→HashMap mapping in type annotations. Got:\n{}",
        output
    );
    assert!(
        output.contains("Map::new()"),
        "Import alias should override stdlib Map→HashMap mapping in path expressions. Got:\n{}",
        output
    );
}
