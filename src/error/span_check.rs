//! Span validation: the single place that decides whether a (line, column, byte range) is
//! well-formed for a given source text. Used by the debug-build assertion layer, the test oracles
//! and `wj check --json` schema validation.
//!
//! Convention (documented in docs/testing/INVARIANTS.md, matches docs/diagnostics-schema.json):
//! lines are 1-based and end at `\n` only (a `\r` is an ordinary column); columns are 1-based and
//! count Unicode scalar values; byte offsets are 0-based and half-open. LSP positions are
//! 0-based lines and UTF-16 code units; use [`char_col_to_utf16`] / [`utf16_to_char_col`].

use std::fmt;

/// A fully resolved span: byte range plus its line/column rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanRecord {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub col_start: usize,
    pub line_end: usize,
    pub col_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanViolation {
    /// Line or column is 0 (both are 1-based).
    ZeroBased { what: &'static str },
    StartAfterEnd,
    OutsideSource { end: usize, len: usize },
    NotOnCharBoundary { offset: usize },
    /// The token the diagnostic is about has width; an empty range cannot be underlined.
    EmptyButExtentRequired,
    /// line/col does not equal the position computed from the byte offset.
    LineColMismatch { which: &'static str, recorded: (usize, usize), computed: (usize, usize) },
    /// (line, col) does not exist in the source (line past EOF or column past end of line).
    PositionOutsideSource { line: usize, col: usize },
}

impl fmt::Display for SpanViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// 1-based (line, column in chars) of `byte` in `text`. `byte` must be a char boundary `<= len`.
pub fn line_col_of_byte(text: &str, byte: usize) -> (usize, usize) {
    let prefix = &text[..byte];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |i| i + 1);
    (line, prefix[line_start..].chars().count() + 1)
}

/// Byte offset of a 1-based (line, col); `col` may be one past the end of the line.
pub fn byte_of_line_col(text: &str, line: usize, col: usize) -> Option<usize> {
    if line == 0 || col == 0 {
        return None;
    }
    let mut start = 0;
    for _ in 1..line {
        start += text[start..].find('\n')? + 1;
    }
    let line_end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    let mut off = start;
    for _ in 1..col {
        if off >= line_end {
            return None;
        }
        off += text[off..].chars().next()?.len_utf8();
    }
    Some(off)
}

/// Validate a 1-based (line, col) that has no extent. Returns its byte offset.
pub fn validate_position(text: &str, line: usize, col: usize) -> Result<usize, SpanViolation> {
    if line == 0 {
        return Err(SpanViolation::ZeroBased { what: "line" });
    }
    if col == 0 {
        return Err(SpanViolation::ZeroBased { what: "column" });
    }
    byte_of_line_col(text, line, col).ok_or(SpanViolation::PositionOutsideSource { line, col })
}

/// Validate a full span against `text`. `require_extent` demands `byte_end > byte_start`.
pub fn validate_span(text: &str, s: &SpanRecord, require_extent: bool) -> Result<(), SpanViolation> {
    for (what, v) in [
        ("line_start", s.line_start),
        ("col_start", s.col_start),
        ("line_end", s.line_end),
        ("col_end", s.col_end),
    ] {
        if v == 0 {
            return Err(SpanViolation::ZeroBased { what });
        }
    }
    if s.byte_start > s.byte_end {
        return Err(SpanViolation::StartAfterEnd);
    }
    if s.byte_end > text.len() {
        return Err(SpanViolation::OutsideSource { end: s.byte_end, len: text.len() });
    }
    for off in [s.byte_start, s.byte_end] {
        if !text.is_char_boundary(off) {
            return Err(SpanViolation::NotOnCharBoundary { offset: off });
        }
    }
    if require_extent && s.byte_start == s.byte_end {
        return Err(SpanViolation::EmptyButExtentRequired);
    }
    let start = line_col_of_byte(text, s.byte_start);
    if start != (s.line_start, s.col_start) {
        return Err(SpanViolation::LineColMismatch {
            which: "start",
            recorded: (s.line_start, s.col_start),
            computed: start,
        });
    }
    let end = line_col_of_byte(text, s.byte_end);
    if end != (s.line_end, s.col_end) {
        return Err(SpanViolation::LineColMismatch {
            which: "end",
            recorded: (s.line_end, s.col_end),
            computed: end,
        });
    }
    Ok(())
}

/// UTF-16 code units in the first `col - 1` chars of `line_text` (LSP `character` of a 1-based
/// char column). Columns past the end clamp to the line length.
pub fn char_col_to_utf16(line_text: &str, col: usize) -> usize {
    line_text.chars().take(col.saturating_sub(1)).map(char::len_utf16).sum()
}

/// Inverse of [`char_col_to_utf16`]. Returns `None` when `utf16` splits a surrogate pair or is
/// past the end of the line.
pub fn utf16_to_char_col(line_text: &str, utf16: usize) -> Option<usize> {
    let mut units = 0;
    let mut col = 1;
    for c in line_text.chars() {
        if units == utf16 {
            return Some(col);
        }
        if units > utf16 {
            return None;
        }
        units += c.len_utf16();
        col += 1;
    }
    (units == utf16).then_some(col)
}

/// LSP position (0-based line, UTF-16 `character`) of a byte offset. Lines end at `\n`; a `\r`
/// before it is an ordinary character of the line (LSP also counts it). `byte` must be a char
/// boundary `<= text.len()`.
pub fn byte_to_lsp_position(text: &str, byte: usize) -> (u32, u32) {
    let prefix = &text[..byte];
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |i| i + 1);
    let units: usize = prefix[line_start..].chars().map(char::len_utf16).sum();
    (line as u32, units as u32)
}

/// Byte offset of an LSP position. Characters past the end of the line clamp to the end of the
/// line (as the LSP specification requires); `None` when the line does not exist or the position
/// splits a surrogate pair.
pub fn lsp_position_to_byte(text: &str, line: u32, character: u32) -> Option<usize> {
    let mut start = 0usize;
    for _ in 0..line {
        start += text[start..].find('\n')? + 1;
    }
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    let mut units = 0u32;
    for (i, c) in text[start..end].char_indices() {
        if units == character {
            return Some(start + i);
        }
        if units > character {
            return None;
        }
        units += c.len_utf16() as u32;
    }
    (units <= character).then_some(end)
}

/// Validate every span of a `wj check --json` / `wj build --json` report (docs/diagnostics-schema.json
/// v2: `diagnostics[].primary_span` / `secondary_spans` / `fixes[].edits[]`, fields `byte_start`,
/// `byte_end`, `line_start`, `col_start`, `line_end`, `col_end`, `file`). `source_of` maps the
/// span's `file` to its text; files it cannot supply are reported. Returns human-readable
/// violations (empty when the report is sound). Library function: callable from `wj check`.
pub fn validate_report_spans(
    report: &serde_json::Value,
    source_of: &dyn Fn(&str) -> Option<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    let Some(diags) = report.get("diagnostics").and_then(|d| d.as_array()) else {
        return vec!["report has no `diagnostics` array".into()];
    };
    for (i, d) in diags.iter().enumerate() {
        let code = d.get("code").and_then(|c| c.as_str()).unwrap_or("?");
        let mut spans: Vec<(String, &serde_json::Value)> = Vec::new();
        if let Some(p) = d.get("primary_span").filter(|p| !p.is_null()) {
            spans.push(("primary_span".into(), p));
        }
        for (j, s) in d.get("secondary_spans").and_then(|s| s.as_array()).into_iter().flatten().enumerate() {
            spans.push((format!("secondary_spans[{j}]"), s));
        }
        for (j, f) in d.get("fixes").and_then(|f| f.as_array()).into_iter().flatten().enumerate() {
            for (k, e) in f.get("edits").and_then(|e| e.as_array()).into_iter().flatten().enumerate() {
                spans.push((format!("fixes[{j}].edits[{k}]"), e));
            }
        }
        for (name, s) in spans {
            let at = format!("diagnostics[{i}] {code} {name}");
            let num = |k: &str| s.get(k).and_then(|v| v.as_u64()).map(|v| v as usize);
            let (Some(bs), Some(be), Some(ls), Some(cs), Some(le), Some(ce)) = (
                num("byte_start"),
                num("byte_end"),
                num("line_start"),
                num("col_start"),
                num("line_end"),
                num("col_end"),
            ) else {
                out.push(format!("{at}: missing or non-integer span field"));
                continue;
            };
            let Some(file) = s.get("file").and_then(|f| f.as_str()) else {
                out.push(format!("{at}: missing file"));
                continue;
            };
            let Some(text) = source_of(file) else {
                out.push(format!("{at}: cannot read source `{file}` to validate span"));
                continue;
            };
            let rec = SpanRecord { byte_start: bs, byte_end: be, line_start: ls, col_start: cs, line_end: le, col_end: ce };
            if let Err(v) = validate_span(&text, &rec, false) {
                out.push(format!("{at}: {v}"));
            }
        }
    }
    out
}

static STRICT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(cfg!(debug_assertions));

/// Turn the span assertion layer on or off at run time. On by default in debug builds; the test
/// oracles switch it on so that `--release` test runs are gated too. `WJ_STRICT_SPANS=1` also
/// enables it.
pub fn set_strict(on: bool) {
    STRICT.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_strict() -> bool {
    STRICT.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var_os("WJ_STRICT_SPANS").is_some_and(|v| v == "1")
}

/// Assertion layer: a location handed to a diagnostic constructor must be structurally valid
/// (1-based line and column). Panics in strict mode (debug builds, tests) so a bad span cannot
/// ship silently; a no-op in release builds. Source-aware validation happens where the source is
/// available (`validate_span`, `validate_report_spans`).
#[inline]
pub fn assert_location(what: &str, line: usize, col: usize) {
    if (line == 0 || col == 0) && is_strict() {
        panic!("invalid diagnostic location for {what}: line {line}, column {col} (both are 1-based)");
    }
}

/// Like [`assert_location`] for a span with a known end.
#[inline]
pub fn assert_extent(what: &str, start: (usize, usize), end: (usize, usize)) {
    assert_location(what, start.0, start.1);
    assert_location(what, end.0, end.1);
    if is_strict() && end < start {
        panic!("invalid diagnostic span for {what}: end {end:?} is before start {start:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_counts_chars_not_bytes() {
        let t = "é日本x\nz😀y";
        assert_eq!(line_col_of_byte(t, t.find('x').unwrap()), (1, 4));
        assert_eq!(line_col_of_byte(t, t.find('y').unwrap()), (2, 3));
    }

    #[test]
    fn utf16_round_trip() {
        let l = "a😀é日b";
        for col in 1..=l.chars().count() + 1 {
            let u = char_col_to_utf16(l, col);
            assert_eq!(utf16_to_char_col(l, u), Some(col));
        }
        assert_eq!(char_col_to_utf16(l, 3), 3); // a + surrogate pair
        assert_eq!(utf16_to_char_col(l, 2), None); // inside the pair
    }

    #[test]
    fn validate_rejects_each_violation_class() {
        let t = "é = 1\nx";
        let good = SpanRecord { byte_start: 3, byte_end: 4, line_start: 1, col_start: 3, line_end: 1, col_end: 4 };
        assert!(validate_span(t, &good, true).is_ok());
        let mut b = good;
        b.col_start = 4; // byte-based column
        assert!(matches!(validate_span(t, &b, true), Err(SpanViolation::LineColMismatch { .. })));
        let mut b = good;
        b.byte_start = 5;
        assert_eq!(validate_span(t, &b, true), Err(SpanViolation::StartAfterEnd));
        let mut b = good;
        b.byte_end = 999;
        assert!(matches!(validate_span(t, &b, true), Err(SpanViolation::OutsideSource { .. })));
        let mut b = good;
        b.byte_start = 1;
        assert!(matches!(validate_span(t, &b, true), Err(SpanViolation::NotOnCharBoundary { .. })));
        let mut b = good;
        b.byte_end = 3;
        assert_eq!(validate_span(t, &b, true), Err(SpanViolation::EmptyButExtentRequired));
        let mut b = good;
        b.line_start = 0;
        assert!(matches!(validate_span(t, &b, false), Err(SpanViolation::ZeroBased { .. })));
    }

    #[test]
    fn lsp_positions_round_trip_with_crlf_and_astral() {
        let t = "ab\r\n😀é日\nz";
        for (i, _) in t.char_indices().chain(std::iter::once((t.len(), ' '))) {
            let (l, c) = byte_to_lsp_position(t, i);
            assert_eq!(lsp_position_to_byte(t, l, c), Some(i), "byte {i}");
        }
        assert_eq!(byte_to_lsp_position(t, t.find('é').unwrap()), (1, 2));
        assert_eq!(lsp_position_to_byte(t, 1, 1), None); // inside the surrogate pair
        assert_eq!(lsp_position_to_byte(t, 0, 99), Some(3)); // clamps to the end of the line (after the `\r`, before `\n`)
        assert_eq!(lsp_position_to_byte(t, 9, 0), None);
    }

    #[test]
    fn validate_position_bounds() {
        let t = "ab\ncd";
        assert_eq!(validate_position(t, 2, 3), Ok(5));
        assert!(validate_position(t, 2, 4).is_err());
        assert!(validate_position(t, 3, 1).is_err());
        assert!(validate_position(t, 1, 0).is_err());
    }
}
