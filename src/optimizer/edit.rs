//! Span-based text edits: passes report byte ranges; formatting and comments
//! outside edited ranges are preserved byte-for-byte.

use proc_macro2::{LineColumn, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub pass: &'static str,
}

/// Maps `proc_macro2` line/column positions to byte offsets of one source.
pub struct SourceIndex<'a> {
    src: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> SourceIndex<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { src, line_starts }
    }

    pub fn src(&self) -> &'a str {
        self.src
    }

    pub fn offset(&self, lc: LineColumn) -> Option<usize> {
        let line_start = *self.line_starts.get(lc.line.checked_sub(1)?)?;
        let rest = &self.src[line_start..];
        let line_end = rest.find('\n').unwrap_or(rest.len());
        let line = &rest[..line_end];
        let off = line
            .char_indices()
            .nth(lc.column)
            .map(|(i, _)| i)
            .or(if lc.column == line.chars().count() { Some(line.len()) } else { None })?;
        Some(line_start + off)
    }

    pub fn range(&self, span: Span) -> Option<(usize, usize)> {
        Some((self.offset(span.start())?, self.offset(span.end())?))
    }

    pub fn text(&self, span: Span) -> Option<&'a str> {
        let (s, e) = self.range(span)?;
        self.src.get(s..e)
    }
}

impl Edit {
    /// Delete `start..end` but keep its line breaks, so every later line keeps its
    /// number (source maps and diagnostics stay aligned).
    pub fn blank(src: &str, start: usize, end: usize, pass: &'static str) -> Edit {
        let text = "\n".repeat(src[start..end].matches('\n').count());
        Edit { start, end, text, pass }
    }
}

/// Apply edits; overlapping edits are dropped (earlier start, then longer, wins).
/// Returns the new text and the edits that were applied.
pub fn apply(src: &str, mut edits: Vec<Edit>) -> (String, Vec<Edit>) {
    edits.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut out = String::with_capacity(src.len());
    let mut applied = Vec::new();
    let mut cursor = 0usize;
    for e in edits {
        if e.start < cursor || e.end < e.start || e.end > src.len() {
            continue;
        }
        out.push_str(&src[cursor..e.start]);
        out.push_str(&e.text);
        cursor = e.end;
        applied.push(e);
    }
    out.push_str(&src[cursor..]);
    (out, applied)
}
