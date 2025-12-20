// TDD Test: Trait impl methods must match trait method signatures
// Bug: Impl methods generate `self` when trait has `&self`
// Expected: Impl methods should match trait ownership (&self, &mut self, owned)

use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};
use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_trait_impl_ref_self_matching() {
    let source = r#"
trait GameLoop {
    fn init(self) {
        println("Default init");
    }
    
    fn update(self) {
        println("Default update");
    }
}

struct TestGame {
    count: int
}

impl TestGame {
    fn new() -> TestGame {
        TestGame { count: 0 }
    }
}

impl GameLoop for TestGame {
    fn init(self) {
        println("TestGame init");
    }
    
    fn update(self) {
        println("TestGame update");
    }
}
"#;

    let output = parse_and_generate(source);
    
    println!("Generated Rust:\n{}", output);
    
    // Both trait and impl should use &self (inferred from usage)
    assert!(output.contains("trait GameLoop"), "Trait not generated");
    assert!(output.contains("fn init(&self)"), "Trait method should use &self");
    
    // Impl should MATCH trait signature
    assert!(output.contains("impl GameLoop for TestGame"), "Impl not generated");
    assert!(output.contains("fn init(&self)") && output.contains("TestGame init"), 
            "Impl method should match trait signature (&self)");
    
    // Should not have owned self in impl
    let impl_section = output.split("impl GameLoop for TestGame").nth(1).unwrap_or("");
    assert!(!impl_section.contains("fn init(self)"), 
            "Impl should not use owned self when trait uses &self, but found:\n{}", impl_section);
}

#[test]
fn test_trait_impl_default_method_read_only() {
    let source = r#"
trait Printable {
    fn display(self) {
        // Read-only default impl - should infer &self
        println("Displaying");
    }
}

struct Item {
    name: string
}

impl Printable for Item {
    fn display(self) {
        println(self.name);
    }
}
"#;

    let output = parse_and_generate(source);
    
    println!("Generated Rust:\n{}", output);
    
    // Trait should infer &self for read-only methods
    assert!(output.contains("fn display(&self)"), 
            "Trait default method should infer &self for read-only access");
    
    // Impl should MATCH trait signature (&self)
    let impl_section = output.split("impl Printable for Item").nth(1).unwrap_or("");
    assert!(impl_section.contains("fn display(&self)"), 
            "Impl method should match trait signature (&self), but found:\n{}", impl_section);
}

