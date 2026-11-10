//! "Run" links: a line right after a Windjammer program that opens it in the book's playground (docs/book/playground).
//! The program travels in the URL fragment as base64url, so the link is only true while it equals the program above it:
//! the gate compares them, and `wj-docs book-regen` rewrites them. The playground itself is a static page and needs no
//! server.

use super::{book, Ctx, Finding};

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding, as the playground's `toFragment` writes it.
pub fn encode(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = String::new();
    for chunk in b.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(B64[(n >> 18 & 63) as usize] as char);
        out.push(B64[(n >> 12 & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64[(n >> 6 & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64[(n & 63) as usize] as char);
        }
    }
    out
}

pub fn decode(s: &str) -> Option<String> {
    let mut bits = 0u32;
    let mut n = 0;
    let mut out = Vec::new();
    for c in s.bytes() {
        let v = B64.iter().position(|&x| x == c)? as u32;
        bits = bits << 6 | v;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((bits >> n & 255) as u8);
        }
    }
    String::from_utf8(out).ok()
}

/// `playground/` or `../playground/`: the path from a page to the playground in the built site.
fn prefix(file: &str) -> String {
    let rel = file.trim_start_matches(&format!("{}/", book::SRC));
    "../".repeat(rel.matches('/').count()) + "playground/"
}

fn link_line(file: &str, code: &str) -> String {
    format!("[Run this in the playground]({}index.html#lang=wj&code={})", prefix(file), encode(code))
}

fn is_run_link(line: &str) -> bool {
    line.trim_start().starts_with("[Run") && line.contains("playground/index.html#")
}

/// (1-based line of the link, the line it should be) for every Run link, and findings for links not after a program.
pub fn expected(ctx: &Ctx) -> (Vec<(String, usize, String)>, Vec<Finding>) {
    let inv = book::inventory(ctx);
    let mut want = Vec::new();
    let mut bad = Vec::new();
    for (file, text) in book::pages(ctx) {
        let lines: Vec<&str> = text.lines().collect();
        let mut attached = std::collections::BTreeSet::new();
        for s in inv.samples.iter().filter(|s| s.file == file && !s.test) {
            // closing fence of the program, then the next non-blank line
            let Some(close) = (s.line..lines.len()).find(|&i| lines[i].trim_start().starts_with("```")) else { continue };
            let mut j = close + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() && is_run_link(lines[j]) {
                attached.insert(j);
                want.push((file.clone(), j + 1, link_line(&file, &s.code)));
            }
        }
        for (i, l) in lines.iter().enumerate() {
            if is_run_link(l) && !attached.contains(&i) {
                bad.push(Finding::fail("book", &file, i + 1, "a Run link must follow the Windjammer program it opens (a `wj` fence that runs)"));
            }
        }
    }
    (want, bad)
}

pub fn check(ctx: &Ctx) -> Vec<Finding> {
    let (want, mut out) = expected(ctx);
    for (file, line, expect) in want {
        let have = ctx.read(&file).lines().nth(line - 1).unwrap_or("").to_string();
        if have.trim() != expect {
            out.push(Finding::fail("book", &file, line, "the Run link does not carry the program above it: run `wj-docs book-regen`"));
        }
    }
    out
}

/// Rewrite stale Run links; returns the files changed.
pub fn regen(ctx: &Ctx) -> Result<Vec<String>, String> {
    let (want, bad) = expected(ctx);
    if let Some(b) = bad.first() {
        return Err(b.render());
    }
    let mut changed = Vec::new();
    let mut files: Vec<String> = want.iter().map(|w| w.0.clone()).collect();
    files.sort();
    files.dedup();
    for file in files {
        let text = ctx.read(&file);
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let mut dirty = false;
        for (f, line, expect) in want.iter().filter(|w| w.0 == file) {
            let _ = f;
            if lines[line - 1].trim() != expect {
                lines[line - 1] = expect.clone();
                dirty = true;
            }
        }
        if dirty {
            std::fs::write(ctx.root.join(&file), lines.join("\n") + if text.ends_with('\n') { "\n" } else { "" }).map_err(|e| e.to_string())?;
            changed.push(file);
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_round_trips_like_the_playground() {
        for s in ["", "a", "ab", "abc", "fn main() {\n    println(\"héllo ${1}\")\n}\n"] {
            assert_eq!(decode(&encode(s)).as_deref(), Some(s));
        }
        // the playground writes btoa(bytes) with + / and padding replaced by - _ and removed
        assert_eq!(encode("hi?>"), "aGk_Pg");
    }

    #[test]
    fn pages_reach_the_playground_by_relative_path() {
        assert_eq!(prefix("docs/book/src/tutorial/closures.md"), "../playground/");
        assert_eq!(prefix("docs/book/src/introduction.md"), "playground/");
    }
}
