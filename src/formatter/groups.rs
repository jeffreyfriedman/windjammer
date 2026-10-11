//! Bracket structure queries over a token slice.

use super::tokens::{Kind, Tok};

/// For every opener, the index of its matching closer (None if unbalanced).
pub fn match_brackets(toks: &[Tok]) -> Vec<Option<usize>> {
    let mut out = vec![None; toks.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if t.is_opener() {
            stack.push(i);
        } else if t.is_closer() {
            if let Some(o) = stack.pop() {
                out[o] = Some(i);
            }
        }
    }
    out
}

/// Indices of commas that separate elements directly inside the group `open..close`.
/// Commas inside nested brackets, generic angle brackets, or closure parameter pipes
/// are not separators.
pub fn separators(toks: &[Tok], open: usize, close: usize) -> Vec<usize> {
    let mut seps = Vec::new();
    let (mut depth, mut angle, mut pipe) = (0i32, 0i32, false);
    for i in open + 1..close {
        let t = &toks[i];
        if t.is_opener() {
            depth += 1;
        } else if t.is_closer() {
            depth -= 1;
        } else if depth == 0 {
            track_angle_and_pipe(toks, i, &mut angle, &mut pipe);
            if t.is(",") && angle <= 0 && !pipe {
                seps.push(i);
            }
        }
    }
    seps
}

fn track_angle_and_pipe(toks: &[Tok], i: usize, angle: &mut i32, pipe: &mut bool) {
    let t = &toks[i];
    if t.is("|") {
        *pipe = !*pipe;
    } else if t.is(",") && *angle < 0 {
        *angle = 0;
    } else if t.is("<") && is_generic_open(toks, i) {
        *angle += 1;
    } else if t.is(">") && *angle > 0 {
        *angle -= 1;
    } else if t.is(">>") && *angle > 0 {
        *angle = (*angle - 2).max(0);
    }
}

/// `<` glued to a preceding path/word (`Vec<`, `::<`) opens generics; spaced `<` compares.
pub fn is_generic_open(toks: &[Tok], i: usize) -> bool {
    let t = &toks[i];
    i > 0 && t.nl == 0 && !t.space && (toks[i - 1].kind == Kind::Word || toks[i - 1].is("::"))
}
