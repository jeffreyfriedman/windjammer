//! `docs/spec/grammar.ebnf` (the normative grammar of the language specification) cross-checked
//! against the real lexer and parser. Reuses the EBNF reader, vocabulary check and deterministic
//! generator of `ebnf.rs`; adds what the spec grammar needs:
//!
//! * directives in `(*@key value*)` comments before a production: `@ctx T` (embed sentences into
//!   template T at `$$`), `@status parsed|unparsed|informative`, `@lex K` (lexical rule: must lex
//!   to one token whose kind starts with K, `A|B` alternatives), `@nl`, `@depth`, `@reject-keywords`;
//! * a seeded random corpus (`spec_random.rs`) on top of the per-alternative sentences;
//! * `@status unparsed` productions document constructs the parser rejects today: every sentence
//!   must be REJECTED, so a parser change that starts accepting them fails until the spec is updated;
//! * every production must be exercised by some checked production.
//!
//! It proves grammar ⊆ parser, not parser ⊆ grammar; the negative examples under `tests/spec/`
//! police the other direction for important cases.

use super::config::Config;
use super::ebnf::{self, Gen, Node};
use super::spec_random::{Flags, RandGen};
use super::{Ctx, Finding};
use std::collections::BTreeMap;
use windjammer::lexer::Lexer;

pub const PATH: &str = "docs/spec/grammar.ebnf";
const RANDOM_PER_PRODUCTION: usize = 150;
const DEPTH: u32 = 4;
const SEED: u64 = 0x5eed_5eed_1234_5678;

struct Prod {
    ctx: Option<String>,
    status: String,
    lex: Option<String>,
}

fn dir_value<'a>(d: &'a [(String, String)], k: &str) -> Option<&'a str> {
    d.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str())
}

pub fn check_all(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let text = ctx.read(PATH);
    let unlisted = cfg.spec_grammar.unlisted_keywords.clone();
    // generation and parsing recurse deeply: run on a big stack
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(move || check_text(&text, &unlisted))
        .expect("spawn spec-grammar thread")
        .join()
        .unwrap_or_else(|_| vec![Finding::fail("spec-grammar", PATH, 1, "the check panicked")])
}

fn check_text(text: &str, unlisted: &BTreeMap<String, String>) -> Vec<Finding> {
    if text.is_empty() {
        return vec![];
    }
    let f = |m: String| Finding::fail("spec-grammar", PATH, 1, m);
    let list = match ebnf::parse(text) {
        Ok(l) => l,
        Err(e) => return vec![f(format!("grammar does not parse as EBNF: {e}"))],
    };
    let prods: BTreeMap<String, Node> = list.iter().cloned().collect();
    let dirs = ebnf::directives(text);
    let mut out: Vec<Finding> = Vec::new();
    if prods.len() != list.len() {
        out.push(f("a production is defined twice".into()));
    }
    out.extend(ebnf::undefined_refs(&list, &prods).into_iter().map(f));
    if !out.is_empty() {
        return out;
    }
    out.extend(
        ebnf::vocabulary(&list, unlisted, "[spec_grammar] unlisted_keywords")
            .into_iter()
            .map(f),
    );
    let mut flags = BTreeMap::new();
    let mut info = BTreeMap::new();
    for (name, d) in &dirs {
        for (k, _) in d {
            if !["ctx", "status", "lex", "nl", "depth", "reject-keywords"].contains(&k.as_str()) {
                out.push(f(format!("{name}: unknown directive @{k}")));
            }
        }
        flags.insert(
            name.clone(),
            Flags {
                nl: dir_value(d, "nl").is_some(),
                depth: dir_value(d, "depth").is_some(),
                reject_keywords: dir_value(d, "reject-keywords").is_some(),
            },
        );
        let status = dir_value(d, "status").unwrap_or("parsed").to_string();
        if !["parsed", "unparsed", "informative"].contains(&status.as_str()) {
            out.push(f(format!("{name}: bad @status {status}")));
        }
        info.insert(
            name.clone(),
            Prod { ctx: dir_value(d, "ctx").map(str::to_string), status, lex: dir_value(d, "lex").map(str::to_string) },
        );
    }
    let mut det = Gen::new(&prods);
    let mut rnd = RandGen::new(&prods, &flags, SEED);
    for (name, _) in &list {
        let Some(p) = info.get(name) else { continue };
        if let Some(kinds) = &p.lex {
            if let Some(m) = check_lexical(&mut rnd, name, kinds) {
                out.push(f(m));
            }
            continue;
        }
        let Some(tpl) = &p.ctx else { continue };
        let mut sentences: Vec<(String, String)> = det.sentences(name);
        for i in 0..RANDOM_PER_PRODUCTION {
            sentences.push((format!("{name}#rand{i}"), rnd.sentence(name, DEPTH)));
        }
        let (mut accepted, mut rejected) = (0usize, Vec::new());
        for (id, s) in &sentences {
            let program = tpl.replace("$$", s.trim_end());
            match ebnf::parse_program(&program) {
                Ok(()) => accepted += 1,
                Err(e) => rejected.push((program, id.clone(), e)),
            }
        }
        match p.status.as_str() {
            "parsed" if !rejected.is_empty() => {
                rejected.sort_by_key(|(prog, _, _)| prog.len());
                let mut m = format!("`{name}`: {} of {} sentences rejected by the parser; shortest:", rejected.len(), sentences.len());
                for (prog, id, e) in rejected.iter().take(3) {
                    m.push_str(&format!("\n    [{id}] {}\n      {}", e.lines().next().unwrap_or(""), prog.trim().replace('\n', " ")));
                }
                out.push(f(m));
            }
            "unparsed" if accepted > 0 => out.push(f(format!(
                "`{name}` is @status unparsed but the parser accepted {accepted} of its sentences: update the spec"
            ))),
            _ => {}
        }
    }
    // every production is exercised from some checked production
    for (name, p) in &info {
        if p.ctx.is_some() || p.lex.is_some() {
            for _ in 0..RANDOM_PER_PRODUCTION * 4 {
                rnd.sentence(name, DEPTH + 2);
            }
        }
    }
    for (name, _) in &list {
        let informative = info.get(name).is_some_and(|p| p.status == "informative");
        if !informative && !rnd.visited.contains(name) {
            out.push(f(format!("`{name}` is never exercised by a checked production (add @ctx somewhere above it)")));
        }
    }
    out
}

/// A lexical production must lex as exactly one token of the stated kind.
fn check_lexical(rnd: &mut RandGen, name: &str, kinds: &str) -> Option<String> {
    for _ in 0..RANDOM_PER_PRODUCTION {
        let s = rnd.sentence(name, DEPTH).trim().to_string();
        let src = s.clone();
        let toks = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                std::panic::catch_unwind(|| {
                    Lexer::new(&src)
                        .tokenize_with_locations()
                        .into_iter()
                        .map(|t| format!("{:?}", t.token))
                        .filter(|t| t != "Newline" && t != "Eof")
                        .collect::<Vec<_>>()
                })
                .ok()
            })
            .ok()?
            .join()
            .ok()
            .flatten();
        let good = toks.as_ref().is_some_and(|v| v.len() == 1 && kinds.split('|').any(|k| v[0].starts_with(k)));
        if !good {
            return Some(format!("lexical `{name}`: {s:?} lexes as {toks:?}, want one token of kind {kinds}"));
        }
    }
    None
}
