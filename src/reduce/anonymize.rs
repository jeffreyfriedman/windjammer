//! Anonymizer: rename every user identifier, string literal and file path component to neutral
//! names (`T1`, `f1`, `field1`, `m1`, `v1`, `s1` ...), consistently across the project; comments and
//! doc comments are dropped. A [`Plan`] lists the renamable items; [`apply`] renames the enabled
//! subset, so the caller can bisect to the largest subset that still reproduces the bug.

use super::names::{Names, User};
use super::project::Project;
use super::toks::{render, scan};
use crate::formatter::{Tok, TokKind};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Key {
    Word(String),
    Str(String),
}

#[derive(Default)]
pub struct Plan {
    pub items: Vec<(Key, String)>,
    names: Names,
    user: User,
}

impl Plan {
    pub(crate) fn index(&self) -> BTreeMap<&Key, usize> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, (k, _))| (k, i))
            .collect()
    }
}

/// Map a user word to its replacement.
type WordFn<'a> = dyn FnMut(&str) -> String + 'a;

struct Ctx<'a> {
    names: &'a Names,
    user: &'a User,
}

fn path_parts(rel: &str) -> Vec<String> {
    rel.split('/').map(str::to_string).collect()
}

fn split_stem(part: &str) -> (&str, &str) {
    match part.strip_suffix(".wj") {
        Some(stem) => (stem, ".wj"),
        None => (part, ""),
    }
}

fn role_prefix(toks: &[Tok], i: usize) -> &'static str {
    let w = &toks[i].text;
    let prev = if i > 0 { toks[i - 1].text.as_str() } else { "" };
    let next = toks.get(i + 1).map(|t| t.text.as_str()).unwrap_or("");
    match prev {
        "fn" => return "f",
        "mod" => return "m",
        "struct" | "enum" | "trait" | "type" | "impl" => return "T",
        "const" | "static" => return "C",
        _ => {}
    }
    if w.len() > 1
        && w.chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
    {
        "C"
    } else if w.starts_with(|c: char| c.is_uppercase()) {
        "T"
    } else if next == ":" && matches!(prev, "{" | "," | "pub") {
        "field"
    } else {
        "v"
    }
}

/// Visit the words of a string literal's interpolations; returns the string with literal runs
/// replaced by `lit` (or untouched when `lit` is `None`) and words mapped through `word`.
fn rewrite_string(text: &str, ctx: &Ctx, word: &mut WordFn, lit: Option<&str>) -> String {
    if text.starts_with('r') {
        return match lit {
            Some(l) => format!("r\"{l}\""),
            None => text.to_string(),
        };
    }
    let body: Vec<char> = text
        .trim_start_matches('"')
        .trim_end_matches('"')
        .chars()
        .collect();
    let mut out = String::from("\"");
    let mut run = String::new();
    let flush = |out: &mut String, run: &mut String| {
        if !run.is_empty() {
            match lit {
                Some(l) => out.push_str(l),
                None => out.push_str(run),
            }
            run.clear();
        }
    };
    let mut i = 0;
    while i < body.len() {
        let c = body[i];
        if c == '\\' && i + 1 < body.len() {
            run.push(c);
            run.push(body[i + 1]);
            i += 2;
        } else if c == '$' && body.get(i + 1) == Some(&'$') {
            flush(&mut out, &mut run);
            out.push_str("$$");
            i += 2;
        } else if c == '$' && body.get(i + 1) == Some(&'{') {
            flush(&mut out, &mut run);
            let (mut depth, mut j) = (1, i + 2);
            while j < body.len() && depth > 0 {
                match body[j] {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            let inner: String = body[i + 2..j.saturating_sub(1).max(i + 2)].iter().collect();
            out.push_str("${");
            out.push_str(&rewrite_expr(&inner, ctx, word));
            out.push('}');
            i = j;
        } else if c == '{' {
            // `{}`, `{0}`, `{:?}` placeholders are formatting, not text
            let end = body[i..].iter().position(|&x| x == '}').map(|p| i + p);
            match end {
                Some(e)
                    if body[i + 1..e].iter().all(|&x| {
                        x.is_ascii_digit()
                            || matches!(
                                x,
                                ':' | '?'
                                    | '.'
                                    | '#'
                                    | '<'
                                    | '>'
                                    | '^'
                                    | '+'
                                    | '-'
                                    | 'x'
                                    | 'X'
                                    | 'e'
                                    | 'b'
                                    | 'o'
                            )
                    }) =>
                {
                    flush(&mut out, &mut run);
                    out.extend(&body[i..=e]);
                    i = e + 1;
                }
                _ => {
                    run.push(c);
                    i += 1;
                }
            }
        } else {
            run.push(c);
            i += 1;
        }
    }
    flush(&mut out, &mut run);
    out.push('"');
    out
}

fn rewrite_expr(src: &str, ctx: &Ctx, word: &mut WordFn) -> String {
    let (expr, spec) = split_format_spec(src);
    let Some(toks) = scan(expr) else {
        return src.to_string();
    };
    let mut out = String::new();
    for (i, t) in toks.iter().enumerate() {
        let text = match t.kind {
            TokKind::Word if !ctx.names.protected_at(ctx.user, &toks, i) => word(&t.text),
            TokKind::Str => rewrite_string(&t.text, ctx, word, None),
            _ => t.text.clone(),
        };
        if i > 0 && t.space {
            out.push(' ');
        }
        out.push_str(&text);
    }
    out + spec
}

/// `${x:.2}` -> (`x`, `:.2`): the format spec after a top-level single colon stays verbatim.
fn split_format_spec(src: &str) -> (&str, &str) {
    let b = src.as_bytes();
    let (mut depth, mut in_str, mut i) = (0i32, false, 0);
    while i < b.len() {
        match b[i] {
            b'"' => in_str = !in_str,
            b'\\' if in_str => i += 1,
            b'(' | b'[' | b'{' if !in_str => depth += 1,
            b')' | b']' | b'}' if !in_str => depth -= 1,
            b':' if !in_str && depth == 0 => {
                if b.get(i + 1) == Some(&b':') {
                    i += 1;
                } else {
                    return (&src[..i], &src[i..]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    (src, "")
}

fn has_text(s: &str, ctx: &Ctx) -> bool {
    let mut id = |w: &str| w.to_string();
    rewrite_string(s, ctx, &mut id, Some("\u{1}")).contains('\u{1}')
        && s.chars().any(|c| c.is_alphabetic())
}

/// Everything that can be renamed, with deterministic neutral names.
pub fn plan(p: &Project) -> Plan {
    let mut taken: HashSet<String> = HashSet::new();
    let mut parsed: Vec<(&String, Vec<Tok>)> = Vec::new();
    for (k, v) in &p.files {
        if let Some(t) = scan(v) {
            taken.extend(
                t.iter()
                    .filter(|x| x.kind == TokKind::Word)
                    .map(|x| x.text.clone()),
            );
            parsed.push((k, t));
        }
    }
    let all: Vec<Vec<Tok>> = parsed.iter().map(|(_, t)| t.clone()).collect();
    let names = Names::new();
    let user = User::collect(&all, &names);
    let mut plan = Plan {
        items: Vec::new(),
        names,
        user,
    };
    let mut seen: HashSet<Key> = HashSet::new();
    let mut counters: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut fresh = |prefix: &'static str, taken: &mut HashSet<String>| loop {
        let n = counters.entry(prefix).or_insert(0);
        *n += 1;
        let name = format!("{prefix}{n}");
        if taken.insert(name.clone()) {
            return name;
        }
    };
    let mut items: Vec<(Key, String)> = Vec::new();
    {
        let ctx = Ctx {
            names: &plan.names,
            user: &plan.user,
        };
        for (rel, toks) in &parsed {
            for part in path_parts(rel) {
                let (stem, _) = split_stem(&part);
                let reserved = matches!(stem, "main" | "lib" | "mod");
                if !reserved && seen.insert(Key::Word(stem.to_string())) {
                    items.push((Key::Word(stem.to_string()), fresh("m", &mut taken)));
                }
            }
            for (i, t) in toks.iter().enumerate() {
                match t.kind {
                    TokKind::Word if !ctx.names.protected_at(ctx.user, toks, i) => {
                        if seen.insert(Key::Word(t.text.clone())) {
                            let prefix = role_prefix(toks, i);
                            items.push((Key::Word(t.text.clone()), fresh(prefix, &mut taken)));
                        }
                    }
                    TokKind::Str => {
                        let mut words = Vec::new();
                        rewrite_string(
                            &t.text,
                            &ctx,
                            &mut |w| {
                                words.push(w.to_string());
                                w.to_string()
                            },
                            None,
                        );
                        for w in words {
                            if seen.insert(Key::Word(w.clone())) {
                                items.push((Key::Word(w), fresh("v", &mut taken)));
                            }
                        }
                        if has_text(&t.text, &ctx) && seen.insert(Key::Str(t.text.clone())) {
                            items.push((Key::Str(t.text.clone()), fresh("s", &mut taken)));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    plan.items = items;
    plan
}

/// Apply the enabled subset of `plan`. Comments are always dropped.
pub fn apply(p: &Project, plan: &Plan, enabled: &[bool]) -> Project {
    let idx = plan.index();
    let ctx = Ctx {
        names: &plan.names,
        user: &plan.user,
    };
    let word_map = |w: &str| -> String {
        match idx.get(&Key::Word(w.to_string())) {
            Some(&i) if enabled[i] => plan.items[i].1.clone(),
            _ => w.to_string(),
        }
    };
    let mut out = Project::default();
    for (rel, text) in &p.files {
        let new_rel = path_parts(rel)
            .into_iter()
            .map(|part| {
                let (stem, ext) = split_stem(&part);
                format!("{}{}", word_map(stem), ext)
            })
            .collect::<Vec<_>>()
            .join("/");
        let Some(toks) = scan(text) else {
            out.files.insert(new_rel, text.clone());
            continue;
        };
        let mut mapped = toks.clone();
        for (i, t) in mapped.iter_mut().enumerate() {
            match t.kind {
                TokKind::Word if !ctx.names.protected_at(ctx.user, &toks, i) => {
                    t.text = word_map(&t.text)
                }
                TokKind::Str => {
                    let lit = match idx.get(&Key::Str(t.text.clone())) {
                        Some(&i) if enabled[i] => Some(plan.items[i].1.clone()),
                        _ => None,
                    };
                    let mut wm = |w: &str| word_map(w);
                    t.text = rewrite_string(&toks[i].text, &ctx, &mut wm, lit.as_deref());
                }
                _ => {}
            }
        }
        mapped.retain(|t| t.kind != TokKind::Comment);
        out.files.insert(new_rel, render(&mapped));
    }
    out
}

impl Plan {
    /// Items that rename identifiers only (strings keep their text).
    pub fn words_only(&self) -> Vec<bool> {
        self.items
            .iter()
            .map(|(k, _)| matches!(k, Key::Word(_)))
            .collect()
    }
}

/// Words that occur in a declaration position (`fn x`, `struct X`, `let x`, `x: T` ...).
pub fn declared_words(p: &Project) -> HashSet<String> {
    let mut out = HashSet::new();
    for text in p.files.values() {
        let Some(toks) = scan(text) else { continue };
        for (i, t) in toks.iter().enumerate() {
            if t.kind != TokKind::Word {
                continue;
            }
            let prev = if i > 0 { toks[i - 1].text.as_str() } else { "" };
            let next = toks.get(i + 1).map(|x| x.text.as_str()).unwrap_or("");
            let decl_kw = matches!(
                prev,
                "fn" | "struct"
                    | "enum"
                    | "trait"
                    | "type"
                    | "mod"
                    | "const"
                    | "static"
                    | "let"
                    | "mut"
                    | "for"
            );
            if decl_kw || (next == ":" && !matches!(prev, "." | "::")) {
                out.insert(t.text.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proj(files: &[(&str, &str)]) -> Project {
        let mut p = Project::default();
        for (k, v) in files {
            p.files.insert((*k).to_string(), (*v).to_string());
        }
        p
    }

    fn all(plan: &Plan) -> Vec<bool> {
        vec![true; plan.items.len()]
    }

    #[test]
    fn renames_identifiers_strings_paths_and_drops_comments() {
        let p = proj(&[
            ("main.wj", "use billing::charge\n// secret note\nfn main() {\n    let total = charge(\"Acme Corp\")\n    println(\"hi ${total} {}\", total)\n}\n"),
            ("billing.wj", "/// doc about Acme\npub struct Invoice {\n    amount: int,\n}\npub fn charge(who: str) -> int {\n    0\n}\n"),
        ]);
        let plan = plan(&p);
        let q = apply(&p, &plan, &all(&plan));
        let joined = q.files.values().cloned().collect::<Vec<_>>().join("\n")
            + &q.files.keys().cloned().collect::<Vec<_>>().join("\n");
        for secret in [
            "billing",
            "charge",
            "total",
            "Invoice",
            "amount",
            "who",
            "Acme",
            "secret",
            "doc about",
        ] {
            assert!(!joined.contains(secret), "{secret} leaked:\n{joined}");
        }
        assert!(joined.contains("fn main"), "language names survive");
        assert!(joined.contains("println"));
        assert!(q.files.contains_key("main.wj"));
    }

    #[test]
    fn consistent_across_files_and_deterministic() {
        let p = proj(&[
            ("a.wj", "pub fn helper() {\n}\n"),
            ("main.wj", "use a\nfn main() {\n    a::helper()\n}\n"),
        ]);
        let (p1, p2) = (plan(&p), plan(&p));
        assert_eq!(p1.items, p2.items);
        let q = apply(&p, &p1, &all(&p1));
        let f = p1
            .items
            .iter()
            .find(|(k, _)| *k == Key::Word("helper".into()))
            .unwrap()
            .1
            .clone();
        assert!(q.files.values().all(|t| !t.contains("helper")));
        assert_eq!(q.files.values().filter(|t| t.contains(&f)).count(), 2);
    }

    #[test]
    fn disabled_items_stay_and_roles_pick_prefixes() {
        let p = proj(&[("main.wj", "struct Point {\n    x: int,\n}\nfn norm(p: Point) -> int {\n    p.x\n}\nfn main() {\n}\n")]);
        let plan = plan(&p);
        let name = |w: &str| {
            plan.items
                .iter()
                .find(|(k, _)| *k == Key::Word(w.into()))
                .unwrap()
                .1
                .clone()
        };
        assert!(name("Point").starts_with('T'));
        assert!(name("norm").starts_with('f'));
        assert!(name("x").starts_with("field"));
        let mut en = all(&plan);
        let i = plan
            .items
            .iter()
            .position(|(k, _)| *k == Key::Word("norm".into()))
            .unwrap();
        en[i] = false;
        let q = apply(&p, &plan, &en);
        assert!(q.files["main.wj"].contains("fn norm"));
        assert!(!q.files["main.wj"].contains("Point"));
    }

    #[test]
    fn format_specs_in_interpolations_are_kept() {
        assert_eq!(split_format_spec("mean:.1"), ("mean", ":.1"));
        assert_eq!(split_format_spec("a::b(c)"), ("a::b(c)", ""));
        assert_eq!(split_format_spec("f(x: 1)"), ("f(x: 1)", ""));
    }

    #[test]
    fn interpolations_are_renamed_inside_strings() {
        let p = proj(&[("main.wj", "fn main() {\n    let secret_total = 1\n    println(\"total: ${secret_total + 1}\")\n}\n")]);
        let plan = plan(&p);
        let q = apply(&p, &plan, &all(&plan));
        assert!(!q.files["main.wj"].contains("secret_total"));
        assert!(!q.files["main.wj"].contains("total:"));
        assert!(q.files["main.wj"].contains("${"));
    }
}
