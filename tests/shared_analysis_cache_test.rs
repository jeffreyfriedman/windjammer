//! The cross-checkout analysis cache (`src/check/shared_cache.rs`, store in `src/cargo_cache/entries.rs`): keys are
//! content hashes, so the same file in two checkouts hits; a different compiler build never hits; concurrent writers
//! leave whole entries; `wj cache prune` leaves a directory alone while a writer holds it; and the cache never
//! changes diagnostics (cached == uncached over the inference corpus and the examples).
//! Run: cargo test --release --test shared_analysis_cache_test

#[path = "common/check_cache/support.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use support::{diags, in_dir, report, write};
use windjammer::cargo_cache::entries::{self, AnalysisStore};
use windjammer::check::shared_cache::{self, SharedStore};

/// The shared configuration is process-wide: tests that install a store take turns.
static INSTALL: Mutex<()> = Mutex::new(());

/// A store that counts the lookups that found an entry.
struct Dir(AnalysisStore);

static FOUND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl SharedStore for Dir {
    fn get(&self, key: &str) -> Option<String> {
        let got = self.0.get(key);
        if got.is_some() {
            FOUND.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        got
    }
    fn put(&self, key: &str, text: &str) {
        self.0.put(key, text)
    }
}

struct Installed<'a>(#[allow(dead_code)] std::sync::MutexGuard<'a, ()>);

impl Drop for Installed<'_> {
    fn drop(&mut self) {
        shared_cache::uninstall();
    }
}

fn install(dir: &Path, compiler: &str) -> Installed<'static> {
    let g = INSTALL.lock().unwrap_or_else(|e| e.into_inner());
    shared_cache::install_for(Arc::new(Dir(AnalysisStore { dir: dir.to_path_buf() })), compiler.to_string());
    Installed(g)
}

const TOML: &str = "[package]\nname = \"p\"\nversion = \"0.1.0\"\n";

fn project(main: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), &[("wj.toml".into(), TOML.into()), ("src/main.wj".into(), main.into())]);
    d
}

fn hits() -> usize {
    FOUND.load(std::sync::atomic::Ordering::Relaxed)
}

fn entry_count(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |rd| rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "json")).count())
}

fn corpus() -> Vec<PathBuf> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pool = Vec::new();
    for dir in ["tests/inference_corpus", "examples/verified", "examples"] {
        let mut v: Vec<_> = std::fs::read_dir(repo.join(dir)).unwrap().flatten().map(|e| e.path()).collect();
        v.retain(|p| p.extension().is_some_and(|e| e == "wj"));
        v.sort();
        pool.extend(v);
    }
    pool.dedup();
    pool
}

/// Two checkouts of one program: the second finds the first's analysis, reports exactly what an uncached run
/// reports (its own file names included), over the inference corpus and the examples.
#[test]
fn second_checkout_hits_and_matches_uncached() {
    let store = tempfile::tempdir().unwrap();
    let _i = install(store.path(), "build-A");
    let pool = corpus();
    assert!(pool.len() > 100, "corpus present: {}", pool.len());
    let (mut hit_programs, mut with_paths) = (0, 0);
    for p in &pool {
        let text = std::fs::read_to_string(p).unwrap();
        let (a, b) = (project(&text), project(&text));
        let want_a = in_dir(a.path(), || diags(&report(&a.path().join("src/main.wj"), false)));
        let got_a = in_dir(a.path(), || diags(&report(&a.path().join("src/main.wj"), true)));
        assert_eq!(got_a, want_a, "first checkout, {}", p.display());
        let before = hits();
        let main_b = b.path().join("src/main.wj");
        let want_b = in_dir(b.path(), || diags(&report(&main_b, false)));
        let got_b = in_dir(b.path(), || diags(&report(&main_b, true)));
        assert_eq!(got_b, want_b, "second checkout (cached) != uncached: {}", p.display());
        hit_programs += (hits() > before) as usize;
        with_paths += (hits() > before && want_b.contains(&b.path().display().to_string())) as usize;
        assert!(!got_b.contains(&a.path().display().to_string()), "second checkout names the first checkout's path: {}", p.display());
    }
    eprintln!("shared analysis cache: {} programs, {hit_programs} hit in the second checkout, {with_paths} of them with diagnostics naming the file", pool.len());
    assert!(hit_programs * 2 > pool.len(), "the shared cache must be exercised: {hit_programs}/{}", pool.len());
}

/// A different compiler build never reads another build's entries. Mutation control: the same entries are read by
/// the same build, so a key that ignored the compiler would make the second assertion fail.
#[test]
fn compiler_change_invalidates() {
    let store = tempfile::tempdir().unwrap();
    let src = "fn main() {\n    let x = 1\n    println(\"${x}\")\n}\n";
    let (a, b, c) = (project(src), project(src), project(src));
    let run = |d: &tempfile::TempDir| {
        let main = d.path().join("src/main.wj");
        let before = hits();
        in_dir(d.path(), || report(&main, true));
        hits() > before
    };
    {
        let _i = install(store.path(), "build-A");
        assert!(!run(&a), "empty store: miss");
        assert!(entry_count(store.path()) > 0, "analysis stored");
        assert!(run(&b), "same build, other checkout: hit");
    }
    {
        let _i = install(store.path(), "build-B");
        assert!(!run(&c), "another compiler build must not hit the entries of build-A");
    }
    let k = |compiler: &str| shared_cache::key(compiler, "amb", "t");
    assert_ne!(k("build-A"), k("build-B"));
    assert_eq!(k("build-A"), k("build-A"));
    assert_ne!(shared_cache::key("c", "amb", "t"), shared_cache::key("c", "amb2", "t"), "ambient sidecars are part of the key");
    assert_ne!(shared_cache::key("c", "a", "t"), shared_cache::key("c", "a", "u"), "source text is part of the key");
}

/// The key names no path: a program checked from two directories with different names has one entry.
#[test]
fn entries_do_not_depend_on_the_checkout_path() {
    let store = tempfile::tempdir().unwrap();
    let _i = install(store.path(), "build-A");
    let src = "fn add(a: int, b: int) -> int {\n    a + b\n}\n\nfn main() {\n    println(\"${add(1, 2)}\")\n}\n";
    for d in [project(src), project(src)] {
        let main = d.path().join("src/main.wj");
        in_dir(d.path(), || report(&main, true));
    }
    assert_eq!(entry_count(store.path()), 1, "one entry for one content");
}

/// Writers race on the same and on different keys: every entry read back is whole.
#[test]
fn concurrent_writers_leave_whole_entries() {
    let dir = tempfile::tempdir().unwrap();
    let keys: Vec<String> = (0..8).map(|i| shared_cache::key("c", "a", &i.to_string())).collect();
    let body = |i: usize| format!("{{\"n\":{i},\"pad\":\"{}\"}}", "x".repeat(50_000));
    std::thread::scope(|s| {
        for t in 0..8 {
            let (dir, keys) = (dir.path(), &keys);
            s.spawn(move || {
                let store = AnalysisStore { dir: dir.to_path_buf() };
                for round in 0..40 {
                    let i = (t + round) % keys.len();
                    store.put(&keys[i], &body(i));
                    if let Some(got) = store.get(&keys[(i + 3) % keys.len()]) {
                        let v: serde_json::Value = serde_json::from_str(&got).expect("a reader never sees half an entry");
                        assert_eq!(got, body(v["n"].as_u64().unwrap() as usize));
                    }
                }
            });
        }
    });
    for (i, k) in keys.iter().enumerate() {
        assert_eq!(AnalysisStore { dir: dir.path().to_path_buf() }.get(k).as_deref(), Some(body(i).as_str()));
    }
    let leftovers = std::fs::read_dir(dir.path()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains(".tmp")).count();
    assert_eq!(leftovers, 0, "no temporary files left behind");
}

/// `wj cache prune` drops old entries, but not from a directory a writer holds, and says so.
#[test]
fn prune_is_lock_aware() {
    let dir = tempfile::tempdir().unwrap();
    let store = AnalysisStore { dir: dir.path().to_path_buf() };
    let key = shared_cache::key("c", "a", "old");
    store.put(&key, "{}");
    let old = std::time::SystemTime::now() - entries::MAX_AGE - std::time::Duration::from_secs(3600);
    std::fs::File::options().write(true).open(dir.path().join(format!("{key}.json"))).unwrap().set_modified(old).unwrap();
    {
        let _writer = windjammer::cargo_cache::active::hold_path(dir.path()).expect("lock");
        let mut report = windjammer::cargo_cache::PruneReport::default();
        entries::prune(dir.path(), 100, false, &mut report);
        assert_eq!(entry_count(dir.path()), 1, "held directory is not pruned");
        assert!(report.messages.iter().any(|m| m.contains("being written")), "{:?}", report.messages);
    }
    let mut report = windjammer::cargo_cache::PruneReport::default();
    entries::prune(dir.path(), 100, false, &mut report);
    assert_eq!(entry_count(dir.path()), 0, "released directory: the old entry goes");
    store.put(&key, "{}");
    entries::prune(dir.path(), 100, false, &mut report);
    assert_eq!(entry_count(dir.path()), 1, "a fresh entry stays");
}
