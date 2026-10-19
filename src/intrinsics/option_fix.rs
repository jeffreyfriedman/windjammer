//! `wj fix` migrations to optional sugar (ADR-0018 feature 5), token based and conservative.
//!
//! * `Option<T>` in a type -> `T?` (only when the rewrite cannot change how the type parses).
//! * `recv.unwrap_or(d)` -> `recv ?? d` when `d` has no effects (no calls), and `recv.unwrap_or_else(|| e)` ->
//!   `recv ?? e`, both only when the whole call is a complete operand (`let x = ..`, an argument, `return ..`,
//!   a match-arm value), because `??` binds looser than arithmetic.
//! * `.unwrap()` is never rewritten: panicking and propagating differ, and `wj fix` never changes semantics.
//!
//! Idempotent: a second run finds nothing. The caller (`cli::fix`) keeps a file only if it parses before and after.

use crate::lexer::{Lexer, Token, TokenWithLocation};

struct Tok {
    t: Token,
    start: usize,
    line: usize,
}

/// One rewrite: replace `src[start..end]` with `text`.
struct Edit {
    start: usize,
    end: usize,
    text: String,
}

/// Count of rewrites and the new text.
pub struct OptionFix {
    pub text: String,
    pub rewrites: usize,
}

fn tokens(src: &str) -> Option<Vec<Tok>> {
    let s = src.to_string();
    let raw: Vec<TokenWithLocation> = std::panic::catch_unwind(move || Lexer::new(&s).tokenize_with_locations()).ok()?;
    let mut line_starts = vec![0usize];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let mut out = Vec::with_capacity(raw.len());
    for r in raw {
        let ls = *line_starts.get(r.line.checked_sub(1)?)?;
        let line_text = &src[ls..];
        let off = line_text.char_indices().nth(r.column.checked_sub(1)?).map(|(b, _)| b).unwrap_or(line_text.len());
        out.push(Tok { t: r.token, start: ls + off, line: r.line });
    }
    Some(out)
}

fn is_type_word(t: &Token) -> bool {
    matches!(t, Token::Ident(_) | Token::Int | Token::Int32 | Token::Uint | Token::Float | Token::Bool | Token::String | Token::ColonColon)
}

/// Index of the token closing the `<` at `open`, and how many bytes of it belong to this `<` and everything
/// before (a `>>` closes two levels: the first `>` ends the inner one, the second the outer one).
fn matching_gt(toks: &[Tok], open: usize) -> Option<(usize, usize)> {
    let mut depth = 0i32;
    for (i, tk) in toks.iter().enumerate().skip(open) {
        match tk.t {
            Token::Lt => depth += 1,
            Token::Gt => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, 1));
                }
            }
            Token::Shr => {
                if depth == 1 {
                    return Some((i, 1));
                }
                depth -= 2;
                if depth == 0 {
                    return Some((i, 2));
                }
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    None
}

fn option_types(src: &str, toks: &[Tok], edits: &mut Vec<Edit>) {
    let mut i = 0;
    while i + 2 < toks.len() {
        let is_option = matches!(&toks[i].t, Token::Ident(n) if n == "Option") && toks[i + 1].t == Token::Lt;
        let prev_blocks = i > 0 && matches!(toks[i - 1].t, Token::For | Token::Impl | Token::ColonColon);
        if !is_option || prev_blocks {
            i += 1;
            continue;
        }
        let Some((close, off)) = matching_gt(toks, i + 1) else {
            i += 1;
            continue;
        };
        let inner = &toks[i + 2..close];
        let mut depth = 0i32;
        let simple = !inner.is_empty()
            && inner.iter().all(|tk| match tk.t {
                Token::Lt => {
                    depth += 1;
                    true
                }
                Token::Gt => {
                    depth -= 1;
                    true
                }
                Token::Comma => depth > 0,
                Token::Question => true,
                ref t => is_type_word(t) && !matches!(t, Token::Ident(n) if n == "Option"),
            })
            && inner.last().is_some_and(|l| l.t != Token::Question)
            && toks.get(close + 1).is_none_or(|n| n.t != Token::ColonColon);
        if simple {
            let inner_src = src[toks[i + 2].start..toks[close].start + off - 1].trim_end();
            edits.push(Edit { start: toks[i].start, end: toks[close].start + off, text: format!("{inner_src}?") });
            i = close + 1;
        } else {
            i += 1;
        }
    }
}

fn matching_paren(toks: &[Tok], open: usize) -> Option<usize> {
    let (o, c) = match toks[open].t {
        Token::LParen => (Token::LParen, Token::RParen),
        Token::LBracket => (Token::LBracket, Token::RBracket),
        _ => return None,
    };
    let mut depth = 0;
    for (i, tk) in toks.iter().enumerate().skip(open) {
        if tk.t == o {
            depth += 1;
        } else if tk.t == c {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn matching_open(toks: &[Tok], close: usize) -> Option<usize> {
    let (o, c) = match toks[close].t {
        Token::RParen => (Token::LParen, Token::RParen),
        Token::RBracket => (Token::LBracket, Token::RBracket),
        _ => return None,
    };
    let mut depth = 0;
    for i in (0..=close).rev() {
        if toks[i].t == c {
            depth += 1;
        } else if toks[i].t == o {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Start index of the postfix chain that ends right before the `.` at `dot` (`a.b(c)[d].e`), if it is plain.
fn chain_start(toks: &[Tok], dot: usize) -> Option<usize> {
    let mut pos = dot;
    while pos > 0 {
        let prev = &toks[pos - 1].t;
        match prev {
            Token::RParen | Token::RBracket => pos = matching_open(toks, pos - 1)?,
            Token::Ident(_) | Token::Dot | Token::ColonColon | Token::Question | Token::Self_ => pos -= 1,
            Token::IntLiteral(_) | Token::FloatLiteral(_) | Token::StringLiteral(_) => pos -= 1,
            Token::Lt | Token::Gt | Token::Shr => return None, // turbofish / generics: not handled
            _ => break,
        }
    }
    (pos < dot).then_some(pos)
}

/// Tokens that may not appear at the top level of a `??` right-hand side without changing its meaning.
fn blocks_coalesce_rhs(t: &Token) -> bool {
    matches!(
        t,
        Token::Comma | Token::Semicolon | Token::Assign | Token::Pipe | Token::PipeOp | Token::PipeTry | Token::FatArrow | Token::Eq | Token::Ne
            | Token::Lt | Token::Le | Token::Gt | Token::Ge | Token::And | Token::Or | Token::QuestionQuestion | Token::DotDot | Token::DotDotEq
            | Token::LBrace | Token::RBrace | Token::Shr | Token::Shl | Token::Ampersand | Token::Caret | Token::Underscore
    )
}

fn unwrap_defaults(src: &str, toks: &[Tok], edits: &mut Vec<Edit>) {
    let mut i = 1;
    while i + 2 < toks.len() {
        let name = match &toks[i + 1].t {
            Token::Ident(n) if toks[i].t == Token::Dot && (n == "unwrap_or" || n == "unwrap_or_else") && toks[i + 2].t == Token::LParen => n.as_str(),
            _ => {
                i += 1;
                continue;
            }
        };
        let lazy = name == "unwrap_or_else";
        let Some(close) = matching_paren(toks, i + 2) else {
            i += 1;
            continue;
        };
        let mut arg = &toks[i + 3..close];
        if lazy {
            // `|| expr` (the lexer's `||` token); a block body or an explicit parameter list is left alone
            if arg.first().map(|t| &t.t) != Some(&Token::Or) {
                i += 1;
                continue;
            }
            arg = &arg[1..];
        }
        let delim_ok = chain_start(toks, i)
            .filter(|&s| s > 0)
            .is_some_and(|s| matches!(toks[s - 1].t, Token::Assign | Token::LParen | Token::LBracket | Token::Comma | Token::Return | Token::FatArrow));
        let next = toks.get(close + 1);
        let next_ok = match next {
            None => true,
            Some(n) => match n.t {
                Token::RParen | Token::Comma | Token::Semicolon | Token::RBrace | Token::RBracket => true,
                Token::Ident(_) | Token::Let | Token::Return | Token::If | Token::For | Token::While | Token::Match | Token::Break | Token::Continue => {
                    n.line > toks[close].line
                }
                _ => false,
            },
        };
        let mut depth = 0i32;
        let arg_ok = !arg.is_empty()
            && arg.iter().all(|tk| {
                match tk.t {
                    Token::LParen | Token::LBracket => depth += 1,
                    Token::RParen | Token::RBracket => depth -= 1,
                    _ => {}
                }
                depth > 0 || !blocks_coalesce_rhs(&tk.t)
            })
            && (lazy || !arg.iter().any(|tk| tk.t == Token::LParen));
        if !(delim_ok && next_ok && arg_ok) {
            i += 1;
            continue;
        }
        let arg_src = src[arg[0].start..toks[close].start].trim();
        if !arg_src.contains("//") && !arg_src.contains("/*") {
            edits.push(Edit { start: toks[i].start, end: toks[close].start + 1, text: format!(" ?? {arg_src}") });
            i = close + 1;
        } else {
            i += 1;
        }
    }
}

fn apply(src: &str, mut edits: Vec<Edit>) -> (String, usize) {
    edits.sort_by_key(|e| e.start);
    let mut kept: Vec<Edit> = Vec::new();
    for e in edits {
        if kept.last().is_none_or(|k| k.end <= e.start) {
            kept.push(e);
        }
    }
    let n = kept.len();
    let mut out = src.to_string();
    for e in kept.into_iter().rev() {
        out.replace_range(e.start..e.end, &e.text);
    }
    (out, n)
}

/// Rewrite `src` to optional sugar; repeats until nothing changes (nested `Option<Option<T>>`, chained defaults).
pub fn fix_options(src: &str) -> OptionFix {
    let mut text = src.to_string();
    let mut total = 0;
    for _ in 0..6 {
        let Some(toks) = tokens(&text) else { break };
        let mut edits = Vec::new();
        option_types(&text, &toks, &mut edits);
        unwrap_defaults(&text, &toks, &mut edits);
        let (next, n) = apply(&text, edits);
        if n == 0 {
            break;
        }
        total += n;
        text = next;
    }
    OptionFix { text, rewrites: total }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(s: &str) -> String {
        fix_options(s).text
    }

    #[test]
    fn option_types_become_suffix_types() {
        assert_eq!(fx("fn f(a: Option<int>) -> Option<string> { a }"), "fn f(a: int?) -> string? { a }");
        assert_eq!(fx("struct S { x: Option<Vec<int>> }"), "struct S { x: Vec<int>? }");
        assert_eq!(fx("let m: HashMap<string, Option<int>> = x"), "let m: HashMap<string, int?> = x");
        assert_eq!(fx("let o: Option<Option<int>> = x"), "let o: Option<int?> = x");
    }

    #[test]
    fn option_types_left_alone_when_unsafe() {
        assert_eq!(fx("let r: Option<&string> = x"), "let r: Option<&string> = x");
        assert_eq!(fx("impl Show for Option<int> {}"), "impl Show for Option<int> {}");
        assert_eq!(fx("let x = Option::<int>::None"), "let x = Option::<int>::None");
        assert_eq!(fx("// Option<int>\nlet s = \"Option<int>\""), "// Option<int>\nlet s = \"Option<int>\"");
    }

    #[test]
    fn unwrap_or_becomes_default_operator_only_as_a_whole_operand() {
        assert_eq!(fx("let n = a.unwrap_or(0)"), "let n = a ?? 0");
        assert_eq!(fx("let n = a.b.get(k).unwrap_or(-1)\nprintln(n)"), "let n = a.b.get(k) ?? -1\nprintln(n)");
        assert_eq!(fx("f(x.unwrap_or(\"d\"), 2)"), "f(x ?? \"d\", 2)");
        assert_eq!(fx("let n = a.unwrap_or_else(|| default_value())"), "let n = a ?? default_value()");
        // not a whole operand, effectful eager default, or other shapes: untouched
        assert_eq!(fx("let n = 1 + a.unwrap_or(0)"), "let n = 1 + a.unwrap_or(0)");
        assert_eq!(fx("let n = a.unwrap_or(0).abs()"), "let n = a.unwrap_or(0).abs()");
        assert_eq!(fx("let n = a.unwrap_or(next())"), "let n = a.unwrap_or(next())");
        assert_eq!(fx("let n = a.unwrap_or(0) > 3"), "let n = a.unwrap_or(0) > 3");
        assert_eq!(fx("let n = a.unwrap()"), "let n = a.unwrap()");
    }

    #[test]
    fn idempotent() {
        let once = fx("fn f(a: Option<int>) -> int { let n = a.unwrap_or(0)\n n }");
        assert_eq!(fx(&once), once);
    }
}
