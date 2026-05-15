use windjammer::analyzer::{Analyzer, SignatureRegistry};
/// TDD Test: Reproduce runtime.wj missing 'use crate::ffi' bug
///
/// Bug: hot_reload.wj gets the import, runtime.wj doesn't
/// Both have identical patterns. What's the difference?
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

#[test]
fn test_runtime_wj_pattern_includes_ffi_import() {
    // Exact pattern from runtime.wj (simplified)
    let source = r#"
use crate::GameLoop
use crate::GameLoopConfig
use crate::ffi

pub struct GameRuntime {
    config: GameLoopConfig,
}

impl GameRuntime {
    pub fn new(config: GameLoopConfig) -> GameRuntime {
        GameRuntime { config }
    }
    
    pub fn run<G: GameLoop>(self, mut game: G) {
        ffi::run_with_event_loop(&mut game, &self.config.window_title, self.config.window_width, self.config.window_height)
    }
}
"#;

    // Parse
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");

    println!("Parsed {} items:", program.items.len());

    // Analyze
    let mut analyzer = Analyzer::new();
    let (analyzed_funcs, _, _) = analyzer.analyze_program(&program).expect("Analysis failed");

    // Generate
    let registry = SignatureRegistry::new();
    let mut generator =
        windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_funcs);

    println!("\nGenerated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: All three imports MUST be present
    assert!(
        rust_code.contains("use crate::GameLoop;"),
        "Missing: use crate::GameLoop;"
    );

    assert!(
        rust_code.contains("use crate::GameLoopConfig;"),
        "Missing: use crate::GameLoopConfig;"
    );

    assert!(
        rust_code.contains("use crate::ffi;"),
        "BUG REPRODUCED: Missing 'use crate::ffi;'\n\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_hot_reload_wj_pattern_includes_ffi_import() {
    // Exact pattern from hot_reload.wj (simplified)
    let source = r#"
use crate::ffi

pub enum AssetType {
    Texture,
    Audio,
    Config,
}

pub fn enable() {
    ffi::hot_reload::hot_reload_enable()
}
"#;

    // Parse
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");

    // Analyze
    let mut analyzer = Analyzer::new();
    let (analyzed_funcs, _, _) = analyzer.analyze_program(&program).expect("Analysis failed");

    // Generate
    let registry = SignatureRegistry::new();
    let mut generator =
        windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_funcs);

    println!("Generated Rust code:\n{}", rust_code);

    // This one should work
    assert!(
        rust_code.contains("use crate::ffi;"),
        "hot_reload should have 'use crate::ffi;'"
    );
}

#[test]
fn test_compare_runtime_vs_hot_reload() {
    // THE WINDJAMMER WAY: Find the difference!
    // What makes runtime.wj different from hot_reload.wj?

    let runtime_source = r#"
use crate::GameLoop
use crate::ffi

pub struct GameRuntime {}

impl GameRuntime {
    pub fn run<G: GameLoop>(self) {
        ffi::some_func()
    }
}
"#;

    let hot_reload_source = r#"
use crate::ffi

pub struct HotReload {}

pub fn enable() {
    ffi::some_func()
}
"#;

    for (name, source) in [
        ("runtime", runtime_source),
        ("hot_reload", hot_reload_source),
    ] {
        println!("\n=== Testing {} ===", name);

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parse failed");

        let mut analyzer = Analyzer::new();
        let (analyzed_funcs, _, _) = analyzer.analyze_program(&program).expect("Analysis failed");

        let registry = SignatureRegistry::new();
        let mut generator =
            windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);
        let rust_code = generator.generate_program(&program, &analyzed_funcs);

        println!("Generated:\n{}", rust_code);

        assert!(
            rust_code.contains("use crate::ffi;"),
            "{} should have 'use crate::ffi;'",
            name
        );
    }
}
