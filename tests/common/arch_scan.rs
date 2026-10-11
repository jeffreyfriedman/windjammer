//! Source scanner used by the architecture fitness tests (`tests/architecture.rs`).
//!
//! Everything here works on plain text: `strip_code` blanks comments and string/char literals
//! (preserving newlines so line numbers stay valid) so later passes can count tokens and match
//! braces without being fooled by text inside literals.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One scanned `.rs` file.
pub struct SrcFile {
    /// Path relative to the repo root, forward slashes (e.g. `src/analyzer/mod.rs`).
    pub rel: String,
    pub raw: String,
    /// `raw` with comments and literal contents blanked.
    pub code: String,
}

/// A function found by `functions`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FnSpan {
    pub name: String,
    /// 1-based line of the `fn` keyword.
    pub start_line: usize,
    /// Lines from the `fn` keyword through the closing brace, inclusive.
    pub len: usize,
}

// ---------------------------------------------------------------------------------------------
// Comment / literal stripping
// ---------------------------------------------------------------------------------------------

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn blank(c: char) -> char {
    if c == '\n' {
        '\n'
    } else {
        ' '
    }
}

/// Length (in chars) of a raw-string opener at `i` (`r"`, `r#"`, `br##"` ...), with hash count.
fn raw_string_open(chars: &[char], i: usize) -> Option<(usize, usize)> {
    if i > 0 && is_ident_char(chars[i - 1]) {
        return None;
    }
    let mut j = i;
    if chars.get(j) == Some(&'b') {
        j += 1;
    }
    if chars.get(j) != Some(&'r') {
        return None;
    }
    j += 1;
    let mut hashes = 0;
    while chars.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    (chars.get(j) == Some(&'"')).then_some((j + 1 - i, hashes))
}

/// Index just past a char literal starting at `i` (which holds `'`), or `None` for a lifetime.
fn char_literal_end(chars: &[char], i: usize) -> Option<usize> {
    match chars.get(i + 1)? {
        '\\' => {
            let mut j = i + 2;
            while j < chars.len() && chars[j] != '\'' && j < i + 12 {
                j += 1;
            }
            (chars.get(j) == Some(&'\'')).then_some(j + 1)
        }
        _ => (chars.get(i + 2) == Some(&'\'')).then_some(i + 3),
    }
}

/// Replace comments and the contents of string/char literals with spaces, keeping newlines.
pub fn strip_code(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i = skip_block_comment(&chars, i, &mut out);
        } else if let Some((open_len, hashes)) = raw_string_open(&chars, i) {
            i = skip_raw_string(&chars, i, open_len, hashes, &mut out);
        } else if c == '"' {
            i = skip_string(&chars, i, &mut out);
        } else if c == '\'' {
            match char_literal_end(&chars, i) {
                Some(end) => {
                    out.extend(std::iter::repeat_n(' ', end - i));
                    i = end;
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn skip_block_comment(chars: &[char], mut i: usize, out: &mut String) -> usize {
    let mut depth = 0;
    while i < chars.len() {
        if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            out.push_str("  ");
            i += 2;
        } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
            depth -= 1;
            out.push_str("  ");
            i += 2;
            if depth == 0 {
                break;
            }
        } else {
            out.push(blank(chars[i]));
            i += 1;
        }
    }
    i
}

fn skip_raw_string(chars: &[char], mut i: usize, open_len: usize, hashes: usize, out: &mut String) -> usize {
    out.extend(std::iter::repeat_n(' ', open_len));
    i += open_len;
    while i < chars.len() {
        if chars[i] == '"' && (1..=hashes).all(|k| chars.get(i + k) == Some(&'#')) {
            out.extend(std::iter::repeat_n(' ', hashes + 1));
            return i + hashes + 1;
        }
        out.push(blank(chars[i]));
        i += 1;
    }
    i
}

fn skip_string(chars: &[char], mut i: usize, out: &mut String) -> usize {
    out.push(' ');
    i += 1;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                out.push(' ');
                if let Some(&n) = chars.get(i + 1) {
                    out.push(blank(n));
                }
                i += 2;
            }
            '"' => {
                out.push(' ');
                return i + 1;
            }
            c => {
                out.push(blank(c));
                i += 1;
            }
        }
    }
    i
}

// ---------------------------------------------------------------------------------------------
// Function scanner
// ---------------------------------------------------------------------------------------------

fn line_starts(code: &str) -> Vec<usize> {
    let mut v = vec![0];
    v.extend(code.match_indices('\n').map(|(i, _)| i + 1));
    v
}

fn line_of(starts: &[usize], byte: usize) -> usize {
    starts.partition_point(|&s| s <= byte)
}

/// Find the byte index of the `{` opening a body, or `None` if a `;` ends the declaration first.
fn body_open(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut paren = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' => paren += 1,
            b')' | b']' => paren -= 1,
            b'{' if paren <= 0 => return Some(i),
            b';' if paren <= 0 => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

fn matching_close(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// All functions with bodies in already-stripped `code`. Nested functions are reported too.
pub fn functions(code: &str) -> Vec<FnSpan> {
    let bytes = code.as_bytes();
    let starts = line_starts(code);
    let mut out = Vec::new();
    for (idx, _) in code.match_indices("fn") {
        let before_ok = idx == 0 || !is_ident_byte(bytes[idx - 1]);
        let after = idx + 2;
        if !before_ok || bytes.get(after).is_none_or(|b| !b.is_ascii_whitespace()) {
            continue;
        }
        let name_start = after + code[after..].len() - code[after..].trim_start().len();
        let name: String = code[name_start..].chars().take_while(|&c| is_ident_char(c)).collect();
        if name.is_empty() {
            continue;
        }
        let Some(open) = body_open(bytes, name_start + name.len()) else { continue };
        let Some(close) = matching_close(bytes, open) else { continue };
        let start_line = line_of(&starts, idx);
        let end_line = line_of(&starts, close);
        out.push(FnSpan { name, start_line, len: end_line - start_line + 1 });
    }
    out
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// ---------------------------------------------------------------------------------------------
// Module edges
// ---------------------------------------------------------------------------------------------

/// First path segment of every `crate::<seg>` reference in stripped code. A brace group
/// (`crate::{a::X, b::Y}`) yields one entry per top-level item.
pub fn crate_refs(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, _) in code.match_indices("crate::") {
        if idx > 0 && is_ident_byte(code.as_bytes()[idx - 1]) {
            continue;
        }
        let rest = &code[idx + "crate::".len()..];
        if let Some(group) = rest.strip_prefix('{') {
            out.extend(group_items(group));
        } else {
            let seg: String = rest.chars().take_while(|&c| is_ident_char(c)).collect();
            if !seg.is_empty() {
                out.push(seg);
            }
        }
    }
    out
}

fn group_items(group: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in group.chars() {
        match c {
            '{' => {
                depth += 1;
                cur.push(c);
            }
            '}' if depth == 0 => break,
            '}' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                items.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    items.push(cur);
    items
        .iter()
        .map(|i| i.trim().chars().take_while(|&c| is_ident_char(c)).collect::<String>())
        .filter(|s| !s.is_empty() && s != "self")
        .collect()
}

/// Top-level `src/` module of a repo-relative path (`src/a/b.rs` -> `a`, `src/a.rs` -> `a`).
pub fn top_module(rel: &str) -> Option<String> {
    let rest = rel.strip_prefix("src/")?;
    let first = rest.split('/').next()?;
    Some(first.strip_suffix(".rs").unwrap_or(first).to_string())
}

// ---------------------------------------------------------------------------------------------
// Line pattern counters (on stripped code)
// ---------------------------------------------------------------------------------------------

pub fn count_env_reads(code: &str) -> u64 {
    code.matches("env::var").count() as u64
}

pub fn count_thread_local(code: &str) -> u64 {
    code.matches("thread_local!").count() as u64
}

/// Lines declaring process-global state: `static` atomics / `OnceLock` / `LazyLock` / `lazy_static!`.
pub fn count_static_state(code: &str) -> u64 {
    let lines = code.lines().filter(|l| {
        let is_static = l.contains("static ")
            && (l.contains("Atomic") || l.contains("OnceLock") || l.contains("LazyLock"));
        is_static || l.contains("lazy_static!")
    });
    lines.count() as u64
}

pub fn count_allow_dead_code(code: &str) -> u64 {
    code.matches("allow(dead_code)").count() as u64
}

pub fn count_crate_level_allow_dead_code(code: &str) -> u64 {
    code.matches("#![allow(dead_code)]").count() as u64
}

pub fn count_get_signature(code: &str) -> u64 {
    code.matches("get_signature(").count() as u64
}

// ---------------------------------------------------------------------------------------------
// File loading and ratchet comparison
// ---------------------------------------------------------------------------------------------

pub fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Load and strip every `.rs` file under `<root>/src`, sorted by path.
pub fn load_src_files(root: &Path) -> Vec<SrcFile> {
    let mut paths = Vec::new();
    collect_rs(&root.join("src"), &mut paths);
    paths.sort();
    paths
        .into_iter()
        .filter_map(|p| {
            let raw = std::fs::read_to_string(&p).ok()?;
            let rel = p.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/");
            let code = strip_code(&raw);
            Some(SrcFile { rel, raw, code })
        })
        .collect()
}

pub type Counts = BTreeMap<String, u64>;

/// Per-file counts of a pattern, omitting zeros.
pub fn per_file(files: &[SrcFile], f: impl Fn(&SrcFile) -> u64) -> Counts {
    files.iter().map(|s| (s.rel.clone(), f(s))).filter(|(_, n)| *n > 0).collect()
}

/// Result of comparing measured counts with an allow-list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RatchetDiff {
    /// `(key, measured, allowed)` where measured > allowed (allowed 0 for new keys).
    pub regressions: Vec<(String, u64, u64)>,
    /// `(key, measured, allowed)` where measured < allowed: the allow-list must shrink.
    pub stale: Vec<(String, u64, u64)>,
}

pub fn compare(measured: &Counts, allowed: &Counts) -> RatchetDiff {
    let mut d = RatchetDiff::default();
    for (k, &m) in measured {
        let a = allowed.get(k).copied().unwrap_or(0);
        if m > a {
            d.regressions.push((k.clone(), m, a));
        } else if m < a {
            d.stale.push((k.clone(), m, a));
        }
    }
    for (k, &a) in allowed {
        if !measured.contains_key(k) {
            d.stale.push((k.clone(), 0, a));
        }
    }
    d
}

pub fn format_diff(kind: &str, list: &[(String, u64, u64)]) -> String {
    list.iter()
        .map(|(k, m, a)| format!("  {kind}: {k}: measured {m}, allowed {a}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fn_lens(src: &str) -> Vec<(String, usize)> {
        functions(&strip_code(src)).into_iter().map(|f| (f.name, f.len)).collect()
    }

    #[test]
    fn strip_blanks_line_comments_and_keeps_newlines() {
        let s = strip_code("a // fn x() {\nb");
        assert_eq!(s.lines().count(), 2);
        assert!(!s.contains("fn"));
    }

    #[test]
    fn strip_handles_nested_block_comments() {
        let s = strip_code("x /* a /* b */ c { */ y");
        assert!(!s.contains('{'));
        assert!(s.contains('x') && s.contains('y'));
    }

    #[test]
    fn strip_blanks_strings_with_escapes_and_braces() {
        let s = strip_code(r#"let a = "}\"{"; b"#);
        assert!(!s.contains('{') && !s.contains('}'));
        assert!(s.ends_with('b'));
    }

    #[test]
    fn strip_handles_raw_strings() {
        let s = strip_code("let a = r#\"} \" {\"#; z");
        assert!(!s.contains('{') && !s.contains('}'));
        assert!(s.ends_with('z'));
    }

    #[test]
    fn strip_distinguishes_char_literals_from_lifetimes() {
        let s = strip_code("fn f<'a>(x: &'a str) { let c = '{'; let d = '\\''; }");
        assert_eq!(s.matches('{').count(), 1);
        assert!(s.contains("'a"));
    }

    #[test]
    fn functions_measures_simple_body() {
        let src = "fn a() {\n  1;\n  2;\n}\n";
        assert_eq!(fn_lens(src), vec![("a".to_string(), 4)]);
    }

    #[test]
    fn functions_ignores_braces_in_strings_and_comments() {
        let src = "fn a() {\n let s = \"}}}\";\n // }\n}\nfn b() {}\n";
        assert_eq!(fn_lens(src), vec![("a".into(), 4), ("b".into(), 1)]);
    }

    #[test]
    fn functions_skips_bodyless_trait_methods_and_fn_pointer_types() {
        let src = "trait T { fn m(&self); }\nfn f(cb: fn(i32) -> i32) -> i32 {\n cb(1)\n}\n";
        assert_eq!(fn_lens(src), vec![("f".into(), 3)]);
    }

    #[test]
    fn functions_handles_where_clauses_and_nested_fns() {
        let src = "fn outer<T>(x: T) -> T\nwhere\n    T: Clone,\n{\n    fn inner() {\n    }\n    x\n}\n";
        let lens = fn_lens(src);
        assert!(lens.contains(&("outer".into(), 8)));
        assert!(lens.contains(&("inner".into(), 2)));
    }

    #[test]
    fn functions_not_confused_by_identifiers_containing_fn() {
        assert!(fn_lens("let confn = 1; fnord(); Fn(x) {}").is_empty());
    }

    #[test]
    fn crate_refs_reads_paths_and_brace_groups() {
        let code = strip_code("use crate::a::X;\nuse crate::{b::Y, c, d::{e, f}};\nlet z = crate::g::h();");
        assert_eq!(crate_refs(&code), vec!["a", "b", "c", "d", "g"]);
    }

    #[test]
    fn crate_refs_ignores_comments_and_strings() {
        let code = strip_code("// use crate::a::X;\nlet s = \"crate::b\";");
        assert!(crate_refs(&code).is_empty());
    }

    #[test]
    fn top_module_handles_dirs_and_files() {
        assert_eq!(top_module("src/analyzer/mod.rs").as_deref(), Some("analyzer"));
        assert_eq!(top_module("src/config.rs").as_deref(), Some("config"));
        assert_eq!(top_module("tests/x.rs"), None);
    }

    #[test]
    fn static_state_counts_only_global_statics() {
        let code = "static A: AtomicBool = x;\nstatic B: i32 = 1;\nstatic R: OnceLock<u8> = y;\n";
        assert_eq!(count_static_state(code), 2);
    }

    #[test]
    fn compare_reports_regressions_and_stale() {
        let m: Counts = [("a".into(), 3), ("b".into(), 1), ("n".into(), 1)].into();
        let a: Counts = [("a".into(), 2), ("b".into(), 2), ("gone".into(), 4)].into();
        let d = compare(&m, &a);
        assert_eq!(d.regressions, vec![("a".into(), 3, 2), ("n".into(), 1, 0)]);
        assert_eq!(d.stale, vec![("b".into(), 1, 2), ("gone".into(), 0, 4)]);
    }
}
