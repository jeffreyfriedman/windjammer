//! Width-aware wrapping: bracketed comma lists on over-long lines get one element per line.

use super::groups::{match_brackets, separators};
use super::layout::Line;
use super::tokens::Tok;

/// Break the first breakable `(`/`[` list on each over-long line. Returns whether anything
/// changed. Only newline flags are modified; token order and text never change.
pub fn wrap_once(toks: &mut [Tok], lines: &[Line], rendered: &[String], max_width: usize) -> bool {
    let matches = match_brackets(toks);
    let mut changed = false;
    for (line, text) in lines.iter().zip(rendered) {
        if text.chars().count() <= max_width || toks[line.start..line.end].iter().any(|t| t.text.contains('\n')) {
            continue;
        }
        if break_pipes(toks, line) {
            changed = true;
        } else if let Some((open, close)) = find_group(toks, &matches, line) {
            break_group(toks, open, close);
            changed = true;
        }
    }
    changed
}

fn find_group(toks: &[Tok], matches: &[Option<usize>], line: &Line) -> Option<(usize, usize)> {
    (line.start..line.end).find_map(|i| {
        let close = matches[i]?;
        let listy = toks[i].is("(") || toks[i].is("[");
        let ok = listy && close < line.end && !separators(toks, i, close).is_empty();
        ok.then_some((i, close))
    })
}

fn break_group(toks: &mut [Tok], open: usize, close: usize) {
    for sep in separators(toks, open, close) {
        toks[sep + 1].nl = 1;
    }
    toks[open + 1].nl = 1;
    toks[close].nl = 1;
}

/// A long line holding two or more top-level `|>` stages gets one stage per line.
fn break_pipes(toks: &mut [Tok], line: &Line) -> bool {
    let mut depth = 0i32;
    let mut pipes = Vec::new();
    for i in line.start..line.end {
        if toks[i].is_opener() {
            depth += 1;
        } else if toks[i].is_closer() {
            depth -= 1;
        } else if depth == 0 && i > line.start && (toks[i].is("|>") || toks[i].is("|>?")) {
            pipes.push(i);
        }
    }
    if pipes.len() < 2 {
        return false;
    }
    for i in pipes {
        toks[i].nl = 1;
    }
    true
}
