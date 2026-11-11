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

/// Output format of a diagnostic: `Human` (the rustc-like layout) or `Agent` (one line per diagnostic: location, code,
/// message, the first help line and the fixes; no code excerpt, notes or explain line). The CLI driver picks it from
/// `--error-format agent` or `WJ_ERROR_FORMAT=agent` (ADR-0037) and passes it down; nothing here reads the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Human,
    Agent,
}

impl Format {
    /// `agent` -> Agent; anything else (or nothing) -> Human.
    pub fn parse(name: Option<&str>) -> Format {
        if name == Some("agent") {
            Format::Agent
        } else {
            Format::Human
        }
    }
}

/// One line: `file:line:col: level CODE: message | help: .. | fix: .. => `replacement``.
pub fn render_agent(d: &Diagnostic) -> String {
    let at = d.primary_span.as_ref().map(|s| format!("{}:{}:{}: ", s.file, s.line_start, s.col_start)).unwrap_or_default();
    let mut out = format!("{at}{} {}: {}", level(d.severity), d.code, d.message.replace('\n', " "));
    if let Some(h) = d.help.first().filter(|h| !d.fixes.iter().any(|f| &f.description == *h)) {
        out.push_str(&format!(" | help: {}", h.replace('\n', " ")));
    }
    for f in &d.fixes {
        let auto = matches!(f.applicability, super::model::Applicability::MachineApplicable);
        out.push_str(&format!(" | fix{}: {}", if auto { "(auto: wj check --fix)" } else { "" }, f.description));
        if let [e] = f.edits.as_slice() {
            if !e.replacement.contains('\n') && e.replacement.len() <= 80 {
                out.push_str(&format!(" => `{}`", e.replacement));
            }
        }
    }
    out.push('\n');
    out
}

/// Render one diagnostic in `format`.
pub fn render_as(d: &Diagnostic, source_of: &dyn Fn(&str) -> Option<String>, format: Format) -> String {
    match format {
        Format::Agent => render_agent(d),
        Format::Human => render(d, source_of),
    }
}

/// Render one diagnostic (human layout). `source_of` returns the text of a file (for the code excerpt).
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
