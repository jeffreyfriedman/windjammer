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
    for file in &files {
        let source = fs::read_to_string(file)?;
        summary.files_scanned += 1;
        let is_md = file.extension().is_some_and(|e| e == "md");
        let out = if is_md {
            let text = surface_fix::fix_markdown(&source);
            surface_fix::FixOutput {
                findings: Vec::new(),
                text,
            }
        } else {
            let mut out = surface_fix::fix_source(&source);
            out.text = fix_trailing_returns(&name_of(file), out.text);
            out.text = surface_fix_options(out.text);
            out.text = crate::linter::redundant_as_str::fix(&out.text);
            out
        };
        summary.skipped += out.skipped_count();
        if out.text == source {
            continue;
        }
        let name = file.display().to_string();
        if !is_md && !(parses(&name, &source) && parses(&name, &out.text)) {
            eprintln!("warning: {name}: does not parse before/after rewrite; left unchanged");
            continue;
        }
        summary.files_changed += 1;
        summary.rewrites += out.changed_count().max(1);
        if !check {
            fs::write(file, &out.text)?;
        }
        println!("fixed {} ({} rewrite(s))", name, out.changed_count());
    }
    println!(
        "wj fix: {} file(s) scanned, {} changed, {} rewrite(s), {} occurrence(s) need manual migration",
        summary.files_scanned, summary.files_changed, summary.rewrites, summary.skipped
    );
    if check && summary.files_changed > 0 {
        bail!("{} file(s) need `wj fix`", summary.files_changed);
    }
    Ok(summary)
}
