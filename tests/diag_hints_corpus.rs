//! Corpus gate: every failure recorded in the agent-preference pilots (bench/agent-dx/results/pilot*)
//! must now come with a fix hint. A failure is one `error` block the agent saw. It is "covered" when
//!   * it is a rustc error that the translator maps to a Windjammer code WITH a help line, or
//!   * it is an interpreter message that `runtime_text::classify` gives a code and help, or
//!   * it already carries its own `help:` / `tip:` / `suggestion:` line.
//! Uncovered kinds live in `tests/diag_hints_ratchet/uncovered.txt` and may only shrink.
//! Also: the recorded `.wj` sources are run through `wj check`; every error must carry help or a fix.
//!
//! Run: `cargo test --release --test diag_hints_corpus`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use windjammer::diagnostics::{rustc_text, runtime_text};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn trials() -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let results = root().join("bench/agent-dx/results");
    let mut dirs: Vec<_> = std::fs::read_dir(&results).unwrap().flatten().map(|e| e.path()).filter(|p| p.join("trials").is_dir()).collect();
    dirs.sort();
    for d in dirs {
        let mut files: Vec<_> = std::fs::read_dir(d.join("trials")).unwrap().flatten().map(|e| e.path()).collect();
        files.sort();
        for f in files {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&f).unwrap()) {
                if v["lang"] == "wj" {
                    out.push(v);
                }
            }
        }
    }
    out
}

struct Block {
    rustc_code: Option<String>,
    msg: String,
    text: String,
}

fn header(line: &str) -> Option<(Option<String>, String)> {
    let rest = line.strip_prefix("error")?;
    let (code, rest) = match rest.strip_prefix('[') {
        Some(r) => {
            let end = r.find(']')?;
            (Some(r[..end].to_string()), &r[end + 1..])
        }
        None => (None, rest),
    };
    Some((code, rest.strip_prefix(": ")?.to_string()))
}

fn blocks(sample: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for line in sample.lines() {
        if let Some((code, msg)) = header(line) {
            out.push(Block { rustc_code: code, msg, text: String::new() });
        } else if let Some(b) = out.last_mut() {
            b.text.push_str(line);
            b.text.push('\n');
        }
    }
    // Samples were cut at a fixed length: a header that is a prefix of "could not compile" is noise.
    out.retain(|b| !b.msg.starts_with("could not compile") && !b.msg.starts_with("aborting") && !(b.msg.len() < 17 && "could not compile".starts_with(b.msg.as_str())));
    out
}

fn between<'a>(t: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let s = t.find(a)? + a.len();
    let r = &t[s..];
    Some(&r[..r.find(b)?])
}

/// Normalised kind of a block (names and numbers erased) for the ratchet file.
fn kind(b: &Block) -> String {
    let mut s = String::new();
    let mut in_tick = false;
    for c in b.msg.chars() {
        if c == '`' {
            in_tick = !in_tick;
            s.push(c);
        } else if !in_tick {
            s.push(c);
        }
    }
    format!("{}: {}", b.rustc_code.clone().unwrap_or_else(|| "-".into()), s.chars().take(90).collect::<String>())
}

/// `(covered, specific)`: has a code+help; and the help is not the generic fallback.
fn judge(b: &Block) -> (bool, bool) {
    if let Some(rc) = &b.rustc_code {
        if rc.starts_with('W') {
            return (true, true); // a lint: carries its own help
        }
        let label = b.text.lines().find(|l| l.contains("expected `") && l.contains("found `")).map(|l| l.trim().trim_start_matches(|c: char| "|^-/ ".contains(c)));
        let t = rustc_text::translate(Some(rc), &b.msg, label);
        let generic = t.help.as_deref().is_some_and(|h| h.starts_with("change the value or the declared type"));
        return (t.code != "WJ0099" && t.help.is_some(), !generic);
    }
    // A registered lint/error code in the header: its help text is part of the registry (explain gate).
    if let Some(code) = b.msg.rfind('[').and_then(|i| b.msg[i + 1..].strip_suffix(']')) {
        if windjammer::diagnostics::codes::lookup(code).is_some_and(|c| !c.fix.is_empty()) {
            return (true, true);
        }
    }
    if b.text.contains("help:") || b.text.contains("tip:") || b.text.contains("suggestion:") {
        return (true, true);
    }
    if let Some(c) = runtime_text::classify(&b.msg) {
        return (!c.help.is_empty(), true);
    }
    (false, false)
}

#[test]
fn recorded_failures_now_carry_fix_hints() {
    let (mut total, mut covered, mut specific) = (0, 0, 0);
    let mut uncovered: BTreeMap<String, usize> = BTreeMap::new();
    for t in trials() {
        for s in t["diag_samples"].as_array().cloned().unwrap_or_default() {
            for b in blocks(s.as_str().unwrap_or("")) {
                total += 1;
                let (c, sp) = judge(&b);
                covered += c as usize;
                specific += sp as usize;
                if !c {
                    *uncovered.entry(kind(&b)).or_default() += 1;
                }
            }
        }
    }
    assert!(total > 100, "corpus unexpectedly small: {total}");
    println!("corpus: {total} recorded error blocks, {covered} covered ({:.1}%), {specific} with a specific hint ({:.1}%)", 100.0 * covered as f64 / total as f64, 100.0 * specific as f64 / total as f64);
    // Ratchet on the share of failures with a SPECIFIC hint (not the generic fallback): may only rise.
    let floor: usize = std::fs::read_to_string(root().join("tests/diag_hints_ratchet/min_specific_percent.txt"))
        .ok()
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0);
    let pct = specific * 100 / total;
    assert!(pct >= floor, "specific-hint share fell to {pct}% (floor {floor}%)");
    let actual: BTreeSet<String> = uncovered.keys().cloned().collect();
    ratchet("uncovered", &actual);
}

fn ratchet(name: &str, actual: &BTreeSet<String>) {
    let p = root().join("tests/diag_hints_ratchet").join(format!("{name}.txt"));
    let allowed: BTreeSet<String> = std::fs::read_to_string(&p)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    if std::env::var("DIAG_HINTS_BLESS").is_ok_and(|v| v == "1") && actual.is_subset(&allowed) {
        let body: String = actual.iter().map(|l| format!("{l}\n")).collect();
        std::fs::write(&p, format!("# Known uncovered failure kinds. May only shrink: add a hint, then delete the line.\n{body}")).unwrap();
        return;
    }
    let new: Vec<_> = actual.difference(&allowed).collect();
    let stale: Vec<_> = allowed.difference(actual).collect();
    assert!(new.is_empty(), "NEW uncovered failure kinds (add a hint; do not add to the ratchet): {new:#?}");
    assert!(stale.is_empty(), "STALE ratchet entries (now covered; delete them, or run with DIAG_HINTS_BLESS=1): {stale:#?}");
}

/// Front end on the recorded sources: whatever `wj check` reports must be actionable.
#[test]
fn recorded_sources_get_actionable_front_end_diagnostics() {
    let mut seen = BTreeSet::new();
    let mut bad = BTreeSet::new();
    let mut with_errors = 0;
    let mut sources = 0;
    for t in trials() {
        for key in ["first_source", "final_source"] {
            let Some(src) = t[key].as_str() else { continue };
            if !seen.insert(src.to_string()) {
                continue;
            }
            sources += 1;
            let ds = windjammer::check::check_sources(Path::new("."), &[("main.wj".to_string(), src.to_string())]);
            let errs: Vec<_> = ds.iter().filter(|d| d.is_error()).collect();
            with_errors += (!errs.is_empty()) as usize;
            for d in errs {
                if d.help.is_empty() && d.fixes.is_empty() {
                    bad.insert(format!("{}: {}", d.code, d.message.chars().take(60).collect::<String>()));
                }
            }
        }
    }
    println!("sources: {sources} unique, {with_errors} with front-end errors, {} error kinds without a hint", bad.len());
    ratchet("front_end_no_hint", &bad);
}
