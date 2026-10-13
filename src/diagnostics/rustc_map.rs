//! Shared-schema view of rustc errors that were translated by [`crate::error_mapper`].

use super::model::{Category, Diagnostic, Origin, Severity, Span};
use super::source::SourceIndex;
use crate::error_mapper::{DiagnosticLevel, WindjammerDiagnostic};
use crate::source_map::Location;
use std::collections::HashMap;

/// Reads `.wj` files once so byte offsets can be computed for mapped locations.
#[derive(Default)]
pub struct SourceCache {
    texts: HashMap<String, Option<String>>,
}

impl SourceCache {
    pub fn text(&mut self, file: &str) -> Option<&str> {
        self.texts
            .entry(file.to_string())
            .or_insert_with(|| std::fs::read_to_string(file).ok())
            .as_deref()
    }

    fn span(&mut self, loc: &Location, len_hint: usize) -> Span {
        let file = loc.file.to_string_lossy().to_string();
        match self.text(&file) {
            Some(t) => SourceIndex::new(file, t).word_span_at_or(loc.line, loc.column, len_hint),
            None => Span {
                file,
                byte_start: 0,
                byte_end: 0,
                line_start: loc.line,
                col_start: loc.column,
                line_end: loc.line,
                col_end: loc.column + len_hint.max(1),
                label: None,
            },
        }
    }
}

/// True when the diagnostic still points into generated Rust instead of `.wj` source.
pub fn points_into_rust(loc: &Location) -> bool {
    loc.file.extension().is_some_and(|e| e == "rs")
}

pub fn from_windjammer(d: &WindjammerDiagnostic, cache: &mut SourceCache) -> Diagnostic {
    let severity = match d.level {
        DiagnosticLevel::Error => Severity::Error,
        DiagnosticLevel::Warning => Severity::Warning,
        DiagnosticLevel::Note => Severity::Note,
        DiagnosticLevel::Help => Severity::Help,
    };
    let known = d
        .code
        .as_deref()
        .filter(|c| super::codes::lookup(c).is_some())
        .map(str::to_string);
    let code = known.unwrap_or_else(|| "WJ0099".to_string());
    let mut out = Diagnostic::new(&code, severity, d.message.clone());
    if code == "WJ0099" {
        out.category = Category::Internal;
        out.origin = Origin::Internal;
    } else {
        out.origin = Origin::RustcMapped;
    }
    out.primary_span = Some(refine(cache, &d.location, &code, &d.message, &mut out.notes));
    for s in d.spans.iter().filter(|s| !s.is_primary) {
        let mut sp = cache.span(&s.location, 1);
        sp.label = s.label.clone();
        out.secondary_spans.push(sp);
    }
    out.help = d.help.clone();
    out.notes.extend(d.notes.iter().cloned());
    out
}

fn is_decl_line(line: &str) -> bool {
    let t = line.trim_start();
    ["fn ", "pub ", "struct ", "enum ", "trait ", "impl ", "mod ", "use ", "//", "async fn "]
        .iter()
        .any(|p| t.starts_with(p))
}

fn whole_word(line: &str, word: &str) -> Option<usize> {
    line.match_indices(word).map(|(i, _)| i).find(|&i| {
        !line[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_')
            && !line[i + word.len()..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// Without a source map the mapped line is unreliable. Keep it when the line mentions the
/// identifier rustc complained about; otherwise find that identifier in the `.wj` text.
fn refine(cache: &mut SourceCache, loc: &Location, code: &str, message: &str, notes: &mut Vec<String>) -> Span {
    let file = loc.file.to_string_lossy().to_string();
    let base = cache.span(loc, 1);
    let Some(text) = cache.text(&file).map(str::to_string) else {
        return base;
    };
    let mut names: Vec<String> = message
        .split('`')
        .skip(1)
        .step_by(2)
        .flat_map(|t| t.split("::").map(str::to_string).collect::<Vec<_>>())
        .map(|t| t.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_')).to_string())
        .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .collect();
    if code == "WJ0010" {
        names = vec!["match".to_string()];
    }
    let ix = SourceIndex::new(file.clone(), &text);
    let lines: Vec<&str> = text.lines().collect();
    if let Some(line) = lines.get(loc.line.saturating_sub(1)) {
        for n in &names {
            if let Some(i) = whole_word(line, n) {
                return ix.span_at(loc.line, line[..i].chars().count() + 1, n.len());
            }
        }
    }
    let mut offset = 0;
    for (ln, line) in text.split_inclusive('\n').enumerate() {
        if !is_decl_line(line) || code == "WJ0010" {
            for n in &names {
                if let Some(i) = whole_word(line, n) {
                    return ix.span(offset + i, offset + i + n.len());
                }
            }
        }
        let _ = ln;
        offset += line.len();
    }
    notes.push("location is approximate (no source map for this file)".to_string());
    let last = lines.len().max(1);
    ix.word_span_at(loc.line.clamp(1, last), 1)
}
