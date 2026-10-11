//! SPEC executable examples: every `tests/spec/**/*.wj` file is one example for one or more rules in
//! `docs/spec/*.md`, run in-process through the real lexer, parser and interpreter.
//!
//! Header comments (all `//` lines before the first non-comment line):
//!   spec: SPEC-XXX-NNN      rule(s) this example demonstrates (required, repeatable)
//!   mode: run | parse-ok | parse-error | panic      (default run)
//!   expect: TEXT            one expected stdout line, in order (run mode); oracle derived by hand
//!   expect-error: TEXT      substring of the parse error / panic message
//!   expect-exit: N          expected exit status for panic mode (default 101)
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
            _ => {}
        }
    }
    s
}

/// Ok(()) when the example behaves as its header says; Err(why) otherwise.
fn check(s: &Spec) -> Result<(), String> {
    let src = s.src.clone();
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
            let mut block = Vec::new();
            for l in lines.iter().skip(i + 2) {
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
    ["spec", "mode", "expect", "expect-error", "expect-exit", "xfail"]
        .iter()
        .any(|k| l.strip_prefix("// ").is_some_and(|r| r.starts_with(&format!("{k}:"))))
}
