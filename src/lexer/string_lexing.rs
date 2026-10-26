use super::{Lexer, StringPart, Token};

impl Lexer {
    /// Length (in chars) of a char literal starting at the cursor (`'x'`, `'\\n'`), or 0.
    fn nested_char_literal_len(&self) -> usize {
        match (self.peek(1), self.peek(2), self.peek(3)) {
            (Some('\\'), Some(_), Some('\'')) => 4,
            (Some(c), Some('\''), _) if c != '\\' && c != '\'' => 3,
            _ => 0,
        }
    }

    /// Copy a nested string literal (cursor on its opening quote) verbatim into `out`.
    fn copy_nested_string(&mut self, out: &mut String) {
        out.push('"');
        self.advance();
        while let Some(c) = self.current_char {
            out.push(c);
            self.advance();
            if c == '\\' {
                if let Some(e) = self.current_char {
                    out.push(e);
                    self.advance();
                }
            } else if c == '"' {
                break;
            }
        }
    }

    /// Cursor is on `$` of `${"`. True when the nested string literal is immediately followed by
    /// `.` or `[`, i.e. it is the receiver of a call/index and not a bare literal followed by `}`.
    fn interpolation_starts_with_string(&self) -> bool {
        let mut i = 3; // after `${"`
        while let Some(c) = self.peek(i) {
            match c {
                '\\' => i += 2,
                '"' => return matches!(self.peek(i + 1), Some('.') | Some('[')),
                '\n' => return false,
                _ => i += 1,
            }
        }
        false
    }

    /// At `escaped` (the character after a backslash): `u{` + 1 to 6 hex digits + `}` names a Unicode scalar value.
    /// Consumes the escape and returns the character when well formed; otherwise consumes nothing.
    fn read_numeric_escape(&mut self, escaped: char) -> Option<char> {
        if escaped != 'u' || self.peek(1) != Some('{') {
            return None;
        }
        let mut offset = 2;
        let mut value: u32 = 0;
        let mut seen = 0;
        while seen < 6 {
            match self.peek(offset).and_then(|c| c.to_digit(16)) {
                Some(d) => value = value * 16 + d,
                None => break,
            }
            offset += 1;
            seen += 1;
        }
        if seen == 0 || self.peek(offset) != Some('}') {
            return None;
        }
        let c = char::from_u32(value)?;
        for _ in 0..=offset {
            self.advance();
        }
        Some(c)
    }

    pub(in crate::lexer) fn read_string(&mut self) -> Token {
        self.advance(); // Skip opening quote
        let mut parts = Vec::new();
        let mut current_literal = String::new();
        let mut has_interpolation = false;

        let mut terminated = false;
        while let Some(ch) = self.current_char {
            if ch == '"' {
                self.advance(); // Skip closing quote
                terminated = true;
                break;
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char {
                    // `\u{H..}` names a character by code point; anything malformed keeps the old meaning
                    // (the escaped character itself)
                    if let Some(c) = self.read_numeric_escape(escaped) {
                        current_literal.push(c);
                        continue;
                    }
                    let unescaped = match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
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
                // `${"` is a literal dollar-brace unless a string literal followed by a method
                // call / index starts the interpolation (`"${"abc".len()}"`).
                if self.peek(2) == Some('"') && !self.interpolation_starts_with_string() {
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
                    self.interp_starts.push((self.line, self.column, self.byte_pos));

                    // Read expression until the matching }, skipping over nested string and
                    // char literals so quotes and braces inside them do not end the expression.
                    let mut expr = String::new();
                    let mut brace_depth = 1;
                    while let Some(expr_ch) = self.current_char {
                        if expr_ch == '\\' && self.peek(1) == Some('"') {
                            // `\"` inside `${..}` is a quote of the expression (`"${f(\"x\")}"`).
                            expr.push('"');
                            self.advance();
                            self.advance();
                        } else if expr_ch == '"' {
                            self.copy_nested_string(&mut expr);
                        } else if expr_ch == '\'' && self.nested_char_literal_len() > 0 {
                            for _ in 0..self.nested_char_literal_len() {
                                expr.extend(self.current_char);
                                self.advance();
                            }
                        } else if expr_ch == '{' {
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

        if !terminated {
            return Token::Invalid("unterminated string literal".to_string());
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
            return Token::Invalid("expected '\"' after r#+ in raw string literal".to_string());
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
        let start = self.position;
        self.advance(); // Skip opening quote

        // `''` is an empty char literal, not the char `'`; a quote at the end of a line or file
        // opens nothing (the newline is not its character).
        if self.current_char == Some('\'') {
            self.advance();
            return Token::Invalid("empty char literal `''`".to_string());
        }
        if matches!(self.current_char, None | Some('\n')) {
            return Token::Invalid("unterminated char literal `'`".to_string());
        }

        let ch = if let Some(c) = self.current_char {
            if c == '\\' {
                // Handle escape sequences
                self.advance();
                if let Some(escaped) = self.current_char {
                    if let Some(c) = self.read_numeric_escape(escaped) {
                        c
                    } else {
                        let unescaped = match escaped {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '\'' => '\'',
                            '"' => '"',
                            '0' => '\0',
                            _ => escaped,
                        };
                        self.advance();
                        unescaped
                    }
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

        // The closing quote is required: `'a` followed by anything else is malformed.
        if self.current_char == Some('\'') {
            self.advance();
            Token::CharLiteral(ch)
        } else {
            Token::Invalid(format!("unterminated char literal `{}`", self.text_since(start).escape_debug()))
        }
    }
}
