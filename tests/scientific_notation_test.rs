use windjammer::lexer::{Lexer, Token};

#[path = "test_utils.rs"]
mod test_utils;

fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        if token == Token::Eof {
            break;
        }
        tokens.push(token);
    }
    tokens
}

#[test]
fn test_scientific_notation_integer_base() {
    let tokens = tokenize("1e10");
    assert_eq!(
        tokens.len(),
        1,
        "1e10 should be a single token, got: {:?}",
        tokens
    );
    match &tokens[0] {
        Token::FloatLiteral(f) => {
            assert!(
                (f - 1e10).abs() < 1.0,
                "Expected 1e10 = 10000000000.0, got {}",
                f
            );
        }
        other => panic!("Expected FloatLiteral, got {:?}", other),
    }
}

#[test]
fn test_scientific_notation_float_base() {
    let tokens = tokenize("2.5e3");
    assert_eq!(
        tokens.len(),
        1,
        "2.5e3 should be a single token, got: {:?}",
        tokens
    );
    match &tokens[0] {
        Token::FloatLiteral(f) => {
            assert!((f - 2500.0).abs() < 0.001, "Expected 2500.0, got {}", f);
        }
        other => panic!("Expected FloatLiteral, got {:?}", other),
    }
}

#[test]
fn test_scientific_notation_negative_exponent() {
    let tokens = tokenize("1e-5");
    assert_eq!(
        tokens.len(),
        1,
        "1e-5 should be a single token, got: {:?}",
        tokens
    );
    match &tokens[0] {
        Token::FloatLiteral(f) => {
            assert!((f - 1e-5).abs() < 1e-10, "Expected 1e-5, got {}", f);
        }
        other => panic!("Expected FloatLiteral, got {:?}", other),
    }
}

#[test]
fn test_scientific_notation_positive_exponent() {
    let tokens = tokenize("3E+2");
    assert_eq!(
        tokens.len(),
        1,
        "3E+2 should be a single token, got: {:?}",
        tokens
    );
    match &tokens[0] {
        Token::FloatLiteral(f) => {
            assert!((f - 300.0).abs() < 0.001, "Expected 300.0, got {}", f);
        }
        other => panic!("Expected FloatLiteral, got {:?}", other),
    }
}

#[test]
fn test_scientific_notation_uppercase_e() {
    let tokens = tokenize("5E10");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::FloatLiteral(f) => {
            assert!((f - 5e10).abs() < 1.0, "Expected 5e10, got {}", f);
        }
        other => panic!("Expected FloatLiteral, got {:?}", other),
    }
}

#[test]
fn test_scientific_notation_float_negative_exp() {
    let tokens = tokenize("6.022e-23");
    assert_eq!(tokens.len(), 1);
    match &tokens[0] {
        Token::FloatLiteral(f) => {
            assert!(
                (f - 6.022e-23).abs() < 1e-30,
                "Expected 6.022e-23, got {}",
                f
            );
        }
        other => panic!("Expected FloatLiteral, got {:?}", other),
    }
}

#[test]
fn test_scientific_notation_in_let_expression() {
    let tokens = tokenize("let x = 1e10");
    let float_tokens: Vec<&Token> = tokens
        .iter()
        .filter(|t| matches!(t, Token::FloatLiteral(_)))
        .collect();
    assert_eq!(
        float_tokens.len(),
        1,
        "Should have exactly one float literal in 'let x = 1e10'"
    );
}

#[test]
fn test_scientific_notation_codegen_rust() {
    let output = test_utils::compile_single(
        r#"
fn get_threshold() -> f64 {
    let x = 1e10
    x
}
"#,
    );
    assert!(
        output.contains("1e10") || output.contains("10000000000"),
        "Generated Rust should contain the scientific notation value. Got:\n{}",
        output
    );
    assert!(
        !output.contains("e10;"),
        "Should NOT generate 'e10' as a separate identifier"
    );
}
