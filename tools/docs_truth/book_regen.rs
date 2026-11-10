//! `wj-docs book-regen`: rewrites every `rust,generated` fence of the book from the Rust the compiler really generates
//! (`wj build --no-cargo`), prints what changed, and refuses to touch anything when a program does not build. Codegen
//! improves all the time; this is how the pages follow without anyone editing generated code by hand.

use super::{book, Ctx};
use std::path::Path;
use std::process::Command;

/// The generated Rust for one program, or why it could not be built.
fn generate(wj: &Path, id: &str, code: &str) -> Result<String, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let src = dir.path().join(format!("{id}.wj"));
    std::fs::write(&src, code).map_err(|e| e.to_string())?;
    let out = dir.path().join("out");
    let run = Command::new(wj).args(["build", "--no-cargo"]).arg(&src).arg("-o").arg(&out).output().map_err(|e| format!("cannot run {}: {e}", wj.display()))?;
    let text = std::fs::read_to_string(out.join(format!("{id}.rs"))).unwrap_or_default();
    if !run.status.success() || text.is_empty() {
        return Err(format!("wj build --no-cargo failed (exit {:?}):\n{}{}", run.status.code(), String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr)));
    }
    Ok(text.trim_end().to_string() + "\n")
}

/// Replace the body of the fence opening on 1-based `fence_line` of `text` with `body`.
pub fn replace_fence_body(text: &str, fence_line: usize, body: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = fence_line; // index of the first body line
    let end = (start..lines.len()).find(|&i| lines[i].trim_start().starts_with("```"))?;
    let mut out: Vec<&str> = lines[..start].to_vec();
    out.extend(body.trim_end_matches('\n').lines());
    out.extend(&lines[end..]);
    Some(out.join("\n") + if text.ends_with('\n') { "\n" } else { "" })
}

/// A short line diff (removed `-`, added `+`).
fn diff(old: &str, new: &str) -> String {
    let (o, n): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let mut out = String::new();
    for l in o.iter().filter(|l| !n.contains(l)) {
        out.push_str(&format!("-{l}\n"));
    }
    for l in n.iter().filter(|l| !o.contains(l)) {
        out.push_str(&format!("+{l}\n"));
    }
    out
}

/// Rewrite every `rust,generated` fence. Returns a report; writes nothing when any program fails to build.
pub fn regen(ctx: &Ctx, wj: &Path) -> Result<String, String> {
    let inv = book::inventory(ctx);
    // Build everything first: no page is touched unless every program builds.
    let mut jobs = Vec::new();
    for s in &inv.samples {
        for c in s.companions.iter().filter(|c| c.kind == "rs") {
            let generated = generate(wj, &s.id, &s.code).map_err(|e| format!("{}:{}: the Windjammer program does not build, nothing was rewritten\n{e}", s.file, s.line))?;
            jobs.push((s.file.clone(), c.line, c.text.clone(), generated));
        }
    }
    let mut report = String::new();
    let mut files: Vec<String> = jobs.iter().map(|j| j.0.clone()).collect();
    files.sort();
    files.dedup();
    for file in files {
        let mut text = ctx.read(&file);
        // Rewrite from the bottom up so earlier line numbers stay valid when a body changes length.
        let mut mine: Vec<&(String, usize, String, String)> = jobs.iter().filter(|j| j.0 == file).collect();
        mine.sort_by_key(|j| std::cmp::Reverse(j.1));
        for (_, line, old, generated) in mine {
            if old.trim_end() == generated.trim_end() {
                continue;
            }
            text = replace_fence_body(&text, *line, generated).ok_or_else(|| format!("{file}:{line}: fence has no closing line"))?;
            report.push_str(&format!("{file}:{line}\n{}\n", diff(old, generated)));
        }
        if std::fs::read_to_string(ctx.root.join(&file)).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(ctx.root.join(&file), text).map_err(|e| e.to_string())?;
        }
    }
    Ok(if report.is_empty() { "every rust,generated fence already matches the compiler\n".to_string() } else { report })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fence_body_is_replaced_and_the_rest_kept() {
        let t = "intro\n\n```rust,generated\nold 1\nold 2\n```\n\nafter\n";
        assert_eq!(replace_fence_body(t, 3, "new\n").unwrap(), "intro\n\n```rust,generated\nnew\n```\n\nafter\n");
    }

    #[test]
    fn the_diff_shows_removed_and_added_lines() {
        assert_eq!(diff("a\nb\n", "a\nc\n"), "-b\n+c\n");
    }
}
