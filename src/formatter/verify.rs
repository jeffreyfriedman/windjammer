//! Safety net: formatting may only change whitespace and optional trailing commas.

use super::tokens::{Kind, Tok};

fn significant(toks: &[Tok]) -> Vec<(Kind, &str)> {
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if t.is(",") && trailing_comma(toks, i) {
            continue;
        }
        out.push((t.kind, t.text.as_str()));
    }
    out
}

fn trailing_comma(toks: &[Tok], i: usize) -> bool {
    toks[i + 1..]
        .iter()
        .find(|t| !t.is_comment())
        .is_some_and(|t| t.is_closer())
}

/// Token streams are equal modulo trailing commas.
pub fn same_tokens(before: &[Tok], after: &[Tok]) -> bool {
    significant(before) == significant(after)
}
