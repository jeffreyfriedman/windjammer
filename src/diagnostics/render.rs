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
    if let Some(h) = d.help.first().filter(|h| !d.fixes.iter().any(|f| (!f.description.is_empty() && h.starts_with(f.description.as_str())))) {
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

/// The agent form of lint blocks printed in the human layout (`warning: MESSAGE [W0016]`, `  --> file:line:col`,
/// `  = note:` / `  = help:` lines, a blank line): one line each, `file:line:col: warning W0016: MESSAGE | help: ..`.
/// Notes are dropped and a help repeated from the previous block with the same code is not said twice. Other lines
/// pass through. `relative_to` is stripped from locations (the agent already knows its working directory).
pub fn agent_lint_text(text: &str, relative_to: Option<&str>) -> String {
    struct Block {
        level: String,
        code: String,
        message: String,
        at: String,
        help: String,
    }
    fn header(line: &str) -> Option<Block> {
        let (level, rest) = line.split_once(": ").filter(|(l, _)| matches!(*l, "warning" | "error"))?;
        let rest = rest.strip_suffix(']')?;
        let (message, code) = rest.rsplit_once(" [")?;
        Some(Block { level: level.to_string(), code: code.to_string(), message: message.to_string(), at: String::new(), help: String::new() })
    }
    let mut out = String::new();
    let mut open: Option<Block> = None;
    let mut last: Option<(String, String)> = None;
    let flush = |b: Block, out: &mut String, last: &mut Option<(String, String)>| {
        let at = if b.at.is_empty() { String::new() } else { format!("{}: ", b.at) };
        out.push_str(&format!("{at}{} {}: {}", b.level, b.code, b.message));
        let repeated = last.as_ref().is_some_and(|(c, h)| *c == b.code && *h == b.help);
        if !b.help.is_empty() && !repeated {
            out.push_str(&format!(" | help: {}", b.help));
        }
        out.push('\n');
        *last = Some((b.code, b.help));
    };
    for line in text.lines() {
        if let Some(b) = header(line) {
            if let Some(prev) = open.replace(b) {
                flush(prev, &mut out, &mut last);
            }
        } else if let Some(b) = open.as_mut() {
            if let Some(loc) = line.trim_start().strip_prefix("--> ") {
                b.at = relative_to.and_then(|base| loc.strip_prefix(base)).map(|l| l.trim_start_matches('/')).unwrap_or(loc).to_string();
            } else if let Some(h) = line.trim_start().strip_prefix("= help: ") {
                if b.help.is_empty() {
                    b.help = h.to_string();
                }
            } else if !line.trim_start().starts_with("= ") && !line.trim().is_empty() {
                let b = open.take().expect("open block");
                flush(b, &mut out, &mut last);
                out.push_str(line);
                out.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if let Some(b) = open {
        flush(b, &mut out, &mut last);
    }
    out
}

/// Lint blocks in the human layout where a block repeats the code and `= help:` line of the one before it: keep its
/// header and location, drop the note and help it would say again (a trait with three receiver-less methods
/// prints the same five lines three times). `relative_to` is stripped from locations. Other text passes through.
pub fn collapse_repeated_lint_blocks(text: &str, relative_to: Option<&str>) -> String {
    let mut out = String::new();
    let mut prev_key: Option<String> = None;
    let mut block: Vec<&str> = Vec::new();
    let mut flush = |block: &mut Vec<&str>, out: &mut String, prev_key: &mut Option<String>| {
        if block.is_empty() {
            return;
        }
        let is_lint = block[0].ends_with(']') && (block[0].starts_with("warning: ") || block[0].starts_with("error: "));
        let help = block.iter().find(|l| l.trim_start().starts_with("= help: ")).copied().unwrap_or("");
        let key = format!("{}|{help}", block[0].rsplit_once(" [").map(|(_, c)| c).unwrap_or(""));
        let repeated = is_lint && prev_key.as_deref() == Some(key.as_str());
        for l in block.iter().filter(|l| !repeated || !l.trim_start().starts_with("= ")) {
            match (l.strip_prefix("  --> "), relative_to) {
                (Some(loc), Some(base)) => out.push_str(&format!("  --> {}", loc.strip_prefix(base).map(|r| r.trim_start_matches('/')).unwrap_or(loc))),
                _ => out.push_str(l),
            }
            out.push('\n');
        }
        *prev_key = is_lint.then_some(key);
        block.clear();
    };
    for line in text.lines() {
        if line.trim().is_empty() {
            block.push(line);
            flush(&mut block, &mut out, &mut prev_key);
        } else {
            block.push(line);
        }
    }
    flush(&mut block, &mut out, &mut prev_key);
    out
}

#[cfg(test)]
mod agent_lint_tests {
    use super::{agent_lint_text, collapse_repeated_lint_blocks};

    #[test]
    fn a_repeated_lint_block_keeps_only_its_header_and_location() {
        let block = |n: &str| format!("warning: m `{n}` [W0016]\n  --> f.wj:1:1\n  = note: long\n  = help: h\n\n");
        let got = collapse_repeated_lint_blocks(&format!("{}{}", block("a"), block("b")), None);
        assert_eq!(got, format!("{}warning: m `b` [W0016]\n  --> f.wj:1:1\n\n", block("a")));
    }

    #[test]
    fn lint_blocks_become_one_line_each_and_repeated_help_is_said_once() {
        let block = |name: &str| {
            format!("warning: trait method `{name}` has no body [W0016]\n  --> /w/main.wj:1:1\n  = note: long note\n  = help: write `&self`\n\n")
        };
        let text = format!("{}{}error[WJ0034]: x\n", block("A::a"), block("A::b"));
        let got = agent_lint_text(&text, Some("/w"));
        assert_eq!(
            got,
            "main.wj:1:1: warning W0016: trait method `A::a` has no body | help: write `&self`\n\
             main.wj:1:1: warning W0016: trait method `A::b` has no body\n\
             error[WJ0034]: x\n"
        );
    }
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
