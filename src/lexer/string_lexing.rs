use super::{Lexer, StringPart, Token};

impl Lexer {
    pub(in crate::lexer) fn read_string(&mut self) -> Token {
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
                        '`' => '`',
                        '$' => '$',
                        _ => escaped,
                    };
                    current_literal.push(unescaped);
                    self.advance();
                }
            } else if ch == '$' && self.peek(1) == Some('$') {
                // Escaped dollar: $$ → literal $
                current_literal.push('$');
                self.advance();
                self.advance();
            } else if ch == '$' && self.peek(1) == Some('{') {
                // `${"` in source is a literal dollar-brace, not interpolation.
                if self.peek(2) == Some('"') {
                    current_literal.push('$');
                    current_literal.push('{');
                    self.advance();
                    self.advance();
                } else {
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
                }
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

    pub(in crate::lexer) fn read_raw_string(&mut self) -> Token {
        self.advance(); // r
        let mut hash_count = 0usize;
        while self.current_char == Some('#') {
            hash_count += 1;
            self.advance();
        }
        if self.current_char != Some('"') {
            panic!("Expected '\"' after r#+ in raw string literal");
        }
        self.advance(); // opening "

        let mut content = String::new();

        'read: while let Some(ch) = self.current_char {
            if ch == '"' {
                let closes = (0..hash_count).all(|i| self.peek(1 + i) == Some('#'));
                if closes {
                    self.advance(); // "
                    for _ in 0..hash_count {
                        self.advance();
                    }
                    break 'read;
                }
            }
            content.push(ch);
            self.advance();
        }

        Token::StringLiteral(content)
    }

    pub(in crate::lexer) fn read_lifetime(&mut self) -> Token {
        self.advance(); // opening '
        let mut name = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        Token::Lifetime(name)
    }

    pub(in crate::lexer) fn read_char(&mut self) -> Token {
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
}
