// wj economics - the source dimension (D7) of docs/economics/DEFINITION.md for a file, optionally against a
// hand-written Rust twin. The counting rules are the ones `scripts/wj/economics.wj` applies to the suite
// (tests/economics_cli.rs holds the two to the same numbers on the recorded suite).

use anyhow::{bail, Context, Result};
use std::path::Path;

/// Source counts of one file (blank lines and full-line `//` comments excluded).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCounts {
    pub lines: usize,
    pub chars: usize,
    pub tokens: usize,
    pub ceremony: usize,
}

/// The source without blank lines and full-line comments.
fn code_of(text: &str) -> String {
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Lexical tokens: identifier/number runs, string literals (one each), single punctuation characters.
fn token_count(code: &str) -> usize {
    let cs: Vec<char> = code.chars().collect();
    let (mut n, mut i) = (0, 0);
    while i < cs.len() {
        let c = cs[i];
        if c == ' ' || c == '\n' || c == '\t' || c == '\r' {
            i += 1;
        } else if c == '"' {
            n += 1;
            i += 1;
            while i < cs.len() && cs[i] != '"' {
                if cs[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
        } else if is_word(c) {
            n += 1;
            while i < cs.len() && is_word(cs[i]) {
                i += 1;
            }
        } else {
            n += 1;
            i += 1;
        }
    }
    n
}

/// Ownership and conversion ceremony: borrows, lifetimes, explicit clones and conversions.
fn ceremony(code: &str) -> usize {
    let c = |s: &str| code.matches(s).count();
    let amp = c("&").saturating_sub(2 * c("&&"));
    amp + c("'a") + c("'_") + c(".clone()") + c(".to_string()") + c(".to_owned()") + c(".unwrap()") + c(".as_str()") + c(".iter()") + c(" as ")
}

pub fn source_counts(text: &str) -> SourceCounts {
    let code = code_of(text);
    SourceCounts {
        lines: if code.is_empty() { 0 } else { code.lines().count() },
        chars: code.chars().filter(|c| !matches!(c, ' ' | '\t' | '\n' | '\r')).count(),
        tokens: token_count(&code),
        ceremony: ceremony(&code),
    }
}

const DIMS: &[&str] = &["static"];

fn json_counts(c: &SourceCounts) -> serde_json::Value {
    serde_json::json!({"lines": c.lines, "chars": c.chars, "tokens": c.tokens, "ceremony": c.ceremony})
}

fn ratio(a: usize, b: usize) -> Option<f64> {
    (b > 0).then(|| a as f64 / b as f64)
}

/// `wj economics FILE [--dims static] [--twin FILE.rs] [--json]`
pub fn execute(file: &Path, dims: &[String], twin: Option<&Path>, json: bool) -> Result<()> {
    for d in dims {
        if !DIMS.contains(&d.as_str()) {
            bail!("unknown dimension `{d}`; available: {} (verify, compile, run, size and memory: scripts/wj/economics.wj)", DIMS.join(", "));
        }
    }
    let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let wj = source_counts(&text);
    let rs = match twin {
        Some(t) => Some(source_counts(&std::fs::read_to_string(t).with_context(|| format!("reading {}", t.display()))?)),
        None => None,
    };
    if json {
        let mut doc = serde_json::json!({"file": file.display().to_string(), "static": json_counts(&wj)});
        if let Some(r) = &rs {
            doc["twin"] = json_counts(r);
            doc["ratio"] = serde_json::json!({
                "lines": ratio(wj.lines, r.lines), "chars": ratio(wj.chars, r.chars),
                "tokens": ratio(wj.tokens, r.tokens), "ceremony": ratio(wj.ceremony, r.ceremony),
            });
        }
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }
    println!("{} (source dimension, docs/economics/DEFINITION.md D7)", file.display());
    let row = |name: &str, w: usize, r: Option<usize>| match r {
        Some(r) => println!(
            "  {name:<9} {w:>6}   twin {r:>6}   ratio {}",
            ratio(w, r).map(|x| format!("{x:.2}")).unwrap_or_else(|| "-".into())
        ),
        None => println!("  {name:<9} {w:>6}"),
    };
    row("lines", wj.lines, rs.map(|r| r.lines));
    row("chars", wj.chars, rs.map(|r| r.chars));
    row("tokens", wj.tokens, rs.map(|r| r.tokens));
    row("ceremony", wj.ceremony, rs.map(|r| r.ceremony));
    Ok(())
}
