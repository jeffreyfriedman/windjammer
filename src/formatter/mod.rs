//! Windjammer source formatter. See MODULE.md.

mod edits;
mod groups;
mod layout;
mod normalize;
mod options;
mod spacing;
mod tokens;
mod verify;
mod wrap;

pub use edits::{diff_lines, restrict_to_range, LineEdit};
pub use options::FormatOptions;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The source contains text the tokenizer does not understand.
    Lex(String),
    /// Internal safety check failed; the source is left untouched.
    Unverified,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Lex(m) => write!(f, "cannot tokenize source: {m}"),
            FormatError::Unverified => write!(f, "formatting would change program tokens"),
        }
    }
}

impl std::error::Error for FormatError {}

const MAX_WRAP_ROUNDS: usize = 64;

/// Format Windjammer source text. Deterministic and idempotent; comments are preserved.
pub fn format_source(src: &str, opts: &FormatOptions) -> Result<String, FormatError> {
    let original = tokens::scan(src)?;
    let mut toks = original.clone();
    for _ in 0..MAX_WRAP_ROUNDS {
        normalize::normalize(&mut toks);
        let lines = layout::layout(&toks);
        let rendered: Vec<String> = lines
            .iter()
            .map(|l| layout::render_line(&toks, l, opts))
            .collect();
        if !wrap::wrap_once(&mut toks, &lines, &rendered, opts.max_width) {
            if !verify::same_tokens(&original, &toks) {
                return Err(FormatError::Unverified);
            }
            return Ok(layout::assemble(&toks, &lines, &rendered));
        }
    }
    Err(FormatError::Unverified)
}

/// Minimal edits that format the whole document.
pub fn document_edits(src: &str, opts: &FormatOptions) -> Result<Vec<LineEdit>, FormatError> {
    Ok(diff_lines(src, &format_source(src, opts)?))
}

/// Minimal edits formatting only the hunks that touch the 0-based inclusive line range.
pub fn range_edits(
    src: &str,
    opts: &FormatOptions,
    first_line: usize,
    last_line: usize,
) -> Result<Vec<LineEdit>, FormatError> {
    Ok(restrict_to_range(document_edits(src, opts)?, first_line, last_line))
}

/// All comments (including doc comments) in source order, trailing whitespace trimmed.
pub fn comments(src: &str) -> Result<Vec<String>, FormatError> {
    Ok(tokens::scan(src)?
        .into_iter()
        .filter(|t| t.is_comment())
        .map(|t| t.text)
        .collect())
}
