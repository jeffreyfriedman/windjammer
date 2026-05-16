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
    Mod,
    Extern,
    Thread,
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
    IntLiteral(i64),                 // No suffix: 42
    IntLiteralSuffixed(i64, String), // With suffix: 42u32, 0usize
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

    // Bitwise operators
    Caret, // ^ (XOR)
    Shl,   // <<
    Shr,   // >>

    Assign,
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    AndAssign,     // &=
    OrAssign,      // |=
    XorAssign,     // ^=
    ShlAssign,     // <<=
    ShrAssign,     // >>=
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

    // Documentation
    DocComment(String), // /// doc comment content
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "Ident({})", s),
            Token::IntLiteral(n) => write!(f, "Int({})", n),
            Token::IntLiteralSuffixed(n, ref s) => write!(f, "Int({}{})", n, s),
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
            Token::IntLiteralSuffixed(n, ref s) => {
                n.hash(state);
                s.hash(state);
            }
            Token::FloatLiteral(f) => f.to_bits().hash(state), // Hash bits of f64
            Token::StringLiteral(s) => s.hash(state),
            Token::InterpolatedString(parts) => parts.hash(state),
            Token::CharLiteral(c) => c.hash(state),
            Token::BoolLiteral(b) => b.hash(state),
            Token::Ident(s) => s.hash(state),
            Token::DocComment(s) => s.hash(state),
            Token::Decorator(s) => s.hash(state),
            _ => {} // Keywords and operators have no data
        }
    }
}

/// Token with source location information for error reporting
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenWithLocation {
    pub token: Token,
    pub line: usize,
    pub column: usize,
}
