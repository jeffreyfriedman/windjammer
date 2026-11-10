//! Generated numbers for the "Why Windjammer" showcase pages: line counts of programs that are executed side by side, and
//! file sizes. Nothing here is typed by hand; the regions are rewritten by `scripts/docs_gen.sh`.

use super::{book, util, Ctx};

fn loc(text: &str) -> usize {
    text.lines().filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with("//")).count()
}

/// The nearest heading above `line` (1-based) in `text`.
fn heading_above(text: &str, line: usize) -> String {
    text.lines().take(line.saturating_sub(1)).filter(|l| l.starts_with('#')).last().map(|l| l.trim_start_matches('#').trim().to_string()).unwrap_or_default()
}

/// (task, windjammer, rust, go, typescript) code lines per program with companions; a missing companion counts 0.
pub fn loc_rows(ctx: &Ctx, page: &str) -> Vec<(String, usize, usize, usize, usize)> {
    let file = format!("{}/{page}", book::SRC);
    let text = ctx.read(&file);
    let inv = book::inventory(ctx);
    inv.samples
        .iter()
        .filter(|s| s.file == file && !s.companions.is_empty())
        .map(|s| {
            let n = |kind: &str| s.companions.iter().find(|c| c.kind == kind).map(|c| loc(&c.text)).unwrap_or(0);
            (heading_above(&text, s.line), loc(&s.code), n("rs"), n("go"), n("ts"))
        })
        .collect()
}

/// A table of code lines for every program in `page` (relative to the book's `src`) that has companion fences: the
/// Windjammer program, the Rust the compiler generates for it, and hand-written Go and TypeScript that print the same output.
pub fn loc_table(ctx: &Ctx, page: &str) -> Result<String, String> {
    let file = format!("{}/{page}", book::SRC);
    let text = ctx.read(&file);
    if text.is_empty() {
        return Err(format!("showcase_loc: no page {file}"));
    }
    let inv = book::inventory(ctx);
    let mut rows = Vec::new();
    for s in inv.samples.iter().filter(|s| s.file == file && !s.companions.is_empty()) {
        let count = |kind: &str| s.companions.iter().find(|c| c.kind == kind).map(|c| loc(&c.text).to_string()).unwrap_or_else(|| "-".to_string());
        rows.push(vec![heading_above(&text, s.line), loc(&s.code).to_string(), count("rs"), count("go"), count("ts")]);
    }
    Ok(util::table(&["Task", "Windjammer", "Rust (generated)", "Go", "TypeScript"], &rows))
}

/// `N bytes` for a repository file.
pub fn file_bytes(ctx: &Ctx, path: &str) -> Result<String, String> {
    let meta = std::fs::metadata(ctx.root.join(path)).map_err(|e| format!("file_bytes: {path}: {e}"))?;
    Ok(format!("{} bytes", meta.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loc_ignores_blank_and_comment_lines() {
        assert_eq!(loc("// expect: 1\nfn main() {\n\n    println(\"x\")\n}\n"), 3);
    }

    #[test]
    fn the_heading_is_the_nearest_one_above() {
        assert_eq!(heading_above("# A\n\n## B task\n\ntext\n```wj\n", 6), "B task");
    }
}
