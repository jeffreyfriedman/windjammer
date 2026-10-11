use super::{Lexer, Token};

/// Build the integer token for `value` (already radix-decoded, `_` removed).
///
/// Typing rule (one place, tested by `tests/lexer_integer_literal_test.rs`):
/// * explicit suffix: the literal carries that suffix; its payload is the two's-complement bit
///   pattern of the value (`bits as i64`), so `u64`/`usize` payloads above `i64::MAX` are
///   negative `i64`s. Consumers must reinterpret with `as u64`.
/// * no suffix, `<= i64::MAX`: plain `IntLiteral`.
/// * no suffix, exactly 2^63: the `i64::MIN` bit pattern (pinned by `hex_literal_u64_test`).
/// * no suffix, `<= u64::MAX`: `u64`-suffixed (an unsuffixed constant that does not fit `i64`
///   can only be a `u64`; this is what FNV-1a / xorshift constants need).
/// * anything wider than 64 bits: `Token::Invalid` with a diagnostic, never a panic.
///   (128-bit literals need a wider `Literal` payload; see docs/design/lexical-grammar.md.)
pub(in crate::lexer) fn int_token(value: u128, suffix: Option<String>) -> Token {
    if value > u64::MAX as u128 {
        return Token::Invalid(format!(
            "integer literal `{value}` does not fit in 64 bits (128-bit integer literals are not supported yet)"
        ));
    }
    match suffix {
        Some(s) => {
            if s.starts_with('i') && value > i64::MAX as u128 {
                return Token::Invalid(format!(
                    "integer literal `{value}` is out of range for `{s}`"
                ));
            }
            Token::IntLiteralSuffixed(value as u64 as i64, s)
        }
        None if value <= i64::MAX as u128 => Token::IntLiteral(value as i64),
        None if value == 1u128 << 63 => Token::IntLiteral(i64::MIN),
        None => Token::IntLiteralSuffixed(value as u64 as i64, "u64".to_string()),
    }
}

impl Lexer {
    /// Consume an optional numeric type suffix (`u64`, `i32`, `usize`, ...). Leaves the cursor
    /// untouched (and returns `None`) when the letters are not a real suffix (`0ux` is `0`, `ux`).
    fn read_int_suffix(&mut self) -> Option<String> {
        if self.current_char != Some('u') && self.current_char != Some('i') {
            return None;
        }
        let mut suffix = String::new();
        let mut ahead = 0;
        while let Some(ch) = self.peek(ahead) {
            if ch.is_ascii_alphanumeric() {
                suffix.push(ch);
                ahead += 1;
            } else {
                break;
            }
        }
        if crate::type_classification::is_numeric_suffix(&suffix) {
            for _ in 0..ahead {
                self.advance();
            }
            Some(suffix)
        } else {
            None
        }
    }

    /// Read digits of `radix` (with `_` separators) after a `0x`/`0b`/`0o` prefix.
    fn read_radix_literal(&mut self, radix: u32, name: &str) -> Token {
        self.advance(); // skip '0'
        self.advance(); // skip the radix letter

        let mut digits = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_digit(radix) {
                digits.push(ch);
                self.advance();
            } else if ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        if digits.is_empty() {
            return Token::Invalid(format!("{name} literal has no digits"));
        }
        let suffix = self.read_int_suffix();
        match u128::from_str_radix(&digits, radix) {
            Ok(v) => int_token(v, suffix),
            Err(_) => Token::Invalid(format!("{name} literal is too large (more than 128 bits)")),
        }
    }

    pub(in crate::lexer) fn read_number(&mut self) -> Token {
        let int_only_after_field_dot = self.numeric_field_index_after_dot;
        self.numeric_field_index_after_dot = false;

        let mut num_str = String::new();
        let mut is_float = false;

        if self.current_char == Some('0') {
            match self.peek(1) {
                Some('x') | Some('X') => return self.read_radix_literal(16, "hexadecimal"),
                Some('b') | Some('B') => return self.read_radix_literal(2, "binary"),
                Some('o') | Some('O') => return self.read_radix_literal(8, "octal"),
                _ => {}
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

        if is_float {
            return match num_str.parse() {
                Ok(f) => Token::FloatLiteral(f),
                Err(_) => Token::Invalid(format!("malformed float literal `{num_str}`")),
            };
        }

        // Type suffixes (0u64, 0i32, ...): without this `0u64` would lex as `0` + `u64`.
        let suffix = self.read_int_suffix();
        match num_str.parse::<u128>() {
            Ok(v) => int_token(v, suffix),
            Err(_) => Token::Invalid(format!(
                "integer literal `{num_str}` is too large (more than 128 bits)"
            )),
        }
    }
}
