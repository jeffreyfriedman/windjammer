//! Gates 5-6: parse errors are always located; LSP/UTF-16 conversions; formatter edits;
//! `--json` span validation; multi-line constructs.

use crate::span_oracle::corpus::*;
use crate::span_oracle::diag_oracle::location_in_message;
use crate::span_oracle::lexer_oracle::lex_all;
use crate::span_oracle::naive::{offset_of, utf16_len};
use crate::span_oracle::ratchet;
use std::collections::BTreeSet;
use windjammer::error::span_check::*;
use windjammer::lexer::{Lexer, Token};
use windjammer::parser_impl::Parser;

// ---- every parse error is located inside the source (fuzzed broken programs) -------------------

fn parse(src: &str) -> Result<(), String> {
    let s = src.to_string();
    std::panic::catch_unwind(move || {
        let tokens = Lexer::new(&s).tokenize_with_locations();
        Parser::new_with_source(tokens, "t.wj".into(), s.clone()).parse().map(|_| ())
    })
    .unwrap_or_else(|_| Err("PANIC".into()))
}

/// Break `src` in one of several ways, deterministically.
fn mutate(src: &str, rng: &mut Rng) -> String {
    let chars: Vec<char> = src.chars().collect();
    if chars.is_empty() {
        return "$".into();
    }
    let i = rng.below(chars.len());
    let mut out: String = chars[..i].iter().collect();
    match rng.below(5) {
        0 => out.extend(&chars[i + 1..]), // delete one char
        1 => {
            out.push_str(rng.pick(&[")", "}", "]", "(", "{", "$", "#", "'", "\"", "|>", "..", "=>"]));
            out.extend(&chars[i..]);
        }
        2 => {} // truncate here
        3 if i + 1 < chars.len() => {
            out.push(chars[i + 1]);
            out.push(chars[i]);
            out.extend(&chars[i + 2..]);
        }
        _ => {
            out.push(chars[i]);
            out.extend(&chars[i..]);
        }
    }
    if out.is_empty() {
        out.push('x');
    }
    out
}

#[test]
fn every_parse_error_is_located_inside_the_source() {
    let mut files: Vec<_> = repo_wj_files()
        .into_iter()
        .filter_map(|p| std::fs::read_to_string(&p).ok().map(|s| (rel(&p), s)))
        .filter(|(_, s)| s.len() < 3000 && parse(s).is_ok())
        .collect();
    files.sort();
    files.truncate(150);
    assert!(files.len() >= 50, "corpus too small: {}", files.len());
    let mut rng = Rng(0xC0FF_EE00_1234_5678);
    let (mut errors, mut bad) = (0usize, Vec::new());
    let mut panics = BTreeSet::new();
    for (name, src) in &files {
        for _ in 0..6 {
            let m = mutate(src, &mut rng);
            match parse(&m) {
                Ok(()) => {}
                Err(e) if e == "PANIC" => {
                    panics.insert(name.clone());
                }
                Err(e) => {
                    errors += 1;
                    match location_in_message(&e) {
                        None => bad.push(format!("{name}: parse error has no line:col: {e}")),
                        Some((l, c)) if offset_of(&m, l, c).is_none() => {
                            bad.push(format!("{name}: parse error at {l}:{c} is outside the source: {e}"))
                        }
                        Some(_) => {}
                    }
                }
            }
        }
    }
    assert!(errors > 100, "fuzzer produced too few parse errors ({errors}); the gate is vacuous");
    assert!(bad.is_empty(), "{} unlocated errors, first ones:\n{}", bad.len(), bad.iter().take(8).cloned().collect::<Vec<_>>().join("\n"));
    ratchet::enforce("parser_panics_on_broken_input", &panics);
}

// ---- LSP / UTF-16 / byte offsets ----------------------------------------------------------------

#[test]
fn lsp_positions_round_trip_over_the_whole_corpus() {
    let mut checked = 0usize;
    for p in repo_wj_files().iter().take(300) {
        let Ok(src) = std::fs::read_to_string(p) else { continue };
        for (i, _) in src.char_indices().chain(std::iter::once((src.len(), ' '))) {
            let (line, ch) = byte_to_lsp_position(&src, i);
            // independent oracle for the UTF-16 character: count the units of the line prefix
            let line_start = src[..i].rfind('\n').map_or(0, |n| n + 1);
            assert_eq!(ch as usize, utf16_len(&src[line_start..i]), "{} byte {i}", rel(p));
            assert_eq!(line as usize, src[..i].matches('\n').count());
            assert_eq!(lsp_position_to_byte(&src, line, ch), Some(i), "{} byte {i}", rel(p));
            checked += 1;
        }
    }
    assert!(checked > 100_000, "checked only {checked} positions");
}

#[test]
fn lexer_columns_convert_to_utf16_consistently() {
    // A token after astral chars: chars-column and UTF-16 column differ by the surrogate pairs.
    let src = "let s = \"😀😀\" + x\n";
    let toks = lex_all(src).unwrap();
    let x = toks.iter().find(|t| t.token == Token::Ident("x".into())).unwrap();
    let line_text = src.lines().next().unwrap();
    let u = char_col_to_utf16(line_text, x.column);
    assert_eq!(u, utf16_len(&line_text[..x.byte_start]));
    assert_eq!(u, x.column - 1 + 2, "two astral chars add two extra UTF-16 units");
    assert_eq!(utf16_to_char_col(line_text, u), Some(x.column));
}

// ---- `wj check --json` span validation ------------------------------------------------------------

fn report(span: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"diagnostics": [{"code": "W0002", "primary_span": span}]})
}
fn sp(bs: usize, be: usize, ls: usize, cs: usize, le: usize, ce: usize) -> serde_json::Value {
    serde_json::json!({"file": "a.wj", "byte_start": bs, "byte_end": be, "line_start": ls, "col_start": cs, "line_end": le, "col_end": ce})
}
const SRC: &str = "let é = 1\nlet y = 2\n";
fn check(r: &serde_json::Value) -> Vec<String> {
    validate_report_spans(r, &|f| (f == "a.wj").then(|| SRC.to_string()))
}

#[test]
fn json_validator_accepts_a_correct_report() {
    // `y` is at byte 15 (é is two bytes): line 2, col 5
    assert_eq!(check(&report(sp(15, 16, 2, 5, 2, 6))), Vec::<String>::new());
    assert_eq!(check(&report(serde_json::Value::Null)), Vec::<String>::new());
}
#[test]
fn json_validator_rejects_each_class_of_bad_span() {
    for (name, r) in [
        ("byte column", report(sp(15, 16, 2, 6, 2, 7))),
        ("start after end", report(sp(16, 15, 2, 5, 2, 5))),
        ("past EOF", report(sp(15, 999, 2, 5, 2, 6))),
        ("mid-char", report(sp(1, 3, 1, 2, 1, 3))),
        ("zero line", report(sp(0, 1, 0, 1, 1, 2))),
        ("missing field", report(serde_json::json!({"file": "a.wj", "byte_start": 0}))),
        ("unknown file", report(serde_json::json!({"file": "zzz.wj", "byte_start": 0, "byte_end": 1, "line_start": 1, "col_start": 1, "line_end": 1, "col_end": 2}))),
    ] {
        assert!(!check(&r).is_empty(), "{name}: not rejected");
    }
}
#[test]
fn json_validator_checks_secondary_spans_and_fix_edits() {
    let mut r = report(sp(15, 16, 2, 5, 2, 6));
    r["diagnostics"][0]["secondary_spans"] = serde_json::json!([sp(0, 3, 1, 1, 1, 9)]);
    assert!(!check(&r).is_empty());
    let mut r = report(sp(15, 16, 2, 5, 2, 6));
    r["diagnostics"][0]["fixes"] = serde_json::json!([{"edits": [sp(15, 16, 2, 5, 2, 7)]}]);
    assert!(!check(&r).is_empty());
}

// ---- formatter edits -------------------------------------------------------------------------------

fn apply(text: &str, edits: &[windjammer::formatter::LineEdit]) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut out = String::new();
    let mut at = 0usize;
    for e in edits {
        assert!(e.start_line >= at, "edits overlap or are unordered");
        assert!(e.start_line <= e.end_line && e.end_line <= lines.len(), "edit outside the document");
        out.extend(lines[at..e.start_line].iter().copied());
        out.push_str(&e.new_text);
        at = e.end_line;
    }
    out.extend(lines[at..].iter().copied());
    out
}

fn tokens_of(src: &str) -> Vec<Token> {
    lex_all(src).map(|t| t.into_iter().map(|t| t.token).filter(|t| *t != Token::Newline).collect()).unwrap_or_default()
}

#[test]
fn formatter_edits_apply_to_the_formatted_text_and_preserve_tokens() {
    let opts = windjammer::formatter::FormatOptions::default();
    let mut checked = 0;
    let mut sources: Vec<(String, String)> = vec![
        ("inline-ugly".into(), "fn a(){\nlet x=\"日本😀\"\n  let y  =  x\n}\n".into()),
        ("inline-crlf".into(), "fn a(){\r\nlet x=1\r\n}\r\n".into()),
        ("inline-multiline-string".into(), "fn a(){\nlet s=\"one\ntwo\"\n}\n".into()),
    ];
    for p in repo_wj_files().iter().filter(|p| rel(p).starts_with("tests/fixtures/formatter") || rel(p).starts_with("examples/verified")).take(60) {
        if let Ok(s) = std::fs::read_to_string(p) {
            sources.push((rel(p), s));
        }
    }
    for (name, src) in sources {
        let Ok(formatted) = windjammer::formatter::format_source(&src, &opts) else { continue };
        let Ok(edits) = windjammer::formatter::document_edits(&src, &opts) else { continue };
        assert_eq!(apply(&src, &edits), formatted, "{name}: applying the line edits does not give the formatted text");
        assert_eq!(tokens_of(&src), tokens_of(&formatted), "{name}: formatting changed the token stream");
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} sources formatted");
}

// ---- other classes: multi-line constructs and CRLF ---------------------------------------------

#[test]
fn line_numbers_after_multiline_strings_comments_and_crlf_are_exact() {
    for eol in ["\n", "\r\n"] {
        let src = format!("let a = \"x{eol}y{eol}z\"{eol}// comment {eol}/// doc{eol}let b = r#\"p{eol}q\"#{eol}let c = \"${{a}}{eol}${{b}}\"{eol}after");
        let toks = lex_all(&src).unwrap();
        let after = toks.iter().find(|t| t.token == Token::Ident("after".into())).unwrap();
        let want_line = src.matches('\n').count() + 1;
        assert_eq!((after.line, after.column), (want_line, 1), "eol {eol:?}");
        assert_eq!(src.lines().count(), want_line);
    }
}

// ---- legacy AST locations must not spread into user-visible diagnostics ---------------------------

/// `Expression::location()` is the position of the token after a node (an inference identity key,
/// see src/parser/MODULE.md). Diagnostics must use `Parser::span_snapshot()` instead. This counts
/// the remaining reads in diagnostic-producing code; the ratchet may only shrink.
#[test]
fn legacy_ast_location_reads_in_diagnostic_code_are_ratcheted() {
    let root = repo_root().join("src");
    let mut counts = std::collections::BTreeMap::new();
    for dir in ["linter", "resolve", "cli", "ide_analysis", "error"] {
        let mut stack = vec![root.join(dir)];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let text = std::fs::read_to_string(&p).unwrap_or_default();
                    let n = text.matches(".location()").count() + text.matches("location.clone()").count();
                    if n > 0 {
                        counts.insert(rel(&p), n);
                    }
                }
            }
        }
    }
    ratchet::enforce_counts("legacy_ast_location_reads", &counts);
}
