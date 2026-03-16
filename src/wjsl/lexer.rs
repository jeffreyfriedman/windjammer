//! WJSL Lexer - Tokenizer for RFC-style shader syntax
//!
//! Handles WGSL-like tokens: @vertex, @fragment, @group, @binding, etc.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Struct,
    Let,
    Var,
    Return,
    If,
    Else,
    For,
    While,
    Loop,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Discard,
    True,
    False,

    // Storage/type keywords
    Uniform,
    Storage,
    Read,
    Write,
    ReadWrite,
    Texture2d,
    Sampler,
    Array,
    Type,

    // Builtin types
    F32,
    F64,
    U32,
    I32,
    Bool,
    Vec2,
    Vec3,
    Vec4,
    Mat2x2,
    Mat3x3,
    Mat4x4,
    Texture2dType,
    TextureCubeType,
    Texture3dType,
    SamplerType,
    SamplerComparisonType,

    // Attributes (after @)
    AtVertex,
    AtFragment,
    AtCompute,
    AtWorkgroupSize,
    AtGroup,
    AtBinding,
    AtLocation,
    AtBuiltin,
    AtAlign,
    AtSize,

    // Identifiers and literals
    Ident(String),
    IntLiteral(u64),
    FloatLiteral(f64),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Assign,
    Arrow,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    Comma,
    Semicolon,
    Colon,
    Dot,

    // Special
    At,
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "Ident({})", s),
            Token::IntLiteral(n) => write!(f, "Int({})", n),
            Token::FloatLiteral(x) => write!(f, "Float({})", x),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// Token with source location
#[derive(Debug, Clone)]
pub struct TokenWithLoc {
    pub token: Token,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Current byte position in the source (for body extraction)
    pub fn position(&self) -> usize {
        self.position
    }

    /// The source string being lexed
    pub fn source(&self) -> &'a str {
        self.input
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += ch.len_utf8();
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.next();
            } else if c == '/' && self.chars.clone().nth(1) == Some('/') {
                // Line comment
                self.next();
                self.next();
                while self.peek().map(|c| c != '\n').unwrap_or(false) {
                    self.next();
                }
            } else {
                break;
            }
        }
    }

    fn read_ident(&mut self, first: char) -> String {
        let mut s = String::from(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.next();
            } else {
                break;
            }
        }
        s
    }

    fn read_number(&mut self, first: char) -> Token {
        let mut s = String::from(first);
        let mut is_float = first == '.';

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.next();
            } else if c == '.' && !is_float {
                is_float = true;
                s.push(c);
                self.next();
            } else if (c == 'u' || c == 'i') && !is_float {
                // Type suffix: 0u, 32u, 1i
                let suffix = self.read_type_suffix();
                return if let Ok(n) = s.parse::<u64>() {
                    Token::IntLiteral(n)
                } else {
                    Token::Ident(s + &suffix)
                };
            } else if c == 'e' || c == 'E' {
                s.push(c);
                self.next();
                if self.peek() == Some('-') || self.peek() == Some('+') {
                    s.push(self.next().unwrap());
                }
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        s.push(c);
                        self.next();
                    } else {
                        break;
                    }
                }
                is_float = true;
            } else {
                break;
            }
        }

        if is_float {
            Token::FloatLiteral(s.parse().unwrap_or(0.0))
        } else {
            Token::IntLiteral(s.parse().unwrap_or(0))
        }
    }

    fn read_type_suffix(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.next();
            } else {
                break;
            }
        }
        s
    }

    fn read_at_attr(&mut self) -> Token {
        let first = self.next().unwrap();
        let ident = self.read_ident(first);
        match ident.as_str() {
            "vertex" => Token::AtVertex,
            "fragment" => Token::AtFragment,
            "compute" => Token::AtCompute,
            "workgroup_size" => Token::AtWorkgroupSize,
            "group" => Token::AtGroup,
            "binding" => Token::AtBinding,
            "location" => Token::AtLocation,
            "builtin" => Token::AtBuiltin,
            "align" => Token::AtAlign,
            "size" => Token::AtSize,
            "position" => Token::AtBuiltin, // @builtin(position) shorthand
            _ => Token::Ident(ident),
        }
    }

    fn keyword_or_ident(&self, s: &str) -> Token {
        match s {
            "fn" => Token::Fn,
            "struct" => Token::Struct,
            "let" => Token::Let,
            "var" => Token::Var,
            "return" => Token::Return,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "while" => Token::While,
            "loop" => Token::Loop,
            "switch" => Token::Switch,
            "case" => Token::Case,
            "default" => Token::Default,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "discard" => Token::Discard,
            "true" => Token::True,
            "false" => Token::False,
            "uniform" => Token::Uniform,
            "storage" => Token::Storage,
            "read" => Token::Read,
            "write" => Token::Write,
            "read_write" => Token::ReadWrite,
            "texture_2d" => Token::Texture2d,
            "sampler" => Token::Sampler,
            "array" => Token::Array,
            "type" => Token::Type,
            "f32" => Token::F32,
            "f64" => Token::F64,
            "u32" => Token::U32,
            "i32" => Token::I32,
            "bool" => Token::Bool,
            "vec2" => Token::Vec2,
            "vec3" => Token::Vec3,
            "vec4" => Token::Vec4,
            "mat2x2" => Token::Mat2x2,
            "mat3x3" => Token::Mat3x3,
            "mat4x4" => Token::Mat4x4,
            "texture_cube" => Token::TextureCubeType,
            "texture_3d" => Token::Texture3dType,
            "sampler_comparison" => Token::SamplerComparisonType,
            _ => Token::Ident(s.to_string()),
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let c = match self.next() {
            Some(c) => c,
            None => return Token::Eof,
        };

        match c {
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            ':' => Token::Colon,
            '.' => Token::Dot,
            '@' => {
                let next = self.peek();
                if next.map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                    self.read_at_attr()
                } else {
                    Token::At
                }
            }
            '+' => Token::Plus,
            '-' => {
                if self.peek() == Some('>') {
                    self.next();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '=' => {
                if self.peek() == Some('=') {
                    self.next();
                    Token::EqEq
                } else {
                    Token::Assign
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.next();
                    Token::Ne
                } else {
                    Token::Not
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.next();
                    Token::Le
                } else if self.peek() == Some('<') {
                    self.next();
                    Token::Shl
                } else {
                    Token::LAngle
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.next();
                    Token::Ge
                } else if self.peek() == Some('>') {
                    self.next();
                    Token::Shr
                } else {
                    Token::RAngle
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.next();
                    Token::And
                } else {
                    Token::BitAnd
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.next();
                    Token::Or
                } else {
                    Token::BitOr
                }
            }
            '^' => Token::BitXor,
            c if c.is_ascii_digit() => self.read_number(c),
            c if c.is_alphabetic() || c == '_' => {
                let s = self.read_ident(c);
                self.keyword_or_ident(&s)
            }
            _ => Token::Eof,
        }
    }
}
