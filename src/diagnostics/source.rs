//! Byte offset <-> line/column conversion for one source text.

use super::model::Span;

/// Precomputed line starts of a source text.
pub struct SourceIndex<'a> {
    pub file: String,
    text: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> SourceIndex<'a> {
    pub fn new(file: impl Into<String>, text: &'a str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Self {
            file: file.into(),
            text,
            line_starts,
        }
    }

    /// 1-based (line, column) of a byte offset; column counts chars.
    pub fn line_col(&self, byte: usize) -> (usize, usize) {
        let byte = byte.min(self.text.len());
        let line = match self.line_starts.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.line_starts[line];
        let col = self.text[start..byte.max(start)].chars().count() + 1;
        (line + 1, col)
    }

    /// Byte offset of a 1-based (line, column); clamps to the line / text end.
    pub fn offset(&self, line: usize, col: usize) -> usize {
        let Some(&start) = self.line_starts.get(line.saturating_sub(1)) else {
            return self.text.len();
        };
        let end = self
            .line_starts
            .get(line)
            .map_or(self.text.len(), |e| e.saturating_sub(1).max(start));
        let line_text = &self.text[start..end];
        let skip = col.saturating_sub(1);
        start
            + line_text
                .char_indices()
                .nth(skip)
                .map_or(line_text.len(), |(i, _)| i)
    }

    pub fn span(&self, byte_start: usize, byte_end: usize) -> Span {
        let byte_end = byte_end.max(byte_start).min(self.text.len());
        let (line_start, col_start) = self.line_col(byte_start);
        let (line_end, col_end) = self.line_col(byte_end);
        Span {
            file: self.file.clone(),
            byte_start,
            byte_end,
            line_start,
            col_start,
            line_end,
            col_end,
            label: None,
        }
    }

    /// Span starting at a 1-based line/column and covering `len` bytes (at least one char).
    pub fn span_at(&self, line: usize, col: usize, len: usize) -> Span {
        let start = self.offset(line, col);
        let mut end = (start + len.max(1)).min(self.text.len());
        while end < self.text.len() && !self.text.is_char_boundary(end) {
            end += 1;
        }
        self.span(start, end)
    }

    /// Span covering the identifier-like word that starts at `line:col` (falls back to one char).
    pub fn word_span_at(&self, line: usize, col: usize) -> Span {
        let start = self.offset(line, col);
        let len: usize = self.text[start..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .map(char::len_utf8)
            .sum();
        self.span_at(line, col, len)
    }

    /// Offset after skipping spaces, tabs and newlines from `off`.
    pub fn skip_ws(&self, off: usize) -> usize {
        let rest = self.text.get(off..).unwrap_or("");
        off + (rest.len() - rest.trim_start().len())
    }

    /// Span of the identifier starting at byte `off`, or of one character when none does.
    pub fn word_span_from(&self, off: usize) -> Span {
        let rest = self.text.get(off..).unwrap_or("");
        let len: usize = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .map(char::len_utf8)
            .sum();
        let len = if len == 0 { rest.chars().next().map_or(0, char::len_utf8) } else { len };
        self.span(off, off + len)
    }

    /// Like [`Self::word_span_at`] but covers at least `min_len` bytes when no word starts there.
    pub fn word_span_at_or(&self, line: usize, col: usize, min_len: usize) -> Span {
        let w = self.word_span_at(line, col);
        if w.byte_end - w.byte_start >= min_len {
            w
        } else {
            self.span_at(line, col, min_len)
        }
    }

    pub fn text(&self) -> &str {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_roundtrip_with_multibyte() {
        let src = "ab\nc\u{e9}d\nxyz";
        let ix = SourceIndex::new("f.wj", src);
        assert_eq!(ix.line_col(0), (1, 1));
        assert_eq!(ix.line_col(3), (2, 1));
        assert_eq!(ix.line_col(6), (2, 3));
        for byte in [0, 1, 3, 4, 6, 7, 8, 10] {
            let (l, c) = ix.line_col(byte);
            assert_eq!(ix.offset(l, c), byte, "byte {byte}");
        }
    }

    #[test]
    fn word_span_covers_identifier() {
        let ix = SourceIndex::new("f.wj", "let total = 1\nprint(count)\n");
        let s = ix.word_span_at(2, 7);
        assert_eq!(&ix.text()[s.byte_start..s.byte_end], "count");
        assert_eq!((s.line_start, s.col_start, s.col_end), (2, 7, 12));
    }
}
