//! `docs/docs-truth.toml`: exclusions (each with a reason), ratchet baselines, pluggable rule-id
//! namespaces and freshness settings.

use super::{util, Ctx, Finding};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub exclude: Vec<Exclude>,
    /// check name -> (file -> tolerated failure count). May only shrink.
    #[serde(default)]
    pub baseline: BTreeMap<String, BTreeMap<String, usize>>,
    #[serde(default)]
    pub rule_ids: Vec<RuleIds>,
    #[serde(default)]
    pub freshness: Freshness,
    #[serde(default)]
    pub verify: Verify,
    #[serde(default)]
    pub ebnf: Ebnf,
}

/// Cross-check of `docs/windjammer.ebnf` against the parser (see `ebnf.rs`). Both tables may only shrink.
#[derive(Debug, Default, Deserialize)]
pub struct Ebnf {
    /// Lexer keywords deliberately absent from the grammar -> reason.
    #[serde(default)]
    pub unlisted_keywords: BTreeMap<String, String>,
    /// `production#alt` sentences the parser is known to reject -> reason.
    #[serde(default)]
    pub divergences: BTreeMap<String, String>,
}

/// Documents whose shell fences that run `wj` must all be `sh,verify` (or explicitly `ignore`d).
#[derive(Debug, Default, Deserialize)]
pub struct Verify {
    #[serde(default)]
    pub required: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Exclude {
    /// "fences" | "links" | "claims" | "freshness" | "all"
    pub check: String,
    pub glob: String,
    pub reason: String,
}

/// A namespace of citable rule ids, e.g. `SPEC-` defined in `docs/spec/**/*.md`.
/// Inactive (never fails) while no file matches `defs`.
#[derive(Debug, Deserialize)]
pub struct RuleIds {
    pub prefix: String,
    pub defs: String,
}

#[derive(Debug, Deserialize)]
pub struct Freshness {
    /// Docs under these globs should carry `status` / `last_verified` front matter.
    #[serde(default)]
    pub tracked: Vec<String>,
    /// Warn when referenced code changed by more than this many lines since `last_verified`.
    #[serde(default = "default_threshold")]
    pub changed_lines: usize,
    /// Missing front matter / `last_verified` on tracked docs fails instead of warning.
    #[serde(default)]
    pub require: bool,
}

fn default_threshold() -> usize {
    200
}

impl Default for Freshness {
    fn default() -> Self {
        Freshness { tracked: vec![], changed_lines: default_threshold(), require: false }
    }
}

pub const PATH: &str = "docs/docs-truth.toml";

pub fn load(ctx: &Ctx) -> Config {
    let text = ctx.read(PATH);
    let mut cfg: Config = toml::from_str(&text).unwrap_or_else(|e| panic!("{PATH}: {e}"));
    // Triage aid: `WJ_DOCS_NO_BASELINE=1 wj-docs check` lists every finding the ratchets currently hide.
    if std::env::var_os("WJ_DOCS_NO_BASELINE").is_some() {
        cfg.baseline.clear();
    }
    cfg
}

impl Config {
    pub fn excluded(&self, check: &str, file: &str) -> bool {
        self.exclude.iter().any(|e| (e.check == check || e.check == "all") && util::glob_match(&e.glob, file))
    }

    /// Apply the ratchet: files at or under their baseline are silent (under-baseline warns so
    /// the baseline gets tightened); files over it report every finding as a failure.
    pub fn ratchet(&self, check: &'static str, raw: Vec<Finding>) -> Vec<Finding> {
        let empty = BTreeMap::new();
        let base = self.baseline.get(check).unwrap_or(&empty);
        let mut by_file: BTreeMap<String, Vec<Finding>> = BTreeMap::new();
        for f in raw {
            by_file.entry(f.file.clone()).or_default().push(f);
        }
        let mut out = Vec::new();
        for (file, fs) in &by_file {
            let allowed = base.get(file).copied().unwrap_or(0);
            if fs.len() > allowed {
                for f in fs {
                    let mut f = f.clone();
                    f.msg = format!("{} (file has {} such findings, baseline allows {allowed})", f.msg, fs.len());
                    out.push(f);
                }
            } else if fs.len() < allowed {
                out.push(Finding::warn(check, file, 0, format!("baseline {allowed} can drop to {}: run scripts/docs_gen.sh --update-baseline", fs.len())));
            }
        }
        for (file, &n) in base {
            if !by_file.contains_key(file) && n > 0 {
                out.push(Finding::warn(check, file, 0, format!("baseline {n} can drop to 0: run scripts/docs_gen.sh --update-baseline")));
            }
        }
        out
    }
}

/// Ratchet guard for `update-baseline`: no per-file count may exceed its recorded baseline, and no
/// file or check without a baseline may appear. Returns every violation.
pub fn check_no_increase(old: &BTreeMap<String, BTreeMap<String, usize>>, new: &BTreeMap<&'static str, BTreeMap<String, usize>>) -> Result<(), String> {
    let mut bad = Vec::new();
    for (check, files) in new {
        for (file, &n) in files {
            let was = old.get(*check).and_then(|m| m.get(file)).copied().unwrap_or(0);
            if n > was {
                bad.push(format!("  {check}: {file}: {n} findings, baseline {was}"));
            }
        }
    }
    if bad.is_empty() { Ok(()) } else { Err(bad.join("\n")) }
}

/// Rewrite the `[baseline.*]` tables of the config file from current raw counts, keeping the rest verbatim.
pub fn render_baseline(counts: &BTreeMap<&'static str, BTreeMap<String, usize>>) -> String {
    let mut s = String::new();
    for (check, files) in counts {
        if files.is_empty() {
            continue;
        }
        s.push_str(&format!("[baseline.{check}]\n"));
        for (f, n) in files {
            s.push_str(&format!("\"{f}\" = {n}\n"));
        }
        s.push('\n');
    }
    s
}

/// Text of the config file up to (excluding) the first `[baseline` table.
pub fn head_without_baseline(text: &str) -> String {
    let mut out = String::new();
    for l in text.lines() {
        if l.starts_with("[baseline") {
            break;
        }
        out.push_str(l);
        out.push('\n');
    }
    out.trim_end().to_string() + "\n\n"
}
