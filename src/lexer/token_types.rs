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
    Lifetime(String),
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
    PipeTry,    // |>? (short-circuiting pipe: `x |>? f` is `f(x?)`)
    Underscore, // _
    Bang,       // ! (for macro invocations)

    // Special
    /// Text the lexer could not tokenize (unexpected character, malformed or out-of-range
    /// number). Never panics: the parser reports it at its location.
    Unknown(String),
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
            Token::Lifetime(s) => s.hash(state),
            Token::BoolLiteral(b) => b.hash(state),
            Token::Ident(s) => s.hash(state),
            Token::DocComment(s) => s.hash(state),
            Token::Decorator(s) => s.hash(state),
            Token::Unknown(s) => s.hash(state),
            _ => {} // Keywords and operators have no data
        }
    }
}

/// Token with source location information for error reporting
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenWithLocation {
    pub token: Token,
    /// 1-based line of the first character of the token.
    pub line: usize,
    /// 1-based column (in chars) of the first character of the token.
    pub column: usize,
    /// 0-based byte offset of the token start.
    pub byte_start: usize,
    /// 0-based byte offset one past the token end (half-open).
    pub byte_end: usize,
    /// 1-based line of the position one past the last character (exclusive end).
    pub end_line: usize,
    /// 1-based column of the position one past the last character (exclusive end).
    pub end_column: usize,
    /// For `InterpolatedString`: where each `${expr}` body starts, as `(line, column, byte)` in
    /// the original text (one entry per `StringPart::Expression`, in order). Empty otherwise.
    pub interpolation_starts: Vec<(usize, usize, usize)>,
}
