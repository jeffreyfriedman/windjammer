//! Gate 1: lexer span invariants, checked against the independent oracle.

use super::lexeme::check_lexeme;
use super::naive::{line_col, offset_of};
use windjammer::lexer::{Lexer, Token, TokenWithLocation};

/// The full raw stream including `Newline` and the final `Eof`; `Err` if the lexer panicked.
pub fn lex_all(src: &str) -> Result<Vec<TokenWithLocation>, String> {
    let owned = src.to_string();
    std::panic::catch_unwind(move || {
        let mut lx = Lexer::new(&owned);
        let mut out = Vec::new();
        loop {
            let t = lx.next_token_with_location();
            let eof = t.token == Token::Eof;
            out.push(t);
            if eof {
                break;
            }
            assert!(out.len() <= owned.len() + 2, "lexer made no progress");
        }
        out
    })
    .map_err(|p| {
        p.downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "lexer panicked".into())
    })
}

/// Everything that is not a token: whitespace (but never `\n`, which is a `Newline` token) and
/// plain `//` comments up to, not including, the newline.
pub fn check_trivia(gap: &str) -> Result<(), String> {
    let mut rest = gap;
    while let Some(c) = rest.chars().next() {
        if c == '\n' {
            return Err("newline inside trivia (it is a Newline token)".into());
        }
        if c.is_whitespace() {
            rest = &rest[c.len_utf8()..];
        } else if rest.starts_with("//") {
            let is_doc = rest.starts_with("///") && !rest.starts_with("////");
            if is_doc {
                return Err("doc comment in trivia gap (it is a DocComment token)".into());
            }
            let end = rest.find('\n').unwrap_or(rest.len());
            if end != rest.len() {
                return Err("comment followed by newline inside gap".into());
            }
            rest = "";
        } else {
            return Err(format!("non-trivia text {:?} between tokens", c));
        }
    }
    Ok(())
}

/// Check a token stream against `src`. Returns every violation found.
pub fn check_stream(src: &str, toks: &[TokenWithLocation]) -> Vec<String> {
    let mut v = Vec::new();
    let mut prev_end = 0usize;
    let mut rebuilt = String::new();
    let mut last_pos = (0usize, 0usize);
    for (i, t) in toks.iter().enumerate() {
        let at = format!("token #{i} {:?} @{}:{}", t.token, t.line, t.column);
        if t.byte_start > t.byte_end || t.byte_end > src.len() {
            v.push(format!("{at}: byte range {}..{} invalid (len {})", t.byte_start, t.byte_end, src.len()));
            continue;
        }
        if !src.is_char_boundary(t.byte_start) || !src.is_char_boundary(t.byte_end) {
            v.push(format!("{at}: byte range {}..{} not on char boundaries", t.byte_start, t.byte_end));
            continue;
        }
        if t.byte_start < prev_end {
            v.push(format!("{at}: overlaps previous token (starts {} < prev end {prev_end})", t.byte_start));
            continue;
        }
        let gap = &src[prev_end..t.byte_start];
        if let Err(e) = check_trivia(gap) {
            v.push(format!("{at}: gap {gap:?}: {e}"));
        }
        rebuilt.push_str(gap);
        let lexeme = &src[t.byte_start..t.byte_end];
        rebuilt.push_str(lexeme);
        if let Err(e) = check_lexeme(&t.token, lexeme, t.byte_end == src.len()) {
            v.push(format!("{at}: lexeme {lexeme:?}: {e}"));
        }
        let want = line_col(src, t.byte_start);
        if (t.line, t.column) != want {
            v.push(format!("{at}: start (line,col) {:?} != oracle {want:?} for byte {}", (t.line, t.column), t.byte_start));
        }
        let want_end = line_col(src, t.byte_end);
        if (t.end_line, t.end_column) != want_end {
            v.push(format!("{at}: end (line,col) {:?} != oracle {want_end:?} for byte {}", (t.end_line, t.end_column), t.byte_end));
        }
        if offset_of(src, t.line, t.column) != Some(t.byte_start) {
            v.push(format!("{at}: (line,col) does not round-trip to byte {}", t.byte_start));
        }
        if (t.line, t.column) < last_pos {
            v.push(format!("{at}: position goes backwards"));
        }
        last_pos = (t.line, t.column);
        prev_end = t.byte_end;
    }
    match toks.last() {
        Some(last) if last.token == Token::Eof => {
            if last.byte_start != src.len() || last.byte_end != src.len() {
                v.push(format!("Eof must be the empty range at {}, got {}..{}", src.len(), last.byte_start, last.byte_end));
            }
        }
        _ => v.push("stream does not end with Eof".into()),
    }
    if toks.iter().filter(|t| t.token == Token::Eof).count() > 1 {
        v.push("more than one Eof".into());
    }
    if rebuilt != src && v.is_empty() {
        v.push("trivia + tokens do not reproduce the source (lossless coverage)".into());
    }
    v
}

pub fn check_source(src: &str) -> Vec<String> {
    match lex_all(src) {
        Ok(toks) => check_stream(src, &toks),
        Err(p) => vec![format!("lexer panicked: {p}")],
    }
}
