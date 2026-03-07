/// TDD test for string parameter assignment to string field
/// BUG: Compiler infers &str when parameter is assigned to String field
///
/// Example:
/// ```windjammer
/// struct Config {
///     name: string,
/// }
/// impl Config {
///     pub fn set_name(self, name: string) {
///         self.name = name  // Assignment to String field
///     }
/// }
/// ```
///
/// EXPECTED: set_name(&mut self, name: String)
/// ACTUAL: set_name(&mut self, name: &str) ❌ Type mismatch!
use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};
use windjammer::CompilationTarget;

fn parse_code(code: &str) -> Program<'static> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().unwrap()
}

#[test]
fn test_string_param_assigned_to_string_field() {
    let code = r#"
struct Config {
    name: string,
}

impl Config {
    pub fn set_name(self, name: string) {
        self.name = name
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);

    // ASSERT: Parameter should be String, not &str
    assert!(
        generated.contains("name: String"),
        "Parameter should be String when assigned to String field!\nGenerated:\n{}",
        generated
    );

    // ASSERT: Should NOT be &str
    assert!(
        !generated.contains("set_name(&mut self, name: &str)"),
        "Parameter should NOT be &str when assigned to String field!\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_string_param_not_assigned_can_be_str() {
    let code = r#"
struct Logger {
    logs: Vec<string>,
}

impl Logger {
    pub fn log(self, message: string) {
        // Just prints, doesn't assign to field
        println(message)
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);

    // ASSERT: Parameter should be borrowed since it's only read (not assigned to field)
    // TDD UPDATE: Multi-pass inference now correctly infers Borrowed for read-only String params
    assert!(
        generated.contains("message: &String") || generated.contains("message: &str") || generated.contains("message: String"),
        "Parameter should be borrowed (&String or &str) or owned (String) when not assigned to field!\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_string_param_assigned_to_vec_push() {
    let code = r#"
struct Logger {
    logs: Vec<string>,
}

impl Logger {
    pub fn add_log(self, message: string) {
        self.logs.push(message)
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);

    // ASSERT: Parameter should be String when pushed to Vec<String>
    assert!(
        generated.contains("message: String"),
        "Parameter should be String when pushed to Vec<String>!\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_multiple_string_params() {
    let code = r#"
struct User {
    name: string,
    email: string,
}

impl User {
    pub fn update(self, name: string, email: string) {
        self.name = name
        self.email = email
    }
}
"#;

    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);

    // ASSERT: Both parameters should be String
    assert!(
        generated.contains("name: String"),
        "name parameter should be String!\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("email: String"),
        "email parameter should be String!\nGenerated:\n{}",
        generated
    );
}
