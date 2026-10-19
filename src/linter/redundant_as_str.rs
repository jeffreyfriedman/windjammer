//! W0012: `.as_str()` is redundant in Windjammer (there is one `string` type; every backend treats it as identity).
//!
//! It used to be a hard error in `wj build` only, so interpreter-first code was rejected when compiled. It is now a
//! warning in every mode (via `linter::lint_text`) and `wj fix` deletes it. Lexical: string literals (including
//! `${..}` interpolations), char literals and comments are skipped, so the rewrite never touches text.

const PATTERN: &str = ".as_str()";

/// Byte offsets of every `.as_str()` call in code (not in strings, chars or comments).
pub fn find(source: &str) -> Vec<usize> {
    let b = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            b'"' => {
                i += 1;
                let mut depth = 0usize;
                while i < b.len() {
                    match b[i] {
                        b'\\' => i += 1,
                        b'$' if depth == 0 && b.get(i + 1) == Some(&b'{') => {
                            depth = 1;
                            i += 1;
                        }
                        b'{' if depth > 0 => depth += 1,
                        b'}' if depth > 0 => depth -= 1,
                        b'"' if depth == 0 => break,
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
            }
            b'\'' if b.get(i + 2) == Some(&b'\'') || (b.get(i + 1) == Some(&b'\\') && b.get(i + 3) == Some(&b'\'')) => {
                i += if b.get(i + 1) == Some(&b'\\') { 4 } else { 3 };
            }
            _ if source[i..].starts_with(PATTERN) => {
                out.push(i);
                i += PATTERN.len();
            }
            _ => i += 1,
        }
    }
    out
}

/// 1-based (line, column) of a byte offset.
pub fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let col = offset - before.rfind('\n').map_or(0, |p| p + 1) + 1;
    (line, col)
}

/// `wj fix`: delete every redundant `.as_str()`. Idempotent.
pub fn fix(source: &str) -> String {
    let mut out = source.to_string();
    for off in find(source).into_iter().rev() {
        out.replace_range(off..off + PATTERN.len(), "");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_calls_but_not_text() {
        let src = "fn f(s: string) {\n    match s.as_str() { _ => {} }\n    println(\".as_str()\") // .as_str()\n}\n";
        assert_eq!(find(src).len(), 1);
        assert_eq!(line_col(src, find(src)[0]), (2, 12));
        let fixed = fix(src);
        assert!(fixed.contains("match s {"), "{fixed}");
        assert!(fixed.contains("\".as_str()\""), "{fixed}");
        assert_eq!(fix(&fixed), fixed);
    }

    #[test]
    fn interpolation_is_text() {
        assert!(find("let a = \"${s.as_str()}\"\n").is_empty());
    }
}
