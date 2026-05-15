// Lexer unit tests - verify correct tokenization

use windjammer::lexer::{Lexer, Token};

#[test]
fn test_keywords() {
    let input = "fn let mut const static struct impl trait match if else for while loop return";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert!(matches!(tokens[0].token, Token::Fn));
    assert!(matches!(tokens[1].token, Token::Let));
    assert!(matches!(tokens[2].token, Token::Mut));
    assert!(matches!(tokens[3].token, Token::Const));
    assert!(matches!(tokens[4].token, Token::Static));
    assert!(matches!(tokens[5].token, Token::Struct));
    assert!(matches!(tokens[6].token, Token::Impl));
    assert!(matches!(tokens[7].token, Token::Trait));
    assert!(matches!(tokens[8].token, Token::Match));
    assert!(matches!(tokens[9].token, Token::If));
    assert!(matches!(tokens[10].token, Token::Else));
    assert!(matches!(tokens[11].token, Token::For));
    assert!(matches!(tokens[12].token, Token::While));
    assert!(matches!(tokens[13].token, Token::Loop));
    assert!(matches!(tokens[14].token, Token::Return));
}

#[test]
fn test_integer_literals() {
    let input = "42 0 999 1234567890";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert_eq!(tokens[0].token, Token::IntLiteral(42));
    assert_eq!(tokens[1].token, Token::IntLiteral(0));
    assert_eq!(tokens[2].token, Token::IntLiteral(999));
    assert_eq!(tokens[3].token, Token::IntLiteral(1234567890));
}

#[test]
fn test_float_literals() {
    let input = "3.14 0.5 99.999";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert!(matches!(tokens[0].token, Token::FloatLiteral(_)));
    assert!(matches!(tokens[1].token, Token::FloatLiteral(_)));
    assert!(matches!(tokens[2].token, Token::FloatLiteral(_)));
}

#[test]
fn test_string_literals() {
    let input = r#""hello" "world" """#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert_eq!(tokens[0].token, Token::StringLiteral("hello".to_string()));
    assert_eq!(tokens[1].token, Token::StringLiteral("world".to_string()));
    assert_eq!(tokens[2].token, Token::StringLiteral("".to_string()));
}

#[test]
fn test_character_literals() {
    let input = "'a' 'x' '0' '\\n' '\\t' '\\''";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert_eq!(tokens[0].token, Token::CharLiteral('a'));
    assert_eq!(tokens[1].token, Token::CharLiteral('x'));
    assert_eq!(tokens[2].token, Token::CharLiteral('0'));
    assert_eq!(tokens[3].token, Token::CharLiteral('\n'));
    assert_eq!(tokens[4].token, Token::CharLiteral('\t'));
    assert_eq!(tokens[5].token, Token::CharLiteral('\''));
}

#[test]
fn test_string_interpolation() {
    let input = r#""Hello, ${name}!""#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert!(matches!(tokens[0].token, Token::InterpolatedString(_)));
}

#[test]
fn test_boolean_literals() {
    let input = "true false";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert_eq!(tokens[0].token, Token::BoolLiteral(true));
    assert_eq!(tokens[1].token, Token::BoolLiteral(false));
}

#[test]
fn test_operators() {
    let input = "+ - * / % == != < <= > >= && || !";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert!(matches!(tokens[0].token, Token::Plus));
    assert!(matches!(tokens[1].token, Token::Minus));
    assert!(matches!(tokens[2].token, Token::Star));
    assert!(matches!(tokens[3].token, Token::Slash));
    assert!(matches!(tokens[4].token, Token::Percent));
    assert!(matches!(tokens[5].token, Token::Eq));
    assert!(matches!(tokens[6].token, Token::Ne));
    assert!(matches!(tokens[7].token, Token::Lt));
    assert!(matches!(tokens[8].token, Token::Le));
    assert!(matches!(tokens[9].token, Token::Gt));
    assert!(matches!(tokens[10].token, Token::Ge));
    assert!(matches!(tokens[11].token, Token::And));
    assert!(matches!(tokens[12].token, Token::Or));
    assert!(matches!(tokens[13].token, Token::Bang));
}

#[test]
fn test_special_operators() {
    let input = "-> => <- |>";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert!(matches!(tokens[0].token, Token::Arrow));
    assert!(matches!(tokens[1].token, Token::FatArrow));
    assert!(matches!(tokens[2].token, Token::LeftArrow));
    assert!(matches!(tokens[3].token, Token::PipeOp));
}

#[test]
fn test_range_operators() {
    let input = ".. ..=";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert!(matches!(tokens[0].token, Token::DotDot));
    assert!(matches!(tokens[1].token, Token::DotDotEq));
}

#[test]
fn test_delimiters() {
    let input = "( ) { } [ ] , . : ; ?";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert!(matches!(tokens[0].token, Token::LParen));
    assert!(matches!(tokens[1].token, Token::RParen));
    assert!(matches!(tokens[2].token, Token::LBrace));
    assert!(matches!(tokens[3].token, Token::RBrace));
    assert!(matches!(tokens[4].token, Token::LBracket));
    assert!(matches!(tokens[5].token, Token::RBracket));
    assert!(matches!(tokens[6].token, Token::Comma));
    assert!(matches!(tokens[7].token, Token::Dot));
    assert!(matches!(tokens[8].token, Token::Colon));
    assert!(matches!(tokens[9].token, Token::Semicolon));
    assert!(matches!(tokens[10].token, Token::Question));
}

#[test]
fn test_decorators() {
    let input = "@route @timing @auto";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert_eq!(tokens[0].token, Token::Decorator("route".to_string()));
    assert_eq!(tokens[1].token, Token::Decorator("timing".to_string()));
    assert_eq!(tokens[2].token, Token::Decorator("auto".to_string()));
}

#[test]
fn test_identifiers() {
    let input = "hello world_123 _private CamelCase";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    assert_eq!(tokens[0].token, Token::Ident("hello".to_string()));
    assert_eq!(tokens[1].token, Token::Ident("world_123".to_string()));
    assert_eq!(tokens[2].token, Token::Ident("_private".to_string()));
    assert_eq!(tokens[3].token, Token::Ident("CamelCase".to_string()));
}

#[test]
fn test_comments() {
    let input = "// This is a comment\nfn main() {}";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    // Comments should be skipped
    assert!(matches!(tokens[0].token, Token::Fn));
    assert!(matches!(tokens[1].token, Token::Ident(_)));
}

#[test]
fn test_whitespace_handling() {
    let input = "   fn    main   (   )   {   }   ";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    // Whitespace should be skipped
    assert!(matches!(tokens[0].token, Token::Fn));
    assert!(matches!(tokens[1].token, Token::Ident(_)));
    assert!(matches!(tokens[2].token, Token::LParen));
    assert!(matches!(tokens[3].token, Token::RParen));
}

#[test]
fn test_realistic_function() {
    let input = r#"fn greet(name: string) -> string {
        "Hello, ${name}!"
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();

    // Should have: fn, ident(greet), lparen, ident(name), colon, ident(string), rparen,
    //             arrow, ident(string), lbrace, interpolated_string, rbrace, eof
    assert!(matches!(tokens[0].token, Token::Fn));
    assert!(matches!(tokens[1].token, Token::Ident(_)));
    assert!(matches!(tokens[2].token, Token::LParen));
    assert!(tokens.len() > 10); // Has many tokens
    assert!(matches!(tokens[tokens.len() - 1].token, Token::Eof));
}
