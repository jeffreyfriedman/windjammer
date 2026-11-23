//! Scanner for the heuristic-surface ratchet (`tests/heuristic_surface_test.rs`).
//!
//! It counts three kinds of name-based guessing in the deciding layers of the compiler:
//!
//! * `name_matches`: a string literal that spells a name (identifier, type, method, module or
//!   path fragment) compared with `==` / `!=` or searched with `starts_with` / `ends_with` /
//!   `contains` / `strip_prefix` / `strip_suffix`.
//! * `match_arm_names`: a string literal that spells a name used as a `match` arm pattern
//!   (`"Foo" =>`, `"a" | "b" =>`, `Some("x") =>`, `"x" if ... =>`).
//! * `fallback_legacy`: identifier tokens containing `fallback` or `legacy` (definitions and uses).
//!
//! Literals that spell a language keyword, a primitive type, a std module name, or a documented
//! convention are exempt. Code inside `#[cfg(test)] mod ... { }` blocks and test-only files is
//! not counted. Everything works on text: literals are found by a small lexer that skips
//! comments, char literals and lifetimes.
#![allow(dead_code)]

use std::collections::BTreeSet;

/// One string literal found in source text: char-index span and decoded contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Rust and Windjammer keywords, primitive types and other words that are part of the language.
const LANGUAGE_WORDS: &[&str] = &[
    // Rust keywords
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
    "yield", "_",
    // Windjammer keywords not already listed
    "bound", "defer", "thread", "int", "uint", "float", "string",
    // primitive types
    "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128",
    "usize", "f32", "f64",
];

/// Rust standard library crate and module names.
const RUST_STD_MODULES: &[&str] = &[
    "std", "core", "alloc", "any", "borrow", "boxed", "cell", "char", "clone", "cmp", "collections",
    "convert", "default", "env", "error", "fmt", "fs", "hash", "io", "iter", "marker", "mem", "net", "num",
    "ops", "option", "os", "path", "process", "ptr", "rc", "result", "slice", "string", "sync", "thread",
    "time", "vec",
];

/// The vocabulary a literal may spell without counting as a name guess.
pub struct Exempt {
    words: BTreeSet<String>,
}

impl Exempt {
    /// Language words, Rust std modules, Windjammer std modules (entries of `std/`) and the
    /// documented conventions passed in.
    pub fn new(wj_std_modules: impl IntoIterator<Item = String>, conventions: impl IntoIterator<Item = String>) -> Self {
        let mut words: BTreeSet<String> = LANGUAGE_WORDS.iter().chain(RUST_STD_MODULES).map(|s| s.to_string()).collect();
        words.extend(wj_std_modules);
        words.extend(conventions);
        Exempt { words }
    }

    /// True when every `::`-separated segment of the literal is exempt vocabulary.
    pub fn covers(&self, text: &str) -> bool {
        let core = text.trim_start_matches("::").trim_start_matches('.').trim_end_matches("::");
        let core = core.trim_end_matches(['<', '(']);
        !core.is_empty() && core.split("::").all(|seg| self.words.contains(seg))
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A literal that spells a name: an identifier or `::` path, optionally with a leading `::` or `.`
/// and a trailing `<`, `(` or `::` (prefix searches such as `"Vec<"` or `"::new"`).
pub fn spells_name(text: &str) -> bool {
    let core = text.strip_prefix("::").or_else(|| text.strip_prefix('.')).unwrap_or(text);
    let core = core.strip_suffix("::").or_else(|| core.strip_suffix('<')).or_else(|| core.strip_suffix('(')).unwrap_or(core);
    if core.is_empty() || !core.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    core.split("::").all(|seg| !seg.is_empty() && seg.chars().all(is_ident_char) && !seg.starts_with(|c: char| c.is_ascii_digit()))
}

// ---------------------------------------------------------------------------------------------
// Lexing
// ---------------------------------------------------------------------------------------------

/// All string literals (plain and raw) outside comments, in order.
pub fn string_literals(chars: &[char]) -> Vec<Lit> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i = skip_block_comment(chars, i);
        } else if let Some((open, hashes)) = raw_open(chars, i) {
            let (end, text) = read_raw(chars, i + open, hashes);
            out.push(Lit { start: i, end, text });
            i = end;
        } else if c == '"' {
            let (end, text) = read_plain(chars, i + 1);
            out.push(Lit { start: i, end, text });
            i = end;
        } else if c == '\'' {
            i = char_literal_end(chars, i).unwrap_or(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

fn skip_block_comment(chars: &[char], mut i: usize) -> usize {
    let mut depth = 0;
    while i < chars.len() {
        if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            i += 2;
        } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    i
}

fn raw_open(chars: &[char], i: usize) -> Option<(usize, usize)> {
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

fn read_raw(chars: &[char], mut i: usize, hashes: usize) -> (usize, String) {
    let mut text = String::new();
    while i < chars.len() {
        if chars[i] == '"' && (1..=hashes).all(|k| chars.get(i + k) == Some(&'#')) {
            return (i + hashes + 1, text);
        }
        text.push(chars[i]);
        i += 1;
    }
    (i, text)
}

fn read_plain(chars: &[char], mut i: usize) -> (usize, String) {
    let mut text = String::new();
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                text.push('\\');
                if let Some(&n) = chars.get(i + 1) {
                    text.push(n);
                }
                i += 2;
            }
            '"' => return (i + 1, text),
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    (i, text)
}

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

// ---------------------------------------------------------------------------------------------
// Context tests around a literal (on stripped code, where literals and comments are blank)
// ---------------------------------------------------------------------------------------------

const SEARCH_METHODS: &[&str] = &["starts_with(", "ends_with(", "contains(", "strip_prefix(", "strip_suffix("];

/// Non-blank code text immediately before `pos`, up to `n` chars, whitespace removed.
fn code_before(code: &[char], pos: usize, n: usize) -> String {
    let mut rev = Vec::new();
    let mut j = pos;
    while j > 0 && rev.len() < n {
        j -= 1;
        if !code[j].is_whitespace() {
            rev.push(code[j]);
        }
    }
    rev.into_iter().rev().collect()
}

/// Non-blank code text from `pos`, up to `n` chars, whitespace removed.
fn code_after(code: &[char], pos: usize, n: usize) -> String {
    code[pos.min(code.len())..].iter().filter(|c| !c.is_whitespace()).take(n).collect()
}

/// `x == "Name"`, `"Name" != x`, `x.starts_with("Name")`, `x.contains(&"Name")` ...
fn is_name_comparison(code: &[char], lit: &Lit) -> bool {
    let before = code_before(code, lit.start, 16);
    let before = before.strip_suffix('&').unwrap_or(&before);
    let after = code_after(code, lit.end, 2);
    let compared = before.ends_with("==") || before.ends_with("!=") || after == "==" || after == "!=";
    let searched = SEARCH_METHODS.iter().any(|m| before.ends_with(&format!(".{m}")));
    compared || searched
}

/// The literal is a pattern in a match arm: the run of `"a" | "b"` (optionally inside `Some(..)`)
/// that it belongs to ends in `=>` or an `if` guard.
fn is_match_arm_pattern(code: &[char], lits: &[Lit], idx: usize) -> bool {
    let mut k = idx;
    loop {
        let mut j = lits[k].end;
        while j < code.len() && (code[j].is_whitespace() || code[j] == ')') {
            j += 1;
        }
        let rest: String = code[j..(j + 3).min(code.len())].iter().collect();
        if rest.starts_with("=>") || (rest.starts_with("if") && rest[2..].starts_with(|c: char| c.is_whitespace())) {
            return true;
        }
        if !rest.starts_with('|') || rest.starts_with("||") {
            return false;
        }
        // The next literal must follow the `|` with only whitespace between (literal text is blank in `code`).
        match lits.get(k + 1) {
            Some(n) if n.start > j && code[j + 1..n.start].iter().all(|c| c.is_whitespace()) => k += 1,
            _ => return false,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Test-code exclusion
// ---------------------------------------------------------------------------------------------

/// Char ranges of `#[cfg(test)] mod name { ... }` blocks in stripped code.
pub fn test_module_ranges(code: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let marker: Vec<char> = "#[cfg(test)]".chars().collect();
    let mut i = 0;
    while i + marker.len() <= code.len() {
        if code[i..i + marker.len()] != marker[..] {
            i += 1;
            continue;
        }
        let after = i + marker.len();
        let tail: String = code[after..(after + 200).min(code.len())].iter().collect();
        let trimmed = tail.trim_start();
        let is_mod = trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") || trimmed.starts_with("pub(crate) mod ");
        let open = code[after..].iter().position(|&c| c == '{' || c == ';').map(|p| after + p);
        match open {
            Some(o) if is_mod && code[o] == '{' => {
                let close = matching_close(code, o).unwrap_or(code.len());
                out.push((i, close));
                i = close;
            }
            _ => i = after,
        }
    }
    out
}

fn matching_close(code: &[char], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, &c) in code.iter().enumerate().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
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

/// Test-only source files inside a counted directory.
pub fn is_test_file(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    name == "tests.rs" || name.ends_with("_tests.rs") || name.ends_with("_test.rs") || rel.contains("/tests/")
}

// ---------------------------------------------------------------------------------------------
// Counting
// ---------------------------------------------------------------------------------------------

/// Per-file counts for one source file.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FileCounts {
    pub name_matches: u64,
    pub match_arm_names: u64,
    pub fallback_legacy: u64,
}

/// Count the three kinds in `raw` source text (`code` is the same text with comments and literal
/// contents blanked, char for char).
pub fn count_file(raw: &str, code: &str, exempt: &Exempt) -> FileCounts {
    let mut c = FileCounts::default();
    for hit in hits(raw, code, exempt) {
        match hit.kind {
            HitKind::NameMatch => c.name_matches += 1,
            HitKind::MatchArmName => c.match_arm_names += 1,
            HitKind::FallbackLegacy => c.fallback_legacy += 1,
        }
    }
    c
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    NameMatch,
    MatchArmName,
    FallbackLegacy,
}

/// One counted occurrence: kind, 1-based line and the literal or identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub kind: HitKind,
    pub line: usize,
    pub text: String,
}

/// Every counted occurrence in a file (literals first, then fallback/legacy identifiers).
pub fn hits(raw: &str, code: &str, exempt: &Exempt) -> Vec<Hit> {
    let raw: Vec<char> = raw.chars().collect();
    let code: Vec<char> = code.chars().collect();
    let line_at = |p: usize| 1 + raw[..p.min(raw.len())].iter().filter(|&&c| c == '\n').count();
    let skip = test_module_ranges(&code);
    let in_test = |p: usize| skip.iter().any(|&(a, b)| p >= a && p <= b);
    let lits = string_literals(&raw);
    let mut out = Vec::new();
    for (idx, lit) in lits.iter().enumerate() {
        if in_test(lit.start) || !spells_name(&lit.text) || exempt.covers(&lit.text) {
            continue;
        }
        let kind = if is_match_arm_pattern(&code, &lits, idx) {
            HitKind::MatchArmName
        } else if is_name_comparison(&code, lit) {
            HitKind::NameMatch
        } else {
            continue;
        };
        out.push(Hit { kind, line: line_at(lit.start), text: lit.text.clone() });
    }
    for (start, tok) in fallback_legacy_tokens(&code) {
        if !in_test(start) {
            out.push(Hit { kind: HitKind::FallbackLegacy, line: line_at(start), text: tok });
        }
    }
    out
}

fn fallback_legacy_tokens(code: &[char]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < code.len() {
        if !is_ident_char(code[i]) || (i > 0 && is_ident_char(code[i - 1])) {
            i += 1;
            continue;
        }
        let start = i;
        while i < code.len() && is_ident_char(code[i]) {
            i += 1;
        }
        let tok: String = code[start..i].iter().collect();
        let lower = tok.to_ascii_lowercase();
        if lower.contains("fallback") || lower.contains("legacy") {
            out.push((start, tok));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Forms the three counts do not see (docs/design/table-driven-std-surface.md, stage 9)
// ---------------------------------------------------------------------------------------------

/// Name guesses the ratchet does not count yet. Report-only: the std-surface migration measures them at stage 1 and
/// stage 9 enforces them with that baseline.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UncountedForms {
    /// A name literal in a tuple pattern of a match arm: `("insert", 0) =>`, `(Ty::Vec(_), "len") if ..`.
    pub tuple_pattern_names: u64,
    /// A name literal in the pattern list of `matches!(x, "a" | "b")`.
    pub matches_list_names: u64,
}

/// The index of the `)` that closes the `(` at `open` in stripped code.
fn closing_paren(code: &[char], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, &c) in code.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
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

/// The index of the innermost `(` that encloses `pos` and has no `{`/`}`/`;` between it and `pos`.
fn enclosing_paren(code: &[char], pos: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut k = pos;
    while k > 0 {
        k -= 1;
        match code[k] {
            ')' => depth += 1,
            '(' if depth == 0 => return Some(k),
            '(' => depth -= 1,
            '{' | '}' | ';' => return None,
            _ => {}
        }
    }
    None
}

/// Count the forms of [`UncountedForms`] in a file. A literal the three counts already include is not counted twice.
pub fn uncounted_forms(raw: &str, code: &str, exempt: &Exempt) -> UncountedForms {
    let raw: Vec<char> = raw.chars().collect();
    let code: Vec<char> = code.chars().collect();
    let skip = test_module_ranges(&code);
    let in_test = |p: usize| skip.iter().any(|&(a, b)| p >= a && p <= b);
    let lits = string_literals(&raw);
    let mut out = UncountedForms::default();
    for (idx, lit) in lits.iter().enumerate() {
        if in_test(lit.start) || !spells_name(&lit.text) || exempt.covers(&lit.text) {
            continue;
        }
        if is_match_arm_pattern(&code, &lits, idx) || is_name_comparison(&code, lit) {
            continue; // counted already
        }
        let Some(open) = enclosing_paren(&code, lit.start) else { continue };
        let Some(close) = closing_paren(&code, open) else { continue };
        // `matches!(x, "a" | "b")`: the literal is after the macro's first top-level comma
        if code_before(&code, open, 8).ends_with("matches!") {
            let comma = (open + 1..close).find(|&k| code[k] == ',' && enclosing_paren(&code, k) == Some(open));
            if comma.is_some_and(|c| lit.start > c) {
                out.matches_list_names += 1;
            }
            continue;
        }
        // a tuple pattern: the parenthesised group is followed by `=>` or an `if` guard (or `|` of another pattern)
        let mut j = close + 1;
        while j < code.len() && code[j].is_whitespace() {
            j += 1;
        }
        let rest: String = code[j..(j + 3).min(code.len())].iter().collect();
        let arm = rest.starts_with("=>") || (rest.starts_with("if") && rest[2..].starts_with(|c: char| c.is_whitespace())) || (rest.starts_with('|') && !rest.starts_with("||"));
        let tuple = (open + 1..lit.start).any(|k| code[k] == ',') || (lit.end..close).any(|k| code[k] == ',');
        if arm && tuple {
            out.tuple_pattern_names += 1;
        }
    }
    out
}
