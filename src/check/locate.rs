//! Turning the parser's AST locations into precise source spans.
//!
//! The parser stamps most expressions with the location of the token *after* the expression
//! (taken once it has been consumed); a few use the start. Rather than depend on which, every
//! lookup first tries the text at the location and then searches backwards.

use crate::diagnostics::{SourceIndex, Span};
use crate::parser::{Literal, SourceLocation};

const LOOKBACK: usize = 600;

pub struct Locator<'a> {
    pub ix: &'a SourceIndex<'a>,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn whole_word_at(text: &str, at: usize, word: &str) -> bool {
    text.get(at..).is_some_and(|t| t.starts_with(word))
        && !text[..at].chars().next_back().is_some_and(is_word)
        && !text[at + word.len()..].chars().next().is_some_and(is_word)
}

impl<'a> Locator<'a> {
    pub fn new(ix: &'a SourceIndex<'a>) -> Self {
        Self { ix }
    }

    fn offset(&self, loc: &SourceLocation) -> Option<usize> {
        // A token's recorded location is where the previous token ended (before any whitespace),
        // 1-based; a location taken after consuming an expression is where it ended.
        loc.as_ref().map(|l| self.ix.offset(l.line, l.column))
    }

    /// Span of the identifier `word` that the AST location refers to.
    pub fn word(&self, loc: &SourceLocation, word: &str) -> Option<Span> {
        let text = self.ix.text();
        let off = self.offset(loc)?;
        for at in [self.ix.skip_ws(off), off] {
            if whole_word_at(text, at, word) {
                return Some(self.ix.span(at, at + word.len()));
            }
        }
        let lo = off.saturating_sub(LOOKBACK);
        let lo = (lo..=off).find(|&i| text.is_char_boundary(i))?;
        let hay = &text[lo..off.min(text.len())];
        let mut best = None;
        for (i, _) in hay.match_indices(word) {
            if whole_word_at(text, lo + i, word) {
                best = Some(lo + i);
            }
        }
        if let Some(s) = best {
            return Some(self.ix.span(s, s + word.len()));
        }
        // Start-positioned locations point at a keyword before the name: look ahead on the line.
        let line_end = text[off.min(text.len())..]
            .find('\n')
            .map_or(text.len(), |i| off + i);
        let ahead = text.get(off..line_end)?;
        ahead
            .match_indices(word)
            .map(|(i, _)| off + i)
            .find(|&s| whole_word_at(text, s, word))
            .map(|s| self.ix.span(s, s + word.len()))
    }

    /// Span of the name in the `nth` top-level (column 0) declaration `<kw> <name>`, optionally
    /// preceded by `pub` / `async`.
    pub fn top_level_decl(&self, kw: &str, name: &str, nth: usize) -> Option<Span> {
        let text = self.ix.text();
        let mut seen = 0;
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let t = line.trim_end();
            let mut rest = t;
            for prefix in ["pub ", "async "] {
                rest = rest.strip_prefix(prefix).unwrap_or(rest);
            }
            if !line.starts_with(char::is_whitespace) {
                if let Some(after) = rest.strip_prefix(kw).and_then(|r| r.strip_prefix(' ')) {
                    if after.starts_with(name)
                        && !after[name.len()..].chars().next().is_some_and(is_word)
                    {
                        if seen == nth {
                            let name_off = offset + (t.len() - after.len());
                            return Some(self.ix.span(name_off, name_off + name.len()));
                        }
                        seen += 1;
                    }
                }
            }
            offset += line.len();
        }
        None
    }

    /// Last resort for text with no usable location (identifiers inside `${..}`): the first whole
    /// word occurrence that sits inside an interpolation or call.
    pub fn first_use(&self, word: &str) -> Option<Span> {
        let text = self.ix.text();
        let mut from = 0;
        while let Some(i) = text[from..].find(word) {
            let s = from + i;
            if whole_word_at(text, s, word) {
                let line_start = text[..s].rfind('\n').map_or(0, |n| n + 1);
                let before = &text[line_start..s];
                let is_decl = before.trim_start().starts_with("fn ") || before.trim_start().starts_with("let ");
                let in_interp = before.contains("${");
                let is_call = text[s + word.len()..].starts_with('(');
                if !is_decl && (in_interp || is_call) {
                    return Some(self.ix.span(s, s + word.len()));
                }
            }
            from = s + word.len();
        }
        None
    }

    /// Span of a literal the AST location refers to.
    pub fn literal(&self, loc: &SourceLocation, lit: &Literal) -> Option<Span> {
        let text = self.ix.text();
        let off = self.offset(loc)?;
        for at in [self.ix.skip_ws(off), off] {
            if let Some(end) = literal_end_at(text, at, lit) {
                return Some(self.ix.span(at, end));
            }
        }
        let end = text[..off.min(text.len())].trim_end().len();
        let start = literal_start_before(text, end, lit)?;
        Some(self.ix.span(start, end))
    }
}

/// If `text[at..]` begins with a literal of the same kind and value, return its end offset.
fn literal_end_at(text: &str, at: usize, lit: &Literal) -> Option<usize> {
    let rest = text.get(at..)?;
    match lit {
        Literal::String(s) => {
            let end = string_end(rest)?;
            let inner = &rest[1..end - 1];
            (inner == s || inner.contains('\\') || s.contains('{')).then_some(at + end)
        }
        Literal::Int(n) => {
            let w: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            let digits: String = w.chars().filter(|c| c.is_ascii_digit()).collect();
            (digits == n.to_string() && !w.contains('.')).then_some(at + w.len())
        }
        Literal::Bool(b) => {
            let w = if *b { "true" } else { "false" };
            whole_word_at(text, at, w).then_some(at + w.len())
        }
        Literal::Float(_) => {
            let w: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '_').collect();
            (w.contains('.')).then_some(at + w.len())
        }
        _ => None,
    }
}

/// End (exclusive) of the string literal that starts `rest`, or `None`.
fn string_end(rest: &str) -> Option<usize> {
    let mut chars = rest.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut escaped = false;
    for (i, c) in chars {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return Some(i + 1),
            _ => {}
        }
    }
    None
}

/// Start of the literal that ends exactly at `end`.
fn literal_start_before(text: &str, end: usize, lit: &Literal) -> Option<usize> {
    let before = &text[..end];
    match lit {
        Literal::String(_) => {
            if !before.ends_with('"') {
                return None;
            }
            // Scan forward over candidate opening quotes on this line; the one whose literal
            // ends at `end` is the start.
            let line_start = before.rfind('\n').map_or(0, |i| i + 1);
            before[line_start..]
                .match_indices('"')
                .map(|(i, _)| line_start + i)
                .find(|&s| string_end(&text[s..]).is_some_and(|e| s + e == end))
        }
        Literal::Int(_) | Literal::Float(_) | Literal::Bool(_) => {
            let n = before.chars().rev().take_while(|c| is_word(*c) || *c == '.').count();
            let start = end - n;
            (n > 0).then_some(start)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::Location;

    fn loc(line: usize, column: usize) -> SourceLocation {
        Some(Location { file: "f.wj".into(), line, column })
    }

    #[test]
    fn word_found_at_or_before_location() {
        let ix = SourceIndex::new("f.wj", "let a = count\nprint(a)\n");
        let l = Locator::new(&ix);
        // pre-position: at the word
        let s = l.word(&loc(1, 9), "count").unwrap();
        assert_eq!((s.byte_start, s.byte_end), (8, 13));
        // post-position: start of the next line
        let s = l.word(&loc(2, 1), "count").unwrap();
        assert_eq!((s.byte_start, s.byte_end), (8, 13));
    }

    #[test]
    fn string_literal_found_before_next_token() {
        let src = "let x: int = \"hi\"\nprint(x)\n";
        let ix = SourceIndex::new("f.wj", src);
        let l = Locator::new(&ix);
        let s = l.literal(&loc(2, 1), &Literal::String("hi".into())).unwrap();
        assert_eq!(&src[s.byte_start..s.byte_end], "\"hi\"");
        let s = l.literal(&loc(1, 14), &Literal::String("hi".into())).unwrap();
        assert_eq!(&src[s.byte_start..s.byte_end], "\"hi\"");
    }

    #[test]
    fn int_literal_span() {
        let src = "f(1, 42)\n";
        let ix = SourceIndex::new("f.wj", src);
        let l = Locator::new(&ix);
        let s = l.literal(&loc(1, 8), &Literal::Int(42)).unwrap();
        assert_eq!(&src[s.byte_start..s.byte_end], "42");
    }
}
