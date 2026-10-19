#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Differential testing of the interpreter against two independent oracles:
//!
//! 1. committed vectors derived from independent Python reference models (`<name>.ref.py`),
//!    re-derived here whenever `python3` exists so vectors cannot be hand-edited to fit;
//! 2. the real compiled-Rust pipeline (`wj build --no-cargo` + `rustc`).
//!
//! Programs are templates with `__SEED__`; each seed yields different data, so a
//! print-faking interpreter cannot pass. `selftest/` holds a program that fakes its output;
//! the harness must reject it (mutation self-test).
//!
//! Corpus conventions (shared with the cross-backend conformance corpus):
//! header lines `// seeds: a b c`, `// expect: line`, `// exit: N`; vectors in
//! `<name>.vectors/<seed>.out`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[derive(Debug, Clone)]
struct Program {
    name: String,
    path: PathBuf,
    template: String,
    seeds: Vec<u64>,
    exit: i32,
    /// `// interp-only: reason`: the compiled pipeline cannot build this program (reason documented in the file); only the oracle applies.
    interp_only: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: String,
    exit: i32,
    stderr: String,
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/interpreter_corpus")
}

fn load(path: &Path) -> Program {
    let template = fs::read_to_string(path).unwrap();
    let mut seeds = Vec::new();
    let mut exit = 0;
    let mut interp_only = false;
    for line in template.lines() {
        if let Some(r) = line.strip_prefix("// seeds:") {
            seeds = r.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        }
        if line.starts_with("// interp-only:") {
            interp_only = true;
        }
        if let Some(r) = line.strip_prefix("// exit:") {
            exit = r.trim().parse().unwrap_or(0);
        }
    }
    assert!(!seeds.is_empty(), "{} needs a `// seeds:` header", path.display());
    Program {
        name: path.file_stem().unwrap().to_string_lossy().to_string(),
        path: path.to_path_buf(),
        template,
        seeds,
        exit,
        interp_only,
    }
}

fn programs(dir: &Path) -> Vec<Program> {
    let mut v: Vec<Program> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "wj").unwrap_or(false))
        .map(|p| load(&p))
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

fn source(p: &Program, seed: u64) -> String {
    p.template.replace("__SEED__", &seed.to_string())
}

fn vector(p: &Program, seed: u64) -> String {
    let path = p.path.parent().unwrap().join(format!("{}.vectors/{}.out", p.name, seed));
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing vector {}", path.display()))
}

/// Run through the real CLI path users take: `wj run --interpret`. A hung program (e.g. a mutant
/// with an infinite loop) is killed after 20s and reported as exit -9.
fn interpret(src: &str) -> Outcome {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let mut child = Command::new(WJ)
        .args(["run", "--interpret", "--no-lint"])
        .arg(&f)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Outcome { stdout: String::new(), exit: -9, stderr: "timeout".into() };
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    Outcome {
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        exit: out.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

fn compiled_rust(src: &str) -> Result<Outcome, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    let build = dir.path().join("build");
    fs::create_dir_all(&build).unwrap();
    let wj = Command::new(WJ)
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(&f)
        .current_dir(dir.path())
        .output()
        .unwrap();
    if !wj.status.success() {
        return Err(format!("wj build failed: {}", String::from_utf8_lossy(&wj.stderr)));
    }
    let rs = build.join("test.rs");
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(&rs).arg("-o").arg(&bin).args(["--edition", "2021"]).output().unwrap();
    if !rustc.status.success() {
        return Err(format!("rustc failed: {}", String::from_utf8_lossy(&rustc.stderr)));
    }
    let run = Command::new(&bin).output().unwrap();
    Ok(Outcome {
        stdout: String::from_utf8_lossy(&run.stdout).to_string(),
        exit: run.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&run.stderr).to_string(),
    })
}

fn python_available() -> bool {
    Command::new("python3").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn corpus_is_non_trivial_and_has_vectors() {
    let ps = programs(&corpus_dir());
    assert!(ps.len() >= 10, "corpus too small: {}", ps.len());
    for p in &ps {
        assert!(p.seeds.len() >= 3, "{}: need >= 3 seeds so a constant cannot pass", p.name);
        for s in &p.seeds {
            assert!(!vector(p, *s).is_empty() || p.exit != 0, "{} seed {}: empty vector", p.name, s);
        }
        // vectors must differ across seeds (otherwise the program does not exercise its input)
        let first = vector(p, p.seeds[0]);
        assert!(p.seeds.iter().any(|s| vector(p, *s) != first), "{}: vectors identical for every seed", p.name);
    }
}

#[test]
fn vectors_are_derived_from_python_reference_models() {
    if !python_available() {
        eprintln!("python3 not available: skipping vector re-derivation");
        return;
    }
    for dir in [corpus_dir(), corpus_dir().join("selftest")] {
        for p in programs(&dir) {
            let script = p.path.with_extension("ref.py");
            for s in &p.seeds {
                let out = Command::new("python3").arg("-I").arg(&script).arg(s.to_string()).output().unwrap();
                assert!(out.status.success(), "{} seed {}: reference model failed", p.name, s);
                assert_eq!(
                    String::from_utf8_lossy(&out.stdout),
                    vector(&p, *s),
                    "{} seed {}: committed vector differs from its reference model",
                    p.name,
                    s
                );
            }
        }
    }
}

#[test]
fn interpreter_matches_independent_oracle_for_every_seed() {
    let mut failures = Vec::new();
    for p in programs(&corpus_dir()) {
        for s in &p.seeds {
            let got = interpret(&source(&p, *s));
            let want = vector(&p, *s);
            if got.stdout != want || got.exit != p.exit {
                failures.push(format!(
                    "{} seed {}: exit {} (want {})\n--- got\n{}--- want\n{}--- stderr\n{}",
                    p.name, s, got.exit, p.exit, got.stdout, want, got.stderr
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{} oracle mismatches:\n{}", failures.len(), failures.join("\n=====\n"));
}

#[test]
fn interpreter_matches_compiled_rust() {
    // Compiling is slow: default to the first and last seed; WJ_DIFF_ALL_SEEDS=1 runs them all.
    let all = std::env::var("WJ_DIFF_ALL_SEEDS").is_ok();
    let mut failures = Vec::new();
    for p in programs(&corpus_dir()).into_iter().filter(|p| !p.interp_only) {
        let seeds: Vec<u64> = if all {
            p.seeds.clone()
        } else {
            vec![p.seeds[0], *p.seeds.last().unwrap()]
        };
        for s in seeds {
            let src = source(&p, s);
            let rust = match compiled_rust(&src) {
                Ok(r) => r,
                Err(e) => {
                    failures.push(format!("{} seed {}: Rust pipeline failed (transpiler/rustc): {}", p.name, s, e));
                    continue;
                }
            };
            let got = interpret(&src);
            if got.stdout != rust.stdout || got.exit != rust.exit {
                failures.push(format!(
                    "{} seed {}: interpreter (exit {}) != rust (exit {})\n--- interpreter\n{}--- rust\n{}",
                    p.name, s, got.exit, rust.exit, got.stdout, rust.stdout
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{} differential failures:\n{}", failures.len(), failures.join("\n=====\n"));
}

// ------------------------------------------------------------ harness self-tests

#[test]
fn selftest_print_faking_program_is_rejected_by_the_oracle() {
    let dir = corpus_dir().join("selftest");
    let cheat = load(&dir.join("cheat_constant_output.wj"));
    let mut rejected = 0;
    for s in &cheat.seeds {
        let got = interpret(&source(&cheat, *s));
        if got.stdout != vector(&cheat, *s) {
            rejected += 1;
        }
    }
    assert!(rejected >= 2, "the oracle must reject a constant-printing program on most seeds (rejected {})", rejected);
}

#[test]
fn selftest_wrong_seed_oracle_is_detected() {
    // Comparing the output of seed A with the vector of seed B must fail: the harness is seed-sensitive.
    let ps = programs(&corpus_dir());
    let mut sensitive = 0;
    for p in &ps {
        if p.exit != 0 {
            continue;
        }
        let a = interpret(&source(p, p.seeds[0]));
        let wrong = vector(p, *p.seeds.last().unwrap());
        if a.stdout != wrong {
            sensitive += 1;
        }
    }
    assert!(sensitive >= 8, "only {} programs are seed-sensitive", sensitive);
}

#[test]
fn selftest_source_mutants_are_killed_by_the_oracle() {
    // Mutation testing of the harness: perturb a program's logic; the oracle comparison must notice.
    // (A harness that only checked "ran without error" or printed-looking output would let these live.)
    fn mutate(src: &str, kind: usize) -> Option<String> {
        match kind {
            0 => src.find(" + ").map(|i| format!("{} - {}", &src[..i], &src[i + 3..])),
            1 => src.find(" * ").map(|i| format!("{} + {}", &src[..i], &src[i + 3..])),
            2 => {
                // bump the first integer literal of 3+ digits found after `fn main`
                let start = src.find("fn main").unwrap_or(0);
                let bytes = src.as_bytes();
                let mut i = start;
                while i < bytes.len() {
                    if bytes[i].is_ascii_digit() && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
                        let mut j = i;
                        while j < bytes.len() && bytes[j].is_ascii_digit() {
                            j += 1;
                        }
                        if j - i >= 2 {
                            let n: u64 = src[i..j].parse().ok()?;
                            return Some(format!("{}{}{}", &src[..i], n + 1, &src[j..]));
                        }
                        i = j;
                    } else {
                        i += 1;
                    }
                }
                None
            }
            _ => None,
        }
    }
    let (mut killed, mut total) = (0, 0);
    for p in programs(&corpus_dir()) {
        if p.exit != 0 {
            continue;
        }
        let seed = p.seeds[0];
        let truth = vector(&p, seed);
        for kind in 0..3 {
            if let Some(m) = mutate(&source(&p, seed), kind) {
                total += 1;
                let got = interpret(&m);
                if got.stdout != truth || got.exit != 0 {
                    killed += 1;
                }
            }
        }
    }
    assert!(total >= 15, "too few mutants generated: {}", total);
    assert!(killed * 10 >= total * 8, "only {}/{} mutants were killed by the oracle", killed, total);
}

// ------------------------------------------------------------ ADR-0014 Phase 1 intrinsics
// stdout and the exit status are compared exactly by the corpus tests above. The tests below add
// the second stream: stderr text (panic messages, eprintln, debug) against compiled Rust, with the
// only legitimately different part (source locations, backtrace footer) normalised away.

/// Keep message content; drop what must differ between a generated `.rs` and the `.wj` source.
fn normalize_stderr(s: &str) -> String {
    let mut out = Vec::new();
    for line in s.lines() {
        if line.starts_with("note: run with") || line.starts_with("stack backtrace") {
            break; // rust footer / interpreter backtrace follow the message
        }
        if line.trim().is_empty() {
            continue;
        }
        // build-time lint diagnostics (`wj run --interpret` reports what `wj build` would) are not
        // program output; the compiled binary's stderr never contains them
        if line.starts_with("warning: ") || line.starts_with("  --> ") || line.starts_with("  = ") {
            continue;
        }
        if line.starts_with("thread 'main'") {
            out.push("PANIC".to_string());
        } else if line.starts_with('[') && line.contains("] ") {
            let i = line.find("] ").unwrap();
            out.push(format!("[LOC] {}", &line[i + 2..]));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

fn intrinsic_program(body: &str) -> String {
    format!(
        "fn main() {{\n    let x: int = 5\n    let one: int = 1\n    let s = \"txt\"\n    {}\n}}\n",
        body
    )
}

#[test]
fn intrinsic_stderr_and_exit_match_compiled_rust() {
    let bodies = [
        "stderr.println(\"e ${x}\")\n    stderr.print(\"p\")\n    eprintln(\"q ${x}\")\n    println(\"o ${x}\")",
        "panic(\"boom ${x}\")",
        "panic()",
        "todo()",
        "todo(\"later ${x}\")",
        "unreachable()",
        "unreachable(\"state ${x}\")",
        // no integer literal in the condition: rustc echoes the generated source (`6_i64`)
        "assert(x == one)",
        "assert(x > 9, \"too small: ${x}\")",
        "assert_eq(x, 6)",
        "assert_eq(x, 6, \"ctx ${x}\")",
        "assert_ne(x, 5)",
        "assert_ne(x, 5, \"same ${x}\")",
        "assert_eq(s, \"other\")",
        "let y = debug(x + one)\n    let z = debug(s)\n    println(\"${y} ${z}\")",
        "let l = [x, one]\n    let m = debug(l)\n    println(\"${m:?}\")",
        "println!(\"legacy {} {:?}\", x, s)\n    eprintln!(\"legacy-err {}\", x)\n    assert!(x == 5, \"m {}\", x)\n    let d = dbg!(x)\n    println(\"${d}\")",
        "assert_eq!(x, 6, \"legacy {}\", x)",
    ];
    let mut failures = Vec::new();
    for body in bodies {
        let src = intrinsic_program(body);
        let rust = match compiled_rust(&src) {
            Ok(r) => r,
            Err(e) => {
                failures.push(format!("{body}: Rust pipeline failed: {e}"));
                continue;
            }
        };
        let got = interpret(&src);
        let (gs, rs) = (normalize_stderr(&got.stderr), normalize_stderr(&rust.stderr));
        if got.stdout != rust.stdout || got.exit != rust.exit || gs != rs {
            failures.push(format!(
                "`{body}`: interpreter exit {} vs rust {}\n--- interpreter stdout/stderr\n{}\n{}\n--- rust\n{}\n{}",
                got.exit, rust.exit, got.stdout, gs, rust.stdout, rs
            ));
        }
    }
    assert!(failures.is_empty(), "{} intrinsic differences:\n{}", failures.len(), failures.join("\n=====\n"));
}

#[test]
fn intrinsic_stderr_has_independent_expected_text() {
    // Expected values hand-derived from rustc's documented panic formats, not from any backend.
    let cases: [(&str, i32, &str, &str); 6] = [
        ("stderr.println(\"e ${x}\")\n    println(\"o ${x}\")", 0, "o 5\n", "e 5"),
        ("todo(\"later\")", 101, "", "not yet implemented: later"),
        ("assert(x == 6)", 101, "", "assertion failed: x == 6"),
        ("assert_eq(x, 6, \"c\")", 101, "", "assertion `left == right` failed: c\n  left: 5\n right: 6"),
        ("let y = debug(x)\n    println(\"${y}\")", 0, "5\n", "] x = 5"),
        ("unreachable()", 101, "", "internal error: entered unreachable code"),
    ];
    for (body, exit, out, err) in cases {
        let got = interpret(&intrinsic_program(body));
        assert_eq!((got.exit, got.stdout.as_str()), (exit, out), "{body}: {}", got.stderr);
        assert!(got.stderr.contains(err), "{body}: stderr {:?} lacks {:?}", got.stderr, err);
    }
}

#[test]
// `println(a: x)` is rejected earlier, by the parser (ADR-0018 named-argument rule), with its own diagnostic.
fn intrinsic_unsupported_shapes_exit_2_with_a_diagnostic() {
    for body in ["let y = debug(x, x)", "assert()", "assert_eq(x)", "let y = dbg!(x, x)"] {
        let got = interpret(&intrinsic_program(body));
        assert_eq!(got.exit, 2, "{body}: stdout {:?} stderr {:?}", got.stdout, got.stderr);
        assert!(got.stderr.contains("unsupported in the interpreter"), "{body}: {}", got.stderr);
        assert_eq!(got.stdout, "", "{body} must not print anything");
    }
}

#[test]
fn selftest_stream_and_message_mutants_are_detected() {
    // The stderr oracle must notice a message that went to the wrong stream or has the wrong text.
    let good = interpret(&intrinsic_program("stderr.println(\"e ${x}\")\n    assert(x == 6, \"bad ${x}\")"));
    let wrong_stream = interpret(&intrinsic_program("stdout.println(\"e ${x}\")\n    assert(x == 6, \"bad ${x}\")"));
    let wrong_text = interpret(&intrinsic_program("stderr.println(\"e ${x}\")\n    assert(x == 6, \"bad ${one}\")"));
    assert_eq!(good.exit, 101);
    assert_ne!(good.stdout, wrong_stream.stdout);
    assert_ne!(normalize_stderr(&good.stderr), normalize_stderr(&wrong_stream.stderr));
    assert_ne!(normalize_stderr(&good.stderr), normalize_stderr(&wrong_text.stderr));
    // normalisation keeps content but drops locations
    assert_eq!(
        normalize_stderr("thread 'main' panicked at a.wj:1:2:\nmsg\nstack backtrace:\n  0: main"),
        normalize_stderr("thread 'main' (9) panicked at build/t.rs:7:5:\nmsg\nnote: run with `RUST_BACKTRACE=1`")
    );
}
