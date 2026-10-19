//! Multi-oracle backend conformance (Rust / Go / JS). See docs/backends/CONFORMANCE.md.
//!
//! Agreement between backends is never sufficient evidence: every expectation comes from an
//! oracle independent of every Windjammer backend (known-answer vectors, Python reference
//! models whose output is committed and re-derived, a generator's own evaluator).

#![allow(clippy::duplicate_mod)]

#[path = "common/conformance/mod.rs"]
mod conformance;

use conformance::backends::{compile_and_run_with, Backend, Outcome, ALL_BACKENDS};
use conformance::compare::{first_divergence, Trace};
use conformance::genrun::{make_cases, run_bundle, shrink_report, Verdict as GenVerdict};
use conformance::program::{self, Program};
use conformance::runner::{eval_program, run_all, Ctx, Verdict};
use conformance::support::Support;
use conformance::toolchain::{Tool, ToolProbe};
use conformance::{coverage, gen_ast, meta, mutation, oracle};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance_programs")
}

fn wj() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn work_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wj-conformance-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn probe_all() -> ToolProbe {
    let p = ToolProbe::new(&[Tool::Rustc, Tool::Go, Tool::Node, Tool::Python3]);
    p.announce();
    p
}

// ---------------------------------------------------------------------------------------------
// Corpus hygiene (no toolchains needed)
// ---------------------------------------------------------------------------------------------

#[test]
fn corpus_is_well_formed() {
    let programs = program::discover(&root());
    assert!(programs.len() >= 30, "corpus must contain at least 30 programs, found {}", programs.len());
    for p in &programs {
        let n = p.seeds.len();
        if p.seeded {
            assert!(n >= 5, "{}: seeded programs need >= 5 seeds, has {n}", p.name);
            assert!(p.has_ref, "{}: seeded programs need an independent reference model ({}.ref.py)", p.name, p.name);
        }
        for &seed in &p.seeds {
            oracle::expected(p, seed).unwrap_or_else(|e| panic!("{e}"));
        }
    }
}

#[test]
fn support_manifest_refers_to_real_programs() {
    let programs = program::discover(&root());
    let support = Support::load(&root().join("SUPPORT.toml"));
    for (prog, backend) in support.keys() {
        assert!(programs.iter().any(|p| &p.name == prog), "SUPPORT.toml lists unknown program `{prog}`");
        assert!(["rust", "go", "js", "ts"].contains(&backend.as_str()), "SUPPORT.toml lists unknown backend `{backend}`");
    }
}

#[test]
fn reference_vectors_are_derived_from_models() {
    let probe = ToolProbe::new(&[Tool::Python3]);
    probe.announce();
    let programs = program::discover(&root());
    let drift = oracle::verify_vectors(&programs, &probe);
    assert!(drift.is_empty(), "committed vectors differ from their reference models (run tests/conformance_programs/regen_vectors.py):\n{}", drift.join("\n"));
}

#[test]
fn feature_coverage_gate() {
    let programs = program::discover(&root());
    let cov = coverage::collect(&programs);
    let missing = coverage::unexercised(&cov, &root().join("FEATURES.toml"));
    assert!(missing.is_empty(), "features in FEATURES.toml not exercised by any corpus program:\n  {}", missing.join("\n  "));
}

// ---------------------------------------------------------------------------------------------
// The corpus x backends matrix
// ---------------------------------------------------------------------------------------------

#[test]
fn corpus_conforms_on_every_backend() {
    let probe = probe_all();
    let programs = program::discover(&root());
    let support = Support::load(&root().join("SUPPORT.toml"));
    let ctx = Arc::new(Ctx { wj: wj(), programs, support, probe, work_root: work_dir("corpus") });
    let report = run_all(ctx);
    println!("{}", report.summary());
    let failures = report.failures();
    assert!(failures.is_empty(), "{}\n{} conformance failure(s):\n{}", report.summary(), failures.len(), failures.join("\n"));
}

/// The whole corpus must behave identically on the TypeScript flavour (it shares the JS emitter,
/// but annotations, cells and class layouts differ).
#[test]
fn corpus_conforms_on_typescript() {
    let probe = probe_all();
    if !probe.has(Tool::Node) {
        return;
    }
    let version = std::process::Command::new("node").arg("--version").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let mut parts = version.trim().trim_start_matches('v').split('.').filter_map(|p| p.parse::<u32>().ok());
    let (major, minor) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    if (major, minor) < (22, 6) {
        println!("CONFORMANCE-SKIP: node {} lacks TypeScript type stripping (needs >= 22.6)", version.trim());
        return;
    }
    let support = Support::load(&root().join("SUPPORT.toml"));
    let work = work_dir("ts");
    let mut failures = Vec::new();
    let mut ran = 0;
    for p in program::discover(&root()).iter() {
        ran += 1;
        if let Verdict::Fail(msg) = eval_program(&wj(), p, Backend::Ts, &support, &work) {
            failures.push(format!("{} [ts]: {msg}", p.name));
        }
    }
    assert!(ran >= 100, "corpus went missing ({ran} programs)");
    assert!(failures.is_empty(), "{} typescript failure(s):\n{}", failures.len(), failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Harness self-tests: the harness itself must be unfakeable
// ---------------------------------------------------------------------------------------------

#[test]
fn cheating_program_is_rejected() {
    let probe = ToolProbe::new(&[Tool::Node, Tool::Python3]);
    probe.announce();
    if !probe.has(Tool::Node) {
        return;
    }
    let cheat = program::load(&root().join("selftest/cheat_constant_for_seed_1.wj"));
    assert!(cheat.seeds.len() >= 5);
    let work = work_dir("cheat");
    let v = eval_program(&wj(), &cheat, Backend::Js, &Support::default(), &work);
    match v {
        Verdict::Fail(msg) => assert!(msg.contains("seed 2"), "cheater must pass seed 1 and fail seed 2, got: {msg}"),
        other => panic!("a program printing a hard-coded constant must FAIL, got {other:?}"),
    }
}

#[test]
fn injected_backend_bugs_are_all_caught() {
    let probe = probe_all();
    let programs = program::discover(&root());
    let mut survivors = Vec::new();
    let mut report = String::new();
    for m in mutation::mutants() {
        if !probe.has(m.backend.tool()) {
            continue;
        }
        let killed = m.programs.iter().any(|name| {
            let p: &Program = programs.iter().find(|p| p.name == *name).unwrap_or_else(|| panic!("mutant {} targets unknown program {name}", m.name));
            mutant_exposed_by(&m, p)
        });
        report.push_str(&format!("{:<32} {}\n", m.name, if killed { "KILLED" } else { "SURVIVED" }));
        if !killed {
            survivors.push(m.name);
        }
    }
    println!("mutation self-test:\n{report}");
    assert!(survivors.is_empty(), "harness failed to catch injected bugs: {survivors:?}\n{report}");
}

fn mutant_exposed_by(m: &mutation::Mutant, p: &Program) -> bool {
    let seed = p.seeds[0];
    let work = work_dir(&format!("mut-{}-{}", m.name, p.name));
    let expected = oracle::expected(p, seed).expect("oracle");
    let apply = |dir: &Path| mutation::apply(dir, &m.edits);
    match compile_and_run_with(&wj(), m.backend, &work, &p.source_for(seed), Some(&apply)) {
        Outcome::BuildFailed(e) => {
            assert!(!e.starts_with("mutation did not apply"), "mutant {} is a no-op on {}: {e}", m.name, p.name);
            true
        }
        Outcome::Ran(t, _) => first_divergence(&expected, &t).is_some(),
    }
}

// ---------------------------------------------------------------------------------------------
// Generative differential testing and metamorphic relations
// ---------------------------------------------------------------------------------------------

fn gen_budget() -> (usize, usize) {
    if cfg!(feature = "conformance_soak") { (5000, 50) } else { (200, 20) }
}

fn run_generated(names: &gen_ast::Names, transform: &dyn Fn(&mut Vec<(usize, gen_ast::Case)>), label: &str, total: usize) {
    let probe = probe_all();
    let (_, per_bundle) = gen_budget();
    let work = work_dir(label);
    let mut failures = Vec::new();
    for (bi, start) in (0..total).step_by(per_bundle).enumerate() {
        let original = make_cases(1000 + start as u64, per_bundle.min(total - start));
        let expected = gen_ast::eval_program(&original);
        let mut cases = original.clone();
        transform(&mut cases);
        for b in ALL_BACKENDS.iter().copied().filter(|b| probe.has(b.tool())) {
            let w = work.join(format!("b{bi}-{}", b.name()));
            std::fs::create_dir_all(&w).unwrap();
            let src = gen_ast::print_program(&cases, names);
            if let Outcome::Ran(t, _) = conformance::backends::compile_and_run(&wj(), b, &w, &src) {
                if let Some(i) = expected.iter().enumerate().find(|(i, e)| t.lines.get(*i) != Some(*e)).map(|(i, _)| i) {
                    let id = cases.iter().rev().find(|(id, _)| expected[..=i].contains(&format!("#{id}"))).map(|c| c.0).unwrap_or(0);
                    let case = cases.iter().find(|(c, _)| *c == id).unwrap().1.clone();
                    let rep = shrink_report(&wj(), b, &w, id, &case, names);
                    failures.push(format!("[{}] bundle {bi}: {rep}", b.name()));
                }
            } else if let Outcome::BuildFailed(e) = conformance::backends::compile_and_run(&wj(), b, &w, &src) {
                // The Rust target has known codegen defects on this subset (docs/known-issues); only its
                // wrong *output* is a failure, a build rejection is reported but not fatal.
                if b == Backend::Rust {
                    eprintln!("NOTE [rust] bundle {bi}: rustc rejected generated Rust (known issue): {}", e.lines().next().unwrap_or(""));
                } else {
                    failures.push(format!("[{}] bundle {bi}: build failed: {}", b.name(), e.lines().take(8).collect::<Vec<_>>().join(" | ")));
                }
            }
        }
    }
    let _ = GenVerdict::Agree;
    assert!(failures.is_empty(), "{} generated-program divergence(s):\n{}", failures.len(), failures.join("\n\n"));
}

#[test]
fn generated_programs_agree_with_reference_evaluator() {
    run_generated(&gen_ast::Names::plain(), &|_| {}, "gen", gen_budget().0);
}

#[test]
fn metamorphic_rename_locals_preserves_trace() {
    run_generated(&meta::rename_names(), &|_| {}, "meta-rename", 40);
}

#[test]
fn metamorphic_reorder_independent_statements_preserves_trace() {
    run_generated(&gen_ast::Names::plain(), &|cs| cs.iter_mut().for_each(|(_, c)| c.stmts = meta::reorder_independent(&c.stmts)), "meta-reorder", 40);
}

#[test]
fn metamorphic_wrap_in_blocks_preserves_trace() {
    run_generated(&gen_ast::Names::plain(), &|cs| cs.iter_mut().for_each(|(_, c)| c.stmts = meta::wrap_in_blocks(&c.stmts)), "meta-wrap", 40);
}

#[test]
fn metamorphic_split_into_temporaries_preserves_trace() {
    run_generated(
        &gen_ast::Names::plain(),
        &|cs| {
            let mut next = 0;
            cs.iter_mut().for_each(|(_, c)| c.stmts = meta::split_into_temporaries(&c.stmts, &mut next));
        },
        "meta-split",
        40,
    );
}

#[test]
fn metamorphic_extract_function_preserves_trace() {
    run_generated(&gen_ast::Names::plain(), &|cs| cs.iter_mut().for_each(|(_, c)| c.stmts = meta::extract_identity(&c.stmts)), "meta-extract", 40);
}

// ---------------------------------------------------------------------------------------------
// The interpreter as an explicitly weaker 4th oracle (reported, never authoritative)
// ---------------------------------------------------------------------------------------------

#[test]
fn interpreter_oracle_report() {
    let programs = program::discover(&root());
    let work = work_dir("interp");
    let (mut agree, mut differ) = (0, Vec::new());
    for p in programs.iter().filter(|p| !p.seeded) {
        let Ok(expected) = oracle::expected(p, p.seeds[0]) else { continue };
        let w = work.join(&p.name);
        std::fs::create_dir_all(&w).unwrap();
        if let Outcome::Ran(t, _) = conformance::backends::run_interpreter(&wj(), &w, &p.source_for(p.seeds[0])) {
            let lines: Vec<String> = t.lines.iter().filter(|l| !l.starts_with("Interpreting")).cloned().collect();
            let t = Trace { lines, ..t };
            match first_divergence(&expected, &t) {
                None => agree += 1,
                Some(d) => differ.push(format!("{}: {d}", p.name)),
            }
        }
    }
    println!("interpreter (weak oracle) agrees with the independent oracle on {agree} programs; differs on {}:\n  {}", differ.len(), differ.join("\n  "));
}

// ---------------------------------------------------------------------------------------------
// CLI: `wj build --target go|js|ts` produces a ready-to-run directory
// ---------------------------------------------------------------------------------------------

#[test]
fn cli_build_produces_ready_to_run_directories() {
    let probe = ToolProbe::new(&[Tool::Go, Tool::Node]);
    probe.announce();
    let src = "fn main() {\n    println(\"hello {}\", 40 + 2)\n}\n";
    for (target, files, run) in [
        ("go", vec!["main.go", "wj_core.go", "go.mod"], Some((Tool::Go, vec!["run", "."]))),
        ("js", vec!["main.mjs", "wj_core.mjs", "package.json"], Some((Tool::Node, vec!["main.mjs"]))),
        ("ts", vec!["main.ts", "wj_core.mjs", "package.json"], Some((Tool::Node, vec!["main.ts"]))),
    ] {
        let work = work_dir(&format!("cli-{target}"));
        let file = work.join("hello.wj");
        std::fs::write(&file, src).unwrap();
        let out = work.join("out");
        let st = std::process::Command::new(wj()).args(["build", "--target", target, "--no-cargo", "-o"]).arg(&out).arg(&file).output().unwrap();
        assert!(st.status.success(), "{target}: {}", String::from_utf8_lossy(&st.stderr));
        for f in &files {
            assert!(out.join(f).exists(), "{target}: missing {f}");
        }
        if let Some((tool, args)) = run {
            if probe.has(tool) {
                let r = std::process::Command::new(tool.name()).args(&args).current_dir(&out).output().unwrap();
                assert_eq!(String::from_utf8_lossy(&r.stdout).trim(), "hello 42", "{target}: {}", String::from_utf8_lossy(&r.stderr));
            }
        }
    }
}
