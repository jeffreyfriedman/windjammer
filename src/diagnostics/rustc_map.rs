//! Shared-schema view of rustc errors that were translated by [`crate::error_mapper`].

use super::model::{Applicability, Category, Diagnostic, Edit, Fix, Origin, Severity, Span};
use super::source::SourceIndex;
use crate::error_mapper::{DiagnosticLevel, WindjammerDiagnostic};
use crate::source_map::Location;
use std::collections::HashMap;

/// Reads `.wj` files once so byte offsets can be computed for mapped locations.
#[derive(Default)]
pub struct SourceCache {
    texts: HashMap<String, Option<String>>,
    /// Directory that relative `.wj` paths in source maps are relative to (the project being run).
    root: Option<std::path::PathBuf>,
}

impl SourceCache {
    pub fn with_root(root: impl Into<std::path::PathBuf>) -> Self {
        Self { texts: HashMap::new(), root: Some(root.into()) }
    }

    /// A relative path that exists under the root becomes that absolute path.
    pub fn resolve(&self, file: &std::path::Path) -> std::path::PathBuf {
        match &self.root {
            Some(r) if file.is_relative() && !file.exists() && r.join(file).exists() => r.join(file),
            _ => file.to_path_buf(),
        }
    }

    pub fn text(&mut self, file: &str) -> Option<&str> {
        self.texts
            .entry(file.to_string())
            .or_insert_with(|| std::fs::read_to_string(file).ok())
            .as_deref()
    }

    fn span(&mut self, loc: &Location, len_hint: usize) -> Span {
        let file = self.resolve(&loc.file).to_string_lossy().to_string();
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

pub fn from_windjammer(d: &WindjammerDiagnostic, cache: &mut SourceCache) -> Diagnostic {
    let mut resolved = d.clone();
    resolved.location.file = cache.resolve(&d.location.file);
    let d = &resolved;
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
    // The expression rustc highlighted usually appears verbatim in the `.wj` source. When it occurs
    // exactly once in the file, that is a better location than the (approximate) line map.
    let mut line_for_fix = d.location.line;
    if let Some(needle) = d.primary_text.as_deref().map(str::trim).filter(|n| n.chars().count() >= 4) {
        let file = d.location.file.to_string_lossy().to_string();
        if let Some(text) = cache.text(&file).map(str::to_string) {
            let hits: Vec<usize> = text.match_indices(needle).map(|(i, _)| i).collect();
            if hits.len() == 1 {
                let sp = SourceIndex::new(file, &text).span(hits[0], hits[0] + needle.len());
                line_for_fix = sp.line_start;
                out.primary_span = Some(sp);
            } else if hits.is_empty() {
                // Generated Rust adds parentheses and spaces: compare with those removed, line by line.
                let norm = |t: &str| {
                    // Codegen also suffixes integer/float literals (`48_i64`).
                    let mut t = t.to_string();
                    for suf in ["_i8", "_i16", "_i32", "_i64", "_i128", "_isize", "_u8", "_u16", "_u32", "_u64", "_u128", "_usize", "_f32", "_f64"] {
                        t = t.replace(suf, "");
                    }
                    t.chars().filter(|c| !c.is_whitespace() && *c != '(' && *c != ')').collect::<String>()
                };
                let n = norm(needle);
                if n.chars().count() >= 4 {
                    let mut found = text.split_inclusive('\n').scan(0usize, |off, l| {
                        let start = *off;
                        *off += l.len();
                        Some((start, l))
                    }).filter(|(_, l)| norm(l).contains(&n));
                    if let (Some((start, l)), None) = (found.next(), found.next()) {
                        let lead = l.len() - l.trim_start().len();
                        let sp = SourceIndex::new(file, &text).span(start + lead, start + l.trim_end().len());
                        line_for_fix = sp.line_start;
                        out.primary_span = Some(sp);
                    }
                }
            }
        }
    }
    if let Some(hint) = &d.fix {
        let file = d.location.file.to_string_lossy().to_string();
        if let Some(text) = cache.text(&file).map(str::to_string) {
            if let Some((fix, exact)) = locate_fix(&file, &text, line_for_fix, hint, d.primary_text.as_deref()) {
                if let Some(sp) = exact {
                    out.primary_span = Some(sp);
                }
                out.fixes.push(fix);
            }
        }
    }
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

/// Find the expression rustc complained about on the mapped `.wj` line and build the edit.
/// Only exact, unique matches yield a fix: the fix is never a guess at a location.
fn locate_fix(
    file: &str,
    text: &str,
    line_no: usize,
    hint: &super::hints::FixHint,
    primary_text: Option<&str>,
) -> Option<(Fix, Option<Span>)> {
    use super::hints::FixAnchor;
    let ix = SourceIndex::new(file.to_string(), text);
    let line_start = ix.offset(line_no, 1);
    let line = text.get(line_start..)?.lines().next()?;
    let unique = |needle: &str, whole_word_only: bool| -> Option<usize> {
        let hits: Vec<usize> = line
            .match_indices(needle)
            .map(|(i, _)| i)
            .filter(|&i| {
                !whole_word_only
                    || (!line[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_')
                        && !line[i + needle.len()..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_'))
            })
            .collect();
        (hits.len() == 1).then(|| hits[0])
    };
    let insert_at = |at: usize, s: &str| Edit { span: ix.span(line_start + at, line_start + at), replacement: s.to_string() };
    let (edits, exact) = match &hint.anchor {
        FixAnchor::Expr { prefix, suffix } => {
            let needle = primary_text?.trim();
            if needle.is_empty() {
                return None;
            }
            let i = unique(needle, true)?;
            let mut edits = Vec::new();
            if !prefix.is_empty() {
                edits.push(insert_at(i, prefix));
            }
            if !suffix.is_empty() {
                edits.push(insert_at(i + needle.len(), suffix));
            }
            (edits, Some(ix.span(line_start + i, line_start + i + needle.len())))
        }
        FixAnchor::BeforeText { needle, insert } => {
            let i = unique(needle, false)?;
            (vec![insert_at(i, insert)], None)
        }
    };
    Some((Fix { description: hint.description.clone(), applicability: Applicability::MaybeIncorrect, edits }, exact))
}
