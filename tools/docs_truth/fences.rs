//! Executable docs: every ```windjammer / ```wj fence in Markdown and rustdoc is checked.
//!
//! Annotations (comma-separated after the language, e.g. `wj,should_fail=WJ0007`):
//! * (none)               must parse (as a program, else as the body of `fn main`)
//! * `ignore`             not checked (use sparingly; counted in the report)
//! * `check`              must also pass the analyzer
//! * `should_fail`        must fail to parse/analyze; `should_fail=WJ0007` the error must carry the code;
//!                        `should_fail=W0002` the Rust-leakage linter must report that lint
//! * `run`                must run in the interpreter
//! * `// expect: line`    inside the code: run in the interpreter, stdout lines must equal (in order)
//! * `sh,verify`          (bash fences) executed in a sandbox temp dir, see `fence_run::run_sh`

use super::config::Config;
use super::{fence_run, md, util, Ctx, Finding};

pub use windjammer::doctest::fence::{parse_info, rustdoc_as_markdown, Ann};

/// A `sh,verify` fence that could not run: a warning locally, a failure in CI (`WJ_DOCS_REQUIRE_EXEC=1`), so a
/// missing `wj` can never turn the quickstart check into a silent pass.
pub fn not_executed(file: &str, line: usize, require: bool) -> Finding {
    let msg = "sh,verify fence not executed (no wj binary: set WJ_BIN)";
    if require { Finding::fail("fences", file, line, msg) } else { Finding::warn("fences", file, line, msg) }
}

/// An `ignore`/`should_fail` fence must be justified by an HTML comment on the nearest non-blank line above it.
pub fn has_reason(lines: &[&str], fence_line: usize) -> bool {
    let mut i = fence_line.saturating_sub(1); // index of the fence line itself (1-based -> 0-based)
    while i > 0 {
        i -= 1;
        let t = lines[i].trim();
        if t.is_empty() {
            continue;
        }
        let low = t.to_lowercase();
        return t.starts_with("<!--") && t.ends_with("-->") && (low.contains("ignore:") || low.contains("should_fail:")) && t.len() > "<!-- ignore: -->".len() + 3;
    }
    false
}

/// Documents scanned: Markdown anywhere, plus rustdoc (`///`, `//!`) of `.rs` under `src/` and `crates/`.
pub fn sources(ctx: &Ctx) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in util::walk(&ctx.root) {
        if f.ends_with(".md") {
            out.push((f.clone(), ctx.read(&f)));
        } else if f.ends_with(".rs") && (f.starts_with("src/") || (f.starts_with("crates/") && f.contains("/src/"))) {
            let text = ctx.read(&f);
            if text.contains("```windjammer") || text.contains("```wj") {
                out.push((f, rustdoc_as_markdown(&text)));
            }
        }
    }
    out
}

pub struct Report {
    pub checked: usize,
    pub ignored: usize,
    pub raw: Vec<Finding>,
}

/// Check one document's fences. `wj_bin` enables `sh,verify` execution.
pub fn check_doc(file: &str, text: &str, wj_bin: Option<&std::path::Path>) -> Report {
    let mut r = Report { checked: 0, ignored: 0, raw: vec![] };
    let lines: Vec<&str> = text.lines().collect();
    // Proposal documents (front matter `status: proposal`, banner enforced by `proposal`) show future syntax.
    let proposal = super::proposal::is_proposal(text);
    for f in md::fences(text) {
        if proposal && parse_info(&f.info).wj {
            r.ignored += 1;
            continue;
        }
        let a = parse_info(&f.info);
        let is_sh = matches!(f.info.split([',', ' ']).next().unwrap_or(""), "sh" | "bash" | "shell" | "console");
        if (a.wj || is_sh) && (a.ignore || a.should_fail.is_some()) && !has_reason(&lines, f.line) {
            r.raw.push(Finding::fail("fences", file, f.line, "`ignore`/`should_fail` fences need a reason: put `<!-- ignore: why -->` (or `<!-- should_fail: why -->`) on the line above"));
        }
        if a.ignore {
            r.ignored += 1;
            continue;
        }
        if a.sh_verify {
            r.checked += 1;
            match wj_bin {
                Some(bin) => {
                    if let Err(e) = fence_run::run_sh(&f.body, bin) {
                        r.raw.push(Finding::fail("fences", file, f.line, format!("sh,verify failed: {e}")));
                    }
                }
                None => r.raw.push(not_executed(file, f.line, std::env::var_os("WJ_DOCS_REQUIRE_EXEC").is_some())),
            }
            continue;
        }
        if !a.wj {
            continue;
        }
        r.checked += 1;
        if let Err(e) = fence_run::check_wj(&f.body, &a) {
            r.raw.push(Finding::fail("fences", file, f.line, e));
        }
    }
    r
}

/// Locate the `wj` binary for `sh,verify` (env override, then Cargo's test-time variable).
pub fn wj_bin() -> Option<std::path::PathBuf> {
    std::env::var_os("WJ_BIN").map(Into::into).or_else(|| option_env!("CARGO_BIN_EXE_wj").map(Into::into)).filter(|p: &std::path::PathBuf| p.exists())
}

/// In documents listed under `[verify] required`, every shell fence that runs `wj` must be
/// `sh,verify` (executed in CI) or explicitly `ignore`d. Not ratcheted: this fails outright.
pub fn unverified_wj_fences(file: &str, text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in md::fences(text) {
        let a = parse_info(&f.info);
        let lang = f.info.split([',', ' ']).next().unwrap_or("");
        if !matches!(lang, "sh" | "bash" | "shell" | "console") || a.sh_verify || a.ignore {
            continue;
        }
        let runs_wj = f.body.lines().any(|l| {
            let l = l.trim_start().trim_start_matches("$ ");
            l.starts_with("wj ") || l.starts_with("./target/release/wj ") || l == "wj"
        });
        if runs_wj {
            out.push(Finding::fail("fences", file, f.line, "shell fence runs `wj` but is neither `sh,verify` nor `ignore` (documents under [verify] required must execute their commands in CI)"));
        }
    }
    out
}

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let bin = wj_bin();
    let mut raw = Vec::new();
    let mut hard = Vec::new();
    for (file, text) in sources(ctx) {
        if cfg.excluded("fences", &file) {
            continue;
        }
        if cfg.verify.required.iter().any(|g| util::glob_match(g, &file)) {
            hard.extend(unverified_wj_fences(&file, &text));
        }
        raw.extend(check_doc(&file, &text, bin.as_deref()).raw);
    }
    let (warns, fails): (Vec<_>, Vec<_>) = raw.into_iter().partition(|f| f.warn);
    let mut out = cfg.ratchet("fences", fails);
    out.extend(warns);
    out.extend(hard);
    out
}

/// Raw per-file failure counts (for `--update-baseline`).
pub fn raw_counts(ctx: &Ctx, cfg: &Config) -> std::collections::BTreeMap<String, usize> {
    let mut m = std::collections::BTreeMap::new();
    for (file, text) in sources(ctx) {
        if cfg.excluded("fences", &file) {
            continue;
        }
        let n = check_doc(&file, &text, None).raw.iter().filter(|f| !f.warn).count();
        if n > 0 {
            m.insert(file, n);
        }
    }
    m
}
