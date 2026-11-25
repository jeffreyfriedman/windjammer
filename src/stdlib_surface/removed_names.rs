//! Spellings `std::test` no longer has, and the one canonical name each became (ADR-0039: one way, no back-compat).
//!
//! One table feeds both fix engines: `wj fix` rewrites the source text with [`fix`], and `wj check --fix` applies the
//! machine-applicable edit of the diagnostic that quotes [`replacement`]. `assert_msg` / `assert_with_message` become
//! the `assert(cond, "why")` intrinsic, which already takes the message.

/// (removed name, canonical name, whether a leading `&` on the first argument is dropped: the removed `assert_is_*`
/// and `assert_result_*` took a reference, the canonical ones take the value and return its payload).
pub const REMOVED_TEST_FNS: &[(&str, &str, bool)] = &[
    ("assert_is_ok", "assert_ok", true),
    ("assert_is_err", "assert_err", true),
    ("assert_result_ok", "assert_ok", true),
    ("assert_result_err", "assert_err", true),
    ("assert_is_some", "assert_some", true),
    ("assert_is_none", "assert_none", true),
    ("assert_gte", "assert_ge", false),
    ("assert_lte", "assert_le", false),
    ("assert_approx", "assert_approx_eq", false),
    ("assert_approx_f32", "assert_approx_eq", false),
    ("assert_approx_f64", "assert_approx_eq", false),
    ("assert_msg", "assert", false),
    ("assert_with_message", "assert", false),
    ("assert_str_contains", "assert_contains", false),
    ("assert_deep_eq", "assert_eq", false),
];

/// The canonical name for a removed `std::test` function, if `name` is one.
pub fn replacement(name: &str) -> Option<&'static str> {
    REMOVED_TEST_FNS.iter().find(|(old, _, _)| *old == name).map(|(_, new, _)| *new)
}

/// Method spellings that were never methods: (removed, canonical). Only a parenthesised range receiver is rewritten
/// (`(0..n).reversed()` -> `(0..n).rev()`); on a list the right answer depends on the type.
pub const REMOVED_RANGE_METHODS: &[(&str, &str)] = &[("reversed", "rev")];

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Byte ranges of source text that is code (outside strings, chars and comments).
fn code_spans(src: &str) -> Vec<(usize, usize)> {
    let b = src.as_bytes();
    let (mut spans, mut start, mut i) = (Vec::new(), 0usize, 0usize);
    let mut close = |spans: &mut Vec<(usize, usize)>, start: usize, end: usize| {
        if end > start {
            spans.push((start, end));
        }
    };
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                close(&mut spans, start, i);
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                start = i;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                close(&mut spans, start, i);
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(b.len());
                start = i;
            }
            b'"' => {
                close(&mut spans, start, i);
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
                i = (i + 1).min(b.len());
                start = i;
            }
            b'\'' if b.get(i + 2) == Some(&b'\'') || (b.get(i + 1) == Some(&b'\\') && b.get(i + 3) == Some(&b'\'')) => {
                close(&mut spans, start, i);
                i += if b.get(i + 1) == Some(&b'\\') { 4 } else { 3 };
                start = i;
            }
            _ => i += 1,
        }
    }
    close(&mut spans, start, b.len().min(i));
    spans
}

/// Names the file defines itself (`fn assert_approx(..)`): calls to them are the program's own.
fn defined_here(src: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let mut prev = "";
    for word in src.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if prev == "fn" && !word.is_empty() {
            out.insert(word.to_string());
        }
        if !word.is_empty() {
            prev = word;
        }
    }
    out
}

/// Whether the parenthesised group that ends at `close` (a `)`) holds a range (`..` at its top level).
fn paren_group_is_range(b: &[u8], close: usize) -> bool {
    let mut depth = 0usize;
    let mut i = close;
    loop {
        match b[i] {
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' => {
                depth -= 1;
                if depth == 0 {
                    return b[i + 1..close].windows(2).enumerate().any(|(k, w)| {
                        w == b".." && {
                            // top level only: count unmatched openers between the group start and the dots
                            let inner = &b[i + 1..i + 1 + k];
                            let opens = inner.iter().filter(|c| matches!(c, b'(' | b'[' | b'{')).count();
                            let closes = inner.iter().filter(|c| matches!(c, b')' | b']' | b'}')).count();
                            opens == closes
                        }
                    });
                }
            }
            _ => {}
        }
        if i == 0 {
            return false;
        }
        i -= 1;
    }
}

/// `wj fix`: rewrite every removed `std::test` spelling and every `(range).reversed()`. Returns the new text and the
/// number of rewrites. Idempotent; strings, chars and comments are never touched.
pub fn fix(src: &str) -> (String, usize) {
    let b = src.as_bytes();
    let own = defined_here(src);
    // `use std::testing` was the second assertion module: it is `std::test` now
    let uses_testing = src.contains("std::testing");
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for (lo, hi) in code_spans(src) {
        let mut i = lo;
        while i < hi {
            if !is_ident(b[i]) || (i > 0 && is_ident(b[i - 1])) {
                i += 1;
                continue;
            }
            let mut j = i;
            while j < hi && is_ident(b[j]) {
                j += 1;
            }
            let word = &src[i..j];
            let before = src[..i].trim_end();
            let after = src[j..hi].trim_start();
            let is_call = after.starts_with('(');
            let is_method = before.ends_with('.');
            if let (true, false, false) = (is_call, is_method, own.contains(word) || before.ends_with("fn")) {
                if let Some((_, new, strip)) = REMOVED_TEST_FNS.iter().find(|(old, _, _)| *old == word) {
                    edits.push((i, j, new.to_string()));
                    if *strip {
                        let open = j + (src[j..].len() - src[j..].trim_start().len());
                        let arg = open + 1;
                        let rest = &src[arg..];
                        let ws = rest.len() - rest.trim_start().len();
                        let at = arg + ws;
                        if b.get(at) == Some(&b'&') && !matches!(b.get(at + 1), Some(b'&') | Some(b'm')) {
                            edits.push((at, at + 1, String::new()));
                        }
                    }
                }
            }
            if is_method && is_call {
                if let Some((_, new)) = REMOVED_RANGE_METHODS.iter().find(|(old, _)| *old == word) {
                    let dot = src[..i].trim_end().len() - 1;
                    let recv_end = src[..dot].trim_end().len();
                    if recv_end > 0 && b[recv_end - 1] == b')' && paren_group_is_range(b, recv_end - 1) {
                        edits.push((i, j, new.to_string()));
                    }
                }
            }
            if uses_testing && word == "testing" && (before.ends_with("std::") || (src[j..].starts_with("::") && !before.ends_with("::"))) {
                edits.push((i, j, "test".to_string()));
            }
            i = j;
        }
    }
    edits.sort_by_key(|e| e.0);
    let mut out = String::with_capacity(src.len());
    let mut last = 0;
    for (s, e, r) in &edits {
        out.push_str(&src[last..*s]);
        out.push_str(r);
        last = *e;
    }
    out.push_str(&src[last..]);
    // a leading `&` removal is part of a rename: count renames only
    let n = edits.iter().filter(|(_, _, r)| !r.is_empty()).count();
    (out, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renames_calls_and_drops_the_reference() {
        let (out, n) = fix("fn t() {\n    assert_is_ok(&r)\n    assert_gte(a, b)\n    assert_approx_f32(x, 1.0, 0.1)\n}\n");
        assert_eq!(out, "fn t() {\n    assert_ok(r)\n    assert_ge(a, b)\n    assert_approx_eq(x, 1.0, 0.1)\n}\n");
        assert_eq!(n, 3);
    }

    #[test]
    fn leaves_strings_comments_methods_and_own_definitions() {
        let src = "fn assert_approx(a: f32) {}\nfn t() {\n    // assert_gte(a, b)\n    println(\"assert_lte(\")\n    assert_approx(1.0)\n    x.assert_gte(1)\n}\n";
        assert_eq!(fix(src), (src.to_string(), 0));
    }

    #[test]
    fn module_spelling_and_range_reversed() {
        let (out, n) = fix("use std::testing\nfn t() {\n    testing::assert_with_message(a, \"m\")\n    for i in (0..n).reversed() {}\n    v.reversed()\n}\n");
        assert_eq!(out, "use std::test\nfn t() {\n    test::assert(a, \"m\")\n    for i in (0..n).rev() {}\n    v.reversed()\n}\n");
        assert_eq!(n, 4);
    }

    #[test]
    fn idempotent() {
        let (once, _) = fix("fn t() { assert_is_some(&x)\n assert_str_contains(s, \"a\") }\n");
        assert_eq!(fix(&once), (once.clone(), 0));
    }
}
