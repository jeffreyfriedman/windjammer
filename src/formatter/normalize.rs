//! Token-level normalization that never reorders or rewrites tokens: brace placement,
//! blank-line collapsing, and trailing commas in multi-line lists.

use super::groups::{match_brackets, separators};
use super::tokens::{Kind, Tok};

pub fn normalize(toks: &mut Vec<Tok>) {
    if let Some(first) = toks.first_mut() {
        first.nl = 0;
    }
    join_braces(toks);
    collapse_blank_lines(toks);
    add_trailing_commas(toks);
}

const HEADER_KEYWORDS: &[&str] = &[
    "fn", "pub", "struct", "enum", "impl", "trait", "if", "else", "while", "for", "match", "loop",
    "mod", "unsafe", "async", "extern", "static", "const", "type",
];

fn line_start(toks: &[Tok], mut i: usize) -> usize {
    while i > 0 && toks[i].nl == 0 {
        i -= 1;
    }
    i
}

/// `{` alone on a line moves up to its header; `else` joins a preceding `}`.
fn join_braces(toks: &mut [Tok]) {
    for i in 1..toks.len() {
        if toks[i].nl == 0 || toks[i - 1].is_comment() {
            continue;
        }
        let brace_on_own_line = toks[i].is("{")
            && !toks[i - 1].is_opener()
            && toks[line_start(toks, i - 1)..i]
                .iter()
                .any(|t| t.kind == Kind::Word && HEADER_KEYWORDS.contains(&t.text.as_str()));
        let else_after_brace = toks[i].kind == Kind::Word && toks[i].text == "else" && toks[i - 1].is("}");
        if brace_on_own_line || else_after_brace {
            toks[i].nl = 0;
            toks[i].space = true;
        }
    }
}

fn collapse_blank_lines(toks: &mut [Tok]) {
    for i in 0..toks.len() {
        let after_opener = i > 0 && toks[i - 1].is_opener();
        let before_closer = toks[i].is_closer();
        let cap = if after_opener || before_closer { 1 } else { 2 };
        toks[i].nl = toks[i].nl.min(cap);
    }
}

fn add_trailing_commas(toks: &mut Vec<Tok>) {
    let matches = match_brackets(toks);
    let mut inserts: Vec<usize> = Vec::new();
    for (open, close) in matches.iter().enumerate() {
        let Some(close) = *close else { continue };
        if toks[close].nl == 0 || !qualifies(toks, open, close) {
            continue;
        }
        let mut last = close - 1;
        while toks[last].is_comment() {
            last -= 1;
        }
        if last != open && !toks[last].is(",") {
            inserts.push(last + 1);
        }
    }
    inserts.sort_unstable();
    for idx in inserts.into_iter().rev() {
        toks.insert(idx, Tok::new(Kind::Punct, ","));
    }
}

/// Only lists that already use commas, and brace groups known to be comma lists.
fn qualifies(toks: &[Tok], open: usize, close: usize) -> bool {
    if separators(toks, open, close).is_empty() {
        return false;
    }
    if !toks[open].is("{") {
        return true;
    }
    let head = &toks[line_start(toks, open)..open];
    let decl = head.iter().take(2).any(|t| t.text == "struct" || t.text == "enum");
    decl || starts_like_fields(toks, open, close)
}

fn starts_like_fields(toks: &[Tok], open: usize, close: usize) -> bool {
    let mut it = toks[open + 1..close].iter().filter(|t| !t.is_comment());
    match (it.next(), it.next()) {
        (Some(a), Some(b)) => a.kind == Kind::Word && (b.is(",") || b.is(":")),
        _ => false,
    }
}
