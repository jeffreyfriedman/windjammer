//! ADR-0021 E4b: `durable fn` in the compiled Rust backend.
//!
//! * the emitted Rust has one `durable::run` per durable function and a `durable::step` per step position, and none of either in
//!   a program without a durable function;
//! * what a compiled build cannot journal yet is refused with the reason, not with a trait error from rustc;
//! * (with `WJ_DURABLE_COMPILED=1`, because it builds the runtime crate) one journal, two implementations: a run that the
//!   interpreter crashes at step k is resumed by the compiled program, and the other way round, and the output and files equal an
//!   uninterrupted run, for every k, on every sample workflow.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn project(src: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!("wj_durable_c_{}_{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    let root = fs::canonicalize(&root).unwrap();
    fs::write(root.join("wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\n").unwrap();
    fs::write(root.join("src/main.wj"), src).unwrap();
    root
}

fn emitted(src: &str) -> (Output, String) {
    let p = project(src);
    let out = Command::new(WJ).args(["build", "--no-cargo", "-o", "out", "."]).current_dir(&p).output().unwrap();
    let rs = fs::read_to_string(p.join("out/main.rs")).unwrap_or_default();
    let _ = fs::remove_dir_all(&p);
    (out, rs)
}

fn sample(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/effects/durable_compiled").join(name)).unwrap()
}

const SAMPLES: [&str; 5] = ["c1_ledger.wj", "c2_pipeline.wj", "c3_counter.wj", "c4_report.wj", "c5_maybe.wj"];

#[test]
fn a_durable_fn_is_lowered_to_a_journaled_run_with_wrapped_steps() {
    let (o, rs) = emitted(&sample("c2_pipeline.wj"));
    assert!(o.status.success(), "{}", text(&o));
    assert!(!rs.contains("#[durable]"), "the marker is not a Rust attribute: {rs}");
    assert_eq!(rs.matches("windjammer_runtime::durable::run(").count(), 1, "{rs}");
    assert!(rs.contains(r#"windjammer_runtime::durable::run("run", vec![windjammer_runtime::durable::to_json(&seed)]"#), "{rs}");
    // three stages and three prints are steps; the plain `main` and the helper `stage` carry none
    assert_eq!(rs.matches("windjammer_runtime::durable::step(").count(), 6, "{rs}");
    assert!(rs.contains(r#"windjammer_runtime::durable::step("stage""#) && rs.contains(r#"windjammer_runtime::durable::step("println""#), "{rs}");
}

#[test]
fn a_program_without_a_durable_fn_has_no_journal_in_its_rust() {
    let (o, rs) = emitted("use std::fs\nfn main() {\n    println(\"${fs::exists(\"a\")}\")\n}\n");
    assert!(o.status.success(), "{}", text(&o));
    assert!(!rs.contains("durable::"), "{rs}");
}

#[test]
fn what_a_compiled_build_cannot_journal_yet_is_refused_with_the_reason() {
    let (o, _) = emitted("struct Point { x: int }\ndurable fn run(p: Point) -> int {\n    p.x\n}\nfn main() { println(\"${run(Point { x: 1 })}\") }\n");
    assert!(!o.status.success());
    let t = text(&o);
    assert!(t.contains("cannot journal yet") && t.contains("argument `p`") && t.contains("Point"), "{t}");
    // a pure helper that returns a struct is not a step: it is not journaled, so it is not refused
    let (o, _) = emitted("struct Point { x: int }\nfn make() -> Point { Point { x: 1 } }\ndurable fn run() -> int {\n    let p = make()\n    p.x\n}\nfn main() { println(\"${run()}\") }\n");
    assert!(o.status.success(), "{}", text(&o));
    let (o, _) = emitted("struct S { n: int }\nimpl S {\n    durable fn go(self) -> int { 1 }\n}\nfn main() {}\n");
    assert!(text(&o).contains("method"), "{}", text(&o));
}

// ------------------------------------------------------------------------------------------------ one journal, two implementations

fn listing(dir: &Path) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file() && e.file_name() != "main.wj")
        .map(|e| (e.file_name().to_string_lossy().to_string(), fs::read_to_string(e.path()).unwrap_or_default()))
        .collect();
    v.sort();
    v
}

fn interpreter(dir: &Path, journal: &Path, crash: Option<String>) -> (i32, String) {
    let mut args = vec!["run".to_string(), "--interpret".into(), "--no-lint".into(), "--journal".into(), journal.display().to_string()];
    if let Some(c) = crash {
        args.extend(["--crash-after".into(), c]);
    }
    args.push("main.wj".into());
    let o = Command::new(WJ).args(&args).current_dir(dir).env_remove("WJ_CAPS").output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).to_string())
}

fn compiled(bin: &Path, dir: &Path, journal: &Path, crash: Option<String>) -> (i32, String) {
    let mut c = Command::new(bin);
    c.current_dir(dir).env("WJ_JOURNAL_DIR", journal);
    if let Some(k) = crash {
        c.env("WJ_CRASH_AFTER", k);
    }
    let o = c.output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).to_string())
}

fn journal_steps(journal: &Path) -> usize {
    fs::read_dir(journal).map(|rd| rd.flatten().map(|e| fs::read_to_string(e.path()).unwrap_or_default().lines().filter(|l| l.contains("\"step\"")).count()).sum()).unwrap_or(0)
}

#[test]
fn one_journal_two_implementations() {
    if std::env::var("WJ_DURABLE_COMPILED").is_err() {
        eprintln!("WJ_DURABLE_COMPILED is not set: skipping the cross-implementation run (it builds the runtime crate)");
        return;
    }
    let mut failures = Vec::new();
    let mut points = 0;
    for name in SAMPLES {
        let src = sample(name);
        // build the program once
        let p = project(&src);
        let b = Command::new(WJ).args(["build", "-o", "out", "."]).current_dir(&p).output().unwrap();
        assert!(b.status.success(), "{name}: {}", text(&b));
        let bin = fs::read_dir(p.join("out/target/release")).map(|rd| rd.flatten().map(|e| e.path()).find(|f| f.is_file() && f.extension().is_none() && f.file_name().is_some_and(|n| n == "p"))).ok().flatten();
        let Some(bin) = bin else {
            failures.push(format!("{name}: no binary found under out/target/release"));
            continue;
        };
        let work = |tag: &str| {
            let d = tempfile::tempdir().unwrap();
            fs::write(d.path().join("main.wj"), &src).unwrap();
            let _ = tag;
            d
        };
        // the uninterrupted run, by the interpreter
        let clean = work("clean");
        let (code, want_out) = interpreter(clean.path(), &clean.path().join("j"), None);
        assert_eq!(code, 0, "{name}: {want_out}");
        let want_files: Vec<_> = listing(clean.path());
        let n = journal_steps(&clean.path().join("j"));
        for k in 1..=n {
            for (first_is_interpreter, label) in [(true, "interpreter then compiled"), (false, "compiled then interpreter")] {
                let d = work("x");
                let j = d.path().join("j");
                let (c1, o1) = if first_is_interpreter { interpreter(d.path(), &j, Some(k.to_string())) } else { compiled(&bin, d.path(), &j, Some(k.to_string())) };
                if c1 != 137 {
                    failures.push(format!("{name} {label} crash {k}: expected 137, got {c1}"));
                    continue;
                }
                let (c2, o2) = if first_is_interpreter { compiled(&bin, d.path(), &j, None) } else { interpreter(d.path(), &j, None) };
                points += 1;
                if c2 != 0 {
                    failures.push(format!("{name} {label} crash {k}: the resume failed with {c2}: {o2}"));
                    continue;
                }
                let got = format!("{o1}{o2}");
                if got != want_out {
                    failures.push(format!("{name} {label} crash {k}: output differs\n  want {want_out:?}\n  got  {got:?}"));
                }
                if listing(d.path()) != want_files {
                    failures.push(format!("{name} {label} crash {k}: files differ"));
                }
            }
        }
        let _ = fs::remove_dir_all(&p);
    }
    eprintln!("cross-implementation crash points: {points}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(points >= 40, "{points}");
}
