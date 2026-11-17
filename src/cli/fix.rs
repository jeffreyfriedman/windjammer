// wj fix - migrate deprecated surface syntax (ADR-0014 Phase 1)
//
// Rewrites `println!("a {}", x)` -> `println("a ${x}")`, `vec![..]` -> `[..]`,
// `eprintln!` -> `stderr.println`, `dbg!` -> `debug`, `format!` -> interpolated strings,
// `assert!`/`assert_eq!`/`panic!`/`todo!`/`unreachable!` -> plain calls, and (ADR-0018) `Option<T>` -> `T?`,
// `x.unwrap_or(d)` -> `x ?? d`.
// Idempotent; a rewritten file is only written when it still parses.

use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::intrinsics::surface_fix;

#[derive(Debug, Default)]
pub struct FixSummary {
    pub files_scanned: usize,
    pub files_changed: usize,
    pub rewrites: usize,
    pub skipped: usize,
}

fn parses(path: &str, source: &str) -> bool {
    // The lexer panics on invalid characters; treat that as "does not parse".
    let (path, source) = (path.to_string(), source.to_string());
    std::panic::catch_unwind(move || {
        let tokens = crate::lexer::Lexer::new(&source).tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(tokens, path, source.clone());
        parser.parse().is_ok()
    })
    .unwrap_or(false)
}

fn parse_program(path: &str, source: &str) -> Option<crate::parser::Program<'static>> {
    let (path, source) = (path.to_string(), source.to_string());
    std::panic::catch_unwind(move || {
        let tokens = crate::lexer::Lexer::new(&source).tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(tokens, path, source.clone());
        parser.parse().ok()
    })
    .ok()
    .flatten()
}

/// Declared-type verdicts for the `vec![..]` occurrences of every `.wj` file (see `intrinsics::vec_context`).
/// The declaration index spans all scanned files, so calls across modules resolve.
fn vec_verdicts(files: &[(PathBuf, String)]) -> Vec<crate::intrinsics::vec_context::Verdicts> {
    use crate::intrinsics::vec_context::{verdicts, DeclIndex};
    let programs: Vec<_> = files
        .iter()
        .map(|(p, src)| {
            let wj = p.extension().is_some_and(|e| e == "wj");
            if wj && src.contains("vec!") {
                parse_program(&name_of(p), src)
            } else {
                None
            }
        })
        .collect();
    let mut index = DeclIndex::default();
    for (p, (path, src)) in programs.iter().zip(files) {
        match p {
            Some(prog) => index.add_program(prog),
            // declarations of a file that only lacks `vec!` still count
            None if path.extension().is_some_and(|e| e == "wj") => {
                if let Some(prog) = parse_program(&name_of(path), src) {
                    index.add_program(&prog);
                }
            }
            None => {}
        }
    }
    programs
        .iter()
        .map(|p| p.as_ref().map(|prog| verdicts(prog, &index)).unwrap_or_default())
        .collect()
}

fn name_of(p: &Path) -> String {
    p.display().to_string()
}

/// W0009: delete redundant trailing `return` keywords (needs a parse; a file that does not parse is untouched).
fn fix_trailing_returns(name: &str, text: String) -> String {
    let (n, t) = (name.to_string(), text.clone());
    let edited = std::panic::catch_unwind(move || {
        let tokens = crate::lexer::Lexer::new(&t).tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(tokens, n, t.clone());
        parser.parse().ok().map(|p| {
            let rs = crate::linter::trailing_return::tail_returns(&p);
            crate::linter::trailing_return::fix_source(&t, &rs)
        })
    });
    match edited {
        Ok(Some(new_text)) => new_text,
        _ => text,
    }
}

/// ADR-0004: `thread { }` -> `spawn { }`, `ch <- v` -> `ch.send(v)`, `<-ch` -> `ch.recv()` (WJ0191), then a task nobody
/// waits on is started inside a `scope { }` that ends with its block (WJ0190). Runs before the parse guard: a file
/// with a removed form does not parse until it is migrated. Returns the text and the number of edits.
fn migrate_concurrency(name: &str, mut text: String) -> (String, usize) {
    let mut n = 0;
    for _ in 0..16 {
        let diags = crate::check::concurrency_diagnostics(name, &text);
        let edits: Vec<_> = crate::diagnostics::apply::collect_edits(&diags).into_values().flatten().collect();
        match crate::diagnostics::apply::apply_edits(&text, &edits) {
            Some(t) if !edits.is_empty() && t != text => {
                n += edits.len();
                text = t;
            }
            _ => break,
        }
    }
    (text, n)
}

/// ADR-0018: `Option<T>` -> `T?` and `x.unwrap_or(d)` -> `x ?? d` (conservative; see `intrinsics::option_fix`).
fn surface_fix_options(text: String) -> String {
    crate::intrinsics::option_fix::fix_options(&text).text
}

fn is_target(p: &Path, docs: bool) -> bool {
    p.extension().is_some_and(|e| e == "wj" || (docs && e == "md"))
}

fn collect(path: &Path, out: &mut Vec<PathBuf>, docs: bool) -> Result<()> {
    if path.is_file() {
        if is_target(path, docs) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let p = entry?.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if matches!(name, "target" | "build" | ".git" | "node_modules" | "out") {
                continue;
            }
            collect(&p, out, docs)?;
        } else if is_target(&p, docs) {
            out.push(p);
        }
    }
    Ok(())
}

/// Insert `mut` where a local binding is changed without it (WJ0051), in the program at `path` and, with `docs`, in
/// every ```wj code fence of the `.md` files under it (each fence is its own program). Returns (files, edits).
fn add_missing_mut(path: &Path, check: bool, docs: bool) -> Result<(usize, usize)> {
    let has_wj = path.is_dir() || path.extension().is_some_and(|e| e == "wj");
    let checked = if has_wj {
        crate::check::local_mutability(path)?
    } else {
        crate::check::Checked { report: crate::diagnostics::Report::new("check", Vec::new()), sources: Default::default() }
    };
    let outcome = crate::check::apply_fixes(&checked.report.diagnostics, &checked.sources, !check)?;
    for f in &outcome.files_skipped {
        eprintln!("warning: {f}: adding `mut` would not parse; left unchanged");
    }
    for d in &checked.report.diagnostics {
        let Some(decl) = d.secondary_spans.first().filter(|s| !outcome.files_skipped.contains(&s.file)) else { continue };
        let name = checked.sources.get(&decl.file).and_then(|t| t.get(decl.byte_start..decl.byte_end)).unwrap_or("?");
        let verb = if check { "is missing" } else { "added" };
        println!("{}:{}: `{name}` is changed: `mut` {verb} ({})", decl.file, decl.line_start, d.code);
    }
    let (mut files, mut edits) = (outcome.files_changed, outcome.edits_applied);
    if docs {
        let mut md = Vec::new();
        collect(path, &mut md, true)?;
        for file in md.iter().filter(|p| p.extension().is_some_and(|e| e == "md")) {
            let text = fs::read_to_string(file)?;
            let (new, n) = mut_in_fences(&text);
            if n > 0 {
                files += 1;
                edits += n;
                println!("fixed {} ({n} `mut` added in code fences)", file.display());
                if !check {
                    fs::write(file, new)?;
                }
            }
        }
    }
    Ok((files, edits))
}

/// Whether a fence info string (```wj, ```wj,test, ```windjammer) holds Windjammer code.
fn is_wj_fence(line: &str) -> bool {
    let info = line.trim().strip_prefix("```").unwrap_or("");
    !line.trim().is_empty() && line.trim().starts_with("```") && matches!(info.split(',').next(), Some("wj" | "windjammer"))
}

/// `md` with `mut` inserted in the code fences that change a binding declared without it, and the edit count.
fn mut_in_fences(md: &str) -> (String, usize) {
    let mut out = String::with_capacity(md.len() + 16);
    let mut count = 0;
    let mut lines = md.split_inclusive('\n');
    while let Some(line) = lines.next() {
        out.push_str(line);
        if !is_wj_fence(line) {
            continue;
        }
        let mut body = String::new();
        let mut closing = None;
        for l in lines.by_ref() {
            if l.trim() == "```" {
                closing = Some(l);
                break;
            }
            body.push_str(l);
        }
        let diags = crate::check::local_mutability_sources(&[("fence.wj".to_string(), body.clone())]);
        let edits: Vec<crate::diagnostics::Edit> = crate::diagnostics::apply::collect_edits(&diags).into_values().flatten().collect();
        match crate::diagnostics::apply::apply_edits(&body, &edits).filter(|new| parses("fence.wj", new)) {
            Some(new) if !edits.is_empty() => {
                count += edits.len();
                out.push_str(&new);
            }
            _ => out.push_str(&body),
        }
        if let Some(c) = closing {
            out.push_str(c);
        }
    }
    (out, count)
}

/// Fix a path (file or directory). With `check`, nothing is written and the call fails if
/// any file would change.
pub fn execute(path: &Path, check: bool, docs: bool) -> Result<FixSummary> {
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    let mut files = Vec::new();
    collect(path, &mut files, docs)?;
    files.sort();
    let mut summary = FixSummary::default();
    let sources: Vec<(PathBuf, String)> = files
        .iter()
        .map(|f| Ok((f.clone(), fs::read_to_string(f)?)))
        .collect::<Result<_>>()?;
    let verdicts = vec_verdicts(&sources);
    for (idx, (file, (_, source))) in files.iter().zip(sources).enumerate() {
        let source = source;
        let mut conc_rewrites = 0;
        summary.files_scanned += 1;
        let is_md = file.extension().is_some_and(|e| e == "md");
        let out = if is_md {
            let text = surface_fix::fix_markdown(&source);
            surface_fix::FixOutput {
                findings: Vec::new(),
                text,
            }
        } else {
            let (migrated, conc) = migrate_concurrency(&name_of(file), source.clone());
            conc_rewrites = conc;
            let mut out = surface_fix::fix_source_with(&migrated, &verdicts[idx]);
            out.text = fix_trailing_returns(&name_of(file), out.text);
            out.text = surface_fix_options(out.text);
            out.text = crate::linter::redundant_as_str::fix(&out.text);
            out
        };
        summary.skipped += out.skipped_count();
        for f in out.findings.iter().filter(|f| f.replacement.is_none() && f.original.starts_with("vec!")) {
            println!("{}:{}:{}: {}", file.display(), f.line, f.col, f.message);
        }
        if out.text == source {
            continue;
        }
        let name = file.display().to_string();
        // a file the ADR-0004 migration changed did not parse before it; the result must parse
        if !is_md && !((conc_rewrites > 0 || parses(&name, &source)) && parses(&name, &out.text)) {
            eprintln!("warning: {name}: does not parse before/after rewrite; left unchanged");
            continue;
        }
        summary.files_changed += 1;
        summary.rewrites += (out.changed_count() + conc_rewrites).max(1);
        if !check {
            fs::write(file, &out.text)?;
        }
        println!("fixed {} ({} rewrite(s))", name, out.changed_count() + conc_rewrites);
    }
    // ADR-0002: every local binding the program changes is declared `mut` (WJ0051), after the rewrites above
    let (mut_files, mut_edits) = add_missing_mut(path, check, docs)?;
    summary.files_changed += mut_files;
    summary.rewrites += mut_edits;
    println!(
        "wj fix: {} file(s) scanned, {} changed, {} rewrite(s), {} occurrence(s) need manual migration",
        summary.files_scanned, summary.files_changed, summary.rewrites, summary.skipped
    );
    if check && summary.files_changed > 0 {
        bail!("{} file(s) need `wj fix`", summary.files_changed);
    }
    Ok(summary)
}
