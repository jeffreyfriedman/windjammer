mod identifier_lexing;
mod number_lexing;
mod string_lexing;
mod token_types;

pub use token_types::{StringPart, Token, TokenWithLocation};

pub struct Lexer {
    pub(in crate::lexer) input: Vec<char>,
    pub(in crate::lexer) position: usize,
    pub(in crate::lexer) current_char: Option<char>,
    pub(in crate::lexer) line: usize,
    pub(in crate::lexer) column: usize,
    /// After a `.` token, the next numeric literal is a tuple/field index: do not merge `0.0` into
    /// one float (so `outer.0.0` tokenizes as `0` `.` `0`, not `0.0`).
    pub(in crate::lexer) numeric_field_index_after_dot: bool,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.first().copied();

        Lexer {
            input: chars,
            position: 0,
            current_char,
            line: 1,
            column: 1,
            numeric_field_index_after_dot: false,
        }
    }

    pub(in crate::lexer) fn advance(&mut self) {
        // Track newlines for line/column tracking
        if self.current_char == Some('\n') {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.position += 1;
        self.current_char = self.input.get(self.position).copied();
    }

    pub(in crate::lexer) fn peek(&self, offset: usize) -> Option<char> {
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

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        // Pending "numeric field index after ." only applies if the next token is a number
        if self.numeric_field_index_after_dot
            && !matches!(self.current_char, Some(c) if c.is_ascii_digit())
        {
            self.numeric_field_index_after_dot = false;
        }

        let token = match self.current_char {
            None => Token::Eof,
            Some('\n') => {
                self.advance();
                Token::Newline
            }
            Some('/')
                if self.peek(1) == Some('/')
                    && self.peek(2) == Some('/')
                    && self.peek(3) != Some('/') =>
            {
                // Doc comment: /// text
                self.advance(); // skip first /
                self.advance(); // skip second /
                self.advance(); // skip third /

                // Skip one leading space if present
                if self.current_char == Some(' ') {
                    self.advance();
                }

                // Read content until end of line
                let mut content = String::new();
                while let Some(ch) = self.current_char {
                    if ch == '\n' {
                        break;
                    }
                    content.push(ch);
                    self.advance();
                }
                Token::DocComment(content)
            }
            Some('/') if self.peek(1) == Some('/') => {
                // Regular comment: // text
                self.skip_comment();
                return self.next_token();
            }
            Some('r') if self.peek(1) == Some('#') && self.peek(2) == Some('"') => {
                // Raw string literal: r#"..."#
                self.read_raw_string()
            }
            Some('"') => self.read_string(),
            Some('\'') => self.read_char(),
            Some(ch) if ch.is_ascii_digit() => self.read_number(),
            Some(ch) if ch.is_alphabetic() || ch == '_' => self.read_identifier(),
            Some('@') => {
                self.advance();
                if let Some(ch) = self.current_char {
                    if ch.is_alphabetic() || ch == '_' {
                        // Read decorator name - support paths like @tokio.main
                        let mut name = String::new();
                        while let Some(ch) = self.current_char {
                            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                                name.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        }
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
            Some('^') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::XorAssign
            }
            Some('^') => {
                self.advance();
                Token::Caret
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
            Some('<') if self.peek(1) == Some('<') && self.peek(2) == Some('=') => {
                self.advance();
                self.advance();
                self.advance();
                Token::ShlAssign
            }
            Some('<') if self.peek(1) == Some('<') => {
                self.advance();
                self.advance();
                Token::Shl
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
            Some('>') if self.peek(1) == Some('>') && self.peek(2) == Some('=') => {
                self.advance();
                self.advance();
                self.advance();
                Token::ShrAssign
            }
            Some('>') if self.peek(1) == Some('>') => {
                self.advance();
                self.advance();
                Token::Shr
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
            Some('&') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::AndAssign
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
            Some('|') if self.peek(1) == Some('=') => {
                self.advance();
                self.advance();
                Token::OrAssign
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
                self.numeric_field_index_after_dot = true;
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

    /// Get next token with its source location
    pub fn next_token_with_location(&mut self) -> TokenWithLocation {
        let line = self.line;
        let column = self.column;
        let token = self.next_token();
        TokenWithLocation {
            token,
            line,
            column,
        }
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

    /// Tokenize the entire input with source locations
    pub fn tokenize_with_locations(&mut self) -> Vec<TokenWithLocation> {
        let mut tokens = Vec::new();

        loop {
            let token_with_loc = self.next_token_with_location();
            let is_eof = token_with_loc.token == Token::Eof;
            let is_newline = token_with_loc.token == Token::Newline;

            if is_eof {
                tokens.push(token_with_loc);
                break;
            }
            // Skip newlines for now
            if !is_newline {
                tokens.push(token_with_loc);
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
    fn test_lexer_spawn_keyword() {
        let mut lexer = Lexer::new("thread async await");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Thread);
        assert_eq!(tokens[1], Token::Async);
        assert_eq!(tokens[2], Token::Await);
    }

    #[test]
    fn test_lexer_doc_comment() {
        let mut lexer = Lexer::new("/// This is a doc comment\nfn foo() {}");
        let tokens = lexer.tokenize();

        assert_eq!(
            tokens[0],
            Token::DocComment("This is a doc comment".to_string())
        );
        assert_eq!(tokens[1], Token::Fn);
        assert_eq!(tokens[2], Token::Ident("foo".to_string()));
    }

    #[test]
    fn test_lexer_doc_comment_no_space() {
        let mut lexer = Lexer::new("///No leading space\nfn bar() {}");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::DocComment("No leading space".to_string()));
        assert_eq!(tokens[1], Token::Fn);
    }

    #[test]
    fn test_lexer_regular_comment_not_doc() {
        // Regular comments should be skipped, not captured as DocComment
        let mut lexer = Lexer::new("// This is a regular comment\nfn baz() {}");
        let tokens = lexer.tokenize();

        // Regular comment is skipped, first token should be Fn
        assert_eq!(tokens[0], Token::Fn);
        assert_eq!(tokens[1], Token::Ident("baz".to_string()));
    }

    #[test]
    fn test_lexer_four_slashes_not_doc() {
        // //// should be treated as regular comment, not doc comment
        let mut lexer = Lexer::new("//// This is not a doc comment\nfn qux() {}");
        let tokens = lexer.tokenize();

        // //// is regular comment, skipped
        assert_eq!(tokens[0], Token::Fn);
    }
}
