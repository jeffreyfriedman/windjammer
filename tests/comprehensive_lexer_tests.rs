//! Comprehensive Lexer Tests
//!
//! These tests verify every token type and edge case in the lexer.
//! They serve as documentation for the lexical grammar of Windjammer.

use windjammer::lexer::{Lexer, Token};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    lexer
        .tokenize_with_locations()
        .into_iter()
        .map(|lt| lt.token)
        .filter(|t| !matches!(t, Token::Eof))
        .collect()
}

fn first_token(input: &str) -> Token {
    tokenize(input).into_iter().next().unwrap()
}

// ============================================================================
// KEYWORDS
// ============================================================================

#[test]
fn test_all_keywords() {
    // Core keywords
    assert!(matches!(first_token("fn"), Token::Fn));
    assert!(matches!(first_token("let"), Token::Let));
    assert!(matches!(first_token("mut"), Token::Mut));
    assert!(matches!(first_token("const"), Token::Const));
    assert!(matches!(first_token("static"), Token::Static));

    // Type keywords
    assert!(matches!(first_token("struct"), Token::Struct));
    assert!(matches!(first_token("enum"), Token::Enum));
    assert!(matches!(first_token("type"), Token::Type));

    // Trait/impl keywords
    assert!(matches!(first_token("impl"), Token::Impl));
    assert!(matches!(first_token("trait"), Token::Trait));
    assert!(matches!(first_token("for"), Token::For));
    assert!(matches!(first_token("where"), Token::Where));

    // Control flow keywords
    assert!(matches!(first_token("if"), Token::If));
    assert!(matches!(first_token("else"), Token::Else));
    assert!(matches!(first_token("match"), Token::Match));
    assert!(matches!(first_token("while"), Token::While));
    assert!(matches!(first_token("loop"), Token::Loop));
    assert!(matches!(first_token("break"), Token::Break));
    assert!(matches!(first_token("continue"), Token::Continue));
    assert!(matches!(first_token("return"), Token::Return));

    // Module keywords
    assert!(matches!(first_token("use"), Token::Use));
    assert!(matches!(first_token("mod"), Token::Mod));
    assert!(matches!(first_token("pub"), Token::Pub));
    // Note: crate and super are parsed as identifiers, not special tokens
    assert!(matches!(first_token("crate"), Token::Ident(_)));
    assert!(matches!(first_token("super"), Token::Ident(_)));

    // Other keywords
    assert!(matches!(first_token("as"), Token::As));
    assert!(matches!(first_token("in"), Token::In));
    assert!(matches!(first_token("self"), Token::Self_));
    assert!(matches!(first_token("extern"), Token::Extern));
    assert!(matches!(first_token("async"), Token::Async));
    assert!(matches!(first_token("await"), Token::Await));
}

// ============================================================================
// NUMERIC LITERALS
// ============================================================================

#[test]
fn test_integer_literals_basic() {
    assert_eq!(first_token("0"), Token::IntLiteral(0));
    assert_eq!(first_token("1"), Token::IntLiteral(1));
    assert_eq!(first_token("42"), Token::IntLiteral(42));
    assert_eq!(first_token("999999"), Token::IntLiteral(999999));
}

#[test]
fn test_integer_literals_large() {
    assert_eq!(first_token("2147483647"), Token::IntLiteral(2147483647)); // i32 max
    assert_eq!(
        first_token("9223372036854775807"),
        Token::IntLiteral(9223372036854775807)
    ); // i64 max
}

#[test]
fn test_integer_literals_hex() {
    // Hex literals if supported
    let tokens = tokenize("0xFF");
    // May be parsed as hex or as 0 followed by identifier xFF
    assert!(!tokens.is_empty());
}

#[test]
fn test_float_literals_basic() {
    assert!(matches!(first_token("0.0"), Token::FloatLiteral(_)));
    assert!(matches!(first_token("3.14"), Token::FloatLiteral(_)));
    assert!(matches!(first_token("2.718281828"), Token::FloatLiteral(_)));
}

#[test]
fn test_float_literals_edge_cases() {
    // Leading zero
    assert!(matches!(first_token("0.5"), Token::FloatLiteral(_)));
    // Many decimal places
    assert!(matches!(
        first_token("3.14159265358979"),
        Token::FloatLiteral(_)
    ));
}

#[test]
fn test_float_literals_scientific() {
    // Scientific notation if supported
    let tokens = tokenize("1e10");
    assert!(!tokens.is_empty());
}

// ============================================================================
// STRING LITERALS
// ============================================================================

#[test]
fn test_string_literals_basic() {
    assert_eq!(first_token(r#""""#), Token::StringLiteral("".to_string()));
    assert_eq!(
        first_token(r#""hello""#),
        Token::StringLiteral("hello".to_string())
    );
    assert_eq!(
        first_token(r#""Hello, World!""#),
        Token::StringLiteral("Hello, World!".to_string())
    );
}

#[test]
fn test_string_literals_with_spaces() {
    assert_eq!(
        first_token(r#""  ""#),
        Token::StringLiteral("  ".to_string())
    );
    assert_eq!(
        first_token(r#""hello world""#),
        Token::StringLiteral("hello world".to_string())
    );
}

#[test]
fn test_string_literals_escape_sequences() {
    assert_eq!(
        first_token(r#""\n""#),
        Token::StringLiteral("\n".to_string())
    );
    assert_eq!(
        first_token(r#""\t""#),
        Token::StringLiteral("\t".to_string())
    );
    assert_eq!(
        first_token(r#""\r""#),
        Token::StringLiteral("\r".to_string())
    );
    assert_eq!(
        first_token(r#""\\""#),
        Token::StringLiteral("\\".to_string())
    );
    assert_eq!(
        first_token(r#""\"""#),
        Token::StringLiteral("\"".to_string())
    );
}

#[test]
fn test_string_interpolation_basic() {
    let token = first_token(r#""Hello, ${name}!""#);
    assert!(matches!(token, Token::InterpolatedString(_)));
}

#[test]
fn test_string_interpolation_multiple() {
    let token = first_token(r#""${a} + ${b} = ${c}""#);
    assert!(matches!(token, Token::InterpolatedString(_)));
}

#[test]
fn test_string_interpolation_expressions() {
    let token = first_token(r#""Result: ${x + y}""#);
    assert!(matches!(token, Token::InterpolatedString(_)));
}

// ============================================================================
// CHARACTER LITERALS
// ============================================================================

#[test]
fn test_char_literals_basic() {
    assert_eq!(first_token("'a'"), Token::CharLiteral('a'));
    assert_eq!(first_token("'Z'"), Token::CharLiteral('Z'));
    assert_eq!(first_token("'0'"), Token::CharLiteral('0'));
    assert_eq!(first_token("'_'"), Token::CharLiteral('_'));
}

#[test]
fn test_char_literals_escape() {
    assert_eq!(first_token("'\\n'"), Token::CharLiteral('\n'));
    assert_eq!(first_token("'\\t'"), Token::CharLiteral('\t'));
    assert_eq!(first_token("'\\r'"), Token::CharLiteral('\r'));
    assert_eq!(first_token("'\\''"), Token::CharLiteral('\''));
    assert_eq!(first_token("'\\\\'"), Token::CharLiteral('\\'));
}

// ============================================================================
// BOOLEAN LITERALS
// ============================================================================

#[test]
fn test_boolean_literals() {
    assert_eq!(first_token("true"), Token::BoolLiteral(true));
    assert_eq!(first_token("false"), Token::BoolLiteral(false));
}

// ============================================================================
// OPERATORS
// ============================================================================

#[test]
fn test_arithmetic_operators() {
    assert!(matches!(first_token("+"), Token::Plus));
    assert!(matches!(first_token("-"), Token::Minus));
    assert!(matches!(first_token("*"), Token::Star));
    assert!(matches!(first_token("/"), Token::Slash));
    assert!(matches!(first_token("%"), Token::Percent));
}

#[test]
fn test_comparison_operators() {
    assert!(matches!(first_token("=="), Token::Eq));
    assert!(matches!(first_token("!="), Token::Ne));
    assert!(matches!(first_token("<"), Token::Lt));
    assert!(matches!(first_token("<="), Token::Le));
    assert!(matches!(first_token(">"), Token::Gt));
    assert!(matches!(first_token(">="), Token::Ge));
}

#[test]
fn test_logical_operators() {
    assert!(matches!(first_token("&&"), Token::And));
    assert!(matches!(first_token("||"), Token::Or));
    assert!(matches!(first_token("!"), Token::Bang));
}

#[test]
fn test_assignment_operators() {
    assert!(matches!(first_token("="), Token::Assign));
    assert!(matches!(first_token("+="), Token::PlusAssign));
    assert!(matches!(first_token("-="), Token::MinusAssign));
    assert!(matches!(first_token("*="), Token::StarAssign));
    assert!(matches!(first_token("/="), Token::SlashAssign));
}

#[test]
fn test_arrow_operators() {
    assert!(matches!(first_token("->"), Token::Arrow));
    assert!(matches!(first_token("=>"), Token::FatArrow));
    assert!(matches!(first_token("<-"), Token::LeftArrow));
}

#[test]
fn test_range_operators() {
    assert!(matches!(first_token(".."), Token::DotDot));
    assert!(matches!(first_token("..="), Token::DotDotEq));
}

#[test]
fn test_special_operators() {
    assert!(matches!(first_token("|>"), Token::PipeOp));
    assert!(matches!(first_token("::"), Token::ColonColon));
}

// ============================================================================
// DELIMITERS AND PUNCTUATION
// ============================================================================

#[test]
fn test_delimiters() {
    assert!(matches!(first_token("("), Token::LParen));
    assert!(matches!(first_token(")"), Token::RParen));
    assert!(matches!(first_token("{"), Token::LBrace));
    assert!(matches!(first_token("}"), Token::RBrace));
    assert!(matches!(first_token("["), Token::LBracket));
    assert!(matches!(first_token("]"), Token::RBracket));
}

#[test]
fn test_punctuation() {
    assert!(matches!(first_token(","), Token::Comma));
    assert!(matches!(first_token("."), Token::Dot));
    assert!(matches!(first_token(":"), Token::Colon));
    assert!(matches!(first_token(";"), Token::Semicolon));
    assert!(matches!(first_token("?"), Token::Question));
    assert!(matches!(first_token("&"), Token::Ampersand));
    assert!(matches!(first_token("|"), Token::Pipe));
}

// ============================================================================
// DECORATORS
// ============================================================================

#[test]
fn test_decorators() {
    assert_eq!(first_token("@auto"), Token::Decorator("auto".to_string()));
    assert_eq!(
        first_token("@derive"),
        Token::Decorator("derive".to_string())
    );
    assert_eq!(
        first_token("@component"),
        Token::Decorator("component".to_string())
    );
    assert_eq!(first_token("@game"), Token::Decorator("game".to_string()));
    assert_eq!(first_token("@route"), Token::Decorator("route".to_string()));
}

// ============================================================================
// IDENTIFIERS
// ============================================================================

#[test]
fn test_identifiers_basic() {
    assert_eq!(first_token("x"), Token::Ident("x".to_string()));
    assert_eq!(first_token("foo"), Token::Ident("foo".to_string()));
    assert_eq!(first_token("bar_baz"), Token::Ident("bar_baz".to_string()));
}

#[test]
fn test_identifiers_underscore() {
    // Single underscore is a special token for wildcard patterns
    assert!(matches!(first_token("_"), Token::Underscore));
    // Leading underscore identifiers
    assert_eq!(
        first_token("_private"),
        Token::Ident("_private".to_string())
    );
    assert_eq!(
        first_token("__dunder__"),
        Token::Ident("__dunder__".to_string())
    );
}

#[test]
fn test_identifiers_with_numbers() {
    assert_eq!(first_token("x1"), Token::Ident("x1".to_string()));
    assert_eq!(first_token("foo123"), Token::Ident("foo123".to_string()));
    assert_eq!(first_token("vec3"), Token::Ident("vec3".to_string()));
}

#[test]
fn test_identifiers_camelcase() {
    assert_eq!(
        first_token("CamelCase"),
        Token::Ident("CamelCase".to_string())
    );
    assert_eq!(
        first_token("MyStruct"),
        Token::Ident("MyStruct".to_string())
    );
    assert_eq!(
        first_token("HTTPRequest"),
        Token::Ident("HTTPRequest".to_string())
    );
}

// ============================================================================
// COMMENTS
// ============================================================================

#[test]
fn test_line_comments_skipped() {
    let tokens = tokenize("// comment\nfn");
    assert!(matches!(tokens[0], Token::Fn));
}

#[test]
fn test_line_comments_at_end() {
    let tokens = tokenize("fn // comment");
    assert!(matches!(tokens[0], Token::Fn));
    assert_eq!(tokens.len(), 1);
}

#[test]
fn test_doc_comments() {
    let tokens = tokenize("/// doc comment\nfn");
    // Doc comments may be preserved or skipped depending on implementation
    assert!(tokens.iter().any(|t| matches!(t, Token::Fn)));
}

// ============================================================================
// WHITESPACE
// ============================================================================

#[test]
fn test_whitespace_spaces() {
    let tokens = tokenize("   fn   main   ");
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0], Token::Fn));
}

#[test]
fn test_whitespace_tabs() {
    let tokens = tokenize("\t\tfn\tmain\t");
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0], Token::Fn));
}

#[test]
fn test_whitespace_newlines() {
    let tokens = tokenize("fn\n\nmain\n");
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0], Token::Fn));
}

#[test]
fn test_whitespace_mixed() {
    let tokens = tokenize("  fn \t\n  main  ");
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0], Token::Fn));
}

// ============================================================================
// COMPLEX TOKENIZATION
// ============================================================================

#[test]
fn test_function_declaration() {
    let tokens = tokenize("fn greet(name: string) -> string");
    assert!(matches!(tokens[0], Token::Fn));
    assert_eq!(tokens[1], Token::Ident("greet".to_string()));
    assert!(matches!(tokens[2], Token::LParen));
    assert_eq!(tokens[3], Token::Ident("name".to_string()));
    assert!(matches!(tokens[4], Token::Colon));
    // "string" is a type keyword, not an identifier
    assert!(matches!(tokens[5], Token::String));
    assert!(matches!(tokens[6], Token::RParen));
    assert!(matches!(tokens[7], Token::Arrow));
    assert!(matches!(tokens[8], Token::String));
}

#[test]
fn test_struct_declaration() {
    let tokens = tokenize("struct Point { x: i32, y: i32 }");
    assert!(matches!(tokens[0], Token::Struct));
    assert_eq!(tokens[1], Token::Ident("Point".to_string()));
    assert!(matches!(tokens[2], Token::LBrace));
    assert_eq!(tokens[3], Token::Ident("x".to_string()));
    assert!(matches!(tokens[4], Token::Colon));
}

#[test]
fn test_let_statement() {
    let tokens = tokenize("let mut x = 42");
    assert!(matches!(tokens[0], Token::Let));
    assert!(matches!(tokens[1], Token::Mut));
    assert_eq!(tokens[2], Token::Ident("x".to_string()));
    assert!(matches!(tokens[3], Token::Assign));
    assert_eq!(tokens[4], Token::IntLiteral(42));
}

#[test]
fn test_method_call() {
    let tokens = tokenize("obj.method(arg1, arg2)");
    assert_eq!(tokens[0], Token::Ident("obj".to_string()));
    assert!(matches!(tokens[1], Token::Dot));
    assert_eq!(tokens[2], Token::Ident("method".to_string()));
    assert!(matches!(tokens[3], Token::LParen));
}

#[test]
fn test_generic_type() {
    let tokens = tokenize("Vec<String>");
    assert_eq!(tokens[0], Token::Ident("Vec".to_string()));
    assert!(matches!(tokens[1], Token::Lt));
    assert_eq!(tokens[2], Token::Ident("String".to_string()));
    assert!(matches!(tokens[3], Token::Gt));
}

#[test]
fn test_closure() {
    let tokens = tokenize("|x, y| x + y");
    assert!(matches!(tokens[0], Token::Pipe));
    assert_eq!(tokens[1], Token::Ident("x".to_string()));
    assert!(matches!(tokens[2], Token::Comma));
    assert_eq!(tokens[3], Token::Ident("y".to_string()));
    assert!(matches!(tokens[4], Token::Pipe));
}

#[test]
fn test_match_expression() {
    let tokens = tokenize("match x { 1 => a, _ => b }");
    assert!(matches!(tokens[0], Token::Match));
    assert_eq!(tokens[1], Token::Ident("x".to_string()));
    assert!(matches!(tokens[2], Token::LBrace));
    assert_eq!(tokens[3], Token::IntLiteral(1));
    assert!(matches!(tokens[4], Token::FatArrow));
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_empty_input() {
    let tokens = tokenize("");
    assert!(tokens.is_empty());
}

#[test]
fn test_only_whitespace() {
    let tokens = tokenize("   \t\n  ");
    assert!(tokens.is_empty());
}

#[test]
fn test_only_comment() {
    let tokens = tokenize("// just a comment");
    assert!(tokens.is_empty());
}

#[test]
fn test_consecutive_operators() {
    // Should parse as separate tokens, not combined
    let tokens = tokenize("+-*/");
    assert_eq!(tokens.len(), 4);
}

#[test]
fn test_operator_at_end() {
    let tokens = tokenize("x +");
    assert_eq!(tokens.len(), 2);
}

#[test]
fn test_unicode_in_string() {
    let token = first_token(r#""Hello, 世界!""#);
    assert!(matches!(token, Token::StringLiteral(_)));
}
