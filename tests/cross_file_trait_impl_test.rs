use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

/// Helper to parse and generate Rust code from Windjammer source
#[allow(dead_code)]
fn parse_and_generate(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

/// Helper to parse and generate Rust code from multiple Windjammer files
/// This simulates the ACTUAL multi-file compilation process where each file
/// is analyzed separately, exposing the real bug
fn parse_and_generate_multi_file(files: &[(&str, &str)]) -> String {
    let mut analyzer = Analyzer::new();
    let mut all_analyzed_functions = Vec::new();
    let mut merged_registry = windjammer::analyzer::SignatureRegistry::new();
    let mut all_analyzed_trait_methods = std::collections::HashMap::new();

    // Parse and analyze each file SEPARATELY (like the real compiler does)
    for (_filename, source) in files {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        // Analyze this file's program with the SHARED analyzer
        // This ensures trait definitions from file 1 are available when analyzing impl in file 2
        let (analyzed_functions, registry, analyzed_trait_methods) =
            analyzer.analyze_program(&program).unwrap();

        all_analyzed_functions.extend(analyzed_functions);
        // Merge registries (note: SignatureRegistry doesn't have a merge method, so we'll just use the last one)
        merged_registry = registry;
        // Merge analyzed trait methods
        for (trait_name, methods) in analyzed_trait_methods {
            all_analyzed_trait_methods
                .entry(trait_name)
                .or_insert_with(std::collections::HashMap::new)
                .extend(methods);
        }
    }

    // For traits WITHOUT default implementations, add the analyzed trait methods to the analyzed functions
    // This ensures the trait signature uses the impl's inferred ownership
    for methods in all_analyzed_trait_methods.values() {
        for analyzed_method in methods.values() {
            all_analyzed_functions.push(analyzed_method.clone());
        }
    }

    // Generate code from the combined analysis
    // For simplicity, re-parse all files and generate
    let combined_source = files
        .iter()
        .map(|(_, source)| *source)
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut lexer = Lexer::new(&combined_source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    // CRITICAL: Call infer_trait_signatures_from_impls to finalize trait signatures
    // This is what the real compiler does in ModuleCompiler::finalize_trait_inference()
    analyzer
        .infer_trait_signatures_from_impls(&program)
        .unwrap();

    // Get the updated analyzed_trait_methods after inference
    let final_analyzed_trait_methods = analyzer.analyzed_trait_methods.clone();

    let mut generator = CodeGenerator::new_for_module(merged_registry, CompilationTarget::Rust);
    generator.set_analyzed_trait_methods(final_analyzed_trait_methods);
    generator.generate_program(&program, &all_analyzed_functions)
}

#[test]
fn test_cross_file_trait_impl_with_default() {
    // Trait in one file with default implementation
    let trait_file = r#"
    pub trait GameLoop {
        fn init(self) {
            // Default: do nothing
        }
        
        fn update(self, delta: f32) {
            // Default: do nothing  
        }
    }
    "#;

    // Impl in another file
    let impl_file = r#"
    use game_loop::GameLoop
    
    struct MyGame { score: i64 }
    
    impl GameLoop for MyGame {
        fn init(self) {
            println!("Initializing game")
        }
        
        fn update(self, delta: f32) {
            println!("Updating with delta:", delta)
        }
    }
    "#;

    let files = vec![("game_loop.wj", trait_file), ("game.wj", impl_file)];

    let output = parse_and_generate_multi_file(&files);
    println!("Generated Rust:\n{}", output);

    // The trait has default implementations with no mutations,
    // so analyzer should infer &self for all methods.
    // The impl MUST match the trait signature exactly.
    assert!(
        output.contains("fn init(&self)") && !output.contains("fn init(self)"),
        "Expected trait method 'init' to be &self (inferred from default impl), got:\n{}",
        output
    );

    assert!(
        output.contains("fn update(&self, delta: f32)"),
        "Expected trait method 'update' to be &self (inferred from default impl), got:\n{}",
        output
    );
}

#[test]
fn test_cross_file_trait_impl_no_default() {
    // Trait with no default implementation
    let trait_file = r#"
    pub trait Processor {
        fn process(self, data: string) -> i64
    }
    "#;

    // Impl in another file
    let impl_file = r#"
    use processor::Processor
    
    struct DataProcessor { multiplier: i64 }
    
    impl Processor for DataProcessor {
        fn process(self, data: string) -> i64 {
            return self.multiplier * data.len() as i64
        }
    }
    "#;

    let files = vec![("processor.wj", trait_file), ("impl.wj", impl_file)];

    let output = parse_and_generate_multi_file(&files);
    println!("Generated Rust:\n{}", output);

    // No default implementation, so trait has no body to analyze.
    // THE WINDJAMMER WAY: Explicit `string` type is honored as `String` (owned)
    // The impl uses `self` (owned) to consume the DataProcessor (Copy type).
    // The impl uses `data: string` which becomes `data: String` (explicit type honored).
    assert!(
        output.contains("fn process(self, data: String) -> i64"),
        "Expected impl to match trait signature with owned self and String data, got:\n{}",
        output
    );
}

#[test]
fn test_cross_file_trait_impl_with_mutation() {
    // Trait with default implementation that mutates
    let trait_file = r#"
    pub trait Counter {
        fn increment(mut self) {
            self.count = self.count + 1
        }
    }
    
    struct CounterData { count: i64 }
    "#;

    // Impl in another file
    let impl_file = r#"
    use counter::Counter
    use counter::CounterData
    
    impl Counter for CounterData {
        fn increment(mut self) {
            self.count = self.count + 2
        }
    }
    "#;

    let files = vec![("counter.wj", trait_file), ("impl.wj", impl_file)];

    let output = parse_and_generate_multi_file(&files);
    println!("Generated Rust:\n{}", output);

    // The trait default impl mutates self, so should infer &mut self.
    // The impl must match.
    assert!(
        output.contains("fn increment(&mut self)"),
        "Expected trait method 'increment' to be &mut self (inferred from mutation), got:\n{}",
        output
    );
}
