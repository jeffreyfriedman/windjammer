use super::{Lexer, Token};

impl Lexer {
    pub(in crate::lexer) fn read_number(&mut self) -> Token {
        let int_only_after_field_dot = self.numeric_field_index_after_dot;
        self.numeric_field_index_after_dot = false;

        let mut num_str = String::new();
        let mut is_float = false;

        // Check for hex, binary, or octal prefix
        if self.current_char == Some('0') {
            if let Some(prefix) = self.peek(1) {
                match prefix {
                    'x' | 'X' => {
                        // Hexadecimal: 0xFF, 0xDEADBEEF
                        self.advance(); // skip '0'
                        self.advance(); // skip 'x'

                        let mut hex_str = String::new();
                        while let Some(ch) = self.current_char {
                            if ch.is_ascii_hexdigit() {
                                hex_str.push(ch);
                                self.advance();
                            } else if ch == '_' {
                                self.advance(); // skip underscore separator
                            } else {
                                break;
                            }
                        }

                        let value = i64::from_str_radix(&hex_str, 16).expect("Invalid hex literal");
                        return Token::IntLiteral(value);
                    }
                    'b' | 'B' => {
                        // Binary: 0b1010, 0b1111_0000
                        self.advance(); // skip '0'
                        self.advance(); // skip 'b'

                        let mut bin_str = String::new();
                        while let Some(ch) = self.current_char {
                            if ch == '0' || ch == '1' {
                                bin_str.push(ch);
                                self.advance();
                            } else if ch == '_' {
                                self.advance(); // skip underscore separator
                            } else {
                                break;
                            }
                        }

                        let value =
                            i64::from_str_radix(&bin_str, 2).expect("Invalid binary literal");
                        return Token::IntLiteral(value);
                    }
                    'o' | 'O' => {
                        // Octal: 0o755, 0o644
                        self.advance(); // skip '0'
                        self.advance(); // skip 'o'

                        let mut oct_str = String::new();
                        while let Some(ch) = self.current_char {
                            if ('0'..='7').contains(&ch) {
                                oct_str.push(ch);
                                self.advance();
                            } else if ch == '_' {
                                self.advance(); // skip underscore separator
                            } else {
                                break;
                            }
                        }

                        let value =
                            i64::from_str_radix(&oct_str, 8).expect("Invalid octal literal");
                        return Token::IntLiteral(value);
                    }
                    _ => {
                        // Regular decimal number starting with 0
                    }
                }
            }
        }

        // Regular decimal number (or float)
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '_' {
                // Skip underscores in numeric literals (e.g., 1_000_000)
                self.advance();
            } else if ch == '.'
                && !is_float
                && !int_only_after_field_dot
                && self.peek(1).is_some_and(|c| c.is_ascii_digit())
            {
                is_float = true;
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Scientific notation: e.g. 1e10, 2.5e-3, 3E+2
        if !int_only_after_field_dot
            && (self.current_char == Some('e') || self.current_char == Some('E'))
        {
            num_str.push(self.current_char.unwrap());
            self.advance();
            if self.current_char == Some('-') || self.current_char == Some('+') {
                num_str.push(self.current_char.unwrap());
                self.advance();
            }
            while let Some(ch) = self.current_char {
                if ch.is_ascii_digit() {
                    num_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            is_float = true;
        }

        // TDD FIX: Handle type suffixes for integer literals (0u64, 0i32, 0u32, etc.)
        // Without this, "0u64" gets tokenized as IntLiteral(0) + Ident("u64"),
        // which causes the "u64" to become a stray expression statement "u64;"
        // resulting in E0423: expected value, found builtin type `u64`
        if !is_float {
            // Check for type suffix: u64, i64, u32, i32, u16, i16, u8, i8, usize, isize
            let _type_suffix = if self.current_char == Some('u') || self.current_char == Some('i') {
                let mut suffix = String::new();
                while let Some(ch) = self.current_char {
                    if ch.is_ascii_alphanumeric() {
                        suffix.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }

                // Validate it's a real type suffix
                match suffix.as_str() {
                    s if crate::type_classification::is_numeric_suffix(s) => Some(suffix),
                    _ => {
                        // Not a valid type suffix, backtrack
                        // This handles cases like "0ux" which should be "0" + "ux" (identifier)
                        for _ in 0..suffix.len() {
                            self.position -= 1;
                        }
                        self.current_char = if self.position < self.input.len() {
                            Some(self.input[self.position])
                        } else {
                            None
                        };
                        None
                    }
                }
            } else {
                None
            };

            if let Some(suffix) = _type_suffix {
                Token::IntLiteralSuffixed(num_str.parse().unwrap(), suffix)
            } else {
                Token::IntLiteral(num_str.parse().unwrap())
            }
        } else {
            Token::FloatLiteral(num_str.parse().unwrap())
        }
    }
}
