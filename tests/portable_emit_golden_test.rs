//! G2 of docs/design/table-driven-std-surface.md: the JavaScript, TypeScript and Go output of every conformance
//! program and every spec run case is pinned by hash in `tests/golden/portable_emit_hashes.tsv`.
//!
//! The table-driven std surface moves std method knowledge out of the three portable emitters in stages. A stage
//! is a refactor: the emitted program must not change by one byte, so these hashes must be identical on the parent
//! commit and on every stage commit. A build that fails is part of the record (the hash is that of the error text
//! with the work directory removed), so a program that gains or loses support shows up as a changed row too.
//!
//! The corpus is the one `scripts/wj/target_tiers.wj` measures: `tests/conformance/**/*.wj` and the spec's `run` /
//! `panic` cases (`tests/spec/**/*.wj` without an `xfail` / `manifest` / `profile` / `seed` / `faults` header).
//!
//! Re-record only for a cause named in the commit (`golden-rerecord-by-cause`):
//! `WJ_RECORD_GOLDEN=1 cargo test --release --test all portable_emit_golden_test`. All 259 programs on three
//! targets take a few seconds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const GOLDEN: &str = "tests/golden/portable_emit_hashes.tsv";
const TARGETS: [(&str, &str); 3] = [("js", "javascript"), ("ts", "typescript"), ("go", "go")];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
}

/// Whether a spec file is a run / panic case with an output to compare (the filter of `target_tiers.wj`).
fn spec_comparable(text: &str) -> bool {
    let mut mode = "run".to_string();
    for line in text.lines() {
        if !line.starts_with("//") {
            break;
        }
        if let Some(v) = line.strip_prefix("// mode:") {
            mode = v.trim().to_string();
        }
        if ["// xfail:", "// manifest:", "// profile:", "// seed:", "// faults:"].iter().any(|p| line.starts_with(p)) {
            return false;
        }
    }
    mode == "run" || mode == "panic"
}

/// `(name, path)` of every program, sorted: `conformance/<rel>` and `spec/<rel>` without the extension.
fn corpus() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    for (suite, comparable_only) in [("conformance", false), ("spec", true)] {
        let base = root().join("tests").join(suite);
        let mut files = Vec::new();
        collect(&base, &mut files);
        for f in files {
            if comparable_only && !spec_comparable(&std::fs::read_to_string(&f).unwrap_or_default()) {
                continue;
            }
            let rel = f.strip_prefix(&base).unwrap().with_extension("");
            v.push((format!("{suite}/{}", rel.to_string_lossy()), f));
        }
    }
    v
}

/// A private build directory, removed when dropped (a build keeps cache metadata next to its project root).
struct Work(PathBuf);

impl Work {
    fn new() -> Work {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("wj-emit-golden-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&dir).unwrap();
        Work(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The hash of everything one build wrote (sorted file names with content hashes), or of its error text.
fn emit_hash(wj: &Path, src: &Path, target: &str) -> String {
    let work = Work::new();
    let name = src.file_name().unwrap().to_owned();
    std::fs::copy(src, work.path().join(&name)).unwrap();
    let out = work.path().join("out");
    let run = Command::new(wj)
        .args(["build".as_ref(), "--no-lint".as_ref(), name.as_os_str(), "-t".as_ref(), target.as_ref(), "-o".as_ref(), out.as_os_str()])
        .current_dir(work.path())
        .output()
        .expect("wj runs");
    if !run.status.success() {
        let text = String::from_utf8_lossy(&run.stderr).replace(work.path().to_string_lossy().as_ref(), "<work>");
        let first = text.lines().find(|l| l.contains("error") || l.contains("Error")).unwrap_or("").to_string();
        return format!("FAILED-{:016x}", fnv(first.as_bytes()));
    }
    let mut files: BTreeMap<String, u64> = BTreeMap::new();
    let mut stack = vec![out.clone()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(&out).unwrap().to_string_lossy().into_owned();
                files.insert(rel, fnv(&std::fs::read(&p).unwrap()));
            }
        }
    }
    let folded = files.iter().fold(String::new(), |a, (k, v)| format!("{a}{k}:{v:016x};"));
    format!("{:016x}", fnv(folded.as_bytes()))
}

/// One row per program: `name <TAB> js <TAB> ts <TAB> go`.
fn measure(programs: &[(String, PathBuf)]) -> BTreeMap<String, Vec<String>> {
    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let next = AtomicUsize::new(0);
    let rows: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());
    let workers = std::thread::available_parallelism().map(|n| n.get().min(6)).unwrap_or(2);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some((name, path)) = programs.get(i) else { break };
                let hashes: Vec<String> = TARGETS.iter().map(|(_, t)| emit_hash(&wj, path, t)).collect();
                rows.lock().unwrap().insert(name.clone(), hashes);
            });
        }
    });
    rows.into_inner().unwrap()
}

fn render(rows: &BTreeMap<String, Vec<String>>) -> String {
    let mut out = String::from("# program <TAB> js <TAB> ts <TAB> go: hash of every file `wj build -t <target>` writes (FAILED-<hash> of the first error line when the build fails). tests/portable_emit_golden_test.rs; re-record only for a named cause.\n");
    for (name, h) in rows {
        out.push_str(&format!("{name}\t{}\n", h.join("\t")));
    }
    out
}

fn recorded() -> BTreeMap<String, Vec<String>> {
    let text = std::fs::read_to_string(root().join(GOLDEN)).unwrap_or_default();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .filter_map(|l| {
            let mut it = l.split('\t');
            let name = it.next()?.to_string();
            Some((name, it.map(str::to_string).collect()))
        })
        .collect()
}

fn compare(programs: &[(String, PathBuf)]) {
    let want = recorded();
    assert!(!want.is_empty(), "{GOLDEN} is empty: record it on the parent commit first");
    let got = measure(programs);
    let mut bad = Vec::new();
    for (name, hashes) in &got {
        let rec = want.get(name);
        if rec != Some(hashes) {
            let cols: Vec<String> = TARGETS
                .iter()
                .enumerate()
                .filter(|(i, _)| rec.and_then(|r| r.get(*i)) != hashes.get(*i))
                .map(|(_, (n, _))| n.to_string())
                .collect();
            bad.push(format!("{name}: {} changed", if rec.is_none() { "unrecorded".to_string() } else { cols.join("+") }));
        }
    }
    assert!(bad.is_empty(), "portable emit output changed (a std-surface stage must emit byte-identical output):\n{}", bad.join("\n"));
}

#[test]
fn program_list_matches_the_recorded_rows() {
    let have: Vec<String> = corpus().into_iter().map(|(n, _)| n).collect();
    let want: Vec<String> = recorded().into_keys().collect();
    let missing: Vec<&String> = have.iter().filter(|n| !want.contains(n)).collect();
    let gone: Vec<&String> = want.iter().filter(|n| !have.contains(n)).collect();
    assert!(missing.is_empty() && gone.is_empty(), "corpus and {GOLDEN} disagree: unrecorded {missing:?}, no longer a program {gone:?}");
    assert!(have.len() >= 200, "corpus unexpectedly small: {}", have.len());
}

#[test]
fn every_program_emits_the_recorded_output() {
    let programs = corpus();
    if std::env::var_os("WJ_RECORD_GOLDEN").is_some() {
        std::fs::write(root().join(GOLDEN), render(&measure(&programs))).unwrap();
        return;
    }
    compare(&programs);
}

/// Every method of `trait Float<T>` (on `f64` and `f32`), of `trait Integer<T>` (on every integer type) and of `trait Char` /
/// `trait String`, `trait Option` / `trait Result`: the JavaScript
/// and the Go text, compared whole (not by hash) with what the emitters printed before those rows moved into
/// `std/rust_core.wji`. A difference names the first differing line, which is the row to look at (`f32` is the Go widening,
/// the kind is the integer function).
#[test]
fn rows_emit_what_the_emitters_printed_before_they_were_rows() {
    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let work = Work::new();
    let mut bad = Vec::new();
    for probe in ["float_rows_probe", "integer_rows_probe", "char_string_rows_probe", "option_result_rows_probe", "map_set_static_rows_probe", "vec_iter_rows_probe", "module_fns_probe"] {
        std::fs::copy(root().join(format!("tests/golden/{probe}.wj")), work.path().join(format!("{probe}.wj"))).unwrap();
        for (target, out_file, ext) in [("javascript", "main.mjs", "js"), ("go", "main.go", "go")] {
            let out = work.path().join(format!("out_{probe}_{target}"));
            let run = Command::new(&wj)
                .args(["build", "--no-lint", &format!("{probe}.wj"), "-t", target, "-o"])
                .arg(&out)
                .current_dir(work.path())
                .output()
                .expect("wj runs");
            assert!(run.status.success(), "{probe} {target}: {}", String::from_utf8_lossy(&run.stderr));
            let got = std::fs::read_to_string(out.join(out_file)).unwrap();
            let want = std::fs::read_to_string(root().join(format!("tests/golden/{probe}.{ext}.txt"))).unwrap();
            if got != want {
                let line = got.lines().zip(want.lines()).position(|(a, b)| a != b).map_or(got.lines().count().min(want.lines().count()), |i| i);
                bad.push(format!("{probe} {target}: line {}: emitted `{}`, recorded `{}`", line + 1, got.lines().nth(line).unwrap_or(""), want.lines().nth(line).unwrap_or("")));
            }
        }
    }
    assert!(bad.is_empty(), "the rows changed the emitted program:\n{}", bad.join("\n"));
}
