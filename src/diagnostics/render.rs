//! Plain-text (no color) rendering of a diagnostic, used by `wj check` and the golden tests.
//! The layout follows rustc: header, `--> file:line:col`, the source line with a caret
//! underline, then `= help:` / `= note:` lines.

use super::model::{Diagnostic, Severity, Span};

fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
        Severity::Help => "help",
    }
}

/// Render one diagnostic. `source_of` returns the text of a file (for the code excerpt).
pub fn render(d: &Diagnostic, source_of: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = format!("{}[{}]: {}\n", level(d.severity), d.code, d.message);
    if let Some(span) = &d.primary_span {
        out.push_str(&excerpt(span, true, source_of));
    }
    for s in &d.secondary_spans {
        out.push_str(&excerpt(s, false, source_of));
    }
    for h in &d.help {
        out.push_str(&format!("  = help: {h}\n"));
    }
    for n in &d.notes {
        out.push_str(&format!("  = note: {n}\n"));
    }
    for f in &d.fixes {
        let tag = match f.applicability {
            super::model::Applicability::MachineApplicable => "auto-fixable",
            _ => "suggested",
        };
        out.push_str(&format!("  = fix ({tag}): {}\n", f.description));
    }
    out.push_str(&format!("  = explain: {}\n", d.docs.explain));
    out
}

fn excerpt(span: &Span, primary: bool, source_of: &dyn Fn(&str) -> Option<String>) -> String {
    let arrow = if primary { "-->" } else { "::: " };
    let mut out = format!(
        "  {arrow} {}:{}:{}\n",
        span.file, span.line_start, span.col_start
    );
    let Some(text) = source_of(&span.file) else {
        return out;
    };
    let Some(line) = text.lines().nth(span.line_start.saturating_sub(1)) else {
        return out;
    };
    let gutter = span.line_start.to_string();
    let pad = " ".repeat(gutter.len());
    let width = if span.line_end == span.line_start {
        span.col_end.saturating_sub(span.col_start).max(1)
    } else {
        line.chars().count().saturating_sub(span.col_start - 1).max(1)
    };
    let mark = if primary { '^' } else { '-' };
    out.push_str(&format!("{pad} |\n{gutter} | {line}\n{pad} | "));
    out.push_str(&" ".repeat(span.col_start.saturating_sub(1)));
    out.push_str(&mark.to_string().repeat(width));
    if let Some(l) = &span.label {
        out.push(' ');
        out.push_str(l);
    }
    out.push('\n');
    out
}
