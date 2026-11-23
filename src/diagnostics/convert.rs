//! Adapters from each producer's native diagnostic type to the shared [`Diagnostic`].

use super::model::{Applicability, Diagnostic, Edit, Fix, Severity};
use super::source::SourceIndex;
use crate::intrinsics::surface_fix::Finding;
use crate::linter::{LintDiagnostic, LintLevel};

/// `mut self`: the span covers `mut` and the space after it; the fix deletes exactly that.
pub fn mut_self_diagnostic(ix: &SourceIndex, msg: &str, l: usize, c: usize) -> Diagnostic {
    let line = ix.text().lines().nth(l.saturating_sub(1)).unwrap_or("");
    let tail: String = line.chars().skip(c.saturating_sub(1)).collect();
    let word = if tail.starts_with("mut") { "mut" } else { "" };
    let gap = tail[word.len()..].chars().take_while(|ch| ch.is_whitespace()).count();
    let sp = ix.span_at(l, c, word.len() + gap);
    let mut d = Diagnostic::error("WJ0011", msg.to_string()).at(sp.clone().with_label("remove `mut`"));
    d = d.with_help("write `self`: Windjammer infers `&self`, `&mut self` or owned from how the body uses it; `let mut x` is for locals");
    if !word.is_empty() {
        d = d.with_fix(Fix {
            description: "remove `mut`".into(),
            applicability: Applicability::MachineApplicable,
            edits: vec![Edit { span: super::model::Span { label: None, ..sp }, replacement: String::new() }],
        });
    }
    d
}

/// Lint names that predate the `Wxxxx` scheme.
fn lint_code(name: &str) -> &str {
    match name {
        "owned-but-not-returned" => "W0011",
        other => other,
    }
}

/// A `LintDiagnostic` (W0001-W0006, W0011) as a shared diagnostic.
pub fn lint(d: &LintDiagnostic, ix: &SourceIndex) -> Option<Diagnostic> {
    let severity = match d.level {
        LintLevel::Error => Severity::Error,
        LintLevel::Warning => Severity::Warning,
        LintLevel::Note => Severity::Note,
        LintLevel::Allow => return None,
    };
    let code = lint_code(&d.lint_name);
    let mut out = Diagnostic::new(code, severity, d.message.clone()).at(lint_span(code, d, ix));
    if let Some(h) = &d.help {
        out = out.with_help(h.clone());
    }
    if let Some(n) = &d.note {
        out = out.with_note(n.clone());
    }
    if let Some(fix) = out.primary_span.as_ref().and_then(|sp| lint_fix(code, sp, ix)) {
        out = out.with_fix(fix);
    }
    Some(out)
}

/// The exact edit for a lint whose fix is mechanical: delete the call the lint names (`"x".to_string()`, `.iter()`), or
/// spell a bodyless trait method's receiver `&self`.
fn lint_fix(code: &str, sp: &super::model::Span, ix: &SourceIndex) -> Option<Fix> {
    let text = ix.text();
    let (start, end) = (sp.byte_start, sp.byte_end);
    let word = text.get(start..end)?;
    match code {
        "W0006" | "W0003" if matches!(word, "to_string" | "iter" | "iter_mut") && start > 0 && text[..start].ends_with('.') && text[end..].starts_with("()") => {
            let (from, to) = (start - 1, end + 2);
            Some(Fix {
                description: format!("remove `{}`", &text[from..to]),
                applicability: Applicability::MachineApplicable,
                edits: vec![Edit { span: ix.span(from, to), replacement: String::new() }],
            })
        }
        "W0016" if word == "self" && !text[..start].ends_with('&') => Some(Fix {
            description: "declare the receiver `&self` (`&mut self` when an implementation changes it)".to_string(),
            applicability: Applicability::MaybeIncorrect,
            edits: vec![Edit { span: ix.span(start, end), replacement: "&self".to_string() }],
        }),
        _ => None,
    }
}

/// A W0007/W0008 finding, with the exact rewrite as a machine-applicable fix.
pub fn finding(f: &Finding, ix: &SourceIndex) -> Diagnostic {
    let span = ix.span(f.offset, f.end);
    let mut d = Diagnostic::warning(f.code, f.message.clone()).at(span.clone());
    match &f.replacement {
        Some(rep) => {
            d = d
                .with_help(format!("replace `{}` with `{}`", f.original, rep))
                .with_fix(Fix {
                    description: format!("replace `{}` with `{}`", f.original, rep),
                    applicability: Applicability::MachineApplicable,
                    edits: vec![Edit {
                        span: ix.span(f.offset, f.end),
                        replacement: rep.clone(),
                    }],
                });
        }
        None => d = d.with_help("migrate manually (see docs/design/surface-syntax.md)"),
    }
    d.with_note("ADR-0014: Windjammer has no macros; the `!` forms become errors in a future release")
}

/// Classify an IR diagnostic message into a stable code.
pub fn ir(severity_is_error: bool, severity_is_warning: bool, message: &str) -> Diagnostic {
    let code = if message.starts_with("Effect violation") {
        "WJ0024"
    } else if message.starts_with("Taint") {
        "WJ0025"
    } else if message.starts_with("Execution") {
        "WJ0026"
    } else {
        "WJ0028"
    };
    let severity = if severity_is_error {
        Severity::Error
    } else if severity_is_warning {
        Severity::Warning
    } else {
        Severity::Note
    };
    Diagnostic::new(code, severity, message.to_string())
}

/// A marker such as `.unwrap` must not be the front of a longer name (`.unwrap_or`).
/// Only a marker that itself ends in a name character (`.unwrap`) is at risk: `&` and `&mut ` end in punctuation and
/// are followed by the borrowed name.
fn ends_at_word_boundary(text: &str, end: usize) -> bool {
    let name_char = |c: char| c.is_alphanumeric() || c == '_';
    !text[..end.min(text.len())].ends_with(name_char) || !text[end.min(text.len())..].starts_with(name_char)
}

/// `text` with every comment blanked (same byte length, newlines kept), so a text search for a method name never
/// lands in a comment that mentions it. Comments are what the lexer leaves between tokens (plain `//`, `/* */`) and
/// its `DocComment` tokens; the code inside a string interpolation belongs to its token and stays.
fn without_comments(text: &str) -> String {
    use crate::lexer::{Lexer, Token};
    let tokens = Lexer::new(text).tokenize_with_locations();
    let mut bytes = text.as_bytes().to_vec();
    let blank = |bytes: &mut Vec<u8>, from: usize, to: usize| {
        for b in &mut bytes[from.min(to)..to.min(text.len())] {
            if !b.is_ascii_whitespace() {
                *b = b' ';
            }
        }
    };
    let mut at = 0;
    for t in &tokens {
        blank(&mut bytes, at, t.byte_start);
        if matches!(t.token, Token::DocComment(_)) {
            blank(&mut bytes, t.byte_start, t.byte_end);
        }
        at = t.byte_end.max(at);
    }
    blank(&mut bytes, at, text.len());
    String::from_utf8(bytes).unwrap_or_else(|_| text.to_string())
}

/// Lint locations come from AST nodes (0-based columns, often just after the construct, or a
/// default of 1:1); find the offending text itself.
fn lint_span(code: &str, d: &LintDiagnostic, ix: &SourceIndex) -> super::model::Span {
    let markers: &[&str] = match code {
        "W0002" => &[".unwrap", ".expect"],
        "W0003" => &[".iter_mut", ".iter"],
        "W0005" => &[".clone"],
        "W0006" => &[".to_string"],
        "W0004" => &["&mut ", "&"],
        "W0001" => &["&mut self", "&self", "&mut ", "&"],
        _ => &[],
    };
    let loc = &d.location;
    if markers.is_empty() {
        return ix.word_span_at(loc.line.max(1), loc.column);
    }
    let code_only = without_comments(ix.text());
    let text = code_only.as_str();
    let offset = ix.offset(loc.line.max(1), loc.column);
    let lo = offset.saturating_sub(600);
    let lo = (lo..=offset).find(|&i| text.is_char_boundary(i)).unwrap_or(0);
    let near = markers
        .iter()
        .filter_map(|m| {
            text[lo..offset.min(text.len())]
                .match_indices(m)
                .map(|(i, _)| lo + i)
                .filter(|&i| ends_at_word_boundary(text, i + m.len()))
                .last()
                .map(|i| (i, m.len()))
        })
        .max_by_key(|(i, _)| *i);
    // A method call's location is where the call's receiver starts, so its `.method` lies AHEAD of it; the match
    // behind it belongs to an earlier call (a second `o.unwrap()` would else land on the first one's span, and
    // the two diagnostics collapse into one). The nearer of the two is the call itself.
    let ahead = || {
        let hi = (offset + 600).min(text.len());
        let hi = (offset..=hi).rev().find(|&i| text.is_char_boundary(i)).unwrap_or(offset);
        markers
            .iter()
            .filter(|m| m.starts_with('.'))
            .filter_map(|m| {
                text.get(offset.min(text.len())..hi)?
                    .match_indices(m)
                    .map(|(i, _)| offset + i)
                    .find(|&i| ends_at_word_boundary(text, i + m.len()))
                    .map(|i| (i, m.len()))
            })
            .min_by_key(|(i, _)| *i)
    };
    let near = match (near, ahead()) {
        (Some(back), Some(fwd)) if fwd.0 - offset <= offset.saturating_sub(back.0) => Some(fwd),
        (None, Some(fwd)) => Some(fwd),
        (back, _) => back,
    };
    let found = near.or_else(|| {
        let from_decl = |i: usize| {
            code != "W0001" || {
                let ls = text[..i].rfind('\n').map_or(0, |n| n + 1);
                text[ls..i].contains("fn ")
            }
        };
        markers
            .iter()
            .filter_map(|m| {
                text.match_indices(m)
                    .map(|(i, _)| i)
                    .find(|&i| from_decl(i) && ends_at_word_boundary(text, i + m.len()))
                    .map(|i| (i, m.len()))
            })
            .min_by_key(|(i, _)| *i)
    });
    match found {
        Some((i, len)) => {
            // `.unwrap` -> point at the method name, not the dot.
            let start = if text[i..].starts_with('.') { i + 1 } else { i };
            let end = (i + len).max(start + 1);
            ix.span(start, end.min(text.len()))
        }
        None => ix.word_span_at(loc.line.max(1), loc.column),
    }
}
