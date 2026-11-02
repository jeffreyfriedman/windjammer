//! Generated-Rust golden gate over the verified example corpus (`examples/verified/*.wj`).
//!
//! `tests/golden/build_output_hashes.tsv` records an FNV-1a hash of every generated `.rs` file per example
//! (`wj build --no-cargo`). The build-speed work changes how cargo is driven, never what is generated, so
//! these hashes must not move. A sampled subset (every 12th example) runs on every commit; the full corpus
//! runs with `--ignored`. After an intentional codegen change, re-record with `WJ_RECORD_GOLDEN=1`:
//! `WJ_RECORD_GOLDEN=1 cargo test --release --test build_output_golden -- --ignored`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn corpus() -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(root().join("examples/verified"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wj"))
        .collect();
    v.sort();
    v
}

/// Hash of all generated Rust for one example: sorted `(file name, content hash)` pairs, folded.
fn generated_hash(wj: &Path, src: &Path) -> Option<String> {
    // Each example is built alone in a private directory: the compiler keeps `.wj-cache` metadata next to
    // the project root, and builds that share a root influence each other's output.
    let work = tempfile::tempdir().unwrap();
    let name = src.file_name()?.to_owned();
    std::fs::copy(src, work.path().join(&name)).ok()?;
    let out = work.path().join("out");
    let st = Command::new(wj)
        .args(["build".as_ref(), name.as_os_str(), "-o".as_ref(), out.as_os_str(), "--no-cargo".as_ref()])
        .current_dir(work.path())
        .output()
        .ok()?;
    if !st.status.success() {
        return None;
    }
    let mut files: BTreeMap<String, u64> = BTreeMap::new();
    for e in std::fs::read_dir(&out).ok()?.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "rs") {
            files.insert(e.file_name().to_string_lossy().into_owned(), fnv(&std::fs::read(&p).ok()?));
        }
    }
    let folded = files.iter().fold(String::new(), |a, (k, v)| format!("{a}{k}:{v:016x};"));
    Some(format!("{:016x}", fnv(folded.as_bytes())))
}

fn recorded() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(root().join("tests/golden/build_output_hashes.tsv")).unwrap_or_default();
    text.lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn check(files: Vec<PathBuf>) {
    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let want = recorded();
    let mut bad = Vec::new();
    for f in files {
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        let got = generated_hash(&wj, &f).unwrap_or_else(|| "TRANSPILE-FAILED".into());
        if want.get(&name) != Some(&got) {
            bad.push(format!("{name}: recorded {:?}, generated {got}", want.get(&name)));
        }
    }
    assert!(bad.is_empty(), "generated Rust changed (re-record with WJ_RECORD_GOLDEN=1 if intended):\n{}", bad.join("\n"));
}

#[test]
fn sampled_examples_generate_the_recorded_rust() {
    let files: Vec<_> = corpus().into_iter().enumerate().filter(|(i, _)| i % 12 == 0).map(|(_, p)| p).collect();
    assert!(files.len() >= 10, "corpus unexpectedly small");
    check(files);
}

#[test]
#[ignore = "full corpus; also re-records with WJ_RECORD_GOLDEN=1"]
fn all_examples_generate_the_recorded_rust() {
    if std::env::var_os("WJ_RECORD_GOLDEN").is_some() {
        let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
        let mut out = String::new();
        for f in corpus() {
            let name = f.file_name().unwrap().to_string_lossy().into_owned();
            let h = generated_hash(&wj, &f).unwrap_or_else(|| "TRANSPILE-FAILED".into());
            out.push_str(&format!("{name}\t{h}\n"));
        }
        std::fs::write(root().join("tests/golden/build_output_hashes.tsv"), out).unwrap();
        return;
    }
    check(corpus());
}
