//! Shared helpers for the optimizer test-suites: compile Rust with rustc, run
//! programs over seeds, and compare against independent expected outputs.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

pub const SEEDS: [u64; 5] = [1, 2, 3, 4, 5];

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/optimizer")
}

pub fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir())
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with('p') && n.ends_with(".rs"))
        .map(|n| n.trim_end_matches(".rs").to_string())
        .collect();
    names.sort();
    names
}

pub fn fixture_source(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(format!("{name}.rs"))).unwrap()
}

pub fn expected(name: &str) -> Vec<String> {
    std::fs::read_to_string(fixtures_dir().join(format!("{name}.expected")))
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Debug-profile build (overflow checks on) so arithmetic traps stay observable.
pub fn compile(src: &str, dir: &Path, name: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).unwrap();
    let rs = dir.join(format!("{name}.rs"));
    let bin = dir.join(name);
    std::fs::write(&rs, src).unwrap();
    let out = Command::new("rustc")
        .args(["--edition", "2021", "-C", "opt-level=0", "-A", "warnings", "-o"])
        .arg(&bin)
        .arg(&rs)
        .output()
        .expect("rustc available");
    if out.status.success() {
        Ok(bin)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).lines().take(6).collect::<Vec<_>>().join("\n"))
    }
}

/// Run `bin <seed>` for each seed; a crash is reported as `CRASH:<status>`.
pub fn run_seeds(bin: &Path, seeds: &[u64]) -> Vec<String> {
    seeds
        .iter()
        .map(|s| {
            let out = Command::new(bin).arg(s.to_string()).output().unwrap();
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).trim_end().to_string()
            } else {
                format!("CRASH:{}", out.status)
            }
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    CompileError(String),
    Mismatch { got: Vec<String>, want: Vec<String> },
}

pub fn check_source(src: &str, want: &[String], dir: &Path, name: &str) -> Verdict {
    match compile(src, dir, name) {
        Err(e) => Verdict::CompileError(e),
        Ok(bin) => {
            let got = run_seeds(&bin, &SEEDS[..want.len().min(SEEDS.len())]);
            if got == want {
                Verdict::Pass
            } else {
                Verdict::Mismatch { got, want: want.to_vec() }
            }
        }
    }
}

pub fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wj-opt-test-{}-{}", tag, std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run `f` over `items` on up to 4 threads, preserving order.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<Option<R>>> = std::sync::Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..4.min(items.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if i >= items.len() {
                    break;
                }
                let r = f(&items[i]);
                results.lock().unwrap()[i] = Some(r);
            });
        }
    });
    results.into_inner().unwrap().into_iter().map(Option::unwrap).collect()
}
