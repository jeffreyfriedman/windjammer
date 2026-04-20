use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_to_rust(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_fns, registry, _) = analyzer.analyze_program(&program).unwrap();
    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed_fns)
}

#[test]
fn test_method_calling_self_consuming_method_inferred_owned() {
    // bind_auto_uniforms calls self.input_uniform() which takes owned self.
    // The analyzer must detect that calling a consuming method requires owned self.
    let source = r#"
struct Uniform {
    value: f32,
}

struct PassBuilder {
    name: String,
    dispatch_x: u32,
}

impl PassBuilder {
    pub fn input_uniform(self, buffer: Uniform) -> PassBuilder {
        let mut r = self
        r.dispatch_x = r.dispatch_x + 1
        r
    }

    pub fn bind_auto_uniforms(self) -> PassBuilder {
        let buffer = Uniform { value: 1.0 }
        self.input_uniform(buffer)
    }
}
"#;

    let code = compile_to_rust(source);

    // input_uniform takes owned self (builder pattern) - may be `self` or `mut self`
    assert!(
        code.contains("fn input_uniform(self") || code.contains("fn input_uniform(mut self"),
        "input_uniform should take owned self, got:\n{}",
        code
    );

    // bind_auto_uniforms must also take owned self because it calls self.input_uniform()
    // which consumes self. If it were &self, you'd get E0507.
    assert!(
        code.contains("fn bind_auto_uniforms(self")
            || code.contains("fn bind_auto_uniforms(mut self"),
        "bind_auto_uniforms should take owned self (calls consuming method), got:\n{}",
        code
    );
    // Must NOT generate &self for bind_auto_uniforms
    assert!(
        !code.contains("fn bind_auto_uniforms(&self"),
        "bind_auto_uniforms must NOT be &self, got:\n{}",
        code
    );
}

#[test]
fn test_method_moving_non_copy_field_mid_body() {
    // build() creates a struct using self.bindings (Vec, non-Copy) in the middle of the body.
    // This is NOT the return expression, it's a let binding mid-body.
    let source = r#"
struct Binding {
    name: String,
}

struct PassDef {
    bindings: Vec<Binding>,
    dispatch_x: u32,
}

struct Builder {
    bindings: Vec<Binding>,
    dispatch_x: u32,
    passes: Vec<PassDef>,
}

impl Builder {
    pub fn build(self) -> Vec<PassDef> {
        let pass = PassDef { bindings: self.bindings, dispatch_x: self.dispatch_x }
        let mut passes = self.passes
        passes.push(pass)
        passes
    }
}
"#;

    let code = compile_to_rust(source);

    // build() moves self.bindings into a struct literal, so it needs owned self
    assert!(
        code.contains("fn build(self") || code.contains("fn build(mut self"),
        "build should take owned self (moves non-Copy fields into struct), got:\n{}",
        code
    );
    assert!(
        !code.contains("fn build(&self"),
        "build must NOT be &self, got:\n{}",
        code
    );
}

#[test]
fn test_method_chaining_two_consuming_calls() {
    // A method that chains: self.step_a().step_b()
    // where step_a takes owned self.
    let source = r#"
struct Chain {
    value: i32,
}

impl Chain {
    pub fn step_a(self) -> Chain {
        let mut r = self
        r.value = r.value + 1
        r
    }

    pub fn step_b(self) -> Chain {
        let mut r = self
        r.value = r.value * 2
        r
    }

    pub fn do_both(self) -> Chain {
        self.step_a().step_b()
    }
}
"#;

    let code = compile_to_rust(source);

    assert!(
        code.contains("fn do_both(self") || code.contains("fn do_both(mut self"),
        "do_both should take owned self (calls consuming step_a), got:\n{}",
        code
    );
    assert!(
        !code.contains("fn do_both(&self"),
        "do_both must NOT be &self, got:\n{}",
        code
    );
}

#[test]
fn test_read_only_method_stays_borrowed() {
    // Ensure we don't break read-only methods: they should stay &self
    let source = r#"
struct Data {
    count: i32,
    name: String,
}

impl Data {
    pub fn get_count(self) -> i32 {
        self.count
    }
}
"#;

    let code = compile_to_rust(source);

    // get_count reads a Copy field - should be &self
    assert!(
        code.contains("fn get_count(&self"),
        "get_count should be &self (reads Copy field), got:\n{}",
        code
    );
}
