//! SPEC grammar conformance: `docs/spec/grammar.ebnf` is tested against the real lexer and parser.
//!
//! For every production carrying an `@ctx` directive, a deterministic corpus of sentences is
//! generated from the EBNF, wrapped in its context, and parsed with the real parser. Productions
//! with `@status parsed` must accept every sentence; `@status unparsed` productions document
//! constructs the parser rejects today and must reject every sentence (so a parser change that
//! starts accepting them fails here and forces the spec to be updated). Lexical productions
//! (`@lex`) must tokenize to exactly one token of the stated kind.
//!
//! Limits: this checks grammar ⊆ parser (everything the grammar derives parses). It does not prove
//! parser ⊆ grammar; over-acceptance is covered only by the negative examples under `tests/spec/`.

#![allow(clippy::duplicate_mod)]

#[path = "common/spec_ebnf/mod.rs"]
mod spec_ebnf;

use spec_ebnf::generate::Gen;
use spec_ebnf::{parse::parse_grammar, Grammar};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use windjammer::interpreter::runner::with_big_stack;

const PER_PRODUCTION: usize = 250;
const DEPTH: u32 = 5;

fn load() -> Grammar {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/spec/grammar.ebnf");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse_grammar(&src).unwrap_or_else(|e| panic!("grammar.ebnf: {e}"))
}

fn parses(src: &str) -> Result<(), String> {
    let src = src.to_string();
    match catch_unwind(AssertUnwindSafe(|| {
        let mut lexer = windjammer::lexer::Lexer::new(&src);
        let tokens = lexer.tokenize_with_locations();
        let parser: &'static mut windjammer::parser::Parser = Box::leak(Box::new(
            windjammer::parser::Parser::new_with_source(tokens, "spec.wj".to_string(), src.clone()),
        ));
        parser.parse().map(|_| ()).map_err(|e| e.to_string())
    })) {
        Ok(r) => r,
        Err(_) => Err("lexer/parser panicked".to_string()),
    }
}

#[test]
fn grammar_file_is_well_formed() {
    let g = load();
    assert!(g.prods.contains_key("program"), "grammar needs a `program` production");
    for (n, p) in &g.prods {
        assert!(
            matches!(p.status.as_str(), "parsed" | "unparsed" | "informative"),
            "{n}: bad @status {}",
            p.status
        );
    }
}

#[test]
fn syntactic_productions_agree_with_the_real_parser() {
    with_big_stack(|| {
        let g = load();
        let mut gen = Gen::new(&g, 0x5eed_5eed_1234_5678);
        let mut report = String::new();
        let mut tested = 0;
        for name in &g.order {
            let p = &g.prods[name];
            let Some(ctx) = &p.ctx else { continue };
            if p.lex.is_some() {
                continue;
            }
            if std::env::var("SPEC_ONLY").is_ok_and(|o| &o != name) {
                continue;
            }
            let (mut ok, mut bad) = (0, Vec::new());
            for _ in 0..PER_PRODUCTION {
                let s = gen.sentence(name, DEPTH);
                let program = ctx.replace("$$", &s);
                match parses(&program) {
                    Ok(()) => ok += 1,
                    Err(e) => bad.push((program, e)),
                }
            }
            tested += 1;
            match p.status.as_str() {
                "parsed" if !bad.is_empty() => {
                    report.push_str(&format!("\n{name}: {} of {PER_PRODUCTION} generated sentences rejected, e.g.\n", bad.len()));
                    // shortest counter-examples first: they are the easiest to read
                    bad.sort_by_key(|(prog, _)| prog.len());
                    let show = std::env::var("SPEC_SHOW").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
                    for (prog, e) in bad.iter().take(show) {
                        report.push_str(&format!("  {e}\n  ---\n{}\n  ---\n", prog.trim()));
                    }
                }
                "unparsed" if ok > 0 => report.push_str(&format!(
                    "\n{name}: marked @status unparsed but the parser accepted {ok} sentences; update the spec\n"
                )),
                _ => {}
            }
        }
        assert!(tested >= 5 || std::env::var("SPEC_ONLY").is_ok(), "too few @ctx productions tested: {tested}");
        assert!(report.is_empty(), "grammar/parser disagreement:{report}");
    });
}

#[test]
fn lexical_productions_produce_a_single_token_of_the_stated_kind() {
    with_big_stack(|| {
        let g = load();
        let mut gen = Gen::new(&g, 0xfeed_beef_0000_0001);
        let mut report = String::new();
        let mut checked = 0;
        for name in &g.order {
            let p = &g.prods[name];
            let Some(kinds) = &p.lex else { continue };
            for _ in 0..PER_PRODUCTION {
                let s = gen.sentence(name, DEPTH).trim().to_string();
                if std::env::var("SPEC_DUMP").is_ok() {
                    // debugging aid: the last line printed before a crash is the offending lexeme
                    eprintln!("LEX {name}: {s:?}");
                }
                let toks = catch_unwind(AssertUnwindSafe(|| {
                    let mut lx = windjammer::lexer::Lexer::new(&s);
                    lx.tokenize_with_locations()
                        .into_iter()
                        .map(|t| format!("{:?}", t.token))
                        .filter(|t| t != "Newline" && t != "Eof")
                        .collect::<Vec<_>>()
                }));
                let good = match &toks {
                    Ok(v) => v.len() == 1 && kinds.split('|').any(|k| v[0].starts_with(k)),
                    Err(_) => false,
                };
                if !good {
                    report.push_str(&format!("{name}: {s:?} lexed as {toks:?}, want one of {kinds}\n"));
                    break;
                }
            }
            checked += 1;
        }
        assert!(checked >= 4, "too few @lex productions: {checked}");
        assert!(report.is_empty(), "lexer disagreement:\n{report}");
    });
}

#[test]
fn every_production_is_reachable_from_a_tested_one() {
    with_big_stack(|| {
        let g = load();
        let mut gen = Gen::new(&g, 0x1234_abcd_0000_0007);
        for name in &g.order {
            let p = &g.prods[name];
            if p.ctx.is_some() || p.lex.is_some() {
                for _ in 0..PER_PRODUCTION * 8 {
                    gen.sentence(name, DEPTH + 2);
                }
            }
        }
        let missing: Vec<_> = g
            .order
            .iter()
            .filter(|n| g.prods[*n].status != "informative" && !gen.visited.contains(*n))
            .collect();
        assert!(missing.is_empty(), "productions never exercised by any tested production: {missing:?}");
    });
}
