//! Lossless token stream: every token (including comments) with its leading trivia.

use super::FormatError;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Word,
    Str,
    Char,
    Lifetime,
    Decorator,
    Punct,
    Comment,
}

/// A token with the whitespace that preceded it in the source.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Tok {
    pub kind: Kind,
    pub text: String,
    /// Newlines before this token (0 = same line as previous token).
    pub nl: usize,
    /// Whether horizontal whitespace separated it from the previous token on the same line.
    pub space: bool,
}

impl Tok {
    pub fn new(kind: Kind, text: &str) -> Tok {
        Tok {
            kind,
            text: text.to_string(),
            nl: 0,
            space: false,
        }
    }
    pub fn is(&self, text: &str) -> bool {
        self.kind == Kind::Punct && self.text == text
    }
    pub fn is_opener(&self) -> bool {
        self.kind == Kind::Punct && matches!(self.text.as_str(), "(" | "[" | "{")
    }
    pub fn is_closer(&self) -> bool {
        self.kind == Kind::Punct && matches!(self.text.as_str(), ")" | "]" | "}")
    }
    pub fn is_comment(&self) -> bool {
        self.kind == Kind::Comment
    }
}

const PUNCTS: &[&str] = &[
    "|>?", "??", "<<=", ">>=", "..=", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "|>", "+=", "-=", "*=",
    "/=", "%=", "&=", "|=", "^=", "->", "<-", "=>", "::", "..", "+", "-", "*", "/", "%", "^", "=",
    "<", ">", "!", "&", "|", "(", ")", "{", "}", "[", "]", ",", ".", ":", ";", "?", "@",
];

pub fn scan(src: &str) -> Result<Vec<Tok>, FormatError> {
    let mut s = Scanner {
        c: src.chars().collect(),
        i: 0,
        out: Vec::new(),
    };
    s.run()?;
    Ok(s.out)
}

struct Scanner {
    c: Vec<char>,
    i: usize,
    out: Vec<Tok>,
}

impl Scanner {
    fn at(&self, off: usize) -> Option<char> {
        self.c.get(self.i + off).copied()
    }

    fn run(&mut self) -> Result<(), FormatError> {
        let (mut nl, mut space) = (0usize, false);
        while let Some(ch) = self.at(0) {
            if ch == '\n' {
                nl += 1;
                space = false;
                self.i += 1;
            } else if ch.is_whitespace() {
                space = true;
                self.i += 1;
            } else {
                let (kind, text) = self.next_token(ch)?;
                self.out.push(Tok {
                    kind,
                    text,
                    nl,
                    space,
                });
                nl = 0;
                space = false;
            }
        }
        Ok(())
    }

    fn next_token(&mut self, ch: char) -> Result<(Kind, String), FormatError> {
        let start = self.i;
        let kind = if ch == '/' && self.at(1) == Some('/') {
            while self.at(0).is_some_and(|c| c != '\n') {
                self.i += 1;
            }
            Kind::Comment
        } else if ch == '/' && self.at(1) == Some('*') {
            self.block_comment()?;
            Kind::Comment
        } else if ch == 'r' && self.is_raw_string() {
            self.raw_string();
            Kind::Str
        } else if ch == '"' {
            self.string();
            Kind::Str
        } else if ch == '\'' {
            self.quote()
        } else if ch.is_ascii_digit() {
            self.number();
            Kind::Word
        } else if ch.is_alphabetic() || ch == '_' {
            self.ident();
            Kind::Word
        } else if ch == '@' && self.at(1).is_some_and(|c| c.is_alphabetic() || c == '_') {
            self.i += 1;
            while self.at(0).is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                self.i += 1;
            }
            Kind::Decorator
        } else {
            self.punct(ch)?
        };
        let text: String = self.c[start..self.i].iter().collect();
        let text = if kind == Kind::Comment {
            text.trim_end().to_string()
        } else {
            text
        };
        Ok((kind, text))
    }

    /// `/* .. */`, nested like the compiler lexer; kept verbatim (it may span lines).
    fn block_comment(&mut self) -> Result<(), FormatError> {
        self.i += 2;
        let mut depth = 1;
        while let Some(c) = self.at(0) {
            if c == '/' && self.at(1) == Some('*') {
                depth += 1;
                self.i += 2;
            } else if c == '*' && self.at(1) == Some('/') {
                depth -= 1;
                self.i += 2;
                if depth == 0 {
                    return Ok(());
                }
            } else {
                self.i += 1;
            }
        }
        Err(FormatError::Lex("unterminated block comment".to_string()))
    }

    fn punct(&mut self, ch: char) -> Result<Kind, FormatError> {
        let rest: String = self.c[self.i..(self.i + 3).min(self.c.len())].iter().collect();
        match PUNCTS.iter().find(|p| rest.starts_with(**p)) {
            Some(p) => {
                self.i += p.len();
                Ok(Kind::Punct)
            }
            None => Err(FormatError::Lex(format!("unexpected character '{}'", ch))),
        }
    }

    fn is_raw_string(&self) -> bool {
        let mut k = 1;
        while self.at(k) == Some('#') {
            k += 1;
        }
        k > 1 && self.at(k) == Some('"')
    }

    fn raw_string(&mut self) {
        self.i += 1;
        let mut hashes = 0;
        while self.at(0) == Some('#') {
            hashes += 1;
            self.i += 1;
        }
        self.i += 1;
        while let Some(ch) = self.at(0) {
            if ch == '"' && (0..hashes).all(|k| self.at(1 + k) == Some('#')) {
                self.i += 1 + hashes;
                return;
            }
            self.i += 1;
        }
    }

    /// Mirrors the compiler lexer: `${...}` runs to the matching brace, quotes included.
    fn string(&mut self) {
        self.i += 1;
        while let Some(ch) = self.at(0) {
            if ch == '"' {
                self.i += 1;
                return;
            }
            if ch == '\\' {
                self.i += 2;
            } else if ch == '$' && self.at(1) == Some('$') {
                self.i += 2;
            } else if ch == '$'
                && self.at(1) == Some('{')
                && (self.at(2) != Some('"') || self.interpolation_starts_with_string())
            {
                self.i += 2;
                self.skip_interpolation();
            } else {
                self.i += 1;
            }
        }
    }

    /// Same rule as the compiler lexer (`src/lexer/string_lexing.rs`; kept in sync by
    /// tests/parser_grammar_roundtrip_test.rs): `${"` opens an interpolation only when the
    /// nested string literal is followed by `.` or `[`.
    fn interpolation_starts_with_string(&self) -> bool {
        let mut k = 3;
        while let Some(c) = self.at(k) {
            match c {
                '\\' => k += 2,
                '"' => return matches!(self.at(k + 1), Some('.') | Some('[')),
                '\n' => return false,
                _ => k += 1,
            }
        }
        false
    }

    /// Skip a nested string literal (cursor on its opening quote).
    fn skip_nested_string(&mut self) {
        self.i += 1;
        while let Some(c) = self.at(0) {
            self.i += 1;
            if c == '\\' {
                self.i += 1;
            } else if c == '"' {
                return;
            }
        }
    }

    fn skip_interpolation(&mut self) {
        let mut depth = 1;
        while let Some(ch) = self.at(0) {
            if ch == '"' {
                self.skip_nested_string();
                continue;
            }
            if ch == '\'' {
                let len = match (self.at(1), self.at(2), self.at(3)) {
                    (Some('\\'), Some(_), Some('\'')) => 4,
                    (Some(c), Some('\''), _) if c != '\\' && c != '\'' => 3,
                    _ => 1,
                };
                self.i += len;
                continue;
            }
            self.i += 1;
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    fn quote(&mut self) -> Kind {
        let lifetime = match self.at(1) {
            Some('\\') => false,
            Some(c) if c.is_alphabetic() || c == '_' => self.at(2) != Some('\''),
            _ => false,
        };
        self.i += 1;
        if lifetime {
            while self.at(0).is_some_and(|c| c.is_alphanumeric() || c == '_') {
                self.i += 1;
            }
            return Kind::Lifetime;
        }
        if self.at(0) == Some('\\') {
            self.i += 1;
        }
        self.i += 1;
        if self.at(0) == Some('\'') {
            self.i += 1;
        }
        Kind::Char
    }

    fn ident(&mut self) {
        if self.at(0) == Some('r') && self.at(1) == Some('#') {
            self.i += 2;
        }
        while self.at(0).is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.i += 1;
        }
    }

    fn number(&mut self) {
        let after_dot = self.out.last().is_some_and(|t| t.is("."));
        let start = self.i;
        let hex = self.at(0) == Some('0') && matches!(self.at(1), Some('x' | 'X' | 'b' | 'o'));
        self.eat_alnum();
        if after_dot || hex {
            return;
        }
        if self.at(0) == Some('.') && self.at(1).is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
            self.eat_alnum();
        }
        let last = self.c[self.i - 1];
        let signed_exp = matches!(last, 'e' | 'E')
            && matches!(self.at(0), Some('+' | '-'))
            && self.at(1).is_some_and(|c| c.is_ascii_digit())
            && self.c[start..self.i].iter().all(|c| c.is_ascii_digit() || *c == '.' || *c == '_' || *c == 'e' || *c == 'E');
        if signed_exp {
            self.i += 1;
            self.eat_alnum();
        }
    }

    fn eat_alnum(&mut self) {
        while self.at(0).is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.i += 1;
        }
    }
}
