//! Adapters from each producer's native diagnostic type to the shared [`Diagnostic`].

use super::model::{Applicability, Diagnostic, Edit, Fix, Severity};
use super::source::SourceIndex;
use crate::intrinsics::surface_fix::Finding;
use crate::linter::{LintDiagnostic, LintLevel};

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
    Some(out)
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
    let text = ix.text();
    let loc = &d.location;
    if markers.is_empty() {
        return ix.word_span_at(loc.line.max(1), loc.column);
    }
    let offset = ix.offset(loc.line.max(1), loc.column);
    let lo = offset.saturating_sub(600);
    let lo = (lo..=offset).find(|&i| text.is_char_boundary(i)).unwrap_or(0);
    let near = markers
        .iter()
        .filter_map(|m| text[lo..offset.min(text.len())].rfind(m).map(|i| (lo + i, m.len())))
        .max_by_key(|(i, _)| *i);
    let found = near.or_else(|| {
        let from_decl = |i: usize| {
            code != "W0001" || {
                let ls = text[..i].rfind('\n').map_or(0, |n| n + 1);
                text[ls..i].contains("fn ")
            }
        };
        markers
            .iter()
            .filter_map(|m| text.match_indices(m).map(|(i, _)| i).find(|&i| from_decl(i)).map(|i| (i, m.len())))
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
