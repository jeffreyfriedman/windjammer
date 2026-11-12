//! Lexical scan of REPL / `wj eval` input: where a statement ends and whether the input is still open.
//!
//! One walker knows strings (with `${..}` interpolation holding code, and escapes), char literals and `//`
//! comments, so a `;`, a newline or a bracket inside a string is text and never a separator or a nesting level.

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Code, with the bracket depth opened in this mode (an interpolation starts at depth 1 for its `{`).
    Code(i32),
    Str,
}

/// Where a scan of the input ended.
#[derive(Debug, PartialEq, Eq)]
pub struct Scan {
    /// Unclosed `{ ( [` in code (including the code of open interpolations).
    pub depth: i32,
    /// The input ends inside a string literal.
    pub in_string: bool,
}

impl Scan {
    /// More text is needed before the input can be complete.
    pub fn open(&self) -> bool {
        self.depth > 0 || self.in_string
    }
}

/// Walk `src`, calling `sep(byte_index, c)` for each `;` or newline outside any string, bracket or comment.
fn walk(src: &str, mut sep: impl FnMut(usize, char)) -> Scan {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut stack = vec![Mode::Code(0)];
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        let next = chars.get(i + 1).map(|p| p.1);
        match *stack.last().expect("the stack always holds the base mode") {
            Mode::Str => match c {
                '\\' => i += 1,
                '"' => {
                    stack.pop();
                }
                '$' if next == Some('{') => {
                    stack.push(Mode::Code(1));
                    i += 1;
                }
                _ => {}
            },
            Mode::Code(depth) => {
                let top = stack.len() - 1;
                match c {
                    '"' => stack.push(Mode::Str),
                    '\'' => i += char_literal_len(&chars[i..]).saturating_sub(1),
                    '/' if next == Some('/') => {
                        while i + 1 < chars.len() && chars[i + 1].1 != '\n' {
                            i += 1;
                        }
                    }
                    '{' | '(' | '[' => stack[top] = Mode::Code(depth + 1),
                    '}' | ')' | ']' => {
                        if depth == 1 && top > 0 {
                            stack.pop(); // the `}` that ends an interpolation
                        } else {
                            stack[top] = Mode::Code(depth - 1);
                        }
                    }
                    ';' | '\n' if depth == 0 && top == 0 => sep(at, c),
                    _ => {}
                }
            }
        }
        i += 1;
    }
    let depth = stack.iter().map(|m| if let Mode::Code(d) = m { *d } else { 0 }).sum();
    Scan { depth, in_string: stack.contains(&Mode::Str) }
}

/// Length in chars of the char literal at the start of `s` (`'x'`, `'\n'`, `'\u{1F600}'`), or 1 when it is not one.
fn char_literal_len(s: &[(usize, char)]) -> usize {
    match (s.get(1).map(|p| p.1), s.get(2).map(|p| p.1)) {
        (Some('\\'), _) => s.iter().skip(3).position(|p| p.1 == '\'').filter(|n| *n < 10).map_or(1, |n| n + 4),
        (Some(c), Some('\'')) if c != '\'' => 3,
        _ => 1,
    }
}

/// Where `src` ends: bracket depth and whether a string is open.
pub fn scan(src: &str) -> Scan {
    walk(src, |_, _| {})
}

/// `src` split into statements at `;` and newlines that are outside strings and brackets, trimmed, empties dropped.
pub fn split_statements(src: &str) -> Vec<String> {
    let mut cuts = Vec::new();
    walk(src, |at, _| cuts.push(at));
    let mut parts = Vec::new();
    let mut from = 0;
    for at in cuts.into_iter().chain(std::iter::once(src.len())) {
        let part = src[from..at].trim();
        if !part.is_empty() {
            parts.push(part.to_string());
        }
        from = (at + 1).min(src.len());
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators_inside_strings_are_text() {
        assert_eq!(split_statements("let s = \"a;b\"; s"), vec!["let s = \"a;b\"", "s"]);
        assert_eq!(split_statements("\"x ${f(\"a;b\")} y; z\""), vec!["\"x ${f(\"a;b\")} y; z\""]);
        assert_eq!(split_statements("let c = ';'; c"), vec!["let c = ';'", "c"]);
        assert_eq!(split_statements("a // not; split\nb"), vec!["a // not; split", "b"]);
    }

    #[test]
    fn brackets_keep_a_block_together() {
        assert_eq!(split_statements("for i in 0..2 {\n    println(\"${i}\");\n}\nx"), vec!["for i in 0..2 {\n    println(\"${i}\");\n}", "x"]);
    }

    #[test]
    fn scan_reports_open_input() {
        assert!(scan("fn f() {").open());
        assert!(scan("let s = \"abc").open());
        assert!(scan("\"${f(1)\"").open());
        assert!(!scan("\"}\" + \"(\"").open());
        assert!(!scan("let c = '{'").open());
        assert!(!scan("f(\"${1 + 2}\")").open());
    }
}
