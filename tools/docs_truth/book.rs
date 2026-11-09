//! The Windjammer Book (docs/book): structure, runnable samples, links, promise coverage and the
//! ratchets that keep all of it honest. Counts are metrics (`book.*`), never typed by hand.
//!
//! * every `wj` fence in a page is a program with a `fn main` that runs, unless it is `ignore` (with a
//!   reason, counted, `[book] max_ignored` may only shrink) or `should_fail`/`sh`;
//! * `~` at the start of a line hides it in the rendered book (mdBook `hidelines`) but keeps it in the program;
//! * `// expect: line` comments are the expected stdout, interpreted and compiled;
//! * `extract` writes the runnable samples to a directory; `scripts/book/book_samples.wj` runs them both ways;
//! * every entry of docs/promises.toml must be mentioned by some page (`max_uncovered_promises` may only shrink).

use super::config::Config;
use super::{md, util, Ctx, Finding};
use std::collections::BTreeSet;
use windjammer::doctest::fence::{expects, parse_info};

pub const ROOT: &str = "docs/book";
pub const SRC: &str = "docs/book/src";
pub const SUMMARY: &str = "docs/book/src/SUMMARY.md";
/// Links to repository files (outside the book) are written with this prefix and checked against the tree.
pub const REPO_URL: &str = "https://github.com/jeffreyfriedman/windjammer/blob/main/";

/// The book exists (a scratch repo that only holds the generated reference pages has no SUMMARY).
pub fn active(ctx: &Ctx) -> bool {
    util::exists(&ctx.root, SUMMARY)
}

pub fn is_book_file(file: &str) -> bool {
    file.starts_with("docs/book/src/") && file.ends_with(".md")
}

/// Remove the `~` hidden-line marker (the lines stay in the program).
pub fn unhide(body: &str) -> String {
    body.lines().map(|l| l.strip_prefix('~').unwrap_or(l)).collect::<Vec<_>>().join("\n") + "\n"
}

pub fn pages(ctx: &Ctx) -> Vec<(String, String)> {
    util::walk(&ctx.root.join(SRC)).into_iter().filter(|f| f.ends_with(".md")).map(|f| (format!("{SRC}/{f}"), ctx.read(&format!("{SRC}/{f}")))).collect()
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub code: String,
    pub expect: Vec<String>,
    pub test: bool,
    /// Runs interpreted only (`wj,interpreter_only` with an `<!-- interpreter-only: why -->` line above): the feature has no
    /// compiled form yet. Counted; `[book] max_interpreter_only` may only shrink.
    pub interp_only: bool,
}

pub struct Inventory {
    pub samples: Vec<Sample>,
    pub ignored: Vec<(String, usize)>,
    /// `interpreter_only` fences lacking their reason comment.
    pub unexplained: Vec<(String, usize)>,
}

fn is_test(info: &str) -> bool {
    info.split([',', ' ']).any(|p| p == "test")
}

fn is_interp_only(info: &str) -> bool {
    info.split([',', ' ']).any(|p| p == "interpreter_only")
}

/// Is the nearest non-blank line above the fence an `<!-- MARKER reason -->` comment with a reason?
fn has_marker(text: &str, fence_line: usize, marker: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = fence_line.saturating_sub(1);
    while i > 0 {
        i -= 1;
        let t = lines[i].trim();
        if t.is_empty() {
            continue;
        }
        return t.starts_with("<!--") && t.ends_with("-->") && t.contains(marker) && t.len() > marker.len() + 12;
    }
    false
}

pub fn inventory(ctx: &Ctx) -> Inventory {
    let mut inv = Inventory { samples: vec![], ignored: vec![], unexplained: vec![] };
    for (file, text) in pages(ctx) {
        for f in md::fences(&text) {
            let a = parse_info(&f.info);
            if !a.wj {
                continue;
            }
            if a.ignore {
                inv.ignored.push((file.clone(), f.line));
                continue;
            }
            if a.should_fail.is_some() {
                continue;
            }
            let interp_only = is_interp_only(&f.info);
            if interp_only && !has_marker(&text, f.line, "interpreter-only:") {
                inv.unexplained.push((file.clone(), f.line));
            }
            let code = unhide(&f.body);
            let stem = file.trim_start_matches(&format!("{SRC}/")).trim_end_matches(".md").replace('/', "_");
            inv.samples.push(Sample { id: format!("{stem}_L{}", f.line), file: file.clone(), line: f.line, expect: expects(&code), test: is_test(&f.info), interp_only, code });
        }
    }
    inv
}

/// Write the runnable samples to `dir`: `ID.wj` (or `ID_test.wj`), `ID.expect`, and `INDEX.tsv`.
pub fn extract(ctx: &Ctx, dir: &std::path::Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(dir)?;
    let inv = inventory(ctx);
    let mut index = String::new();
    for s in &inv.samples {
        let name = if s.test { format!("{}_test", s.id) } else { s.id.clone() };
        std::fs::write(dir.join(format!("{name}.wj")), &s.code)?;
        if !s.expect.is_empty() {
            std::fs::write(dir.join(format!("{name}.expect")), s.expect.join("\n") + "\n")?;
        }
        if s.interp_only {
            std::fs::write(dir.join(format!("{name}.interp")), "")?;
        }
        index.push_str(&format!("{name}\t{}:{}\n", s.file, s.line));
    }
    std::fs::write(dir.join("INDEX.tsv"), index)?;
    super::book_projects::extract(ctx, dir)?;
    Ok(inv.samples.len())
}

/// Main-less runnable samples are a mistake: every sample is a whole program (or a test file).
fn sample_shape(inv: &Inventory) -> Vec<Finding> {
    inv.samples.iter().filter(|s| !s.test && !s.code.contains("fn main")).map(|s| Finding::fail("book", &s.file, s.line, "wj fence has no `fn main`: make it a whole program, hide scaffolding with `~` lines, or mark it `ignore` with a reason")).collect()
}

// ---- SUMMARY ----

/// (title, target) of each SUMMARY entry; an empty target is a draft chapter.
pub fn summary_entries(text: &str) -> Vec<(usize, String, String)> {
    let mut out = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let t = l.trim_start().trim_start_matches(['-', '*']).trim_start();
        if let (Some(a), Some(b)) = (t.strip_prefix('['), t.find("](")) {
            let title = a[..b - 1].to_string();
            if let Some(end) = t[b + 2..].find(')') {
                out.push((i + 1, title, t[b + 2..b + 2 + end].to_string()));
            }
        }
    }
    out
}

fn summary_check(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    let text = ctx.read(SUMMARY);
    if text.is_empty() {
        return vec![Finding::fail("book", SUMMARY, 0, "missing SUMMARY.md")];
    }
    let entries = summary_entries(&text);
    let mut listed = BTreeSet::new();
    for (ln, title, target) in &entries {
        if target.is_empty() {
            continue;
        }
        if !util::exists(&ctx.root, &format!("{SRC}/{target}")) {
            out.push(Finding::fail("book", SUMMARY, *ln, format!("`{title}` points at {target}, which does not exist")));
        }
        listed.insert(target.clone());
    }
    for (file, _) in pages(ctx) {
        let rel = file.trim_start_matches(&format!("{SRC}/"));
        if rel != "SUMMARY.md" && !listed.contains(rel) {
            out.push(Finding::fail("book", &file, 0, "page is not listed in SUMMARY.md (it would not be published)"));
        }
    }
    out
}

// ---- links ----

fn slug(h: &str) -> String {
    h.trim().to_lowercase().chars().filter_map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { Some(c) } else if c == ' ' { Some('-') } else { None }).collect()
}

fn anchors(text: &str) -> BTreeSet<String> {
    md::prose_lines(text).into_iter().filter(|(_, l)| l.starts_with('#')).map(|(_, l)| slug(l.trim_start_matches('#'))).collect()
}

fn links_in(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(i) = rest.find("](") {
        let after = &rest[i + 2..];
        if let Some(e) = after.find(')') {
            out.push(after[..e].trim().to_string());
            rest = &after[e..];
        } else {
            break;
        }
    }
    out
}

fn normalize(dir: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

fn links_check(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    let all = pages(ctx);
    for (file, text) in &all {
        let dir = file.rsplit_once('/').map(|x| x.0).unwrap_or("");
        for (ln, line) in md::prose_lines(text) {
            for l in links_in(line) {
                if l.is_empty() || l.starts_with("mailto:") {
                    continue;
                }
                if let Some(path) = l.strip_prefix(REPO_URL) {
                    let path = path.split('#').next().unwrap_or(path);
                    if !util::exists(&ctx.root, path) {
                        out.push(Finding::fail("book", file, ln, format!("repository link {path} does not exist")));
                    }
                } else if l.starts_with("http://") || l.starts_with("https://") {
                    // external: not fetched offline
                } else if l.starts_with('#') {
                    if !anchors(text).contains(&l[1..]) {
                        out.push(Finding::fail("book", file, ln, format!("no heading for anchor {l}")));
                    }
                } else {
                    let (target, anchor) = l.split_once('#').unwrap_or((&l, ""));
                    match normalize(dir, target) {
                        Some(p) if p.starts_with(SRC) && util::exists(&ctx.root, &p) => {
                            if !anchor.is_empty() && !anchors(&ctx.read(&p)).contains(anchor) {
                                out.push(Finding::fail("book", file, ln, format!("{target} has no heading for #{anchor}")));
                            }
                        }
                        Some(p) if p.starts_with(SRC) => out.push(Finding::fail("book", file, ln, format!("link target {target} does not exist"))),
                        _ => out.push(Finding::fail("book", file, ln, format!("link {l} leaves the book; link repository files as {REPO_URL}<path>"))),
                    }
                }
            }
        }
    }
    out
}

// ---- promise coverage ----

fn promises(ctx: &Ctx) -> toml::Value {
    let text = ctx.read("docs/promises.toml");
    if text.is_empty() {
        return toml::Value::Table(Default::default());
    }
    toml::from_str(&text).unwrap_or_else(|e| panic!("docs/promises.toml: {e}"))
}

pub fn promise_ids(ctx: &Ctx) -> Vec<String> {
    let v = promises(ctx);
    v.get("promise").and_then(|p| p.as_array()).into_iter().flatten().filter_map(|p| p.get("id").and_then(|i| i.as_str()).map(String::from)).collect()
}

/// Is promise `id` mentioned in the book text? `wj sub` by name, `wj sub --flag` by the flag, `code X` by the
/// code, `feature slug` by a `promise: feature slug` marker (an HTML comment on the page that teaches it).
pub fn covered(id: &str, book_text: &str) -> bool {
    if let Some(rest) = id.strip_prefix("wj ") {
        return match rest.split_once(" --") {
            Some((sub, flag)) => book_text.contains(&format!("--{flag}")) && book_text.contains(&format!("wj {sub}")),
            None => book_text.contains(id),
        };
    }
    if let Some(code) = id.strip_prefix("code ") {
        return book_text.contains(code);
    }
    book_text.contains(&format!("promise: {id}"))
}

pub fn uncovered_promises(ctx: &Ctx) -> Vec<String> {
    let text: String = pages(ctx).into_iter().map(|(_, t)| t).collect::<Vec<_>>().join("\n");
    promise_ids(ctx).into_iter().filter(|id| !covered(id, &text)).collect()
}

// ---- counts and ratchets ----

pub struct Counts {
    pub pages: usize,
    pub drafts: usize,
    pub examples: usize,
    pub ignored: usize,
    pub interp_only: usize,
    pub uncovered: usize,
}

pub fn counts(ctx: &Ctx) -> Counts {
    let entries = summary_entries(&ctx.read(SUMMARY));
    let inv = inventory(ctx);
    Counts {
        pages: entries.iter().filter(|e| !e.2.is_empty()).count(),
        drafts: entries.iter().filter(|e| e.2.is_empty()).count(),
        examples: inv.samples.len(),
        ignored: inv.ignored.len(),
        interp_only: inv.samples.iter().filter(|s| s.interp_only).count(),
        uncovered: if active(ctx) { uncovered_promises(ctx).len() } else { 0 },
    }
}

fn ratchet(out: &mut Vec<Finding>, what: &str, have: usize, max: usize, hint: &str) {
    if have > max {
        out.push(Finding::fail("book", "docs/docs-truth.toml", 0, format!("{what}: {have} exceeds the ratchet {max} ({hint}); the ratchet only shrinks")));
    } else if have < max {
        out.push(Finding::fail("book", "docs/docs-truth.toml", 0, format!("{what}: {have} is below the ratchet {max}: lower `[book]` to {have} in docs/docs-truth.toml")));
    }
}

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    if !active(ctx) {
        return vec![];
    }
    let mut out = summary_check(ctx);
    out.extend(links_check(ctx));
    let inv = inventory(ctx);
    out.extend(sample_shape(&inv));
    for (file, line) in &inv.unexplained {
        out.push(Finding::fail("book", file, *line, "`interpreter_only` fences need a reason: put `<!-- interpreter-only: why -->` on the line above"));
    }
    let c = counts(ctx);
    ratchet(&mut out, "ignored wj fences", c.ignored, cfg.book.max_ignored, "fix the example instead of ignoring it");
    ratchet(&mut out, "interpreter-only fences", c.interp_only, cfg.book.max_interpreter_only, "give the feature a compiled form, or drop the example");
    ratchet(&mut out, "draft chapters", c.drafts, cfg.book.max_drafts, "write the chapter");
    ratchet(&mut out, "uncovered promises", c.uncovered, cfg.book.max_uncovered_promises, "mention the flag, code or `<!-- promise: ID -->` on the page that teaches it");
    out
}

/// Shown by `wj-docs`: the uncovered promise ids.
pub fn report_uncovered(ctx: &Ctx) -> String {
    uncovered_promises(ctx).join("\n")
}

// ---- generated pages ----

/// The numbers of the "How this book is tested" page (a `wj:gen` region).
pub fn stats_table(ctx: &Ctx) -> String {
    let c = counts(ctx);
    util::table(
        &["Measure", "Count"],
        &[
            vec!["Programs in the book".into(), c.examples.to_string()],
            vec!["Blocks marked `ignore`".into(), c.ignored.to_string()],
            vec!["Programs that run interpreted only".into(), c.interp_only.to_string()],
            vec!["Chapters still to be written".into(), c.drafts.to_string()],
            vec!["Pages".into(), c.pages.to_string()],
            vec!["Promises no page mentions yet".into(), c.uncovered.to_string()],
        ],
    )
}

/// Grammar page: the parser-checked EBNF (docs/spec/grammar.ebnf, see `spec_grammar`) in a fence.
pub fn grammar(ctx: &Ctx) -> String {
    format!("# Grammar\n\nThe grammar below is checked against the real lexer and parser: every alternative is derived into a sentence and parsed. Prose rules for each area live in the language specification chapters in the repository (`docs/spec/`).\n\n```ebnf\n{}\n```\n", ctx.read("docs/spec/grammar.ebnf").trim_end())
}

/// CLI promises: every `wj` command and flag the project promises, with whether acceptance cases exist yet.
pub fn cli_promises(ctx: &Ctx) -> String {
    let v = promises(ctx);
    let mut rows = Vec::new();
    for p in v.get("promise").and_then(|p| p.as_array()).into_iter().flatten() {
        let id = p.get("id").and_then(|i| i.as_str()).unwrap_or("");
        if !id.starts_with("wj ") {
            continue;
        }
        let st = p.get("statement").and_then(|s| s.as_str()).unwrap_or("").replace('\n', " ");
        let status = if p.get("pending").is_some() { "preview" } else { "tested" };
        rows.push(vec![format!("`{id}`"), st, status.to_string()]);
    }
    format!("# Command and flag guarantees\n\nGenerated from `docs/promises.toml`. *Tested* entries have acceptance cases that run through the installed `wj`; *preview* entries are in place and still gaining cases.\n\n{}", util::table(&["Command or flag", "What it does", "Status"], &rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_lines_stay_in_the_program() {
        assert_eq!(unhide("~fn main() {\n    println(\"a\")\n~}\n"), "fn main() {\n    println(\"a\")\n}\n");
    }
    #[test]
    fn coverage_rules() {
        assert!(covered("wj build --no-cargo", "run wj build --no-cargo x"));
        assert!(!covered("wj build --no-cargo", "run wj build"));
        assert!(covered("code W0001", "see W0001"));
        assert!(covered("feature x-y", "<!-- promise: feature x-y -->"));
        assert!(!covered("feature x-y", "feature x-y"));
    }
    #[test]
    fn slugs() {
        assert_eq!(slug(" Hello, World_1 "), "hello-world_1");
    }
}
