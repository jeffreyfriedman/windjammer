use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let mut generator = codegen::CodeGenerator::new(signatures, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_no_clone_on_method_receiver_in_ref_self() {
    let source = r#"
    struct Pool {
        available: Vec<i32>,
    }
    impl Pool {
        fn is_empty(self) -> bool {
            self.available.len() == 0
        }
    }
    "#;
    let output = compile_and_get_rust(source);
    eprintln!("Generated:\n{}", output);
    assert!(
        !output.contains("self.available.clone()"),
        "Field used as method receiver in &self method should NOT be cloned. Got:\n{}",
        output
    );
}

#[test]
fn test_no_clone_on_for_loop_iterable_in_ref_self() {
    let source = r#"
    trait System {
        fn is_enabled(self) -> bool
        fn update(self, dt: f32)
    }
    struct SystemManager {
        systems: Vec<i32>,
    }
    impl SystemManager {
        fn enabled_count(self) -> i32 {
            let mut count = 0
            for system in self.systems {
                count = count + 1
            }
            count
        }
    }
    "#;
    let output = compile_and_get_rust(source);
    eprintln!("Generated:\n{}", output);
    assert!(
        !output.contains("self.systems.clone()"),
        "Field used as for-loop iterable in &self method should NOT be cloned. Got:\n{}",
        output
    );
}

#[test]
fn test_no_clone_on_generic_vec_in_ref_self() {
    let source = r#"
    struct ObjectPool<T> {
        available: Vec<T>,
        capacity: i32,
    }
    impl ObjectPool<T> {
        fn is_empty(self) -> bool {
            self.available.len() == 0
        }
        fn is_full(self) -> bool {
            self.available.len() == self.capacity
        }
    }
    "#;
    let output = compile_and_get_rust(source);
    eprintln!("Generated:\n{}", output);
    assert!(
        !output.contains("self.available.clone()"),
        "Generic Vec<T> field used as method receiver should NOT be cloned. Got:\n{}",
        output
    );
}
