//! Freshness metadata: `status` and `last_verified` (a commit) front matter on design docs and ADRs,
//! and a git heuristic flagging docs whose referenced code changed a lot since. Warn, not fail,
//! except for malformed metadata.

use super::config::Config;
use super::{md, util, Ctx, Finding};
use std::process::Command;

pub const STATUSES: &[&str] = &["draft", "proposed", "proposal", "accepted", "superseded", "rejected", "current", "stale", "archived"];

#[derive(Debug, Default, PartialEq)]
pub struct Meta {
    pub status: Option<String>,
    pub last_verified: Option<String>,
}

/// Front matter between a leading `---` line and the next `---`.
pub fn front_matter(text: &str) -> Option<Meta> {
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut m = Meta::default();
    for l in lines {
        if l.trim() == "---" {
            return Some(m);
        }
        if let Some((k, v)) = l.split_once(':') {
            let v = v.trim().to_string();
            match k.trim() {
                "status" => m.status = Some(v),
                "last_verified" => m.last_verified = Some(v),
                _ => {}
            }
        }
    }
    None
}

fn git(ctx: &Ctx, args: &[&str]) -> Option<String> {
    let o = Command::new("git").args(args).current_dir(&ctx.root).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
}

pub fn head(ctx: &Ctx) -> Option<String> {
    git(ctx, &["rev-parse", "--short=12", "HEAD"]).map(|s| s.trim().to_string())
}

/// Lines changed in `paths` between `sha` and HEAD (None when git cannot tell).
pub fn changed_lines(ctx: &Ctx, sha: &str, paths: &[String]) -> Option<usize> {
    let mut args = vec!["diff", "--numstat", sha, "HEAD", "--"];
    args.extend(paths.iter().map(|p| p.as_str()));
    let out = git(ctx, &args)?;
    Some(out.lines().filter_map(|l| {
        let mut p = l.split_whitespace();
        Some(p.next()?.parse::<usize>().ok()? + p.next()?.parse::<usize>().ok()?)
    }).sum())
}

fn referenced_paths(ctx: &Ctx, text: &str) -> Vec<String> {
    let mut v = Vec::new();
    for (_, l) in md::prose_lines(text) {
        for s in md::code_spans(l) {
            let s = s.trim_end_matches(['.', ',']).split(':').next().unwrap_or("").to_string();
            if (s.starts_with("src/") || s.starts_with("std/") || s.starts_with("tests/")) && !s.contains(['*', ' ', '<']) && util::exists(&ctx.root, &s) && !v.contains(&s) {
                v.push(s);
            }
        }
    }
    v
}

pub fn check_doc(ctx: &Ctx, cfg: &Config, file: &str, text: &str) -> Vec<Finding> {
    let Some(m) = front_matter(text) else {
        let msg = "no front matter (status, last_verified); run `wj-docs stamp`";
        return vec![if cfg.freshness.require { Finding::fail("freshness", file, 1, msg) } else { Finding::warn("freshness", file, 1, msg) }];
    };
    let mut out = Vec::new();
    match &m.status {
        None => out.push(Finding::fail("freshness", file, 1, "front matter lacks `status`")),
        Some(s) if !STATUSES.contains(&s.to_lowercase().as_str()) => out.push(Finding::fail("freshness", file, 1, format!("unknown status `{s}` (one of {STATUSES:?})"))),
        _ => {}
    }
    // An ADR states its status twice (front matter, and the `- **Status:**` bullet the index reads): they must agree.
    if file.starts_with("docs/adr/") {
        let body = text.lines().find_map(|l| l.strip_prefix("- **Status:**")).map(|s| s.trim().split(|c: char| !c.is_alphabetic()).next().unwrap_or("").to_lowercase());
        if let (Some(b), Some(fm)) = (body, &m.status) {
            if !b.is_empty() && &b != &fm.to_lowercase() {
                out.push(Finding::fail("freshness", file, 1, format!("front matter status `{fm}` disagrees with the `- **Status:** {b}` line")));
            }
        }
    }
    let Some(sha) = &m.last_verified else {
        let msg = "front matter lacks `last_verified`; run `wj-docs stamp`";
        out.push(if cfg.freshness.require { Finding::fail("freshness", file, 1, msg) } else { Finding::warn("freshness", file, 1, msg) });
        return out;
    };
    if git(ctx, &["cat-file", "-e", &format!("{sha}^{{commit}}")]).is_none() {
        if git(ctx, &["rev-parse", "HEAD"]).is_some() {
            out.push(Finding::fail("freshness", file, 1, format!("last_verified `{sha}` is not a commit of this repository")));
        }
        return out;
    }
    let paths = referenced_paths(ctx, text);
    if let Some(n) = changed_lines(ctx, sha, &paths) {
        if n > cfg.freshness.changed_lines {
            out.push(Finding::warn("freshness", file, 1, format!("{n} lines changed in the {} code paths it references since {sha}; re-verify and `wj-docs stamp`", paths.len())));
        }
    }
    out
}

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in util::walk(&ctx.root).into_iter().filter(|f| f.ends_with(".md")) {
        if cfg.freshness.tracked.iter().any(|g| util::glob_match(g, &f)) && !cfg.excluded("freshness", &f) {
            out.extend(check_doc(ctx, cfg, &f, &ctx.read(&f)));
        }
    }
    out
}

/// Set `status` (kept if present, else `status`) and `last_verified: HEAD` in a doc's front matter.
pub fn stamp(ctx: &Ctx, file: &str, status: &str) -> Result<(), String> {
    let text = ctx.read(file);
    let sha = head(ctx).ok_or("not a git repository")?;
    let (old, body) = match (front_matter(&text), text.strip_prefix("---\n")) {
        (Some(m), Some(rest)) => (m, rest.split_once("\n---\n").map(|x| x.1).unwrap_or("").to_string()),
        _ => (Meta::default(), text.clone()),
    };
    let st = old.status.unwrap_or_else(|| status.to_string());
    std::fs::write(ctx.root.join(file), format!("---\nstatus: {st}\nlast_verified: {sha}\n---\n{}", body.trim_start_matches('\n'))).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_front_matter() {
        let m = front_matter("---\nstatus: accepted\nlast_verified: abc123\n---\n# T\n").unwrap();
        assert_eq!(m.status.as_deref(), Some("accepted"));
        assert_eq!(m.last_verified.as_deref(), Some("abc123"));
        assert!(front_matter("# T\n").is_none());
        assert!(front_matter("---\nstatus: x\n").is_none());
    }
}
