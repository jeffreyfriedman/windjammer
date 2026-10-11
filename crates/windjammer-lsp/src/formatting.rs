//! LSP adapter for the Windjammer formatter: converts the pure formatter's line edits into
//! `textDocument/formatting` and `textDocument/rangeFormatting` results.

use std::path::Path;
use tower_lsp::lsp_types::{FormattingOptions, Position, Range, TextEdit, Url};
use windjammer::formatter::{document_edits, range_edits, FormatOptions, LineEdit};

/// Formatter options for a document: the nearest `wj.toml [fmt]` if one exists above the
/// file, otherwise defaults with the editor's tab size as indent width.
pub fn options_for(uri: &Url, editor: &FormattingOptions) -> FormatOptions {
    let from_toml = uri.to_file_path().ok().and_then(|p| nearest_wj_toml(&p));
    match from_toml {
        Some(text) => FormatOptions::from_toml_str(&text),
        None => FormatOptions {
            indent: (editor.tab_size as usize).clamp(1, 16),
            ..FormatOptions::default()
        },
    }
}

fn nearest_wj_toml(file: &Path) -> Option<String> {
    file.ancestors()
        .skip(1)
        .find_map(|dir| std::fs::read_to_string(dir.join("wj.toml")).ok())
}

/// Edits formatting the whole document; `None` if the source cannot be tokenized.
pub fn format_document(text: &str, opts: &FormatOptions) -> Option<Vec<TextEdit>> {
    let edits = document_edits(text, opts).ok()?;
    Some(to_text_edits(text, edits))
}

/// Edits formatting only the hunks touching `range`.
pub fn format_range(text: &str, opts: &FormatOptions, range: Range) -> Option<Vec<TextEdit>> {
    let last = if range.end.character == 0 && range.end.line > range.start.line {
        range.end.line - 1
    } else {
        range.end.line
    };
    let edits = range_edits(text, opts, range.start.line as usize, last as usize).ok()?;
    Some(to_text_edits(text, edits))
}

fn to_text_edits(text: &str, edits: Vec<LineEdit>) -> Vec<TextEdit> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    edits
        .into_iter()
        .map(|e| TextEdit {
            range: Range {
                start: line_start(&lines, e.start_line),
                end: line_start(&lines, e.end_line),
            },
            new_text: e.new_text,
        })
        .collect()
}

/// Position at the start of `line`; past the last line means end of document.
fn line_start(lines: &[&str], line: usize) -> Position {
    match lines.get(line) {
        Some(_) => Position::new(line as u32, 0),
        None => match lines.last() {
            Some(last) if !last.ends_with('\n') => {
                Position::new(lines.len() as u32 - 1, last.encode_utf16().count() as u32)
            }
            _ => Position::new(lines.len() as u32, 0),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, edits: &[TextEdit]) -> String {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let offset = |p: Position| -> usize {
            lines.iter().take(p.line as usize).map(|l| l.len()).sum::<usize>() + p.character as usize
        };
        let mut sorted: Vec<&TextEdit> = edits.iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse((e.range.start.line, e.range.start.character)));
        let mut out = text.to_string();
        for e in sorted {
            out.replace_range(offset(e.range.start)..offset(e.range.end), &e.new_text);
        }
        out
    }

    const UGLY: &str = "fn a(){\nlet x=1\n}\n\nfn b(){\nlet y=2\n}\n";
    const PRETTY: &str = "fn a() {\n    let x = 1\n}\n\nfn b() {\n    let y = 2\n}\n";

    #[test]
    fn document_formatting_returns_edits_that_produce_formatted_text() {
        let edits = format_document(UGLY, &FormatOptions::default()).unwrap();
        assert!(!edits.is_empty());
        assert_eq!(apply(UGLY, &edits), PRETTY);
    }

    #[test]
    fn formatted_document_yields_no_edits() {
        assert_eq!(format_document(PRETTY, &FormatOptions::default()).unwrap(), vec![]);
    }

    #[test]
    fn edits_are_minimal_untouched_lines_stay_out() {
        let edits = format_document(UGLY, &FormatOptions::default()).unwrap();
        assert!(edits.iter().all(|e| e.range.start.line != 3), "blank line must not be rewritten");
        assert!(edits.len() >= 2);
    }

    #[test]
    fn range_formatting_only_touches_requested_lines() {
        let range = Range::new(Position::new(4, 0), Position::new(6, 1));
        let edits = format_range(UGLY, &FormatOptions::default(), range).unwrap();
        let out = apply(UGLY, &edits);
        assert_eq!(out, "fn a(){\nlet x=1\n}\n\nfn b() {\n    let y = 2\n}\n");
    }

    #[test]
    fn unformattable_source_returns_none() {
        assert!(format_document("fn f() { # }", &FormatOptions::default()).is_none());
    }

    #[test]
    fn missing_trailing_newline_is_handled() {
        let src = "fn a(){\nlet x=1\n}";
        let edits = format_document(src, &FormatOptions::default()).unwrap();
        assert_eq!(apply(src, &edits), "fn a() {\n    let x = 1\n}\n");
    }
}
