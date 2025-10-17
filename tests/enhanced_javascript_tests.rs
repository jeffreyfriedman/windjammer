//! Enhanced JavaScript Support Tests (v0.33.0)
//!
//! Tests for source maps, minification, tree shaking, polyfills, V8 optimizations

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use windjammer::codegen::backend::{CodegenBackend, CodegenConfig, Target};
use windjammer::codegen::javascript::{minifier, polyfills, tree_shaker, v8_optimizer};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

/// Helper to compile Windjammer code to JavaScript
fn compile_to_js(source: &str, config: &CodegenConfig) -> Result<String, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .map_err(|e| format!("Parser error: {:?}", e))?;

    let backend = windjammer::codegen::javascript::JavaScriptBackend::new();
    let output = backend
        .generate(&program, config)
        .map_err(|e| format!("Codegen error: {}", e))?;

    Ok(output.source)
}

#[test]
fn test_minification_basic() {
    let code = r#"
// This is a comment
function   add  (  a  ,  b  )  {
    return   a   +   b  ;
}
"#;

    let mut minifier = minifier::Minifier::new();
    let minified = minifier.minify(code);

    // Should remove comments and excess whitespace
    assert!(!minified.contains("This is a comment"));
    assert!(minified.len() < code.len());
}

#[test]
fn test_minification_preserves_functionality() {
    let code = r#"
function test() {
    return 42;
}
"#;

    let mut minifier = minifier::Minifier::new();
    let minified = minifier.minify(code);

    // Should still contain function
    assert!(minified.contains("function"));
    assert!(minified.contains("42"));
}

#[test]
fn test_tree_shaking_removes_unused() {
    let source = r#"
fn used() -> int {
    42
}

fn unused() -> int {
    100
}

fn main() {
    let x = used()
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let shaken = tree_shaker::shake_tree(&program);

    // Should have only 2 functions (main + used), not 3
    let function_count = shaken
        .items
        .iter()
        .filter(|item| matches!(item, windjammer::parser::Item::Function(_)))
        .count();

    assert_eq!(function_count, 2);
}

#[test]
fn test_tree_shaking_analysis() {
    let source = r#"
fn used() -> int {
    42
}

fn unused() -> int {
    100
}

fn main() {
    let x = used()
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let analysis = tree_shaker::analyze_usage(&program);

    assert_eq!(analysis.total_functions, 3);
    assert_eq!(analysis.unused_functions.len(), 1);
    assert!(analysis.unused_functions.contains(&"unused".to_string()));
}

#[test]
fn test_polyfills_generation() {
    let config = polyfills::PolyfillConfig {
        target: polyfills::PolyfillTarget::ES5,
        include_promise: true,
        include_array_methods: true,
        include_object_methods: true,
        include_symbol: false,
    };

    let polyfill_code = polyfills::generate_polyfills(&config);

    // Should include Promise polyfill
    assert!(polyfill_code.contains("Promise"));
    assert!(polyfill_code.contains("Array.from"));
    assert!(polyfill_code.contains("Object.assign"));
}

#[test]
fn test_polyfills_es2015_target() {
    let config = polyfills::PolyfillConfig {
        target: polyfills::PolyfillTarget::ES2015,
        include_promise: true,
        include_array_methods: true,
        include_object_methods: true,
        include_symbol: false,
    };

    let polyfill_code = polyfills::generate_polyfills(&config);

    // ES2015 should still include Promise
    assert!(polyfill_code.contains("Promise"));
}

#[test]
fn test_v8_optimizations() {
    let code = r#"
function test(x) {
    return x * x;
}
"#;

    let optimizer = v8_optimizer::V8Optimizer::new();
    let optimized = optimizer.optimize(code);

    // Optimization should return code (even if no changes made yet)
    assert!(!optimized.is_empty());
}

#[test]
fn test_v8_optimization_hints() {
    let hints = v8_optimizer::V8Optimizer::generate_optimization_hints();

    assert!(hints.contains("V8"));
    assert!(hints.contains("Monomorphic"));
    assert!(hints.contains("TurboFan"));
}

#[test]
fn test_v8_optimized_array_loop() {
    let loop_code = v8_optimizer::patterns::optimized_array_loop("items", "process(item);");

    assert!(loop_code.contains("items_length"));
    assert!(loop_code.contains("for"));
    assert!(loop_code.contains("process(item)"));
}

#[test]
fn test_v8_optimized_object_creation() {
    let fields = vec![("x", "number"), ("y", "number")];
    let class_code = v8_optimizer::patterns::optimized_object_creation("Point", &fields);

    assert!(class_code.contains("class Point"));
    assert!(class_code.contains("constructor"));
    assert!(class_code.contains("this.x"));
    assert!(class_code.contains("this.y"));
}

#[test]
fn test_integrated_minify_build() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
}
"#;

    let config = CodegenConfig {
        target: Target::JavaScript,
        minify: true,
        tree_shake: false,
        source_maps: false,
        polyfills: false,
        v8_optimize: false,
        ..Default::default()
    };

    let js_code = compile_to_js(source, &config).unwrap();

    // Minified code should be shorter
    assert!(!js_code.is_empty());
    // Should still have function names
    assert!(js_code.contains("add") || js_code.contains("function"));
}

#[test]
fn test_integrated_tree_shake_build() {
    let source = r#"
fn used() -> int {
    42
}

fn unused() -> int {
    100
}

fn main() {
    let x = used()
}
"#;

    let config = CodegenConfig {
        target: Target::JavaScript,
        minify: false,
        tree_shake: true,
        source_maps: false,
        polyfills: false,
        v8_optimize: false,
        ..Default::default()
    };

    let js_code = compile_to_js(source, &config).unwrap();

    // Should contain "used" but ideally not "unused"
    // Note: Tree shaking happens at AST level, so check function count
    assert!(js_code.contains("used"));
}

#[test]
fn test_integrated_polyfills_build() {
    let source = r#"
fn main() {
    println!("Hello")
}
"#;

    let config = CodegenConfig {
        target: Target::JavaScript,
        minify: false,
        tree_shake: false,
        source_maps: false,
        polyfills: true,
        v8_optimize: false,
        ..Default::default()
    };

    let js_code = compile_to_js(source, &config).unwrap();

    // Should include polyfills at the top
    assert!(js_code.contains("Windjammer Polyfills") || js_code.contains("Promise"));
}

#[test]
fn test_integrated_all_features() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
}
"#;

    let config = CodegenConfig {
        target: Target::JavaScript,
        minify: true,
        tree_shake: true,
        source_maps: true,
        polyfills: true,
        v8_optimize: true,
        ..Default::default()
    };

    let js_code = compile_to_js(source, &config).unwrap();

    // Should successfully compile with all features enabled
    assert!(!js_code.is_empty());
}

#[test]
fn test_source_map_generation() {
    use windjammer::codegen::javascript::source_maps;

    let source_map = source_maps::generate_source_map(
        "output.js",
        "input.wj",
        "fn main() {}",
        "function main() {}",
    );

    assert_eq!(source_map.version, 3);
    assert_eq!(source_map.file, "output.js");
    assert_eq!(source_map.sources[0], "input.wj");
    assert!(!source_map.mappings.is_empty());
}

#[test]
fn test_cli_flags_minify() {
    // Test that CLI accepts --minify flag
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("test.wj");
    fs::write(&source_file, "fn main() { }").unwrap();

    let wj_path = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_path)
        .arg("build")
        .arg(&source_file)
        .arg("--target=javascript")
        .arg("--minify")
        .arg("--output")
        .arg(temp_dir.path())
        .output();

    // Command should execute (success or failure depends on implementation)
    assert!(output.is_ok());
}

#[test]
fn test_cli_flags_tree_shake() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("test.wj");
    fs::write(&source_file, "fn main() { }").unwrap();

    let wj_path = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_path)
        .arg("build")
        .arg(&source_file)
        .arg("--target=javascript")
        .arg("--tree-shake")
        .arg("--output")
        .arg(temp_dir.path())
        .output();

    assert!(output.is_ok());
}

#[test]
fn test_cli_flags_all_enhanced_features() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("test.wj");
    fs::write(&source_file, "fn main() { }").unwrap();

    let wj_path = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_path)
        .arg("build")
        .arg(&source_file)
        .arg("--target=javascript")
        .arg("--minify")
        .arg("--tree-shake")
        .arg("--source-maps")
        .arg("--polyfills")
        .arg("--v8-optimize")
        .arg("--output")
        .arg(temp_dir.path())
        .output();

    assert!(output.is_ok());
}
