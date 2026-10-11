//! Link and reference integrity: links, file paths, `src/...` references, ADR numbers, error and lint
//! codes, `wj` subcommands/flags and pluggable rule ids (`SPEC-...`) mentioned in docs must exist.

use super::config::Config;
use super::{gen_cli, gen_errors, md, util, Ctx, Finding};
use clap::Command;
use std::collections::BTreeSet;
use std::path::Path;

const PATH_ROOTS: &[&str] = &["src/", "tests/", "docs/", "scripts/", "std/", "crates/", "examples/", "tools/", "build_support/", ".github/", "stdlib_meta/", "agent_index/"];

/// Ids like `PREFIX` + `n` digits found in `text` (not glued to a preceding alphanumeric).
pub fn scan_ids(text: &str, prefix: &str, digits: usize) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = text[from..].find(prefix) {
        let i = from + p;
        from = i + prefix.len();
        if i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') {
            continue;
        }
        let d: Vec<u8> = b[from..].iter().take_while(|c| c.is_ascii_digit()).copied().collect();
        if d.len() == digits {
            out.push(format!("{prefix}{}", String::from_utf8_lossy(&d)));
        }
    }
    out
}

fn clean_path(span: &str) -> Option<String> {
    let s = span.trim().trim_end_matches(['.', ',', ';', ')']);
    if !PATH_ROOTS.iter().any(|r| s.starts_with(r)) || s.contains(char::is_whitespace) || s.contains(['*', '<', '>', '{', '}', '$', '(', '[', '|', '"', '\'', '=']) || s.contains("...") || s.contains("//") {
        return None;
    }
    let s = s.split("::").next().unwrap_or(s);
    let s = s.split('#').next().unwrap_or(s);
    // strip :LINE or :LINE-LINE
    let s = match s.rsplit_once(':') {
        Some((p, tail)) if tail.chars().all(|c| c.is_ascii_digit() || c == '-') && !tail.is_empty() => p,
        _ => s,
    };
    // must look like a path: has an extension or ends with '/'
    let last = s.rsplit('/').next().unwrap_or("");
    (s.ends_with('/') || last.contains('.')).then(|| s.to_string())
}

/// Sections under a heading containing one of these words describe what does not exist yet (or what is missing),
/// so the commands they name are not checked against the CLI.
const FUTURE_HEADINGS: &[&str] = &["planned", "not implemented", "future", "roadmap", "gaps", "unsupported", "not started", "open question"];

/// A line that says a command is missing may name it: status documents list what does not exist yet.
const NEGATIONS: &[&str] = &["does not exist", "do not exist", "no longer", "removed", "not a ", "no such", "not started", "unstarted", "not implemented", "does not ship", "never existed"];

fn check_wj_command(cli: &Command, text: &str) -> Vec<String> {
    let mut errs = Vec::new();
    let low = text.to_lowercase();
    if NEGATIONS.iter().any(|n| low.contains(n)) {
        return errs;
    }
    let toks: Vec<&str> = text.split_whitespace().collect();
    let Some(start) = toks.iter().position(|t| *t == "wj") else { return errs };
    let mut chain: Vec<&Command> = vec![cli];
    let mut positional = false;
    for t in &toks[start + 1..] {
        let t = t.trim_end_matches(['`', ',', ';', '.']);
        if matches!(t, "|" | ">" | ">>" | "&&" | "||" | ";" | "2>&1") || t.starts_with('#') || t.starts_with('<') || t.starts_with('[') || t.starts_with('$') || t.starts_with('"') {
            break;
        }
        if let Some(flag) = t.strip_prefix("--") {
            let flag = flag.split('=').next().unwrap_or(flag);
            if flag.is_empty() || flag == "help" || flag == "version" {
                continue;
            }
            if !chain.iter().any(|c| gen_cli::flags_of(c).iter().any(|f| f == flag)) {
                errs.push(format!("`wj {}` has no flag `--{flag}`", chain.iter().skip(1).map(|c| c.get_name()).collect::<Vec<_>>().join(" ")).replace("`wj `", "`wj`"));
            }
        } else if t.starts_with('-') {
            continue;
        } else if !positional {
            let cur = chain.last().unwrap();
            let sub = cur.get_subcommands().find(|s| s.get_name() == t || s.get_all_aliases().any(|a| a == t));
            match sub {
                Some(s) => chain.push(s),
                None if cur.has_subcommands() && chain.len() == 1 => {
                    if t.chars().all(|c| c.is_ascii_lowercase() || c == '-') && t != "help" {
                        errs.push(format!("`wj {t}` is not a subcommand"));
                    }
                    break;
                }
                None => positional = true,
            }
        }
    }
    errs
}

fn rule_ids(ctx: &Ctx, cfg: &Config, files: &[String]) -> Vec<(String, BTreeSet<String>)> {
    cfg.rule_ids
        .iter()
        .map(|r| {
            let mut defs = BTreeSet::new();
            for f in files.iter().filter(|f| util::glob_match(&r.defs, f)) {
                for w in ctx.read(f).split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')) {
                    if w.starts_with(&r.prefix) {
                        defs.insert(w.to_string());
                    }
                }
            }
            (r.prefix.clone(), defs)
        })
        .collect()
}

pub fn check_doc(ctx: &Ctx, cfg: &Config, file: &str, text: &str, known: &BTreeSet<String>, adrs: &BTreeSet<String>, rules: &[(String, BTreeSet<String>)], all: &BTreeSet<String>) -> Vec<Finding> {
    let mut out = Vec::new();
    // `status: proposal` documents name commands, flags and codes that do not exist yet.
    let future = super::proposal::is_proposal(text);
    let dir = Path::new(file).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let mut fail = |line: usize, msg: String| out.push(Finding::fail("links", file, line, msg));
    let mut future_section = false;
    for (ln, line) in md::prose_lines(text) {
        if let Some(h) = line.strip_prefix('#') {
            let h = h.trim_start_matches('#').trim().to_lowercase();
            future_section = FUTURE_HEADINGS.iter().any(|k| h.contains(k));
        }
        for t in md::link_targets(line) {
            if t.starts_with("http") || t.starts_with("mailto:") || t.contains("://") {
                continue;
            }
            let (path, frag) = t.split_once('#').map(|(p, f)| (p, Some(f))).unwrap_or((t.as_str(), None));
            let rel = if path.is_empty() {
                file.to_string()
            } else if let Some(r) = path.strip_prefix('/') {
                r.to_string()
            } else {
                normalize(&format!("{dir}/{path}"))
            };
            if !util::exists(&ctx.root, &rel) {
                fail(ln, format!("dead link: `{t}` -> {rel} does not exist"));
            } else if let Some(fr) = frag {
                if rel.ends_with(".md") && !fr.is_empty() && !fr.starts_with('L') && !md::anchors(&ctx.read(&rel)).iter().any(|a| a == fr) {
                    fail(ln, format!("dead anchor: `{t}` ({rel} has no heading #{fr})"));
                }
            }
        }
        let spans = md::code_spans(line);
        for s in &spans {
            if let Some(p) = clean_path(s) {
                // A path may be given from the repository root or relative to the document's directory.
                let local = normalize(&format!("{dir}/{p}"));
                let found = |q: &str| util::exists(&ctx.root, q) || all.iter().any(|f| f.starts_with(q));
                if !found(&p) && !found(&local) {
                    fail(ln, format!("referenced path `{p}` does not exist"));
                }
            }
            if s.starts_with("wj ") && !future && !future_section && !NEGATIONS.iter().any(|n| line.to_lowercase().contains(n)) {
                for e in check_wj_command(&ctx.cli, s) {
                    fail(ln, e);
                }
            }
        }
        let scan = format!("{} {}", md::strip_code_spans(line), spans.join(" "));
        for n in scan_ids(&scan, "ADR-", 4) {
            if !adrs.contains(&n[4..]) {
                fail(ln, format!("{n} does not exist in docs/adr/"));
            }
        }
        for c in scan_ids(&scan, "WJ", 4).into_iter().chain(scan_ids(&scan, "W", 4)) {
            if !future && !known.contains(&c) {
                fail(ln, format!("error/lint code {c} is not registered in the compiler"));
            }
        }
        for (prefix, defs) in rules {
            if defs.is_empty() || file_defines(cfg, prefix, file) {
                continue;
            }
            for w in scan.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')) {
                if w.starts_with(prefix.as_str()) && w.len() > prefix.len() && !defs.contains(w) {
                    fail(ln, format!("rule id {w} is not defined (namespace `{prefix}`)"));
                }
            }
        }
    }
    for f in md::fences(text) {
        let sh = matches!(f.info.split([',', ' ']).next().unwrap_or(""), "sh" | "bash" | "shell" | "console");
        if !sh || future {
            continue;
        }
        for (i, l) in f.body.lines().enumerate() {
            let l = l.trim_start().trim_start_matches("$ ");
            if l.starts_with("wj ") {
                for e in check_wj_command(&ctx.cli, l) {
                    out.push(Finding::fail("links", file, f.line + 1 + i, e));
                }
            }
        }
    }
    out
}

fn file_defines(cfg: &Config, prefix: &str, file: &str) -> bool {
    cfg.rule_ids.iter().any(|r| r.prefix == prefix && util::glob_match(&r.defs, file))
}

pub fn normalize(p: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in p.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let all_files = util::walk(&ctx.root);
    let all: BTreeSet<String> = all_files.iter().cloned().collect();
    let known: BTreeSet<String> = gen_errors::known_codes().into_iter().collect();
    let adrs: BTreeSet<String> = all_files.iter().filter_map(|f| f.strip_prefix("docs/adr/").filter(|r| r.len() > 4 && r[..4].chars().all(|c| c.is_ascii_digit())).map(|r| r[..4].to_string())).collect();
    let rules = rule_ids(ctx, cfg, &all_files);
    let mut raw = Vec::new();
    for f in all_files.iter().filter(|f| f.ends_with(".md")) {
        if cfg.excluded("links", f) {
            continue;
        }
        raw.extend(check_doc(ctx, cfg, f, &ctx.read(f), &known, &adrs, &rules, &all));
    }
    cfg.ratchet("links", raw)
}

pub fn raw_counts(ctx: &Ctx, cfg: &Config) -> std::collections::BTreeMap<String, usize> {
    let all_files = util::walk(&ctx.root);
    let all: BTreeSet<String> = all_files.iter().cloned().collect();
    let known: BTreeSet<String> = gen_errors::known_codes().into_iter().collect();
    let adrs: BTreeSet<String> = all_files.iter().filter_map(|f| f.strip_prefix("docs/adr/").filter(|r| r.len() > 4 && r[..4].chars().all(|c| c.is_ascii_digit())).map(|r| r[..4].to_string())).collect();
    let rules = rule_ids(ctx, cfg, &all_files);
    let mut m = std::collections::BTreeMap::new();
    for f in all_files.iter().filter(|f| f.ends_with(".md") && !cfg.excluded("links", f)) {
        let n = check_doc(ctx, cfg, f, &ctx.read(f), &known, &adrs, &rules, &all).len();
        if n > 0 {
            m.insert(f.clone(), n);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_paths() {
        assert_eq!(scan_ids("see WJ0007, W0002 and XW0001 WJ00071", "WJ", 4), vec!["WJ0007"]);
        assert_eq!(scan_ids("see WJ0007, W0002 and XW0001 W00021", "W", 4), vec!["W0002"]);
        assert_eq!(clean_path("src/lexer/mod.rs:12-30").as_deref(), Some("src/lexer/mod.rs"));
        assert_eq!(clean_path("src/foo/").as_deref(), Some("src/foo/"));
        assert!(clean_path("src/*.rs").is_none());
        assert!(clean_path("src/foo").is_none());
        assert_eq!(normalize("docs/adr/../x/./y.md"), "docs/x/y.md");
    }
}
