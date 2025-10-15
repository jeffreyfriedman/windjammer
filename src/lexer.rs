use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StringPart {
    Literal(String),
    Expression(String), // The expression text inside ${}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Let,
    Mut,
    Const,
    Static,
    Struct,
    Enum,
    Trait,
    Impl,
    Match,
    If,
    Else,
    For,
    In,
    While,
    Loop,
    Return,
    Break,
    Continue,
    Use,
    Go,
    Async,
    Await,
    Defer,
    Pub,
    Self_,
    Unsafe,
    As,
    Where,
    Type,
    Dyn,
    Bound,

    // Types
    Int,
    Int32,
    Uint,
    Float,
    Bool,
    String,

    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    InterpolatedString(Vec<StringPart>), // For strings with ${expr}
    CharLiteral(char),
    BoolLiteral(bool),

    // Identifiers
    Ident(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    And,
    Or,
    Not,

    Assign,
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    Arrow,         // ->
    LeftArrow,     // <-
    FatArrow,      // =>

    // Decorators
    At,                // @
    Decorator(String), // @route, @timing, etc.

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Comma,
    Dot,
    DotDot,   // ..
    DotDotEq, // ..=
    Colon,
    ColonColon, // :: (for turbofish and paths)
    Semicolon,
    Question,
    Ampersand,  // &
    Pipe,       // |
    PipeOp,     // |> (pipe operator)
    Underscore, // _
    Bang,       // ! (for macro invocations)

    // Special
    Eof,
    Newline,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "Ident({})", s),
            Token::IntLiteral(n) => write!(f, "Int({})", n),
            Token::StringLiteral(s) => write!(f, "String(\"{}\")", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

// Manual Eq implementation to handle f64 (using bit representation)
impl Eq for Token {}

// Manual Hash implementation to handle f64 (using bit representation)
impl std::hash::Hash for Token {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Token::IntLiteral(n) => n.hash(state),
            Token::FloatLiteral(f) => f.to_bits().hash(state), // Hash bits of f64
            Token::StringLiteral(s) => s.hash(state),
            Token::InterpolatedString(parts) => parts.hash(state),
            Token::CharLiteral(c) => c.hash(state),
            Token::BoolLiteral(b) => b.hash(state),
            Token::Ident(s) => s.hash(state),
            _ => {} // Keywords and operators have no data
        }
    }
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    current_char: Option<char>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.first().copied();

        Lexer {
            input: chars,
            position: 0,
            current_char,
        }
    }

    fn advance(&mut self) {
        self.position += 1;
        self.current_char = self.input.get(self.position).copied();
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.input.get(self.position + offset).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // Skip until end of line
        while let Some(ch) = self.current_char {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_number(&mut self) -> Token {
        let mut num_str = String::new();
        let mut is_float = false;

        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
                is_float = true;
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            Token::FloatLiteral(num_str.parse().unwrap())
        } else {
            Token::IntLiteral(num_str.parse().unwrap())
        }
    }

    fn read_string(&mut self) -> Token {
        self.advance(); // Skip opening quote
        let mut parts = Vec::new();
        let mut current_literal = String::new();
        let mut has_interpolation = false;

        while let Some(ch) = self.current_char {
            if ch == '"' {
                self.advance(); // Skip closing quote
                break;
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char {
                    let unescaped = match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        _ => escaped,
                    };
                    current_literal.push(unescaped);
                    self.advance();
                }
            } else if ch == '$' && self.peek(1) == Some('{') {
                // Found interpolation: ${expr}
                has_interpolation = true;

                // Save current literal part
                if !current_literal.is_empty() {
                    parts.push(StringPart::Literal(current_literal.clone()));
                    current_literal.clear();
                }

                // Skip ${
                self.advance();
                self.advance();

                // Read expression until }
                let mut expr = String::new();
                let mut brace_depth = 1;
                while let Some(expr_ch) = self.current_char {
                    if expr_ch == '{' {
                        brace_depth += 1;
                        expr.push(expr_ch);
                        self.advance();
                    } else if expr_ch == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            self.advance(); // Skip closing }
                            break;
                        }
                        expr.push(expr_ch);
                        self.advance();
                    } else {
                        expr.push(expr_ch);
                        self.advance();
                    }
                }

                parts.push(StringPart::Expression(expr));
            } else {
                current_literal.push(ch);
                self.advance();
            }
        }

        // Add final literal part if any
        if !current_literal.is_empty() || parts.is_empty() {
            parts.push(StringPart::Literal(current_literal));
        }

        if has_interpolation {
            Token::InterpolatedString(parts)
        } else {
            // Simple string with no interpolation
            if let Some(StringPart::Literal(s)) = parts.into_iter().next() {
                Token::StringLiteral(s)
            } else {
                Token::StringLiteral(String::new())
            }
        }
    }

    fn read_char(&mut self) -> Token {
        self.advance(); // Skip opening quote

        let ch = if let Some(c) = self.current_char {
            if c == '\\' {
                // Handle escape sequences
                self.advance();
                if let Some(escaped) = self.current_char {
                    let unescaped = match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '\'' => '\'',
                        '0' => '\0',
                        _ => escaped,
                    };
                    self.advance();
                    unescaped
                } else {
                    '\0'
                }
            } else {
                let character = c;
                self.advance();
                character
            }
        } else {
            '\0'
        };

        // Skip closing quote
        if self.current_char == Some('\'') {
            self.advance();
        }

        Token::CharLiteral(ch)
    }

    fn read_identifier_string(&mut self) -> String {
        let mut ident = String::new();

        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        ident
    }

    fn read_identifier(&mut self) -> Token {
        let ident = self.read_identifier_string();

        // Check for keywords
        match ident.as_str() {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "mut" => Token::Mut,
            "const" => Token::Const,
            "static" => Token::Static,
            "struct" => Token::Struct,
            "enum" => Token::Enum,
            "trait" => Token::Trait,
            "impl" => Token::Impl,
            "match" => Token::Match,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "while" => Token::While,
            "loop" => Token::Loop,
            "return" => Token::Return,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "use" => Token::Use,
            "go" => Token::Go,
            "async" => Token::Async,
            "await" => Token::Await,
            "defer" => Token::Defer,
            "pub" => Token::Pub,
            "self" => Token::Self_,
            "Self" => Token::Ident("Self".to_string()), // Capital Self is a type, not keyword
            "unsafe" => Token::Unsafe,
            "as" => Token::As,
            "where" => Token::Where,
            "type" => Token::Type,
            "dyn" => Token::Dyn,
            "bound" => Token::Bound,
            "int" => Token::Int,
            "int32" => Token::Int32,
            "uint" => Token::Uint,
            "float" => Token::Float,
            "bool" => Token::Bool,
            "string" => Token::String,
            "true" => Token::BoolLiteral(true),
            "false" => Token::BoolLiteral(false),
            "_" => Token::Underscore,
            _ => Token::Ident(ident),
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let token = match self.current_char {
            None => Token::Eof,
            Some('\n') => {
                self.advance();
                Token::Newline
            }
            Some('/') if self.peek(1) == Some('/') => {
                self.skip_comment();
                return self.next_token();
            }
            Some('"') => self.read_string(),
            Some('\'') => self.read_char(),
            Some(ch) if ch.is_ascii_digit() => self.read_number(),
            Some(ch) if ch.is_alphabetic() || ch == '_' => self.read_identifier(),
            Some('@') => {
                self.advance();
                if let Some(ch) = self.current_char {
                    if ch.is_alphabetic() || ch == '_' {
                        // Read decorator name - don't treat as keyword
                        let name = self.read_identifier_string();
                        Token::Decorator(name)
                    } else {
                        Token::At
                    }
                } else {
                    Token::At
                }
            }
            Some('+') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::PlusAssign
            }
            Some('+') => {
                self.advance();
                Token::Plus
            }
            Some('-') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::MinusAssign
            }
            Some('-') if self.peek(1) == Some('>') => {
                self.advance();
                self.advance();
                Token::Arrow
            }
            Some('-') => {
                self.advance();
                Token::Minus
            }
            Some('*') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::StarAssign
            }
            Some('*') => {
                self.advance();
                Token::Star
            }
            Some('/') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::SlashAssign
            }
            Some('/') => {
                self.advance();
                Token::Slash
            }
            Some('%') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::PercentAssign
            }
            Some('%') => {
                self.advance();
                Token::Percent
            }
            Some('=') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::Eq
            }
            Some('=') if self.peek(1) == Some('>') => {
                self.advance();
                self.advance();
                Token::FatArrow
            }
            Some('=') => {
                self.advance();
                Token::Assign
            }
            Some('!') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::Ne
            }
            Some('!') => {
                self.advance();
                Token::Bang
            }
            Some('<') if self.peek(1) == Some('-') => {
                self.advance();
                self.advance();
                Token::LeftArrow
            }
            Some('<') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::Le
            }
            Some('<') => {
                self.advance();
                Token::Lt
            }
            Some('>') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::Ge
            }
            Some('>') => {
                self.advance();
                Token::Gt
            }
            Some('&') if self.peek(1) == Some('&') => {
                self.advance();
                self.advance();
                Token::And
            }
            Some('&') => {
                self.advance();
                Token::Ampersand
            }
            Some('|') if self.peek(1) == Some('|') => {
                self.advance();
                self.advance();
                Token::Or
            }
            Some('|') if self.peek(1) == Some('>') => {
                self.advance();
                self.advance();
                Token::PipeOp
            }
            Some('|') => {
                self.advance();
                Token::Pipe
            }
            Some('(') => {
                self.advance();
                Token::LParen
            }
            Some(')') => {
                self.advance();
                Token::RParen
            }
            Some('{') => {
                self.advance();
                Token::LBrace
            }
            Some('}') => {
                self.advance();
                Token::RBrace
            }
            Some('[') => {
                self.advance();
                Token::LBracket
            }
            Some(']') => {
                self.advance();
                Token::RBracket
            }
            Some(',') => {
                self.advance();
                Token::Comma
            }
            Some('.') if self.peek(1) == Some('.') && self.peek(2) == Some('=') => {
                self.advance();
                self.advance();
                self.advance();
                Token::DotDotEq
            }
            Some('.') if self.peek(1) == Some('.') => {
                self.advance();
                self.advance();
                Token::DotDot
            }
            Some('.') => {
                self.advance();
                Token::Dot
            }
            Some(':') => {
                self.advance();
                if self.current_char == Some(':') {
                    self.advance();
                    Token::ColonColon
                } else {
                    Token::Colon
                }
            }
            Some(';') => {
                self.advance();
                Token::Semicolon
            }
            Some('?') => {
                self.advance();
                Token::Question
            }
            Some(ch) => {
                self.advance();
                panic!("Unexpected character: {}", ch);
            }
        };

        token
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            if token == Token::Eof {
                tokens.push(token);
                break;
            }
            // Skip newlines for now
            if token != Token::Newline {
                tokens.push(token);
            }
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let mut lexer = Lexer::new("fn main() { let x = 42 }");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Fn);
        assert_eq!(tokens[1], Token::Ident("main".to_string()));
        assert_eq!(tokens[2], Token::LParen);
        assert_eq!(tokens[3], Token::RParen);
    }

    #[test]
    fn test_lexer_decorators() {
        let mut lexer = Lexer::new("@route @timing @cache");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Decorator("route".to_string()));
        assert_eq!(tokens[1], Token::Decorator("timing".to_string()));
        assert_eq!(tokens[2], Token::Decorator("cache".to_string()));
    }

    #[test]
    fn test_lexer_go_keyword() {
        let mut lexer = Lexer::new("go async await");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Go);
        assert_eq!(tokens[1], Token::Async);
        assert_eq!(tokens[2], Token::Await);
    }
}
