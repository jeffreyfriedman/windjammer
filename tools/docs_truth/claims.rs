//! Claims registry: numbers in prose must come from machine-written files via
//! `<!-- wj:gen name=metric key=... -->`, never be typed by hand.
//!
//! The lint flags `<number> <noun>` where a metric exists for the noun (tests, programs, keywords,
//! intrinsics, subcommands, error codes, lint rules) and `N / M` / `N of M` ratios on lines about
//! conformance, programs, tests or probes. Existing hand-typed numbers are ratcheted per file.

use super::config::Config;
use super::{md, util, Ctx, Finding};

const NOUNS: &[(&str, &str)] = &[
    ("tests", "tests.rust_files (or a CI-measured key in docs/generated/measured.json)"),
    ("programs", "conformance.programs"),
    ("keywords", "lexer.keywords"),
    ("intrinsics", "intrinsics.count"),
    ("subcommands", "cli.subcommands"),
    ("adrs", "adr.count"),
];
const PHRASES: &[(&str, &str)] = &[("error codes", "errors.codes_total"), ("lint rules", "lints.w_codes"), ("lint codes", "lints.w_codes")];
const RATIO_CONTEXT: &[&str] = &["conformance", "programs", "tests", "probes", "oracle", "compile"];

fn is_number(w: &str) -> bool {
    let w = w.trim_start_matches('~').trim_end_matches('+');
    !w.is_empty() && w.chars().next().unwrap().is_ascii_digit() && w.chars().all(|c| c.is_ascii_digit() || c == ',') && w.chars().filter(|c| c.is_ascii_digit()).count() >= 2
}

fn word(w: &str) -> String {
    w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-').to_lowercase()
}

/// Hand-typed statistics found in one prose line: (matched text, suggested metric).
pub fn stats_in_line(line: &str) -> Vec<(String, String)> {
    let plain = md::strip_code_spans(line);
    let ws: Vec<&str> = plain.split_whitespace().collect();
    let mut out = Vec::new();
    for (i, w) in ws.iter().enumerate() {
        let bare = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != ',' && c != '+' && c != '~');
        // "98-program corpus"
        if let Some((n, rest)) = bare.split_once('-') {
            if is_number(n) {
                let r = word(rest);
                if let Some((_, k)) = NOUNS.iter().find(|(noun, _)| r == noun.trim_end_matches('s')) {
                    out.push((bare.to_string(), k.to_string()));
                }
            }
            continue;
        }
        if !is_number(bare) {
            continue;
        }
        let next: Vec<String> = ws.iter().skip(i + 1).take(3).map(|x| word(x)).collect();
        for (j, nx) in next.iter().enumerate() {
            if let Some((n, k)) = NOUNS.iter().find(|(n, _)| nx == n) {
                if *n == "tests" || j < 2 {
                    out.push((format!("{bare} ... {nx}"), k.to_string()));
                }
                break;
            }
            let pair = next.get(j..j + 2).map(|p| p.join(" ")).unwrap_or_default();
            if let Some((_, k)) = PHRASES.iter().find(|(p, _)| *p == pair) {
                out.push((format!("{bare} {pair}"), k.to_string()));
                break;
            }
            if nx == "files" && j > 0 && next[j - 1].starts_with("test") {
                out.push((format!("{bare} test files"), "tests.rust_files".to_string()));
                break;
            }
        }
    }
    // ratios: "69 / 90", "27 of 97"
    let low = plain.to_lowercase();
    if RATIO_CONTEXT.iter().any(|c| low.contains(c)) {
        for (i, w) in ws.iter().enumerate() {
            let sep_ok = |s: &str| s == "/" || s == "of";
            if i + 2 < ws.len() && is_ratio_num(w) && sep_ok(ws[i + 1]) && is_ratio_num(ws[i + 2]) {
                out.push((format!("{w} {} {}", ws[i + 1], ws[i + 2]), "a CI-measured key in docs/generated/measured.json".to_string()));
            }
        }
    }
    out
}

fn is_ratio_num(w: &str) -> bool {
    let t = w.trim_matches(|c: char| !c.is_ascii_digit());
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit())
}

/// Prose lines of `text` outside fences and outside `wj:gen` regions.
fn prose_outside_regions(text: &str) -> Vec<(usize, &str)> {
    let mut in_region = false;
    md::prose_lines(text)
        .into_iter()
        .filter(|(_, l)| {
            let t = l.trim();
            if t.starts_with(super::regions::BEGIN) {
                in_region = true;
                return false;
            }
            if t == super::regions::END {
                in_region = false;
                return false;
            }
            !in_region && !t.starts_with('#')
        })
        .collect()
}

pub fn check_doc(file: &str, text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for (ln, line) in prose_outside_regions(text) {
        for (m, key) in stats_in_line(line) {
            out.push(Finding::fail("claims", file, ln, format!("hand-typed statistic `{m}`; use <!-- wj:gen name=metric key={key} --> (see docs/CONTRIBUTING-docs.md)")));
        }
    }
    out
}

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let mut raw = Vec::new();
    for f in util::walk(&ctx.root).into_iter().filter(|f| f.ends_with(".md")) {
        if !cfg.excluded("claims", &f) {
            raw.extend(check_doc(&f, &ctx.read(&f)));
        }
    }
    cfg.ratchet("claims", raw)
}

pub fn raw_counts(ctx: &Ctx, cfg: &Config) -> std::collections::BTreeMap<String, usize> {
    let mut m = std::collections::BTreeMap::new();
    for f in util::walk(&ctx.root).into_iter().filter(|f| f.ends_with(".md") && !cfg.excluded("claims", f)) {
        let n = check_doc(&f, &ctx.read(&f)).len();
        if n > 0 {
            m.insert(f, n);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_hand_typed_stats() {
        assert_eq!(stats_in_line("We have 244 tests today").len(), 1);
        assert_eq!(stats_in_line("the 98-program corpus").len(), 1);
        assert_eq!(stats_in_line("42 error codes exist").len(), 1);
        assert_eq!(stats_in_line("conformance: 69 / 90 equal").len(), 1);
        assert!(stats_in_line("`244 tests` and version 0.50.0 in 2026").is_empty());
        assert!(stats_in_line("Run 3 tests").is_empty());
    }
}
