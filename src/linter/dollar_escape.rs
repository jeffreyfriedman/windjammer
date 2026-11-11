//! W0017: `$${x}` in a string literal prints `${x}`: `$$` is an escaped `$`, so the braces after it are plain text and
//! the value is never interpolated. Agents writing prices (`"$${d}.${c}"`) meant a dollar sign followed by the value
//! (`"\$${d}"`); the program runs and prints the wrong text, with no diagnostic (agent pilot 6, task t103: every
//! Windjammer trial lost turns to it). Lexical, like `redundant_as_str`: only string literals are scanned, never
//! comments, chars or the inside of an interpolation.

/// The text the lint points at and its suggested replacement (a dollar sign, then the interpolation).
pub const PATTERN: &str = "$${";
pub const REPLACEMENT: &str = "\\$${";

pub const MESSAGE: &str = "`$${` prints `${` literally: `$$` is an escaped `$`, so the value in the braces is not interpolated";
pub const HELP: &str = "to print a dollar sign followed by the value, write `\\$${..}`; keep `$${` only if you want the text `${`";

/// Byte offsets of every `$${` inside a string literal (outside interpolations).
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
            b'\'' if b.get(i + 2) == Some(&b'\'') || (b.get(i + 1) == Some(&b'\\') && b.get(i + 3) == Some(&b'\'')) => {
                i += if b.get(i + 1) == Some(&b'\\') { 4 } else { 3 };
            }
            b'"' => {
                i += 1;
                let mut depth = 0usize;
                while i < b.len() {
                    match b[i] {
                        b'\\' => i += 1,
                        b'$' if depth == 0 && b.get(i + 1) == Some(&b'$') => {
                            if b.get(i + 2) == Some(&b'{') {
                                out.push(i);
                            }
                            i += 1;
                        }
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
            _ => i += 1,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::find;

    #[test]
    fn finds_escaped_dollar_before_braces_in_strings_only() {
        assert_eq!(find("let s = \"$${d}.50\""), vec![9]);
        assert_eq!(find("let s = \"a $${d} b $${e}\""), vec![11, 19]);
        // a dollar sign then the value, a lone escaped dollar, a plain interpolation: fine
        assert!(find("let s = \"\\$${d} $$ ${e} $$x\"").is_empty());
        // comments, chars and code are not strings
        assert!(find("// \"$${d}\"\nlet c = '$'\n/* \"$${x}\" */").is_empty());
        // inside an interpolation the text is code, not the literal
        assert!(find("let s = \"${f(\"a\")} ok\"").is_empty());
    }
}
