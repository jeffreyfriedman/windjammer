//! docs-truth: the machinery that keeps documentation true. See docs/CONTRIBUTING-docs.md.
//!
//! Included (via `#[path]`) by the `wj-docs` binary and by `tests/docs_truth_test.rs`, so the
//! CLI and the test-suite run exactly the same code. Every check takes a `Ctx` (repo root plus the
//! live clap definition) and returns `Finding`s; nothing here prints or exits.

#![allow(dead_code)]

pub mod claims;
pub mod config;
pub mod ebnf;
pub mod fence_run;
pub mod fences;
pub mod freshness;
pub mod gen_cli;
pub mod gen_errors;
pub mod gen_interp;
pub mod gen_lang;
pub mod gen_meta;
pub mod gen_stdlib;
pub mod gen_targets;
pub mod genfiles;
pub mod links;
pub mod md;
pub mod proposal;
pub mod regions;
pub mod util;

use std::path::PathBuf;

/// Everything a check needs: where the repo is and the real CLI definition.
pub struct Ctx {
    pub root: PathBuf,
    pub cli: clap::Command,
}

impl Ctx {
    pub fn rel(&self, p: &std::path::Path) -> String {
        p.strip_prefix(&self.root).unwrap_or(p).to_string_lossy().replace('\\', "/")
    }
    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root.join(rel)).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub check: &'static str,
    pub file: String,
    pub line: usize,
    pub msg: String,
    /// Warnings are reported but never fail the run.
    pub warn: bool,
}

impl Finding {
    pub fn fail(check: &'static str, file: &str, line: usize, msg: impl Into<String>) -> Finding {
        Finding { check, file: file.to_string(), line, msg: msg.into(), warn: false }
    }
    pub fn warn(check: &'static str, file: &str, line: usize, msg: impl Into<String>) -> Finding {
        Finding { check, file: file.to_string(), line, msg: msg.into(), warn: true }
    }
    pub fn render(&self) -> String {
        let sev = if self.warn { "warn" } else { "FAIL" };
        format!("[{sev}] {}: {}:{}: {}", self.check, self.file, self.line, self.msg)
    }
}

pub fn failures(fs: &[Finding]) -> usize {
    fs.iter().filter(|f| !f.warn).count()
}

/// Run every check except the ones that need external processes. Order is cheap-first.
pub fn run_all(ctx: &Ctx) -> Vec<Finding> {
    let cfg = config::load(ctx);
    let mut out = genfiles::check(ctx);
    out.extend(regions::check_all(ctx, &cfg));
    out.extend(fences::check_all(ctx, &cfg));
    out.extend(links::check_all(ctx, &cfg));
    out.extend(claims::check_all(ctx, &cfg));
    out.extend(freshness::check_all(ctx, &cfg));
    out.extend(proposal::check_all(ctx));
    out.extend(ebnf::check_all(ctx, &cfg));
    out
}
