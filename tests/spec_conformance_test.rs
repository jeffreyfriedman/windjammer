// consumes: tests/spec/**/*.wj
//! SPEC executable examples: every `tests/spec/**/*.wj` file is one example for one or more rules in
//! `docs/spec/*.md`, run in-process through the real lexer, parser and interpreter.
//!
//! Header comments (all `//` lines before the first non-comment line):
//!   spec: SPEC-XXX-NNN      rule(s) this example demonstrates (required, repeatable)
//!   mode: run | parse-ok | parse-error | panic      (default run)
//!         | check-ok | check-error                  `wj check` front end (no run); manifest becomes wj.toml
//!         | effects                                 `expect:` lines are `fn: effects` from the effect report (ADR-0021)
//!         | simulate                                deterministic world (virtual clock, seeded scheduler and random):
//!                                                   run twice with `seed:`, both runs must agree (and match `expect:`)
//!         | replay                                  record a run, replay it from the effect log; outputs must agree
//!   expect: TEXT            one expected stdout line, in order (run mode); oracle derived by hand
//!   expect-error: TEXT      substring of the parse error / panic message
//!   expect-exit: N          expected exit status for panic mode (default 101)
//!   manifest: TOML-LINE     one line of the program's wj.toml (repeatable; ADR-0021 `[capabilities]`, `[policy]`)
//!   seed: N                 simulation seed (simulate mode, default 1)
//!   faults: P               probability that a file or network call fails (simulate mode, default 0)
//!   profile: local         run under the default local profile (project directory + time/random/console), enforced
//!   xfail: Q-NNN            the implementation currently contradicts the spec (docs/spec/OPEN_QUESTIONS.md);
//!                           the example must FAIL, and the run fails if it starts passing
//!
//! Rule-ID/test consistency (every rule has a test, every test names a known rule) is enforced by
//! `scripts/spec_check.sh`, which `rule_ids_and_tests_are_consistent` runs.

#![allow(clippy::duplicate_mod)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use windjammer::interpreter::runner::{exit_code_for, with_big_stack};
use windjammer::interpreter::Interpreter;

#[derive(Default, Debug)]
struct Spec {
    path: PathBuf,
    rules: Vec<String>,
    mode: String,
    expect: Vec<String>,
    expect_error: Option<String>,
    expect_exit: Option<i32>,
    xfail: Option<String>,
    manifest: Vec<String>,
    profile: Option<String>,
    seed: Option<u64>,
    faults: Option<f64>,
    src: String,
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
}

fn load(path: &Path) -> Spec {
    let src = std::fs::read_to_string(path).unwrap();
    let mut s = Spec { path: path.to_path_buf(), mode: "run".into(), src: src.clone(), ..Default::default() };
    for line in src.lines() {
        let Some(c) = line.strip_prefix("//") else {
            if line.trim().is_empty() {
                continue;
            }
            break;
        };
        let c = c.strip_prefix(' ').unwrap_or(c);
        let Some((k, v)) = c.split_once(':') else { continue };
        let v = v.strip_prefix(' ').unwrap_or(v);
        match k {
            "spec" => s.rules.push(v.trim().to_string()),
            "mode" => s.mode = v.trim().to_string(),
            "expect" => s.expect.push(v.to_string()),
            "expect-error" => s.expect_error = Some(v.trim().to_string()),
            "expect-exit" => s.expect_exit = Some(v.trim().parse().expect("expect-exit")),
            "xfail" => s.xfail = Some(v.trim().to_string()),
            "manifest" => s.manifest.push(v.trim().to_string()),
            "seed" => s.seed = Some(v.trim().parse().expect("seed")),
            "faults" => s.faults = Some(v.trim().parse().expect("faults")),
            "profile" => s.profile = Some(v.trim().to_string()),
            _ => {}
        }
    }
    s
}

/// Ok(()) when the example behaves as its header says; Err(why) otherwise.
fn check(s: &Spec) -> Result<(), String> {
    match s.mode.as_str() {
        "check-ok" | "check-error" => return check_front_end(s),
        "simulate" => return simulate(s),
        "effects" => return effect_report(s),
        "replay" => return replay(s),
        _ => {}
    }
    let src = s.src.clone();
    let manifest = s.manifest.clone();
    let local_profile = s.profile.as_deref() == Some("local");
    let name = s.path.display().to_string();
    let (parsed, out, exit, err) = with_big_stack(move || {
        catch_unwind(AssertUnwindSafe(|| {
            let mut lexer = windjammer::lexer::Lexer::new(&src);
            let tokens = lexer.tokenize_with_locations();
            let parser: &'static mut windjammer::parser::Parser = Box::leak(Box::new(
                windjammer::parser::Parser::new_with_source(tokens, name.clone(), src.clone()),
            ));
            let program = match parser.parse() {
                Ok(p) => p,
                Err(e) => return (Err(e.to_string()), String::new(), 1, String::new()),
            };
            let mut it = Interpreter::new_capturing();
            if local_profile {
                it.set_caps(Some(windjammer::interpreter::caps::CapGate::local(&std::env::current_dir().unwrap())));
            } else if !manifest.is_empty() {
                match gate_from_manifest(&manifest) {
                    Ok(g) => it.set_caps(Some(g)),
                    Err(e) => return (Err(e), String::new(), 1, String::new()),
                }
            }
            match it.run_checked(&program) {
                Ok(_) => (Ok(()), it.get_output(), 0, String::new()),
                Err(e) => (Ok(()), it.get_output(), exit_code_for(&e), e.render()),
            }
        }))
        .unwrap_or_else(|_| (Err("lexer/parser panicked".to_string()), String::new(), 1, String::new()))
    });
    let want_err = |msg: &str| match &s.expect_error {
        Some(e) if !msg.contains(e.as_str()) => Err(format!("error {msg:?} lacks {e:?}")),
        _ => Ok(()),
    };
    match s.mode.as_str() {
        "parse-ok" => parsed.map_err(|e| format!("expected to parse, got: {e}")),
        "parse-error" => match parsed {
            Ok(()) => Err("expected a parse error but it parsed".into()),
            Err(e) => want_err(&e),
        },
        "run" | "panic" => {
            parsed.map_err(|e| format!("parse error: {e}"))?;
            if s.mode == "panic" {
                let want = s.expect_exit.unwrap_or(101);
                if exit != want {
                    return Err(format!("exit status {exit}, want {want} ({err})"));
                }
                want_err(&err)?;
            } else if exit != 0 {
                return Err(format!("exit status {exit}: {err}"));
            }
            let got: Vec<&str> = out.lines().collect();
            let want: Vec<&str> = s.expect.iter().map(|x| x.as_str()).collect();
            if got != want {
                return Err(format!("stdout mismatch\n  want: {want:?}\n  got:  {got:?}"));
            }
            Ok(())
        }
        m => Err(format!("unknown mode {m}")),
    }
}

/// The `manifest:` header lines as a `wj.toml`.
fn manifest_text(lines: &[String]) -> String {
    let mut toml = String::from("[package]\nname = \"spec\"\nversion = \"0.1.0\"\n");
    for l in lines {
        toml.push_str(l);
        toml.push('\n');
    }
    toml
}

/// An enforcing capability gate from the manifest's `[capabilities]` table (relative paths anchor at the cwd).
fn gate_from_manifest(lines: &[String]) -> Result<windjammer::interpreter::caps::CapGate, String> {
    use windjammer::interpreter::caps::{CapGate, CapMode};
    let root = std::env::current_dir().unwrap();
    match windjammer::ir::effect_facts::grant_from_manifest_text(&manifest_text(lines), &root)? {
        Some(g) => Ok(CapGate::new(g, CapMode::Enforce)),
        None => Err("manifest has no [capabilities] table".into()),
    }
}

/// `effects` mode: every `expect:` line (`fn: effects`) must appear in the compact effect report.
fn effect_report(s: &Spec) -> Result<(), String> {
    let src = s.src.clone();
    let name = s.path.display().to_string();
    let lines = with_big_stack(move || {
        catch_unwind(AssertUnwindSafe(|| {
            let mut lexer = windjammer::lexer::Lexer::new(&src);
            let tokens = lexer.tokenize_with_locations();
            let parser: &'static mut windjammer::parser::Parser = Box::leak(Box::new(
                windjammer::parser::Parser::new_with_source(tokens, name.clone(), src.clone()),
            ));
            let program: &'static _ = Box::leak(Box::new(parser.parse().map_err(|e| e.to_string())?));
            let files = [windjammer::resolve::ParsedFile::new(vec![], name.into(), program)];
            let r = windjammer::resolve::resolve_program(&files);
            let facts = windjammer::ir::effect_facts::analyze(&r);
            Ok::<Vec<String>, String>(
                windjammer::ir::effect_facts::report::compact_lines(&r, &facts).into_iter().map(|(f, e)| format!("{f}: {e}")).collect(),
            )
        }))
        .unwrap_or_else(|_| Err("effect analysis panicked".to_string()))
    })?;
    let missing: Vec<&String> = s.expect.iter().filter(|w| !lines.contains(w)).collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("effect report lacks {missing:?}; report is {lines:?}"))
    }
}

/// `wj check` on the example as `main.wj` in a project whose `wj.toml` is the `manifest:` lines.
fn check_front_end(s: &Spec) -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!(
        "wj-spec-check-{}-{}",
        std::process::id(),
        s.path.file_stem().unwrap().to_string_lossy()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let toml = manifest_text(&s.manifest);
    std::fs::write(dir.join("wj.toml"), toml).map_err(|e| e.to_string())?;
    let main = dir.join("main.wj");
    std::fs::write(&main, &s.src).map_err(|e| e.to_string())?;
    let sources = [(main.display().to_string(), s.src.clone())];
    let diags = catch_unwind(AssertUnwindSafe(|| windjammer::check::check_sources(&dir, &sources)));
    let _ = std::fs::remove_dir_all(&dir);
    let diags = diags.map_err(|_| "wj check panicked".to_string())?;
    let errors: Vec<String> = diags
        .iter()
        .filter(|d| matches!(d.severity, windjammer::diagnostics::Severity::Error))
        .map(|d| format!("{}: {}\n{}\n{}", d.code, d.message, d.notes.join("\n"), d.help.join("\n")))
        .collect();
    match s.mode.as_str() {
        "check-ok" if errors.is_empty() => Ok(()),
        "check-ok" => Err(format!("expected no errors, got {errors:?}")),
        _ => {
            let want = s.expect_error.as_deref().unwrap_or("");
            if errors.iter().any(|e| e.contains(want)) {
                Ok(())
            } else {
                Err(format!("expected an error containing {want:?}, got {errors:?}"))
            }
        }
    }
}

/// Run twice in the deterministic world with the same seed; the runs must agree and match `expect:`.
fn simulate(s: &Spec) -> Result<(), String> {
    let seed = s.seed.unwrap_or(1);
    let faults = s.faults.unwrap_or(0.0);
    let once = || {
        let src = s.src.clone();
        let name = s.path.display().to_string();
        with_big_stack(move || {
            catch_unwind(AssertUnwindSafe(|| {
                let mut lexer = windjammer::lexer::Lexer::new(&src);
                let tokens = lexer.tokenize_with_locations();
                let parser: &'static mut windjammer::parser::Parser = Box::leak(Box::new(
                    windjammer::parser::Parser::new_with_source(tokens, name, src.clone()),
                ));
                let program = parser.parse().map_err(|e| format!("parse error: {e}"))?;
                let mut it = Interpreter::new_capturing();
                // the same world `wj test --simulate` builds: virtual clock, seeded random numbers and scheduler, in-memory files
                let cfg = windjammer::interpreter::effect_port::sim::SimConfig { seed, faults };
                let here = std::env::current_dir().unwrap_or_default();
                windjammer::interpreter::runner::install_simulation(&mut it, &cfg, &here, &here.join(".wj-sim"));
                it.run_checked(&program).map_err(|e| e.render())?;
                Ok::<String, String>(it.get_output())
            }))
            .unwrap_or_else(|_| Err("lexer/parser/interpreter panicked".to_string()))
        })
    };
    let (a, b) = (once()?, once()?);
    if a != b {
        return Err(format!("two runs with seed {seed} differ:\n  {a:?}\n  {b:?}"));
    }
    let got: Vec<&str> = a.lines().collect();
    let want: Vec<&str> = s.expect.iter().map(|x| x.as_str()).collect();
    if !want.is_empty() && got != want {
        return Err(format!("stdout mismatch\n  want: {want:?}\n  got:  {got:?}"));
    }
    Ok(())
}

/// Record the run into an effect log, replay it from the log: the replay prints the same lines and reports no divergence.
fn replay(s: &Spec) -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let (entry, log) = (dir.path().join("main.wj"), dir.path().join("run.wjlog"));
    std::fs::write(&entry, &s.src).map_err(|e| e.to_string())?;
    let once = |record: bool| {
        let (src, name, entry, log) = (s.src.clone(), s.path.display().to_string(), entry.clone(), log.clone());
        with_big_stack(move || {
            catch_unwind(AssertUnwindSafe(|| {
                let mut lexer = windjammer::lexer::Lexer::new(&src);
                let tokens = lexer.tokenize_with_locations();
                let parser: &'static mut windjammer::parser::Parser = Box::leak(Box::new(windjammer::parser::Parser::new_with_source(tokens, name, src.clone())));
                let program = parser.parse().map_err(|e| format!("parse error: {e}"))?;
                let mut it = Interpreter::new_capturing();
                let mut opts = windjammer::interpreter::runner::RunOptions::default();
                if record {
                    opts.record = Some(log.clone());
                } else {
                    opts.replay = Some(log.clone());
                }
                windjammer::interpreter::runner::setup_effect_port(&mut it, &opts, &entry)?;
                it.run_checked(&program).map_err(|e| e.render())?;
                Ok::<String, String>(it.get_output())
            }))
            .unwrap_or_else(|_| Err("lexer/parser/interpreter panicked".to_string()))
        })
    };
    let (recorded, replayed) = (once(true)?, once(false)?);
    if recorded != replayed {
        return Err(format!("replay differs from the recording:\n  recorded: {recorded:?}\n  replayed: {replayed:?}"));
    }
    let got: Vec<&str> = recorded.lines().collect();
    let want: Vec<&str> = s.expect.iter().map(|x| x.as_str()).collect();
    if !want.is_empty() && got != want {
        return Err(format!("stdout mismatch\n  want: {want:?}\n  got:  {got:?}"));
    }
    Ok(())
}

#[test]
fn spec_examples_behave_as_the_spec_says() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/spec");
    let mut files = Vec::new();
    collect(&root, &mut files);
    assert!(files.len() >= 20, "expected the spec example corpus under tests/spec, found {}", files.len());
    let mut failures = String::new();
    for f in &files {
        let s = load(f);
        if s.rules.is_empty() {
            failures.push_str(&format!("{}: no `// spec:` header\n", f.display()));
            continue;
        }
        let res = check(&s);
        let rel = f.strip_prefix(&root).unwrap().display();
        match (&s.xfail, res) {
            (None, Err(e)) => failures.push_str(&format!("{rel} [{}]: {e}\n", s.rules.join(","))),
            (Some(q), Ok(())) => failures.push_str(&format!(
                "{rel}: marked xfail {q} but now passes; remove the xfail and close {q} in OPEN_QUESTIONS.md\n"
            )),
            _ => {}
        }
    }
    assert!(failures.is_empty(), "spec examples failed:\n{failures}");
}

#[test]
fn rule_ids_and_tests_are_consistent() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = std::process::Command::new("bash")
        .arg(root.join("scripts/spec_check.sh"))
        .current_dir(root)
        .output()
        .expect("run scripts/spec_check.sh");
    assert!(
        out.status.success(),
        "spec_check.sh failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn intrinsic_registry_is_documented() {
    let doc = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/spec/intrinsics.md")).unwrap();
    for i in windjammer::intrinsics::Intrinsic::ALL {
        let needle = format!("`{}`", i.call_name());
        assert!(doc.contains(&needle), "docs/spec/intrinsics.md does not list intrinsic {needle}");
    }
}

#[test]
fn stdlib_index_lists_every_std_module() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("docs/spec/stdlib.md")).unwrap();
    for e in std::fs::read_dir(root.join("std")).unwrap() {
        let name = e.unwrap().file_name().to_string_lossy().to_string();
        if name == "README.md" {
            continue;
        }
        let stem = name.strip_suffix(".wj").unwrap_or(&name);
        assert!(doc.contains(&format!("`{stem}`")), "docs/spec/stdlib.md does not index std/{name}");
    }
}

#[test]
fn rule_examples_in_chapters_match_the_test_files() {
    // Each `<!-- example: PATH -->` marker is followed by a fenced block that must equal the
    // test file with its leading `//` header removed.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for e in std::fs::read_dir(root.join("docs/spec")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "md") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            let Some(rest) = l.strip_prefix("<!-- example: ") else { continue };
            let rel = rest.trim_end_matches(" -->");
            let src = std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("{rel} missing"));
            let body: Vec<&str> = src.lines().skip_while(|l| l.starts_with("//") && l.contains(": ") && is_header(l)).collect();
            // the fence follows the marker, optionally after one `<!-- ignore: .. -->` / `<!-- should_fail: .. -->` reason line
            let start = lines.iter().skip(i + 1).position(|l| l.starts_with("```")).map(|k| i + 1 + k + 1).unwrap_or(lines.len());
            let mut block = Vec::new();
            for l in lines.iter().skip(start) {
                if *l == "```" {
                    break;
                }
                block.push(*l);
            }
            assert_eq!(block, body, "{}: example for {rel} differs from the test file", p.display());
            checked += 1;
        }
    }
    assert!(checked >= 100, "only {checked} examples cross-checked");
}

fn is_header(l: &str) -> bool {
    ["spec", "mode", "expect", "expect-error", "expect-exit", "xfail", "manifest", "seed", "faults", "profile"]
        .iter()
        .any(|k| l.strip_prefix("// ").is_some_and(|r| r.starts_with(&format!("{k}:"))))
}
