//! Gate 3: diagnostics must be where the markers say, and every diagnostic must have a marker.

use super::span_oracle::corpus::*;
use super::span_oracle::diag_oracle::*;
use super::span_oracle::ratchet;
use std::collections::BTreeSet;

fn oracle_dir() -> std::path::PathBuf {
    repo_root().join("tests/diagnostics_oracle")
}

fn marker_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|x| x == "wj"));
    files.sort();
    files
}

fn check_files_against_markers(files: &[std::path::PathBuf], producer: fn(&str, &str) -> Vec<Found>) {
    let mut bad = Vec::new();
    for p in files {
        let src = std::fs::read_to_string(p).unwrap();
        let markers = parse_markers(&src).unwrap_or_else(|e| panic!("{}: {e}", rel(p)));
        assert!(!markers.is_empty(), "{} has no markers", rel(p));
        let found = producer(&rel(p), &src);
        let v = compare(&src, &markers, &found);
        if !v.is_empty() {
            bad.push(format!("{}:\n  {}", rel(p), v.join("\n  ")));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Parser / linter stages (`wj lint`, the parser's own diagnostics).
#[test]
fn annotated_negative_programs_match_their_markers() {
    let files = marker_files(&oracle_dir());
    assert!(files.len() >= 8, "annotated corpus too small: {}", files.len());
    check_files_against_markers(&files, collect);
}

/// `wj check`: every diagnostic of the real checker, with exact start AND length.
#[test]
fn annotated_check_programs_match_their_markers() {
    let files = marker_files(&oracle_dir().join("check"));
    assert!(files.len() >= 30, "annotated check corpus too small: {}", files.len());
    check_files_against_markers(&files, collect_check);
}

/// Every repo `.wj` is run through `wj check`. Files with markers must match them. Files without
/// markers may emit no diagnostic, unless listed in the ratchet (which can only shrink). Whatever
/// the checker reports, its spans must be well-formed for the source (`validate_report_spans`).
#[test]
fn repo_wide_check_diagnostics_are_annotated_or_ratcheted() {
    let mut unannotated = BTreeSet::new();
    let mut broken = Vec::new();
    let mut invalid_spans = BTreeSet::new();
    for p in repo_wj_files() {
        let Ok(src) = std::fs::read_to_string(&p) else { continue };
        let name = rel(&p);
        if name.starts_with("tests/diagnostics_oracle/") && !name.starts_with("tests/diagnostics_oracle/check/") {
            continue; // legacy-producer files are covered by their own test
        }
        let diags = check_diagnostics(&name, &src);
        let report = serde_json::to_value(windjammer::diagnostics::Report::new("check", diags.clone())).unwrap();
        for v in windjammer::error::span_check::validate_report_spans(&report, &|_| Some(src.clone())) {
            invalid_spans.insert(format!("{name}: {v}"));
        }
        let found = collect_check(&name, &src);
        if src.lines().any(is_marker_line) {
            let v = compare(&src, &parse_markers(&src).unwrap_or_default(), &found);
            if !v.is_empty() {
                broken.push(format!("{name}: {}", v.join("; ")));
            }
        } else if !found.is_empty() {
            unannotated.insert(name);
        }
    }
    assert!(broken.is_empty(), "annotated files out of sync:\n{}", broken.join("\n"));
    ratchet::enforce("check_invalid_spans", &invalid_spans);
    ratchet::enforce("unannotated_check_diagnostics", &unannotated);
}

/// Legacy producers (parser warnings + rust-leakage linter) on every repo file.
#[test]
fn repo_wide_diagnostics_are_annotated_or_ratcheted() {
    let mut unannotated = BTreeSet::new();
    let mut broken = Vec::new();
    for p in repo_wj_files() {
        let Ok(src) = std::fs::read_to_string(&p) else { continue };
        let name = rel(&p);
        if name.starts_with("tests/diagnostics_oracle/check/") {
            continue;
        }
        let has_markers = src.lines().any(is_marker_line);
        let found = collect(&name, &src);
        if has_markers {
            let markers = parse_markers(&src).unwrap_or_default();
            let v = compare(&src, &markers, &found);
            if !v.is_empty() {
                broken.push(format!("{name}: {}", v.join("; ")));
            }
        } else if !found.is_empty() {
            unannotated.insert(name);
        }
    }
    assert!(broken.is_empty(), "annotated files out of sync:\n{}", broken.join("\n"));
    ratchet::enforce("unannotated_diagnostics", &unannotated);
}

// ---- marker grammar self-tests ----------------------------------------------------------------

#[test]
fn underline_marker_columns_are_character_columns() {
    let src = "let é = bad(1)\n//~     ^^^ WARN W9999 hello world\n";
    // `^` is the 9th char of the marker line: `//~` is 3 chars, then 5 spaces => column 9.
    let m = parse_markers(src).unwrap();
    assert_eq!(m, vec![Marker { line: 1, col: 9, len: Some(3), severity: "WARN".into(), code: "W9999".into(), message: "hello world".into() }]);
}

#[test]
fn explicit_marker_and_crlf_and_stacking() {
    let src = "a\r\n//~@1+2 ERROR parse\r\n//~@3 NOTE X1\r\nb\r\n//~@2+1 WARN W1\r\n";
    let m = parse_markers(src).unwrap();
    assert_eq!((m[0].line, m[0].col, m[0].len), (1, 1, Some(2)));
    assert_eq!((m[1].line, m[1].col, m[1].len), (1, 3, None));
    assert_eq!((m[2].line, m[2].col), (4, 2));
}

#[test]
fn malformed_markers_are_rejected() {
    assert!(parse_markers("x\n//~ nothing\n").is_err());
    assert!(parse_markers("x\n//~@1 FATAL W1\n").is_err());
    assert!(parse_markers("//~@1 ERROR W1\n").is_err());
    assert!(parse_markers("x\n//~@1 ERROR\n").is_err());
}

// ---- mutation self-tests: compare() must catch every kind of drift ----------------------------

fn found(line: usize, col: usize, code: &str) -> Found {
    Found { producer: "t", line, col, len: Some(3), severity: "WARN".into(), code: code.into(), message: "msg here".into() }
}
fn marker(line: usize, col: usize, code: &str) -> Marker {
    Marker { line, col, len: Some(3), severity: "WARN".into(), code: code.into(), message: "msg".into() }
}
const SRC: &str = "abcdef\nghijkl\n";

#[test]
fn compare_accepts_exact_match() {
    assert!(compare(SRC, &[marker(1, 2, "W1")], &[found(1, 2, "W1")]).is_empty());
}
#[test]
fn compare_rejects_column_drift() {
    assert!(!compare(SRC, &[marker(1, 2, "W1")], &[found(1, 1, "W1")]).is_empty());
}
#[test]
fn compare_rejects_line_drift() {
    assert!(!compare(SRC, &[marker(1, 2, "W1")], &[found(2, 2, "W1")]).is_empty());
}
#[test]
fn compare_rejects_length_drift() {
    let mut f = found(1, 2, "W1");
    f.len = Some(4);
    assert!(!compare(SRC, &[marker(1, 2, "W1")], &[f]).is_empty());
}
#[test]
fn compare_rejects_unannotated_diagnostic() {
    assert!(!compare(SRC, &[], &[found(1, 2, "W1")]).is_empty());
}
#[test]
fn compare_rejects_unmatched_marker() {
    assert!(!compare(SRC, &[marker(1, 2, "W1")], &[]).is_empty());
}
#[test]
fn compare_rejects_wrong_code_and_message() {
    assert!(!compare(SRC, &[marker(1, 2, "W2")], &[found(1, 2, "W1")]).is_empty());
    let mut m = marker(1, 2, "W1");
    m.message = "other".into();
    assert!(!compare(SRC, &[m], &[found(1, 2, "W1")]).is_empty());
}
#[test]
fn compare_rejects_missing_or_outside_location() {
    assert!(!compare(SRC, &[marker(0, 0, "W1")], &[found(0, 0, "W1")]).is_empty());
    assert!(!compare(SRC, &[marker(9, 9, "W1")], &[found(9, 9, "W1")]).is_empty());
}

// ---- assertion layer ----------------------------------------------------------------------------

#[test]
fn invalid_location_panics_in_strict_mode() {
    windjammer::error::span_check::set_strict(true);
    let r = std::panic::catch_unwind(|| windjammer::error::SourceLocation::new("f.wj", 0, 1));
    assert!(r.is_err(), "line 0 must be rejected");
    let r = std::panic::catch_unwind(|| windjammer::error::SourceLocation::new("f.wj", 1, 0));
    assert!(r.is_err(), "column 0 must be rejected");
    assert!(std::panic::catch_unwind(|| windjammer::error::SourceLocation::new("f.wj", 1, 1)).is_ok());
}

/// Developer aid: `cargo test --release --test span_oracle_suite dump_check -- --ignored --nocapture`
/// prints what `wj check` reports for every case in tests/diagnostics/cases, with an underline,
/// to draft markers (review each underline by eye once, then freeze it as a marker).
#[test]
#[ignore]
fn dump_check_cases() {
    let dir = repo_root().join("tests/diagnostics/cases");
    for p in marker_files(&dir) {
        let src = std::fs::read_to_string(&p).unwrap();
        println!("=== {}", rel(&p));
        for f in collect_check(&rel(&p), &src) {
            println!("  {} {} {}:{} len={:?} {}", f.severity, f.code, f.line, f.col, f.len, f.message.lines().next().unwrap_or(""));
            if f.line > 0 {
                if let Some(l) = src.lines().nth(f.line - 1) {
                    println!("  | {l}");
                    println!("  | {}{}", " ".repeat(f.col - 1), "^".repeat(f.len.unwrap_or(1).max(1)));
                }
            }
        }
    }
}
