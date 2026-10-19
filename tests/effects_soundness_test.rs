// consumes: tests/spec/**/*.wj, tests/effects/corpus/*.wj, src/ir/effect_facts
//! Soundness of the effect pass (ADR-0021 stage E1 gate): for every runnable spec example, each effect the
//! interpreter performs (observed at the capability gate) is in the static effect set of `main`.
//! Reported, not gated: precision (how often the static set names an effect the run never exercised).
//!
//! `console` is static-only in E1 (the interpreter's `println` intrinsic is not a gated builtin), so it is
//! excluded from the dynamic side.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use windjammer::interpreter::caps::{CapGate, CapMode};
use windjammer::interpreter::runner::with_big_stack;
use windjammer::interpreter::Interpreter;
use windjammer::ir::effect_facts::{analyze, EffectKind, Grant};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut es: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    es.sort();
    for p in es {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
}

/// (dynamic kinds, static kinds of main) or None when the example is not a plain runnable program.
fn measure(path: &Path) -> Option<(BTreeSet<EffectKind>, BTreeSet<EffectKind>)> {
    let src = std::fs::read_to_string(path).ok()?;
    let header: Vec<&str> = src.lines().take_while(|l| l.starts_with("//") || l.trim().is_empty()).collect();
    let has = |k: &str| header.iter().any(|l| l.trim_start_matches("//").trim_start().starts_with(k));
    let mode_ok = !header.iter().any(|l| l.contains("mode:") && !l.contains("mode: run") && !l.contains("mode: panic") && !l.contains("mode: simulate"));
    if has("xfail:") || has("manifest:") || !mode_ok {
        return None;
    }
    let name = path.display().to_string();
    with_big_stack(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut lexer = windjammer::lexer::Lexer::new(&src);
            let tokens = lexer.tokenize_with_locations();
            let parser: &'static mut windjammer::parser::Parser =
                Box::leak(Box::new(windjammer::parser::Parser::new_with_source(tokens, name.clone(), src.clone())));
            let program: &'static _ = Box::leak(Box::new(parser.parse().ok()?));
            let files = [windjammer::resolve::ParsedFile::new(vec![], name.clone().into(), program)];
            let r = windjammer::resolve::resolve_program(&files);
            let facts = analyze(&r);
            let main = facts.fns.values().find(|f| f.path == "main")?;
            let static_kinds = main.kinds();
            let mut it = Interpreter::new_capturing();
            it.set_deterministic_time(true);
            it.set_caps(Some(CapGate::new(Grant::unrestricted(&std::env::current_dir().unwrap()), CapMode::Enforce)));
            let _ = it.run_checked(program); // panics and exits are fine: we only observe effects
            let dynamic: BTreeSet<EffectKind> = it.caps_seen().into_iter().collect();
            Some((dynamic, static_kinds))
        }))
        .ok()
        .flatten()
    })
}

#[test]
fn dynamic_effects_are_a_subset_of_the_static_effects_of_main() {
    let mut files = Vec::new();
    for dir in ["tests/spec", "tests/effects/corpus"] {
        collect(&Path::new(env!("CARGO_MANIFEST_DIR")).join(dir), &mut files);
    }
    let (mut measured, mut with_effects, mut imprecise) = (0, 0, 0);
    let mut unsound = Vec::new();
    for f in &files {
        let Some((dynamic, stat)) = measure(f) else { continue };
        measured += 1;
        if !dynamic.is_empty() {
            with_effects += 1;
        }
        let missing: Vec<String> = dynamic.iter().filter(|k| !stat.contains(k)).map(|k| k.name()).collect();
        if !missing.is_empty() {
            unsound.push(format!("{}: performed {:?} but the static set is {:?}", f.display(), missing, stat.iter().map(|k| k.name()).collect::<Vec<_>>()));
        }
        let extra: Vec<&EffectKind> = stat.iter().filter(|k| **k != EffectKind::Console && !dynamic.contains(k)).collect();
        if !extra.is_empty() {
            imprecise += 1;
        }
    }
    eprintln!("effects soundness: {measured} programs measured, {with_effects} perform gated effects, {imprecise} have a static effect never exercised");
    assert!(measured >= 100, "only {measured} programs measured");
    assert!(with_effects >= 12, "the corpus must exercise effects (found {with_effects})");
    assert!(unsound.is_empty(), "UNSOUND (dynamic effect missing from the static set):\n{}", unsound.join("\n"));
}
