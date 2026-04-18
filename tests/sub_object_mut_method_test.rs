//! E0596 / ownership: methods that call `&mut self` methods on `self.field` must infer `&mut self`.

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
fn test_method_calling_mut_method_on_sub_object() {
    let source = r#"
struct Keyboard {
    keys: Vec<bool>,
}

impl Keyboard {
    fn update_key(self, key: i32, down: bool) {
        self.keys[0] = down
    }
}

struct Game {
    keyboard: Keyboard,
}

impl Game {
    fn poll_input(self) {
        self.keyboard.update_key(0, true)
    }
}
"#;
    let code = compile_to_rust(source);
    assert!(
        code.contains("fn poll_input(&mut self"),
        "poll_input should be &mut self since it calls a mutating method on sub-object. Got:\n{}",
        code
    );
}

#[test]
fn test_match_arm_mut_method_on_sub_object() {
    let source = r#"
struct Companion {
    loyalty: f32,
}

impl Companion {
    fn adjust_loyalty(self, delta: f32) {
        self.loyalty = self.loyalty + delta
    }
}

struct Kestrel {
    companion: Companion,
}

impl Kestrel {
    pub fn respond_to_player_choice(self, choice: i32) {
        match choice {
            0 => self.companion.adjust_loyalty(-5.0),
            1 => self.companion.adjust_loyalty(5.0),
            _ => {}
        }
    }
}
"#;
    let code = compile_to_rust(source);
    assert!(
        code.contains("fn respond_to_player_choice(&mut self"),
        "respond_to_player_choice should be &mut self (match arms call mutating sub-object methods). Got:\n{}",
        code
    );
}
