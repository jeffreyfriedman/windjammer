//! Scratch-directory hygiene: harnesses must not leak `wj-*` directories into the OS temp dir.
//!
//! * Static gate: `std::env::temp_dir()` uses under `tests/` (outside `tests/common/scratch.rs`) and
//!   `mktemp` / `/tmp/wj-` uses under `scripts/` are counted per file and compared with
//!   `docs/testing/tmp-allowlist.txt`. A new use fails; a removed use also fails until the
//!   allowlist is shrunk (ratchet). New harness code uses `tests/common/scratch.rs`.
//! * End-to-end gate: converted tests run in a child process with a private `TMPDIR`; afterwards the
//!   directory must hold no `wj-*` entry (and with `WJ_KEEP_TMP=1` it must, so the check is not vacuous).
//! * Guard behaviour: removal on panic unwinding, `WJ_KEEP_TMP=1` opt-out.
//! * Janitor: `wj clean --tmp` is a dry run by default, honours `--yes`, age and live pids.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const ALLOWLIST: &str = "docs/testing/tmp-allowlist.txt";

fn repo() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name != "target" && name != "node_modules" && name != "drift_corpus" && !name.starts_with('.') {
                walk(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// Number of leak-prone temp uses in `text` of file `rel`.
fn violations_in(rel: &str, text: &str) -> usize {
    let is_rs = rel.ends_with(".rs");
    let has_trap = text.contains("trap ") || text.contains("atexit");
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .filter(|l| {
            if is_rs {
                l.contains("std::env::temp_dir()")
            } else {
                l.contains("/tmp/wj") || (l.contains("mktemp") && !has_trap)
            }
        })
        .count()
}

fn scan(root: &Path) -> BTreeMap<String, usize> {
    let mut files = Vec::new();
    walk(&root.join("tests"), &mut files);
    walk(&root.join("scripts"), &mut files);
    let mut out = BTreeMap::new();
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if rel == "tests/common/scratch.rs" || rel == "tests/tmp_hygiene_test.rs" || rel.contains("/__pycache__/") {
            continue;
        }
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        let in_scope = (rel.starts_with("tests/") && ext == "rs") || (rel.starts_with("scripts/") && ["sh", "wj", "py"].contains(&ext));
        if !in_scope {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let n = violations_in(&rel, &text);
        if n > 0 {
            out.insert(rel, n);
        }
    }
    out
}

fn allowlist() -> BTreeMap<String, usize> {
    let text = std::fs::read_to_string(repo().join(ALLOWLIST)).expect("allowlist");
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (p, n) = l.split_once('\t').expect("path<TAB>count");
            (p.to_string(), n.trim().parse().unwrap())
        })
        .collect()
}

#[test]
fn temp_dir_uses_outside_the_helper_match_the_shrink_only_allowlist() {
    let found = scan(repo());
    let allowed = allowlist();
    let mut problems = Vec::new();
    for (f, n) in &found {
        match allowed.get(f) {
            None => problems.push(format!("NEW leak-prone temp use in {f} ({n}): use tests/common/scratch.rs (Scratch::new) or a trap in scripts")),
            Some(a) if n > a => problems.push(format!("{f}: {n} temp uses, allowlist permits {a}")),
            Some(a) if n < a => problems.push(format!("{f}: now {n} (allowlist says {a}): shrink {ALLOWLIST}")),
            _ => {}
        }
    }
    for f in allowed.keys().filter(|f| !found.contains_key(*f)) {
        problems.push(format!("{f}: no longer has temp uses: remove it from {ALLOWLIST}"));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn detector_reports_planted_violations() {
    let bad_rs = "fn t() {\n    let d = std::env::temp_dir().join(\"wj-x\");\n}\n";
    assert_eq!(violations_in("tests/a.rs", bad_rs), 1);
    assert_eq!(violations_in("tests/a.rs", "// std::env::temp_dir() in a comment\n"), 0);
    assert_eq!(violations_in("scripts/a.sh", "d=$(mktemp -d)\n"), 1);
    assert_eq!(violations_in("scripts/a.sh", "d=$(mktemp -d); trap 'rm -rf $d' EXIT\n"), 0);
    assert_eq!(violations_in("scripts/a.wj", "let d = \"/tmp/wj-bench\"\n"), 1);
    assert!(scan(repo()).len() >= 3, "scan found nothing: the gate would be vacuous");
}

// ---------------------------------------------------------------------------------------------
// End to end: run the converted tests with a private TMPDIR
// ---------------------------------------------------------------------------------------------

const CONVERTED: &[&str] = &[
    "lint_modes_agree_test::",
    "interpolation_nested_quotes_test::",
    "stdin_modes_test::",
    "toolchain_stale_test::a_binary_younger_than_thirty_days_pays_almost_nothing",
    "backend_conformance_test::cli_build_produces_ready_to_run_directories",
    "tmp_hygiene_test::planted_scratch_user",
];

/// Deliberate shared caches `wj` keeps in the temp dir (not harness scratch).
const WJ_OWN_CACHES: &[&str] = &["wj-embedded", "wj-meta"];

fn wj_dirs(tmp: &Path) -> Vec<String> {
    std::fs::read_dir(tmp)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("wj-") && !WJ_OWN_CACHES.contains(&n.as_str()))
        .collect()
}

/// Re-run this very test binary on `filters` with `TMPDIR` pointing at a fresh directory.
fn run_child(filters: &[&str], keep: bool, ignored: bool) -> (std::process::Output, Vec<String>) {
    let tmp = tempfile::tempdir().unwrap();
    let mut c = Command::new(std::env::current_exe().unwrap());
    c.args(filters).arg("--nocapture").env("TMPDIR", tmp.path()).env_remove("WJ_KEEP_TMP");
    if ignored {
        c.arg("--ignored");
    }
    if keep {
        c.env("WJ_KEEP_TMP", "1");
    }
    let out = c.output().unwrap();
    (out, wj_dirs(tmp.path()))
}

/// Child-only: holds a scratch directory and panics (run via `--ignored` by the tests below).
#[test]
#[ignore]
fn planted_scratch_user() {
    let s = Scratch::new("planted");
    std::fs::write(s.join("f"), "x").unwrap();
    if std::env::var("WJ_PLANTED_PANIC").is_ok() {
        panic!("planted panic with a live scratch dir");
    }
}

#[test]
fn converted_tests_leave_no_wj_dirs_in_tmpdir() {
    let (out, left) = run_child(CONVERTED, false, false);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "child run failed:\n{text}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(text.contains("test result: ok"), "{text}");
    assert!(left.is_empty(), "leaked into TMPDIR: {left:?}");
}

#[test]
fn keep_flag_keeps_dirs_so_the_leak_check_is_not_vacuous() {
    let (out, kept) = run_child(CONVERTED, true, false);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    assert!(kept.len() >= 3, "the converted tests created too few scratch dirs: {kept:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("WJ_KEEP_TMP=1: keeping"), "kept path must be printed:\n{err}");
}

#[test]
fn scratch_is_removed_while_a_panic_unwinds() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["tmp_hygiene_test::planted_scratch_user", "--ignored"])
        .env("TMPDIR", tmp.path())
        .env("WJ_PLANTED_PANIC", "1")
        .env_remove("WJ_KEEP_TMP")
        .output()
        .unwrap();
    assert!(!out.status.success(), "the planted test must panic");
    assert!(String::from_utf8_lossy(&out.stdout).contains("planted panic"));
    assert!(wj_dirs(tmp.path()).is_empty(), "scratch survived the panic: {:?}", wj_dirs(tmp.path()));
}

// ---------------------------------------------------------------------------------------------
// `wj clean --tmp`
// ---------------------------------------------------------------------------------------------

fn age(path: &Path, hours: u32) {
    let st = Command::new("touch").args(["-t", &old_stamp(hours)]).arg(path).status().unwrap();
    assert!(st.success());
}

fn old_stamp(hours: u32) -> String {
    let out = Command::new("date").args(["-u", "-v", &format!("-{hours}H"), "+%Y%m%d%H%M"]).output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => {
            let o = Command::new("date").args(["-u", "-d", &format!("{hours} hours ago"), "+%Y%m%d%H%M"]).output().unwrap();
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
    }
}

fn clean(tmp: &Path, extra: &[&str]) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["clean", "--tmp"]).args(extra).env("TMPDIR", tmp).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).to_string()
}

#[test]
fn clean_tmp_is_a_dry_run_and_spares_young_live_and_foreign_dirs() {
    let t = tempfile::tempdir().unwrap();
    let mk = |n: &str| {
        let d = t.path().join(n);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("f"), vec![0u8; 2048]).unwrap();
        d
    };
    let stale = mk("wj-conformance-999999-corpus");
    let young = mk("wj-conformance-999998-corpus");
    let live = mk(&format!("wj-conformance-{}-corpus", std::process::id()));
    let foreign = mk("other-999997");
    for d in [&stale, &live, &foreign] {
        age(d, 12);
    }

    let dry = clean(t.path(), &[]);
    assert!(dry.contains("would remove") && dry.contains("wj-conformance-999999-corpus"), "{dry}");
    assert!(dry.contains("dry run"), "{dry}");
    assert!(stale.exists(), "dry run must not delete");

    let done = clean(t.path(), &["--yes"]);
    assert!(done.contains("removed 1 dir"), "{done}");
    assert!(!stale.exists(), "stale dir with a dead pid must go");
    assert!(young.exists(), "younger than 6h stays");
    assert!(live.exists(), "owner pid is alive (this test process)");
    assert!(foreign.exists(), "only wj-* directories are touched");

    age(&young, 8);
    let custom = clean(t.path(), &["--older-than-hours", "24", "--yes"]);
    assert!(young.exists(), "8h old is younger than a 24h threshold: {custom}");
}

#[test]
fn doctor_mentions_stale_scratch_dirs() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join("wj-lintmodes-999999-5");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("f"), vec![0u8; 4096]).unwrap();
    age(&d, 12);
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).arg("doctor").env("TMPDIR", t.path()).output().unwrap();
    let text = String::from_utf8_lossy(&o.stdout);
    assert!(text.contains("1 stale wj-* scratch dir(s)") && text.contains("wj clean --tmp"), "{text}");
}
