// Lexer unit tests - verify correct tokenization

use windjammer::lexer::{Lexer, Token};

#[test]
fn test_keywords() {
    let input = "fn let mut const static struct impl trait match if else for while loop return";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0], Token::Fn));
    assert!(matches!(tokens[1], Token::Let));
    assert!(matches!(tokens[2], Token::Mut));
    assert!(matches!(tokens[3], Token::Const));
    assert!(matches!(tokens[4], Token::Static));
    assert!(matches!(tokens[5], Token::Struct));
    assert!(matches!(tokens[6], Token::Impl));
    assert!(matches!(tokens[7], Token::Trait));
    assert!(matches!(tokens[8], Token::Match));
    assert!(matches!(tokens[9], Token::If));
    assert!(matches!(tokens[10], Token::Else));
    assert!(matches!(tokens[11], Token::For));
    assert!(matches!(tokens[12], Token::While));
    assert!(matches!(tokens[13], Token::Loop));
    assert!(matches!(tokens[14], Token::Return));
}

#[test]
fn test_integer_literals() {
    let input = "42 0 999 1234567890";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0], Token::IntLiteral(42));
    assert_eq!(tokens[1], Token::IntLiteral(0));
    assert_eq!(tokens[2], Token::IntLiteral(999));
    assert_eq!(tokens[3], Token::IntLiteral(1234567890));
}

#[test]
fn test_float_literals() {
    let input = "3.14 0.5 99.999";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0], Token::FloatLiteral(_)));
    assert!(matches!(tokens[1], Token::FloatLiteral(_)));
    assert!(matches!(tokens[2], Token::FloatLiteral(_)));
}

#[test]
fn test_string_literals() {
    let input = r#""hello" "world" """#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0], Token::StringLiteral("hello".to_string()));
    assert_eq!(tokens[1], Token::StringLiteral("world".to_string()));
    assert_eq!(tokens[2], Token::StringLiteral("".to_string()));
}

#[test]
fn test_character_literals() {
    let input = "'a' 'x' '0' '\\n' '\\t' '\\''";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0], Token::CharLiteral('a'));
    assert_eq!(tokens[1], Token::CharLiteral('x'));
    assert_eq!(tokens[2], Token::CharLiteral('0'));
    assert_eq!(tokens[3], Token::CharLiteral('\n'));
    assert_eq!(tokens[4], Token::CharLiteral('\t'));
    assert_eq!(tokens[5], Token::CharLiteral('\''));
}

#[test]
fn test_string_interpolation() {
    let input = r#""Hello, ${name}!""#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0], Token::InterpolatedString(_)));
}

#[test]
fn test_boolean_literals() {
    let input = "true false";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0], Token::BoolLiteral(true));
    assert_eq!(tokens[1], Token::BoolLiteral(false));
}

#[test]
fn test_operators() {
    let input = "+ - * / % == != < <= > >= && || !";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0], Token::Plus));
    assert!(matches!(tokens[1], Token::Minus));
    assert!(matches!(tokens[2], Token::Star));
    assert!(matches!(tokens[3], Token::Slash));
    assert!(matches!(tokens[4], Token::Percent));
    assert!(matches!(tokens[5], Token::Eq));
    assert!(matches!(tokens[6], Token::Ne));
    assert!(matches!(tokens[7], Token::Lt));
    assert!(matches!(tokens[8], Token::Le));
    assert!(matches!(tokens[9], Token::Gt));
    assert!(matches!(tokens[10], Token::Ge));
    assert!(matches!(tokens[11], Token::And));
    assert!(matches!(tokens[12], Token::Or));
    assert!(matches!(tokens[13], Token::Bang));
}

#[test]
fn test_special_operators() {
    let input = "-> => <- |>";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0], Token::Arrow));
    assert!(matches!(tokens[1], Token::FatArrow));
    assert!(matches!(tokens[2], Token::LeftArrow));
    assert!(matches!(tokens[3], Token::PipeOp));
}

#[test]
fn test_range_operators() {
    let input = ".. ..=";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0], Token::DotDot));
    assert!(matches!(tokens[1], Token::DotDotEq));
}

#[test]
fn test_delimiters() {
    let input = "( ) { } [ ] , . : ; ?";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0], Token::LParen));
    assert!(matches!(tokens[1], Token::RParen));
    assert!(matches!(tokens[2], Token::LBrace));
    assert!(matches!(tokens[3], Token::RBrace));
    assert!(matches!(tokens[4], Token::LBracket));
    assert!(matches!(tokens[5], Token::RBracket));
    assert!(matches!(tokens[6], Token::Comma));
    assert!(matches!(tokens[7], Token::Dot));
    assert!(matches!(tokens[8], Token::Colon));
    assert!(matches!(tokens[9], Token::Semicolon));
    assert!(matches!(tokens[10], Token::Question));
}

#[test]
fn test_decorators() {
    let input = "@route @timing @auto";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0], Token::Decorator("route".to_string()));
    assert_eq!(tokens[1], Token::Decorator("timing".to_string()));
    assert_eq!(tokens[2], Token::Decorator("auto".to_string()));
}

#[test]
fn test_identifiers() {
    let input = "hello world_123 _private CamelCase";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0], Token::Ident("hello".to_string()));
    assert_eq!(tokens[1], Token::Ident("world_123".to_string()));
    assert_eq!(tokens[2], Token::Ident("_private".to_string()));
    assert_eq!(tokens[3], Token::Ident("CamelCase".to_string()));
}

#[test]
fn test_comments() {
    let input = "// This is a comment\nfn main() {}";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    // Comments should be skipped
    assert!(matches!(tokens[0], Token::Fn));
    assert!(matches!(tokens[1], Token::Ident(_)));
}

#[test]
fn test_whitespace_handling() {
    let input = "   fn    main   (   )   {   }   ";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    // Whitespace should be skipped
    assert!(matches!(tokens[0], Token::Fn));
    assert!(matches!(tokens[1], Token::Ident(_)));
    assert!(matches!(tokens[2], Token::LParen));
    assert!(matches!(tokens[3], Token::RParen));
}

#[test]
fn test_realistic_function() {
    let input = r#"fn greet(name: string) -> string {
        "Hello, ${name}!"
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    // Should have: fn, ident(greet), lparen, ident(name), colon, ident(string), rparen,
    //             arrow, ident(string), lbrace, interpolated_string, rbrace, eof
    assert!(matches!(tokens[0], Token::Fn));
    assert!(matches!(tokens[1], Token::Ident(_)));
    assert!(matches!(tokens[2], Token::LParen));
    assert!(tokens.len() > 10); // Has many tokens
    assert!(matches!(tokens[tokens.len() - 1], Token::Eof));
}
