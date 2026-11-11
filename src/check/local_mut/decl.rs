//! Where a binding is declared in the source text: the byte offset of its name in the `let` / `for` pattern, where
//! the fix inserts `mut `. The parser records a statement's end, so the keyword is found by walking back from there
//! over balanced brackets (a `let` inside the initializer's blocks is deeper), skipping comments and strings.

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn word_at(b: &[u8], at: usize, w: &str) -> bool {
    b.get(at..at + w.len()) == Some(w.as_bytes())
        && (at == 0 || !is_word(b[at - 1]))
        && b.get(at + w.len()).is_none_or(|&c| !is_word(c))
}

fn previous_word(b: &[u8], mut at: usize) -> &[u8] {
    while at > 0 && b[at - 1].is_ascii_whitespace() {
        at -= 1;
    }
    let end = at;
    while at > 0 && is_word(b[at - 1]) {
        at -= 1;
    }
    &b[at..end]
}

/// Offset of `name` as a binding in the pattern of the `keyword` (`let` / `for`) statement that ends at `end`.
pub fn binding_offset(text: &str, masked: &[(usize, usize)], end: usize, keyword: &str, name: &str) -> Option<usize> {
    let b = text.as_bytes();
    let is_masked = |i: usize| masked.iter().any(|&(s, e)| i >= s && i < e);
    let mut depth: i32 = 0;
    let mut i = end.min(b.len());
    while i > 0 {
        i -= 1;
        if is_masked(i) {
            continue;
        }
        match b[i] {
            b'}' | b')' | b']' => depth += 1,
            b'{' | b'(' | b'[' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ if depth == 0 && word_at(b, i, keyword) => {
                // `if let` / `while let` inside the initializer is not the statement's own `let`
                if keyword == "let" && matches!(previous_word(b, i), b"if" | b"while") {
                    continue;
                }
                return in_pattern(b, &is_masked, i + keyword.len(), keyword, name);
            }
            _ => {}
        }
    }
    None
}

/// The first occurrence of `name` as a binding in the pattern starting at `start`: not a struct pattern's field name
/// (`field: binding`), not after `ref`, and before the pattern ends (`=`, a type annotation's `:`, or `in`).
fn in_pattern(b: &[u8], is_masked: &dyn Fn(usize) -> bool, start: usize, keyword: &str, name: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = start;
    while i < b.len() {
        if is_masked(i) {
            i += 1;
            continue;
        }
        let c = b[i];
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 && keyword == "let" => return None,
            b':' if depth == 0 && keyword == "let" && b.get(i + 1) != Some(&b':') && (i == 0 || b[i - 1] != b':') => return None,
            _ if depth == 0 && keyword == "for" && word_at(b, i, "in") => return None,
            _ if word_at(b, i, name) => {
                let mut j = i + name.len();
                while j < b.len() && b[j].is_ascii_whitespace() {
                    j += 1;
                }
                let field_name = depth > 0 && b.get(j) == Some(&b':') && b.get(j + 1) != Some(&b':');
                let after_ref = matches!(previous_word(b, i), b"ref" | b"mut");
                if !field_name && !after_ref {
                    return Some(i);
                }
                i += name.len();
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str, kw: &str, name: &str) -> Option<String> {
        let masked = crate::check::locate::masked_ranges(src);
        binding_offset(src, &masked, src.len(), kw, name).map(|o| format!("{}|{}", &src[..o], &src[o..]))
    }

    #[test]
    fn finds_the_name_in_simple_tuple_and_struct_patterns() {
        assert_eq!(at("    let x = 1", "let", "x").unwrap(), "    let |x = 1");
        assert_eq!(at("let (a, b) = p", "let", "b").unwrap(), "let (a, |b) = p");
        assert_eq!(at("let Point { x: px, y } = p", "let", "px").unwrap(), "let Point { x: |px, y } = p");
        assert_eq!(at("let Point { x, y } = p", "let", "x").unwrap(), "let Point { |x, y } = p");
        assert_eq!(at("let c: Counter = Counter { n: 0 }", "let", "c").unwrap(), "let |c: Counter = Counter { n: 0 }");
        assert_eq!(at("for (i, x) in xs { x = 1 }", "for", "x").unwrap(), "for (i, |x) in xs { x = 1 }");
    }

    #[test]
    fn skips_lets_nested_in_the_initializer() {
        let src = "let total = { let total = 2\n total }";
        assert_eq!(at(src, "let", "total").unwrap(), "let |total = { let total = 2\n total }");
        let src = "let n = if let Some(v) = o { v } else { 0 }";
        assert_eq!(at(src, "let", "n").unwrap(), "let |n = if let Some(v) = o { v } else { 0 }");
    }

    #[test]
    fn a_name_that_is_only_a_field_or_absent_is_not_found() {
        assert_eq!(at("let Point { x: px } = p", "let", "x"), None);
        assert_eq!(at("let y = 1", "let", "x"), None);
        assert_eq!(at("// let x = 1\nfoo()", "let", "x"), None);
    }
}
