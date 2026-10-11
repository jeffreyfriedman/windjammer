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
            let (l, c) = parser.error_position().unwrap_or((1, 1));
            out.push(
                Diagnostic::error("WJ0011", tidy_parse_message(&msg))
                    .at(error_span(&ix, l, c).with_label("unexpected here")),
            );
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
    let mut mc = crate::errors::MutabilityChecker::new(std::path::PathBuf::from(&p.file));
    let mut fns: Vec<&crate::parser::FunctionDecl<'_>> = Vec::new();
    for item in &p.program.items {
        match item {
            crate::parser::Item::Function { decl, .. } => fns.push(decl),
            crate::parser::Item::Impl { block, .. } => fns.extend(block.functions.iter()),
            _ => {}
        }
    }
    let loc = crate::check::locate::Locator::new(&ix);
    for f in fns {
        for e in mc.check_function(f) {
            out.push(mutability(&e, &loc));
        }
    }
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
    d.with_help("Windjammer converts strings automatically; delete the construct")
}

fn mutability(e: &crate::errors::mutability::MutabilityError, loc: &crate::check::locate::Locator) -> Diagnostic {
    use crate::errors::mutability::MutabilityErrorType as T;
    let (code, msg) = match e.error_type {
        T::Reassignment => ("WJ0004", format!("cannot assign twice to immutable variable `{}`", e.variable)),
        T::CompoundAssignment => ("WJ0004", format!("cannot use compound assignment on immutable variable `{}`", e.variable)),
        T::FieldMutation => ("WJ0004", format!("cannot mutate a field of immutable binding `{}`", e.variable)),
        T::MutatingMethodCall => ("WJ0008", format!("cannot borrow `{}` as mutable, as it is not declared `mut`", e.variable)),
    };
    let mut d = Diagnostic::error(code, msg).with_help(e.suggestion.clone());
    if let Some(sp) = loc.word(&e.location, &e.variable) {
        d = d.at(sp.clone().with_label("not mutable"));
        if let Some(fix) = add_mut_fix(loc.ix, &e.variable, sp.byte_start) {
            d = d.with_fix(fix);
        }
    }
    d
}

/// `let x = ..` -> `let mut x = ..` for the nearest `let x` declaration before `before`.
fn add_mut_fix(ix: &SourceIndex, name: &str, before: usize) -> Option<crate::diagnostics::Fix> {
    let text = ix.text();
    let needle = format!("let {name}");
    let at = text[..before.min(text.len())].rfind(&needle)?;
    let after = text.get(at + needle.len()..)?;
    if after.chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let insert_at = at + 4;
    Some(crate::diagnostics::Fix {
        description: format!("declare `{name}` as mutable: `let mut {name}`"),
        applicability: crate::diagnostics::Applicability::MachineApplicable,
        edits: vec![crate::diagnostics::Edit {
            span: ix.span(insert_at, insert_at),
            replacement: "mut ".to_string(),
        }],
    })
}

/// Span of the token the parser stopped at; at end of file, the last character of the text.
fn error_span(ix: &SourceIndex, l: usize, c: usize) -> crate::diagnostics::Span {
    let sp = ix.word_span_from(ix.skip_ws(ix.offset(l, c)));
    if sp.byte_end > sp.byte_start {
        return sp;
    }
    let end = ix.text().trim_end().len();
    let start = ix.text()[..end].char_indices().next_back().map_or(0, |(i, _)| i);
    ix.span(start, end)
}
