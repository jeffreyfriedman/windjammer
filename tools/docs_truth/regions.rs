//! Marked generated regions inside hand-written docs:
//! `<!-- wj:gen name=NAME [key=value ...] -->` ... `<!-- wj:end -->`.

use super::config::Config;
use super::{genfiles, util, Ctx, Finding};
use std::collections::BTreeMap;

pub const BEGIN: &str = "<!-- wj:gen ";
pub const END: &str = "<!-- wj:end -->";

#[derive(Debug, Clone)]
pub struct Region {
    pub name: String,
    pub args: BTreeMap<String, String>,
    /// 1-based line of the begin marker.
    pub line: usize,
    /// Byte range of the content between the markers.
    pub start: usize,
    pub end: usize,
}

pub fn parse_marker(line: &str) -> Option<(String, BTreeMap<String, String>)> {
    let t = line.trim();
    let inner = t.strip_prefix(BEGIN)?.strip_suffix("-->")?.trim();
    let mut args = BTreeMap::new();
    for part in inner.split_whitespace() {
        let (k, v) = part.split_once('=')?;
        args.insert(k.to_string(), v.to_string());
    }
    let name = args.remove("name")?;
    Some((name, args))
}

/// Find regions; `Err` describes an unbalanced/unparsable marker.
pub fn find(text: &str) -> Result<Vec<Region>, (usize, String)> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut cur: Option<(String, BTreeMap<String, String>, usize, usize)> = None;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let t = line.trim();
        if t.starts_with(BEGIN) {
            if cur.is_some() {
                return Err((i + 1, "nested or unterminated wj:gen region".into()));
            }
            let Some((name, args)) = parse_marker(t) else { return Err((i + 1, format!("malformed marker: {t}"))) };
            cur = Some((name, args, i + 1, pos + line.len()));
        } else if t == END {
            let Some((name, args, ln, start)) = cur.take() else { return Err((i + 1, "wj:end without wj:gen".into())) };
            out.push(Region { name, args, line: ln, start, end: pos });
        }
        pos += line.len();
    }
    match cur {
        Some((n, _, ln, _)) => Err((ln, format!("region `{n}` has no wj:end"))),
        None => Ok(out),
    }
}

fn body(generated: &str) -> String {
    format!("{}\n", generated.trim_end())
}

/// Rewrite all regions of `text` with freshly generated content. Returns (new text, errors).
pub fn rewrite(ctx: &Ctx, text: &str) -> Result<String, (usize, String)> {
    let regions = find(text)?;
    let mut out = String::new();
    let mut last = 0;
    for r in &regions {
        out.push_str(&text[last..r.start]);
        match genfiles::region(ctx, &r.name, &r.args) {
            Ok(g) => out.push_str(&body(&g)),
            Err(e) => return Err((r.line, format!("region `{}`: {e}", r.name))),
        }
        last = r.end;
    }
    out.push_str(&text[last..]);
    Ok(out)
}

/// Markdown files that may contain regions.
pub fn md_files(ctx: &Ctx) -> Vec<String> {
    util::walk(&ctx.root).into_iter().filter(|f| f.ends_with(".md")).collect()
}

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in md_files(ctx) {
        if cfg.excluded("regions", &f) {
            continue;
        }
        let text = ctx.read(&f);
        if !text.contains(BEGIN) {
            continue;
        }
        match rewrite(ctx, &text) {
            Err((line, msg)) => out.push(Finding::fail("regions", &f, line, msg)),
            Ok(new) if new != text => {
                let line = first_diff_line(&text, &new);
                out.push(Finding::fail("regions", &f, line, "generated region is stale: run scripts/docs_gen.sh"));
            }
            Ok(_) => {}
        }
    }
    out
}

fn first_diff_line(a: &str, b: &str) -> usize {
    a.lines().zip(b.lines()).position(|(x, y)| x != y).map(|i| i + 1).unwrap_or_else(|| a.lines().count().min(b.lines().count()) + 1)
}

/// Write every stale region in place; returns the files changed.
pub fn write_all(ctx: &Ctx, cfg: &Config) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    for f in md_files(ctx) {
        if cfg.excluded("regions", &f) {
            continue;
        }
        let text = ctx.read(&f);
        if !text.contains(BEGIN) {
            continue;
        }
        let new = rewrite(ctx, &text).map_err(|(l, m)| format!("{f}:{l}: {m}"))?;
        if new != text {
            std::fs::write(ctx.root.join(&f), new).map_err(|e| e.to_string())?;
            changed.push(f);
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_and_validates() {
        let t = "x\n<!-- wj:gen name=a k=v -->\nold\n<!-- wj:end -->\ny\n";
        let r = find(t).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(&t[r[0].start..r[0].end], "old\n");
        assert_eq!(r[0].args["k"], "v");
        assert!(find("<!-- wj:gen name=a -->\n").is_err());
        assert!(find("<!-- wj:end -->\n").is_err());
        assert!(find("<!-- wj:gen bogus -->\n<!-- wj:end -->\n").is_err());
    }
}
