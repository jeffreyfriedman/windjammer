//! Gate 3: diagnostic location oracle (rust-ui-test style inline markers).
//!
//! A negative program declares each expected diagnostic in a marker comment on the line(s)
//! after the offending source line:
//!
//! ```text
//!     let x = foo(1,2)
//!     //~     ^^^ WARN W0001 message substring
//!     //~@5+3 WARN W0002            (explicit form: 1-based column, optional +length)
//! ```
//!
//! * A marker refers to the nearest preceding non-marker line.
//! * Underline form: the first `^` sits in the same character column as the first character of
//!   the diagnostic; the number of carets is its length in chars. Needs column >= 5 because of the
//!   `//~ ` prefix; use the explicit form for columns 1-4.
//! * `//~@eof` marks the end-of-file position (an error found at the EOF token).
//! * Severity: `ERROR`, `WARN` or `NOTE`. Code: the diagnostic code (`W0007`, ...) or the
//!   producer class for code-less diagnostics: `parse` (parser error), `lex` (lexer failure).
//! * Everything after the code is a message substring that must occur in the message.
//!
//! The harness fails on (a) a diagnostic with no marker, (b) a marker with no diagnostic, (c) a
//! location/length/code mismatch, (d) a diagnostic without a real (line, column), and (e) in
//! strict mode, an invalid span handed to a diagnostic constructor.

use super::naive::offset_of;
use windjammer::lexer::Lexer;
use windjammer::linter::LintLevel;
use windjammer::parser_impl::Parser;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Marker {
    pub line: usize,
    pub col: usize,
    pub len: Option<usize>,
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Found {
    pub producer: &'static str,
    pub line: usize,
    pub col: usize,
    pub len: Option<usize>,
    pub severity: String,
    pub code: String,
    pub message: String,
}

pub fn is_marker_line(l: &str) -> bool {
    l.trim_start().starts_with("//~")
}

pub fn parse_markers(src: &str) -> Result<Vec<Marker>, String> {
    let mut out = Vec::new();
    let mut target = 0usize;
    let mut eof_line: Option<usize> = None;
    for (i, raw) in src.split('\n').enumerate() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if !is_marker_line(raw) {
            target = i + 1;
            continue;
        }
        let at = raw.find("//~").unwrap();
        let body = &raw[at + 3..];
        let col_of_body = raw[..at + 3].chars().count() + 1;
        let (col, len, rest) = if let Some(r) = body.strip_prefix("@eof") {
            // End of file: the position just past the last character of the text.
            let (l, c) = super::naive::line_col(src, src.len());
            eof_line = Some(l);
            (c, None, r)
        } else if let Some(r) = body.strip_prefix('@') {
            let end = r.find(char::is_whitespace).unwrap_or(r.len());
            let (spec, rest) = r.split_at(end);
            let (c, l) = match spec.split_once('+') {
                Some((c, l)) => (c, Some(l)),
                None => (spec, None),
            };
            let col: usize = c.parse().map_err(|_| format!("line {}: bad column in `{spec}`", i + 1))?;
            let len = l.map(|l| l.parse::<usize>()).transpose().map_err(|_| format!("line {}: bad length", i + 1))?;
            (col, len, rest)
        } else {
            let first = body.find('^').ok_or_else(|| format!("line {}: marker needs `^^^` or `@col`", i + 1))?;
            let carets = body[first..].chars().take_while(|c| *c == '^').count();
            let col = col_of_body + body[..first].chars().count();
            (col, Some(carets), &body[first + carets..])
        };
        let mut words = rest.split_whitespace();
        let severity = words.next().ok_or_else(|| format!("line {}: missing severity", i + 1))?.to_string();
        if !["ERROR", "WARN", "NOTE"].contains(&severity.as_str()) {
            return Err(format!("line {}: severity must be ERROR|WARN|NOTE, got {severity}", i + 1));
        }
        let code = words.next().ok_or_else(|| format!("line {}: missing code", i + 1))?.to_string();
        let message = words.collect::<Vec<_>>().join(" ");
        if target == 0 {
            return Err(format!("line {}: marker before any source line", i + 1));
        }
        out.push(Marker { line: eof_line.take().unwrap_or(target), col, len, severity, code, message });
    }
    Ok(out)
}

/// Position of a located message: a `file:LINE:COL` prefix (`t.wj:2:13: error: ...`, pipe and
/// interpolation errors) or an `(at line L, column C)` suffix (parser and lexer errors).
pub fn location_in_message(msg: &str) -> Option<(usize, usize)> {
    colon_pair(msg).or_else(|| line_column_words(msg))
}

fn line_column_words(msg: &str) -> Option<(usize, usize)> {
    let i = msg.find("line ")?;
    let rest = &msg[i + 5..];
    let line: usize = rest.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()?;
    let j = rest.find("column ")?;
    let col: usize = rest[j + 7..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()?;
    Some((line, col))
}

fn colon_pair(msg: &str) -> Option<(usize, usize)> {
    let b = msg.as_bytes();
    let num = |from: usize| -> Option<(usize, usize)> {
        let mut j = from;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > from {
            msg[from..j].parse().ok().map(|n| (n, j))
        } else {
            None
        }
    };
    for i in 0..b.len() {
        if b[i] == b':' {
            if let Some((line, j)) = num(i + 1) {
                if j < b.len() && b[j] == b':' {
                    if let Some((col, _)) = num(j + 1) {
                        return Some((line, col));
                    }
                }
            }
        }
    }
    None
}

fn bracket_code(msg: &str) -> Option<String> {
    let s = msg.trim_start().strip_prefix('[')?;
    let end = s.find(']')?;
    Some(s[..end].to_string())
}

/// Run `wj check`'s in-process front end (`windjammer::check::check_sources`) on one file and
/// return its diagnostics in the oracle's normalised form.
pub fn collect_check(file: &str, src: &str) -> Vec<Found> {
    check_diagnostics(file, src)
        .into_iter()
        .map(|d| {
            let (line, col, len) = match &d.primary_span {
                Some(s) => {
                    let len = (s.line_start == s.line_end && s.byte_end <= src.len() && s.byte_start <= s.byte_end && src.is_char_boundary(s.byte_start) && src.is_char_boundary(s.byte_end))
                        .then(|| src[s.byte_start..s.byte_end].chars().count());
                    (s.line_start, s.col_start, len)
                }
                None => (0, 0, None),
            };
            Found {
                producer: "check",
                line,
                col,
                len,
                severity: match d.severity {
                    windjammer::diagnostics::Severity::Error => "ERROR",
                    windjammer::diagnostics::Severity::Warning => "WARN",
                    _ => "NOTE",
                }
                .into(),
                code: d.code.clone(),
                message: d.message.clone(),
            }
        })
        .collect()
}

/// The raw diagnostics of `check_sources` (panics inside the checker become one `panic` diagnostic).
pub fn check_diagnostics(file: &str, src: &str) -> Vec<windjammer::diagnostics::Diagnostic> {
    windjammer::error::span_check::set_strict(true);
    let (f, s) = (file.to_string(), src.to_string());
    std::panic::catch_unwind(move || windjammer::check::check_sources(std::path::Path::new("."), &[(f, s)])).unwrap_or_else(|p| {
        let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
        vec![windjammer::diagnostics::Diagnostic::error("WJ0099", format!("checker panicked: {msg}"))]
    })
}

/// Run every in-process front-end producer on `src` (legacy: parser, parser warnings, rust-leakage linter).
pub fn collect(file: &str, src: &str) -> Vec<Found> {
    windjammer::error::span_check::set_strict(true);
    let (f, s) = (file.to_string(), src.to_string());
    let r = std::panic::catch_unwind(move || collect_inner(&f, &s));
    match r {
        Ok(v) => v,
        Err(p) => {
            let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            vec![Found { producer: "panic", line: 0, col: 0, len: None, severity: "ERROR".into(), code: "panic".into(), message: msg }]
        }
    }
}

fn collect_inner(file: &str, src: &str) -> Vec<Found> {
    let mut out = Vec::new();
    let tokens = Lexer::new(src).tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, file.to_string(), src.to_string());
    parser.enable_span_log();
    match parser.parse() {
        Err(msg) => {
            // The parser recovers and reports up to 25 errors, one per line.
            for m in msg.lines().filter(|l| !l.trim().is_empty()) {
                let (line, col) = location_in_message(m).unwrap_or((0, 0));
                out.push(Found { producer: "parser", line, col, len: None, severity: "ERROR".into(), code: "parse".into(), message: m.to_string() });
            }
            return out;
        }
        Ok(program) => {
            let mut seen = std::collections::BTreeSet::new();
            for w in parser.warnings() {
                let (line, col) = (w.line.unwrap_or(0), w.column.unwrap_or(0));
                let code = bracket_code(&w.message).unwrap_or_else(|| "parse-warning".into());
                seen.insert((code.clone(), line, col));
                out.push(Found {
                    producer: "parser-warning",
                    line,
                    col,
                    len: None,
                    severity: if w.is_error { "ERROR" } else { "WARN" }.into(),
                    code,
                    message: w.message.clone(),
                });
            }
            let mut linter = windjammer::linter::rust_leakage::RustLeakageLinter::new(file).with_spans(parser.span_snapshot());
            linter.lint_program(&program);
            linter.lint_source(src);
            for d in linter.into_diagnostics() {
                if seen.contains(&(d.lint_name.clone(), d.location.line, d.location.column)) {
                    continue; // the same finding already reported by the parser
                }
                out.push(Found {
                    producer: "lint",
                    line: d.location.line,
                    col: d.location.column,
                    len: None,
                    severity: match d.level {
                        LintLevel::Error => "ERROR",
                        LintLevel::Warning => "WARN",
                        _ => "NOTE",
                    }
                    .into(),
                    code: d.lint_name.clone(),
                    message: d.message.clone(),
                });
            }
        }
    }
    // Extent-carrying producer: the surface-syntax engine knows the byte range of each finding.
    for f in windjammer::intrinsics::surface_fix::lint_source(src) {
        for o in out.iter_mut().filter(|o| o.code == f.code && (o.line, o.col) == (f.line, f.col)) {
            o.len = Some(f.original.chars().count());
        }
    }
    out
}

/// Compare markers and diagnostics for one file; returns violations.
pub fn compare(src: &str, markers: &[Marker], found: &[Found]) -> Vec<String> {
    let mut v = Vec::new();
    let mut unmatched: Vec<&Found> = found.iter().collect();
    for m in markers {
        let pos = unmatched.iter().position(|f| f.line == m.line && f.col == m.col && f.code == m.code && f.severity == m.severity);
        let Some(i) = pos else {
            let near: Vec<_> = unmatched.iter().filter(|f| f.code == m.code).map(|f| format!("{}:{}", f.line, f.col)).collect();
            v.push(format!("marker not matched: {}:{} {} {} (same code reported at: {:?})", m.line, m.col, m.severity, m.code, near));
            continue;
        };
        let f = unmatched.remove(i);
        if !f.message.contains(&m.message) {
            v.push(format!("{}:{} {}: message {:?} lacks {:?}", m.line, m.col, m.code, f.message, m.message));
        }
        if let (Some(ml), Some(fl)) = (m.len, f.len) {
            if ml != fl {
                v.push(format!("{}:{} {}: length {fl} != marker {ml}", m.line, m.col, m.code));
            }
        }
    }
    for f in unmatched {
        v.push(format!("unannotated diagnostic: {}:{} {} {} [{}] {}", f.line, f.col, f.severity, f.code, f.producer, f.message.lines().next().unwrap_or("")));
    }
    // Real position (independent of markers): every diagnostic with a location must exist.
    for f in found {
        if f.line == 0 || f.col == 0 {
            v.push(format!("diagnostic without a location: {} [{}] {}", f.code, f.producer, f.message.lines().next().unwrap_or("")));
        } else if offset_of(src, f.line, f.col).is_none() {
            v.push(format!("diagnostic location {}:{} is outside the source: {} [{}]", f.line, f.col, f.code, f.producer));
        }
    }
    v
}
