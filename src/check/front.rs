//! File-local front end: lex, parse, forbidden patterns, lints, surface-syntax findings.

use crate::diagnostics::{convert, Diagnostic, SourceIndex};
use crate::lexer::Lexer;
use crate::parser::{Parser, Program};
use std::panic::{catch_unwind, AssertUnwindSafe};

pub struct Parsed {
    pub file: String,
    pub text: String,
    /// Keeps the arenas that `program` points into alive.
    pub parser: Parser,
    pub program: Program<'static>,
}

fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "internal error".to_string())
}

/// Run `f`, turning a panic into an `Err`. (The CLI silences the default panic message around
/// whole runs; library callers keep their own hook.)
fn quiet<T>(f: impl FnOnce() -> T) -> std::thread::Result<T> {
    catch_unwind(AssertUnwindSafe(f))
}

/// "Expected X, got Y (at token position N)" -> "expected X, found Y".
fn tidy_parse_message(msg: &str) -> String {
    let m = msg.split(" (at token position").next().unwrap_or(msg);
    let mut m = m
        .replacen("Expected ", "expected ", 1)
        .replacen(", got ", ", found ", 1)
        .replacen("Unexpected token in expression: ", "unexpected token in expression: ", 1);
    for (name, shown) in [
        ("RBrace", "`}`"), ("LBrace", "`{`"), ("RParen", "`)`"), ("LParen", "`(`"),
        ("RBracket", "`]`"), ("LBracket", "`[`"), ("Comma", "`,`"), ("Colon", "`:`"),
        ("Assign", "`=`"), ("Eof", "end of file"), ("Semicolon", "`;`"), ("Arrow", "`->`"),
        ("FatArrow", "`=>`"), ("Dot", "`.`"),
    ] {
        m = replace_word(&m, name, shown);
    }
    // Ident("x") -> `x`
    while let Some(i) = m.find("Ident(\"") {
        let Some(j) = m[i..].find("\")") else { break };
        let inner = m[i + 7..i + j].to_string();
        m.replace_range(i..i + j + 2, &format!("`{inner}`"));
    }
    m.trim().to_string()
}

fn replace_word(text: &str, word: &str, with: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(word) {
        let before_ok = !rest[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric());
        let after_ok = !rest[i + word.len()..].chars().next().is_some_and(|c| c.is_alphanumeric());
        out.push_str(&rest[..i]);
        out.push_str(if before_ok && after_ok { with } else { word });
        rest = &rest[i + word.len()..];
    }
    out.push_str(rest);
    out
}

/// Parse a file; syntax problems become diagnostics and `None` is returned.
pub fn parse(file: &str, text: &str, out: &mut Vec<Diagnostic>) -> Option<Parsed> {
    let ix = SourceIndex::new(file, text);
    let mut lexer = Lexer::new(text);
    let tokens = match quiet(|| lexer.tokenize_with_locations()) {
        Ok(t) => t,
        Err(p) => {
            let msg = panic_text(&p);
            let ch = msg.strip_prefix("Unexpected character: ").unwrap_or("?");
            let (l, c) = lexer.position();
            let span = ix.span_at(l, c.saturating_sub(1).max(1), ch.len());
            out.push(
                Diagnostic::error("WJ0012", format!("invalid character `{ch}`"))
                    .at(span.with_label("not valid here"))
                    .with_help("delete it, or put it inside a string literal"),
            );
            return None;
        }
    };
    let mut parser = Parser::new_with_source(tokens, file.to_string(), text.to_string());
    match quiet(|| parser.parse()) {
        Ok(Ok(program)) => Some(Parsed {
            file: file.to_string(),
            text: text.to_string(),
            parser,
            program,
        }),
        Ok(Err(msg)) => {
            let (msg, at) = split_position(&msg);
            let (l, c) = at.or_else(|| parser.error_position()).unwrap_or((1, 1));
            let (code, text, label) = match msg.strip_prefix("Lexical error: ") {
                Some(rest) => ("WJ0012", rest.to_string(), "not valid here"),
                None => ("WJ0011", tidy_parse_message(&msg), "unexpected here"),
            };
            out.push(Diagnostic::error(code, text).at(error_span(&ix, l, c).with_label(label)));
            None
        }
        Err(p) => {
            let (l, c) = parser.error_position().unwrap_or((1, 1));
            out.push(
                Diagnostic::error("WJ0011", format!("could not parse this file ({})", panic_text(&p)))
                    .at(error_span(&ix, l, c)),
            );
            None
        }
    }
}

/// Diagnostics that depend only on this file's AST and text.
pub fn file_checks(p: &Parsed, out: &mut Vec<Diagnostic>) {
    let ix = SourceIndex::new(p.file.clone(), &p.text);
    for w in p.parser.warnings() {
        if let Some(rest) = w.message.strip_prefix("W0010: ") {
            let (l, c) = (w.line.unwrap_or(1), w.column.unwrap_or(1));
            let span = locate_type_word(&ix, l, c);
            out.push(
                Diagnostic::error("W0010", rest.to_string())
                    .at(span)
                    .with_help("write `string`"),
            );
        }
    }
    for f in crate::intrinsics::surface_fix::lint_source(&p.text) {
        out.push(convert::finding(&f, &ix));
    }
    if let Err(msg) = crate::analyzer::Analyzer::new().check_forbidden_rust_patterns(&p.program) {
        out.push(forbidden(&msg, &ix));
    }
    let mut leak = crate::linter::rust_leakage::RustLeakageLinter::new(&p.file);
    leak.lint_program(&p.program);
    for d in leak.diagnostics() {
        if let Some(d) = convert::lint(d, &ix) {
            out.push(d);
        }
    }
    // No front-end mutability check: Windjammer infers `mut` for locals (the build accepts
    // `let x = 1` followed by `x = 2`), so the legacy MutabilityChecker would report false errors.
}

fn locate_type_word(ix: &SourceIndex, l: usize, c: usize) -> crate::diagnostics::Span {
    let line = ix.text().lines().nth(l.saturating_sub(1)).unwrap_or("");
    for w in ["String", "str"] {
        // The location is where the parser is *after* the type; search the line before it.
        let upto: String = line.chars().take(c.saturating_sub(1)).collect();
        if let Some(i) = upto.rfind(w) {
            return ix.span_at(l, upto[..i].chars().count() + 1, w.len());
        }
    }
    ix.word_span_at(l, c)
}

fn forbidden(msg: &str, ix: &SourceIndex) -> Diagnostic {
    let first = msg.lines().next().unwrap_or(msg).trim_start_matches("error: ");
    let mut d = Diagnostic::error("WJ0023", first.to_string());
    let needle = if first.contains("as_str") { ".as_str()" } else { "@derive" };
    if let Some(off) = ix.text().find(needle) {
        let mut sp = ix.span(off, off + needle.len());
        sp.label = Some("remove this".into());
        if needle == ".as_str()" {
            d = d.with_fix(crate::diagnostics::Fix {
                description: "remove `.as_str()`".into(),
                applicability: crate::diagnostics::Applicability::MachineApplicable,
                edits: vec![crate::diagnostics::Edit {
                    span: crate::diagnostics::Span { label: None, ..sp.clone() },
                    replacement: String::new(),
                }],
            });
        }
        d = d.at(sp);
    }
    d.with_help(if first.contains("derive") {
        "the compiler derives this automatically; delete the decorator argument"
    } else {
        "Windjammer converts strings automatically; delete the construct"
    })
}


/// Span of the token the parser stopped at; at end of file, the last character of the last token
/// (trailing whitespace and comments are not code).
fn error_span(ix: &SourceIndex, l: usize, c: usize) -> crate::diagnostics::Span {
    let sp = ix.word_span_from(ix.skip_ws(ix.offset(l, c)));
    if sp.byte_end > sp.byte_start {
        return sp;
    }
    let last = Lexer::new(ix.text())
        .tokenize_with_locations()
        .into_iter()
        .rev()
        .find(|t| !matches!(t.token, crate::lexer::Token::Eof | crate::lexer::Token::Newline));
    let end = last.map_or(ix.text().trim_end().len(), |t| t.byte_end);
    let start = ix.text()[..end].char_indices().next_back().map_or(0, |(i, _)| i);
    ix.span(start, end)
}

/// Split a trailing-or-embedded `(at line L, column C)` out of a parser message.
fn split_position(msg: &str) -> (String, Option<(usize, usize)>) {
    let Some(i) = msg.find(" (at line ") else {
        return (msg.to_string(), None);
    };
    let rest = &msg[i + " (at line ".len()..];
    let Some(close) = rest.find(')') else {
        return (msg.to_string(), None);
    };
    let inner = &rest[..close];
    let mut nums = inner.split(", column ").map(|n| n.trim().parse::<usize>().ok());
    let pos = match (nums.next().flatten(), nums.next().flatten()) {
        (Some(l), Some(c)) => Some((l, c)),
        _ => None,
    };
    let cleaned = format!("{}{}", &msg[..i], &rest[close + 1..]);
    (cleaned, pos)
}
