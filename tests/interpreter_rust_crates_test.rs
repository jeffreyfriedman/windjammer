//! ADR-0031: `wj run --interpret` calls Rust crate dependencies through the crate host.
//! Run: `cargo test --release --test interpreter_rust_crates`.
//!
//! Oracles are independent of the interpreter: the compiled build of the same project (rustc and the real crate),
//! and textbook values (levenshtein("kitten", "sitting") == 3, CRC-32 of "123456789" == 0xCBF43926,
//! jaro("martha", "marhta") == 17/18). Tests that must prove the host was *not* called (replay, a denied grant)
//! replace the host binary with one that cannot answer, in a private cache root (`WJ_CARGO_TARGET_ROOT`).
//! `WJ_INTEROP_TARGET_DIR` shares the compiled builds' cargo target directory between runs.

use std::path::PathBuf;
use std::process::{Command, Output};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Project {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    /// `Some`: a private wj cache root (its own crate hosts); `None`: the user's cache.
    cache: Option<PathBuf>,
    /// `--allow` grant for interpreted runs (`wj run` grants nothing outside the project by default).
    allow: Option<&'static str>,
}

fn text(o: &Output) -> String {
    format!("status {:?}\n--- stdout\n{}\n--- stderr\n{}", o.status.code(), String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

const SHAPES_LIB: &str = r#"
pub fn shout(s: &str) -> String { s.to_uppercase() }
pub fn bytes_len(b: &[u8]) -> usize { b.len() }
pub fn push_twice(v: &mut Vec<i64>, x: i64) { v.push(x); v.push(x); }
pub fn sum(v: &Vec<i64>) -> i64 { v.iter().sum() }
pub fn split(s: &str) -> Vec<String> { s.split(',').map(str::to_string).collect() }
pub fn parse(s: &str) -> Result<u32, String> { s.parse::<u32>().map_err(|e| e.to_string()) }
pub fn first(v: Vec<String>) -> Option<String> { v.into_iter().next() }
pub fn pair(a: u8, b: bool) -> (u8, bool) { (a, b) }
pub fn initial(s: &str) -> char { s.chars().next().unwrap_or('?') }
pub fn half(x: f32) -> f32 { x / 2.0 }
pub fn boom(s: &str) -> u32 { panic!("boom: {s}") }
pub fn generic<T: Clone>(x: T) -> T { x }
"#;

impl Project {
    fn new(deps: &str, main: &str) -> Project {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("app");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("wj.toml"), format!("[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\n{deps}")).unwrap();
        std::fs::write(root.join("src/main.wj"), main).unwrap();
        Project { _tmp: tmp, root, cache: None, allow: None }
    }

    /// A project with the `shapes` path crate next to it.
    fn shapes(main: &str, extra_manifest: &str) -> Project {
        let p = Project::new(&format!("shapes = {{ path = \"../shapes\" }}\n{extra_manifest}"), main);
        let c = p.root.parent().unwrap().join("shapes");
        std::fs::create_dir_all(c.join("src")).unwrap();
        std::fs::write(c.join("Cargo.toml"), "[package]\nname = \"shapes\"\nversion = \"0.3.0\"\nedition = \"2021\"\n").unwrap();
        std::fs::write(c.join("src/lib.rs"), SHAPES_LIB).unwrap();
        Project { allow: Some("ffi=shapes"), ..p }
    }

    fn private_cache(mut self) -> Project {
        let c = self.root.parent().unwrap().join("wjcache");
        std::fs::create_dir_all(&c).unwrap();
        self.cache = Some(c);
        self
    }

    fn wj(&self, args: &[&str]) -> Output {
        let mut c = Command::new(WJ);
        c.args(args).current_dir(&self.root).env_remove("WJ_CAPS");
        if let Some(cache) = &self.cache {
            c.env("WJ_CARGO_TARGET_ROOT", cache);
        }
        c.output().unwrap()
    }

    fn interp(&self, extra: &[&str]) -> Output {
        let mut args = vec!["run", "--interpret", "--no-lint"];
        if let Some(a) = self.allow {
            args.extend_from_slice(&["--allow", a]);
        }
        args.extend_from_slice(extra);
        self.wj(&args)
    }

    /// The compiled program's output: `wj build` + `cargo run` of the generated crate.
    fn compiled(&self) -> Output {
        let out = self.root.join("out");
        let o = self.wj(&["build", ".", "-o", out.to_str().unwrap(), "--no-cargo"]);
        assert!(o.status.success(), "{}", text(&o));
        let target = std::env::var_os("WJ_INTEROP_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| self.root.join("target"));
        Command::new("cargo")
            .args(["run", "--quiet", "--offline", "--manifest-path"])
            .arg(out.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &target)
            .env("CARGO_PROFILE_RELEASE_LTO", "false")
            .output()
            .unwrap()
    }

    /// The crate host binaries in the private cache.
    fn hosts(&self) -> Vec<PathBuf> {
        let dir = self.cache.as_ref().expect("private cache").join("host").join("crates");
        std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path().join("wj-crate-host")).filter(|p| p.is_file()).collect()).unwrap_or_default()
    }
}

/// Replace every host binary with one that cannot answer: a test that still passes never called it.
fn poison(hosts: &[PathBuf]) {
    assert!(!hosts.is_empty(), "no crate host was built");
    for h in hosts {
        std::fs::write(h, "#!/bin/sh\necho 'poisoned crate host ran' >&2\nexit 7\n").unwrap();
    }
}

/// The equivalence rule: same exit status and stdout, interpreted and compiled.
fn equivalent(interp: &Output, compiled: &Output) -> Result<(), String> {
    if interp.status.code() == compiled.status.code() && interp.stdout == compiled.stdout {
        Ok(())
    } else {
        Err(format!("interpreted and compiled differ\n=== interpreted\n{}\n=== compiled\n{}", text(interp), text(compiled)))
    }
}

const STRSIM_MAIN: &str = "\
use strsim
use strsim::jaro

fn main() {
    let a = \"kitten\"
    let b = \"sitting\"
    println(\"${strsim::levenshtein(a, b)}\")
    println(\"${jaro(\"martha\", \"marhta\")}\")
    let d = strsim::levenshtein(\"flaw\", \"lawn\")
    println(\"${d + 1}\")
}
";

/// levenshtein("kitten", "sitting") = 3, jaro("martha", "marhta") = 17/18, levenshtein("flaw", "lawn") + 1 = 3.
fn assert_textbook(out: &str) {
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert_eq!((lines[0], lines[2]), ("3", "3"), "{out}");
    let j: f64 = lines[1].parse().unwrap();
    assert!((j - 17.0 / 18.0).abs() < 1e-12, "{out}");
}

#[test]
fn strsim_interpreted_equals_compiled_and_the_textbook() {
    let p = Project::new("strsim = \"0.11\"\n", STRSIM_MAIN).private_cache();
    let t0 = std::time::Instant::now();
    let cold = p.interp(&[]);
    let cold_s = t0.elapsed();
    assert!(cold.status.success(), "{}", text(&cold));
    assert_textbook(&stdout(&cold));
    assert!(String::from_utf8_lossy(&cold.stderr).contains("building the crate host for strsim"), "{}", text(&cold));
    let t1 = std::time::Instant::now();
    let warm = p.interp(&[]);
    let warm_s = t1.elapsed();
    assert_eq!(stdout(&warm), stdout(&cold));
    assert!(!String::from_utf8_lossy(&warm.stderr).contains("building the crate host"), "{}", text(&warm));
    eprintln!("crate host strsim: cold {:.2}s, warm {:.3}s", cold_s.as_secs_f64(), warm_s.as_secs_f64());
    equivalent(&warm, &p.compiled()).unwrap();
}

const SHAPES_MAIN: &str = "\
use shapes

fn main() {
    println(shapes::shout(\"hi\"))
    let bytes: Vec<u8> = [1, 2, 3]
    println(\"${shapes::bytes_len(bytes)}\")
    let mut v: Vec<i64> = []
    shapes::push_twice(v, 21)
    println(\"${shapes::sum(v)} ${v.len()}\")
    let parts = shapes::split(\"a,b\")
    println(\"${parts.len()}\")
    match shapes::parse(\"42\") {
        Ok(n) => println(\"parsed ${n}\"),
        Err(e) => println(\"error ${e}\"),
    }
    match shapes::parse(\"x\") {
        Ok(n) => println(\"parsed ${n}\"),
        Err(e) => println(\"error ${e}\"),
    }
    match shapes::first(parts) {
        Some(s) => println(\"first ${s}\"),
        None => println(\"none\"),
    }
    let empty: Vec<string> = []
    match shapes::first(empty) {
        Some(s) => println(\"first ${s}\"),
        None => println(\"none\"),
    }
    let (a, b) = shapes::pair(7, true)
    println(\"${a} ${b}\")
    // a typed binding: an untyped `3.0` argument to an `f32` crate parameter is emitted as `3.0_f64` by the
    // compiled build (rustc E0308), see the report of ADR-0031
    let x: f32 = 3.0
    println(\"${shapes::initial(\"windjammer\")} ${shapes::half(x)}\")
}
";

#[test]
fn every_surface_shape_crosses_and_matches_the_compiled_build() {
    let p = Project::shapes(SHAPES_MAIN, "");
    let i = p.interp(&[]);
    assert!(i.status.success(), "{}", text(&i));
    assert_eq!(stdout(&i), "HI\n3\n42 2\n2\nparsed 42\nerror invalid digit found in string\nfirst a\nnone\n7 true\nw 1.5\n", "{}", text(&i));
    equivalent(&i, &p.compiled()).unwrap();
}

#[test]
fn a_second_crate_set_returns_option_vec_and_takes_bytes() {
    let main = "\
use shlex
use crc32fast

fn main() {
    match shlex::split(\"cp 'my file' dest\") {
        Some(words) => {
            for w in words {
                println(\"[${w}]\")
            }
        }
        None => println(\"unbalanced\"),
    }
    match shlex::split(\"'open\") {
        Some(_) => println(\"words\"),
        None => println(\"unbalanced\"),
    }
    let bytes: Vec<u8> = \"123456789\".as_bytes().to_vec()
    println(\"${crc32fast::hash(bytes)}\")
}
";
    let caps = "\n[capabilities]\nffi = [\"shlex\", \"crc32fast\"]\n";
    let p = Project::new(&format!("shlex = \"1.3\"\ncrc32fast = \"1.5\"\n{caps}"), main);
    let i = p.interp(&[]);
    assert!(i.status.success(), "{}", text(&i));
    // 0xCBF43926 is the CRC-32 check value of "123456789"
    assert_eq!(stdout(&i), "[cp]\n[my file]\n[dest]\nunbalanced\n3421780262\n", "{}", text(&i));
    equivalent(&i, &p.compiled()).unwrap();
}

#[test]
fn a_crate_panic_is_an_interpreter_panic_naming_the_function() {
    let main = "use shapes\n\nfn main() {\n    println(\"before\")\n    let n = shapes::boom(\"x\")\n    println(\"after ${n}\")\n}\n";
    let p = Project::shapes(main, "");
    let i = p.interp(&[]);
    let err = String::from_utf8_lossy(&i.stderr);
    assert_eq!(i.status.code(), Some(101), "{}", text(&i));
    assert_eq!(stdout(&i), "before\n");
    assert!(err.contains("Rust crate function `shapes::boom` panicked") && err.contains("boom: x"), "{}", text(&i));
    let c = p.compiled();
    assert_eq!(c.status.code(), Some(101), "{}", text(&c));
    assert_eq!(stdout(&c), "before\n");
}

#[test]
fn a_missing_grant_exits_126_before_any_host_call() {
    let main = "use shlex\n\nfn main() {\n    println(\"start\")\n    match shlex::split(\"a b\") {\n        Some(w) => println(\"${w.len()}\"),\n        None => println(\"none\"),\n    }\n}\n";
    let p = Project::new("shlex = \"1.3\"\n", main).private_cache();
    let ok = p.interp(&["--allow", "ffi=shlex"]);
    assert!(ok.status.success(), "{}", text(&ok));
    assert_eq!(stdout(&ok), "start\n2\n");
    poison(&p.hosts());
    // no [capabilities] table: `wj run` grants nothing outside the project, so ffi(shlex) is denied
    let denied = p.interp(&[]);
    let err = String::from_utf8_lossy(&denied.stderr);
    assert_eq!(denied.status.code(), Some(126), "{}", text(&denied));
    assert!(err.contains("capability denied") && err.contains("ffi"), "{}", text(&denied));
    assert!(!err.contains("poisoned") && !err.contains("crate host"), "the host ran before the gate: {}", text(&denied));
    // mutation check: with the grant the poisoned host *is* called, so the test above can see a call
    let called = p.interp(&["--allow", "ffi=shlex"]);
    assert_ne!(called.status.code(), Some(0), "{}", text(&called));
}

#[test]
fn replay_answers_crate_calls_from_the_log_without_the_host() {
    let p = Project::new("strsim = \"0.11\"\n", STRSIM_MAIN).private_cache();
    let rec = p.interp(&["--record", "run.wjlog"]);
    assert!(rec.status.success(), "{}", text(&rec));
    let log = std::fs::read_to_string(p.root.join("run.wjlog")).unwrap();
    assert!(log.contains("strsim::levenshtein") && log.contains("strsim::jaro"), "{log}");
    poison(&p.hosts());
    let rep = p.interp(&["--replay", "run.wjlog"]);
    assert!(rep.status.success(), "{}", text(&rep));
    assert_eq!(stdout(&rep), stdout(&rec));
    // and without the log the poisoned host is what answers
    let live = p.interp(&[]);
    assert!(!live.status.success(), "{}", text(&live));
}

/// A crate call inside a `durable fn` is a journaled step when its effect is `ffi(<crate>)` (a `@pure` crate function, such as
/// strsim's, performs nothing and is not a step): after a crash that came after the step, the resumed run takes the result from
/// the journal and never starts the crate host.
#[test]
fn a_crate_call_in_a_durable_fn_is_a_journaled_step_and_resumes_without_the_host() {
    let main = "use shapes\ndurable fn run() -> string {\n    let s = shapes::shout(\"abc\")\n    println(\"shouted ${s}\")\n    s\n}\nfn main() {\n    println(\"result ${run()}\")\n}\n";
    let p = Project::shapes(main, "").private_cache();
    let allow = ["--journal", "j"];
    // the clean run, to know the output
    let clean = p.interp(&[]);
    assert!(clean.status.success(), "{}", text(&clean));
    let want = stdout(&clean);
    assert!(want.contains("shouted ABC") && want.contains("result ABC"), "{want}");
    // crash right after the first step (the crate call) was journaled
    let mut crash = allow.to_vec();
    crash.extend_from_slice(&["--crash-after", "1"]);
    let first = p.interp(&crash);
    assert_eq!(first.status.code(), Some(137), "{}", text(&first));
    let journal = std::fs::read_dir(p.root.join("j")).unwrap().flatten().map(|e| std::fs::read_to_string(e.path()).unwrap()).collect::<String>();
    assert!(journal.contains("shout"), "the crate call is not a journaled step: {journal}");
    // resume with a host that cannot answer: the journaled result stands in for the call
    poison(&p.hosts());
    let resumed = p.interp(&allow);
    assert!(resumed.status.success(), "{}", text(&resumed));
    assert!(stdout(&resumed).contains("result ABC"), "{}", text(&resumed));
    assert!(!String::from_utf8_lossy(&resumed.stderr).contains("poisoned"), "the host ran on resume: {}", text(&resumed));
}

#[test]
fn simulate_runs_pure_functions_and_needs_a_stub_for_the_rest() {
    let pure = Project::new("strsim = \"0.11\"\n", STRSIM_MAIN);
    let s = pure.interp(&["--simulate"]);
    assert!(s.status.success(), "{}", text(&s));
    assert_textbook(&stdout(&s));
    let main = "use shlex\n\nfn main() {\n    match shlex::split(\"a b\") {\n        Some(w) => println(\"${w.len()} ${w[0]}\"),\n        None => println(\"none\"),\n    }\n}\n";
    let p = Project::new("shlex = \"1.3\"\n\n[capabilities]\nffi = [\"shlex\"]\n", main);
    let no_stub = p.interp(&["--simulate"]);
    assert_eq!(no_stub.status.code(), Some(2), "{}", text(&no_stub));
    assert!(String::from_utf8_lossy(&no_stub.stderr).contains("fixtures/rust_stubs.json"), "{}", text(&no_stub));
    std::fs::create_dir_all(p.root.join("fixtures")).unwrap();
    std::fs::write(p.root.join("fixtures/rust_stubs.json"), r#"{ "shlex::split": ["stub", "x", "y"] }"#).unwrap();
    let stubbed = p.interp(&["--simulate"]);
    assert!(stubbed.status.success(), "{}", text(&stubbed));
    assert_eq!(stdout(&stubbed), "3 stub\n");
}

/// Mutation check of the equivalence rule: a host that returns a wrong value for one function must be caught.
#[test]
fn a_host_returning_a_wrong_value_fails_the_equivalence_check() {
    let p = Project::new("strsim = \"0.11\"\n", STRSIM_MAIN).private_cache();
    let good = p.interp(&[]);
    assert!(good.status.success(), "{}", text(&good));
    let compiled = p.compiled();
    equivalent(&good, &compiled).unwrap();
    // rebuild the host with levenshtein off by one, announcing the same key
    let exe = p.hosts().pop().unwrap();
    let dir = exe.parent().unwrap().to_path_buf();
    let src = std::fs::read_to_string(dir.join("src/main.rs")).unwrap();
    let call = "let r: usize = ::strsim::levenshtein(&a0, &a1);";
    assert!(src.contains(call), "{src}");
    std::fs::write(dir.join("src/main.rs"), src.replace(call, &format!("{call} let r = r + 1;"))).unwrap();
    let target = dir.join("mutant-target");
    let b = Command::new("cargo").args(["build", "--release", "--quiet", "--offline"]).current_dir(&dir).env("CARGO_TARGET_DIR", &target).output().unwrap();
    assert!(b.status.success(), "{}", text(&b));
    let key = dir.file_name().unwrap().to_string_lossy().to_string();
    std::fs::copy(target.join("release").join(&key), &exe).unwrap();
    let _ = std::fs::remove_dir_all(&target);
    let bad = p.interp(&[]);
    let out = stdout(&bad);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!((lines[0], lines[2]), ("4", "4"), "{}", text(&bad));
    assert!(equivalent(&bad, &compiled).is_err(), "the equivalence check did not see the wrong value");
}

#[test]
fn a_crate_without_a_bridgeable_surface_still_falls_back() {
    // `wj run` (auto) runs compiled when a called function cannot cross; `--interpret` names it and exits 2
    let main = "use shapes\n\nfn main() {\n    println(\"${shapes::sum([1, 2])}\")\n}\n";
    let p = Project::shapes(main, "");
    std::fs::create_dir_all(p.root.join("rust")).unwrap();
    // a hand-written surface whose one function takes a map: it cannot cross to the host
    std::fs::write(p.root.join("rust/shapes.wji"), "pub fn sum(v: &Vec<i64>) -> i64\npub fn tally(m: HashMap<string, int>) -> int\n").unwrap();
    let i = p.interp(&[]);
    assert!(i.status.success(), "{}", text(&i));
    assert_eq!(stdout(&i), "3\n");
    let main2 = "use shapes\nuse std::collections::HashMap\n\nfn main() {\n    let m: HashMap<string, int> = HashMap::new()\n    println(\"${shapes::tally(m)}\")\n}\n";
    std::fs::write(p.root.join("src/main.wj"), main2).unwrap();
    let i = p.interp(&[]);
    assert_eq!(i.status.code(), Some(2), "{}", text(&i));
    assert!(String::from_utf8_lossy(&i.stderr).contains("`shapes::tally`"), "{}", text(&i));
}

// ---- the other interpreted entry points: `wj test --interpret`, `wj repl` ----

#[test]
fn wj_test_interpret_calls_crates_through_one_host() {
    let p = Project::new("strsim = \"0.11\"\n", "fn main() {}\n");
    std::fs::create_dir_all(p.root.join("tests")).unwrap();
    let tests = "\
use strsim

fn test_levenshtein_textbook() {
    assert_eq(strsim::levenshtein(\"kitten\", \"sitting\"), 3)
}

fn test_jaro_is_close_to_17_18() {
    let j = strsim::jaro(\"martha\", \"marhta\")
    assert(j > 0.944 && j < 0.945)
}
";
    std::fs::write(p.root.join("tests/distance_test.wj"), tests).unwrap();
    let o = p.wj(&["test", "--interpret", "tests"]);
    assert!(o.status.success(), "{}", text(&o));
    let out = text(&o);
    assert!(out.contains("test_levenshtein_textbook") && out.contains("test_jaro_is_close_to_17_18"), "{out}");
    // the oracle is not the host: a wrong expectation must fail
    std::fs::write(p.root.join("tests/distance_test.wj"), tests.replace("\"sitting\"), 3)", "\"sitting\"), 4)")).unwrap();
    let bad = p.wj(&["test", "--interpret", "tests"]);
    assert!(!bad.status.success(), "{}", text(&bad));
}

#[test]
fn the_repl_calls_crates() {
    let p = Project::new("strsim = \"0.11\"\n", "fn main() {}\n");
    let mut c = Command::new(WJ);
    c.arg("repl").current_dir(&p.root).env_remove("WJ_CAPS").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = c.spawn().unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(b"use strsim\nlet d = strsim::levenshtein(\"kitten\", \"sitting\")\nprintln(\"lev ${d}\")\n").unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(stdout(&o).contains("lev 3"), "{}", text(&o));
}

// ---- opaque crate values (handles) ----

const CTR_LIB: &str = r#"
use std::sync::atomic::{AtomicI64, Ordering};
static LIVE: AtomicI64 = AtomicI64::new(0);
pub struct Counter { n: i64 }
impl Drop for Counter {
    fn drop(&mut self) { LIVE.fetch_sub(1, Ordering::SeqCst); }
}
impl Counter {
    pub fn new(start: i64) -> Counter { LIVE.fetch_add(1, Ordering::SeqCst); Counter { n: start } }
    pub fn bump(&mut self) -> i64 { self.n += 1; self.n }
    pub fn get(&self) -> i64 { self.n }
}
pub fn total(c: &Counter, extra: i64) -> i64 { c.get() + extra }
/// Counters alive in this process: the release protocol keeps it at what the program still holds.
pub fn live() -> i64 { LIVE.load(Ordering::SeqCst) }
"#;

const CTR_SURFACE: &str = "\
pub struct Counter {}

impl Counter {
    pub fn new(start: i64) -> Counter
    pub fn bump(&mut self) -> i64
    pub fn get(&self) -> i64
}

pub fn total(c: &Counter, extra: i64) -> i64
pub fn live() -> i64
";

const CTR_MAIN: &str = "\
use ctr
use ctr::Counter

fn main() {
    let mut c = Counter::new(5)
    c.bump()
    let n = c.bump()
    println(\"${n} ${c.get()} ${ctr::total(c, 10)}\")
    let mut sum = 0
    let mut i = 0
    while i < 300 {
        let mut t = Counter::new(i)
        sum += t.bump()
        i += 1
    }
    println(\"${sum} ${c.get()} live ${ctr::live()}\")
}
";

fn ctr_project() -> Project {
    let p = Project::new("ctr = { path = \"../ctr\" }\n", CTR_MAIN);
    let c = p.root.parent().unwrap().join("ctr");
    std::fs::create_dir_all(c.join("src")).unwrap();
    std::fs::write(c.join("Cargo.toml"), "[package]\nname = \"ctr\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(c.join("src/lib.rs"), CTR_LIB).unwrap();
    std::fs::create_dir_all(p.root.join("rust")).unwrap();
    std::fs::write(p.root.join("rust/ctr.wji"), CTR_SURFACE).unwrap();
    Project { allow: Some("ffi=ctr"), ..p }
}

#[test]
fn opaque_values_live_in_the_host_and_match_the_compiled_build() {
    let p = ctr_project();
    let i = p.interp(&[]);
    assert!(i.status.success(), "{}", text(&i));
    // 5 -> 6 -> 7; total = 7 + 10; sum over i of (i + 1) for i < 300 = 45150
    assert_eq!(stdout(&i), "7 7 17\n45150 7 live 1\n", "{}", text(&i));
    equivalent(&i, &p.compiled()).unwrap();
}

#[test]
fn replay_answers_handle_calls_without_the_host() {
    let p = ctr_project().private_cache();
    let rec = p.interp(&["--record", "run.wjlog"]);
    assert!(rec.status.success(), "{}", text(&rec));
    poison(&p.hosts());
    let rep = p.interp(&["--replay", "run.wjlog"]);
    assert!(rep.status.success(), "{}", text(&rep));
    assert_eq!(stdout(&rep), stdout(&rec));
}

const REGEX_SURFACE: &str = "\
pub struct Regex {}
pub struct Error {}

impl Regex {
    pub fn new(re: string) -> Result<Regex, Error>
    pub fn is_match(&self, haystack: string) -> bool
    pub fn shortest_match(&self, haystack: string) -> Option<usize>
    pub fn captures_len(&self) -> usize
}

impl Error {
    pub fn to_string(&self) -> string
}
";

#[test]
fn regex_end_to_end_matches_the_compiled_build() {
    let main = "\
use regex
use regex::Regex

fn main() {
    match Regex::new(\"^([a-z]+)([0-9]{2})$\") {
        Ok(re) => {
            println(\"${re.is_match(\"abc12\")} ${re.is_match(\"abc1\")} ${re.captures_len()}\")
            match re.shortest_match(\"abc12\") {
                Some(n) => println(\"shortest ${n}\"),
                None => println(\"no match\"),
            }
        }
        Err(e) => println(\"error ${e.to_string()}\"),
    }
    match Regex::new(\"(unclosed\") {
        Ok(_) => println(\"compiled\"),
        Err(e) => println(\"rejected: ${e.to_string().len() > 0}\"),
    }
}
";
    let p = Project::new("regex = \"1\"\n", main);
    std::fs::create_dir_all(p.root.join("rust")).unwrap();
    std::fs::write(p.root.join("rust/regex.wji"), REGEX_SURFACE).unwrap();
    let p = Project { allow: Some("ffi=regex"), ..p };
    let i = p.interp(&[]);
    assert!(i.status.success(), "{}", text(&i));
    assert_eq!(stdout(&i), "true false 3\nshortest 5\nrejected: true\n", "{}", text(&i));
    equivalent(&i, &p.compiled()).unwrap();
}

#[test]
fn an_unknown_crate_in_the_repl_is_an_error_not_a_stack_overflow() {
    // `use name` of something that is neither a module nor an item used to import itself, so `name::f` recursed forever
    let tmp = tempfile::TempDir::new().unwrap();
    let mut c = Command::new(WJ);
    c.arg("repl").current_dir(tmp.path()).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = c.spawn().unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(b"use nosuchcrate\nnosuchcrate::f(1)\nprintln(\"alive\")\n").unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(stdout(&o).contains("alive") && String::from_utf8_lossy(&o.stderr).contains("nosuchcrate::f"), "{}", text(&o));
}
