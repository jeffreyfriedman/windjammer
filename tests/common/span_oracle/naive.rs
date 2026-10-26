//! Deliberately naive, independent position arithmetic. It shares no code with the compiler: it
//! walks the text one char at a time, so it cannot share a bug with the lexer's incremental
//! counters or with `windjammer::span_check`.
//!
//! Convention under test (docs/testing/INVARIANTS.md): lines are 1-based and end at `\n` only;
//! columns are 1-based and count Unicode scalar values; byte offsets are 0-based, half-open.

/// 1-based (line, column) of byte offset `off` (must be a char boundary, `off <= text.len()`).
pub fn line_col(text: &str, off: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in text.char_indices() {
        if i >= off {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Byte offset of a 1-based (line, col), or None when it does not exist in `text`.
/// `col` may equal line length + 1 (the position just before the `\n` or at EOF).
pub fn offset_of(text: &str, line: usize, col: usize) -> Option<usize> {
    if line == 0 || col == 0 {
        return None;
    }
    let (mut l, mut c) = (1, 1);
    for (i, ch) in text.char_indices() {
        if l == line && c == col {
            return Some(i);
        }
        if ch == '\n' {
            if l == line {
                return None;
            }
            l += 1;
            c = 1;
        } else {
            c += 1;
        }
    }
    (l == line && c == col).then_some(text.len())
}

/// Number of UTF-16 code units in `s` (independent of `str::encode_utf16`).
pub fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| if (c as u32) >= 0x1_0000 { 2 } else { 1 }).sum()
}
