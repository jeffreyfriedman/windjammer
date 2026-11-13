//! Build memory meter and output-identity check for the library multipass build.
//!
//! Peak memory of `wj build` is a product meter: a library has to build on a modest laptop. A synthetic
//! library (`tests/mem_meter/gen_library.wj`, N neutral modules with structs, impls, an enum, a trait and
//! cross-module imports) is generated at two sizes and built the way large libraries are built
//! (`wj build src/mod.wj --output gen --library --no-cargo --module-file --mem-report`). The peak resident set
//! of each build comes from its own `[wj-mem]` report (`src/mem_report.rs`).
//!
//! 1. **Memory and cost ratchet** (`tests/mem_meter/ratchet.toml`, shrink-only): the peak growth per added
//!    module, the peak ratio when the module count doubles, and the doubling ratios of the deterministic work
//!    counters (`[wj-work]`: registry entries copied, trait keys scanned) may not exceed the recorded limits;
//!    the wall-time doubling ratio has a loose limit because timing is noisy. Memory that grows
//!    with the square of the module count shows up as a doubling ratio near 4 and a per-module growth that
//!    rises with N. An improvement is written into the ratchet in the same change; the limits never rise.
//!    `WJ_MEM_METER_BIN=<wj>` measures another binary: run it against a build from before the Q-MEM-1 fix to
//!    see the gate fail (mutation check).
//! 2. **Output identity** (`WJ_BASELINE_WJ=<wj built from the base commit>`): memory work must not change what
//!    is generated. Every file of the generated crate, for the synthetic library and for each project of
//!    `tests/project_corpus`, is compared byte for byte against the baseline binary's output. Without the
//!    variable the check reports that it was skipped (the project goldens in `tests/project_gates_test.rs`
//!    still pin the corpus output).
//!
//! Standalone: `cargo test --release --test library_build_memory -- --nocapture`. Debug builds skip the
//! meter (unoptimized timings and allocations are not the product).
//!
//! consumes: tests/mem_meter/**
//! consumes: tests/project_corpus/**

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Module counts of the two meter builds (the second is twice the first).
const N_SMALL: usize = 25;
const N_LARGE: usize = 50;
/// Measurement noise allowed over a ratchet limit before the gate fails.
const NOISE: f64 = 1.15;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wj_under_test() -> PathBuf {
    std::env::var_os("WJ_MEM_METER_BIN").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_wj")))
}

/// Generate the synthetic library with `n` modules under `dir/lib` (the generator runs from a private copy, so
/// its `wj.toml` write grant covers the output). Returns the library root (it has `src/mod.wj`).
fn generate(dir: &Path, n: usize) -> PathBuf {
    let gen_dir = dir.join("gen-tool");
    std::fs::create_dir_all(&gen_dir).unwrap();
    for f in ["gen_library.wj", "wj.toml"] {
        std::fs::copy(root().join("tests/mem_meter").join(f), gen_dir.join(f)).unwrap();
    }
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["run", "gen_library.wj", &n.to_string(), "lib"])
        .current_dir(&gen_dir)
        .output()
        .expect("run the generator");
    let lib = gen_dir.join("lib");
    assert!(
        lib.join("src/mod.wj").is_file(),
        "generator wrote no library: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    lib
}

struct Built {
    peak_mib: u64,
    phases: Vec<(u64, u64, String)>,
    wall_s: f64,
    /// Deterministic work counters of the build (`[wj-work]` line).
    work: BTreeMap<String, u64>,
}

/// `wj build` of a library root, the way large libraries are built, with the memory report on.
fn build_library(wj: &Path, lib: &Path) -> Built {
    let start = std::time::Instant::now();
    let out = Command::new(wj)
        .args(["build", "src/mod.wj", "--output", "gen", "--library", "--no-cargo", "--module-file", "--mem-report"])
        .current_dir(lib)
        .output()
        .expect("run wj build");
    let wall_s = start.elapsed().as_secs_f64();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "wj build failed in {}:\n{stderr}", lib.display());
    let phases: Vec<(u64, u64, String)> = stderr.lines().filter_map(windjammer::mem_report::parse_line).collect();
    let peak_mib = phases.iter().map(|(_, peak, _)| *peak).max().unwrap_or(0);
    assert!(
        phases.iter().any(|(_, _, p)| p == "build end") && peak_mib > 0,
        "no [wj-mem] report from {} (is it older than --mem-report?):\n{stderr}",
        wj.display()
    );
    let work: BTreeMap<String, u64> =
        stderr.lines().filter_map(windjammer::mem_report::parse_work_line).flatten().collect();
    Built { peak_mib, phases, wall_s, work }
}

/// `key = number` lines of the ratchet file.
fn read_ratchet() -> BTreeMap<String, f64> {
    let text = std::fs::read_to_string(root().join("tests/mem_meter/ratchet.toml")).expect("tests/mem_meter/ratchet.toml");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .filter_map(|l| l.split_once('='))
        .filter_map(|(k, v)| Some((k.trim().to_string(), v.trim().parse::<f64>().ok()?)))
        .collect()
}

/// Ratchet violations of the measured values (`shrink`-only: measured <= limit x NOISE), plus slack notes.
fn judge(measured: &BTreeMap<&str, f64>, ratchet: &BTreeMap<String, f64>) -> (Vec<String>, Vec<String>) {
    let mut fail = Vec::new();
    let mut slack = Vec::new();
    for (k, v) in measured {
        let Some(limit) = ratchet.get(*k) else {
            fail.push(format!("{k}: missing from tests/mem_meter/ratchet.toml"));
            continue;
        };
        if *v > limit * NOISE {
            fail.push(format!("{k}: measured {v:.2} exceeds the ratchet {limit:.2} (+{:.0}% noise allowed)", (NOISE - 1.0) * 100.0));
        } else if *v < limit * 0.7 {
            slack.push(format!("{k}: measured {v:.2} is well under the ratchet {limit:.2}: lower it in tests/mem_meter/ratchet.toml"));
        }
    }
    (fail, slack)
}

#[test]
fn ratchet_judge_fails_on_growth_and_notes_slack() {
    let ratchet: BTreeMap<String, f64> = [("a".to_string(), 100.0), ("b".to_string(), 2.0)].into_iter().collect();
    let ok: BTreeMap<&str, f64> = [("a", 110.0), ("b", 2.0)].into_iter().collect();
    assert_eq!(judge(&ok, &ratchet), (vec![], vec![]));
    let worse: BTreeMap<&str, f64> = [("a", 120.0), ("b", 3.9)].into_iter().collect();
    assert_eq!(judge(&worse, &ratchet).0.len(), 2);
    let better: BTreeMap<&str, f64> = [("a", 50.0), ("b", 2.0)].into_iter().collect();
    assert_eq!(judge(&better, &ratchet).1.len(), 1);
    let missing: BTreeMap<&str, f64> = [("c", 1.0)].into_iter().collect();
    assert_eq!(judge(&missing, &ratchet).0.len(), 1);
}

#[test]
fn library_build_memory_grows_linearly_with_module_count() {
    if cfg!(debug_assertions) {
        eprintln!("skipped: the memory meter runs on release builds (cargo test --release)");
        return;
    }
    let wj = wj_under_test();
    let work = tempfile::tempdir().unwrap();
    let small = build_library(&wj, &generate(&work.path().join("small"), N_SMALL));
    let large = build_library(&wj, &generate(&work.path().join("large"), N_LARGE));
    for (n, b) in [(N_SMALL, &small), (N_LARGE, &large)] {
        eprintln!("N={n}: peak {} MiB, wall {:.1} s", b.peak_mib, b.wall_s);
        for (rss, peak, phase) in &b.phases {
            eprintln!("    rss {rss:>6} MiB  peak {peak:>6} MiB  {phase}");
        }
    }
    let growth_kib_per_module =
        (large.peak_mib.saturating_sub(small.peak_mib) as f64) * 1024.0 / (N_LARGE - N_SMALL) as f64;
    let doubling_ratio = large.peak_mib as f64 / small.peak_mib.max(1) as f64;
    eprintln!("growth {growth_kib_per_module:.0} KiB per module, doubling ratio {doubling_ratio:.2}");
    // Build cost: deterministic work counters must double (not quadruple) with the module count; wall time is
    // reported and bounded loosely (noisy under load).
    let ratio = |key: &str| {
        let (Some(a), Some(b)) = (small.work.get(key).copied(), large.work.get(key).copied()) else {
            panic!("no `{key}` work counter in the [wj-work] report of {}", wj.display());
        };
        // No such work at either size is the best case (ratio 0); work appearing only at the larger size fails.
        match (a, b) {
            (0, 0) => 0.0,
            (0, b) => b as f64,
            (a, b) => b as f64 / a as f64,
        }
    };
    let registry_ratio = ratio("registry_entries_cloned");
    let trait_ratio = ratio("trait_keys_scanned");
    let wall_ratio = large.wall_s / small.wall_s.max(0.001);
    eprintln!(
        "work doubling ratios: registry entries cloned {registry_ratio:.2}, trait keys scanned {trait_ratio:.2}; wall {wall_ratio:.2}"
    );
    let measured: BTreeMap<&str, f64> = [
        ("max_peak_growth_kib_per_module", growth_kib_per_module),
        ("max_peak_doubling_ratio", doubling_ratio),
        ("max_registry_entries_cloned_doubling_ratio", registry_ratio),
        ("max_trait_keys_scanned_doubling_ratio", trait_ratio),
        ("max_wall_doubling_ratio", wall_ratio),
    ]
    .into_iter()
    .collect();
    let (fail, slack) = judge(&measured, &read_ratchet());
    for s in &slack {
        eprintln!("note: {s}");
    }
    assert!(fail.is_empty(), "build memory ratchet ({}):\n  {}", wj.display(), fail.join("\n  "));
}

/// Every file under `dir` (relative path -> bytes), skipping build caches and stamps that record times or paths.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(".wj") || name == "target" {
                continue;
            }
            if p.is_dir() {
                walk(base, &p, out);
            } else if p.extension().is_some_and(|x| x == "rs" || x == "json" || x == "toml") {
                out.insert(p.strip_prefix(base).unwrap().display().to_string(), std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// The generated crate of one build of a private copy of `project` (`args` with `{out}` for the output dir).
fn generated(wj: &Path, project: &Path, args: &[&str], work: &Path) -> BTreeMap<String, Vec<u8>> {
    // Both binaries build in the same place (a fresh copy each time), so recorded paths cannot differ.
    let copy = work.join("p");
    let _ = std::fs::remove_dir_all(&copy);
    copy_dir(project, &copy);
    let out_dir = copy.join("gen");
    let args: Vec<String> = args.iter().map(|a| a.replace("{out}", &out_dir.display().to_string())).collect();
    let _ = Command::new(wj).args(&args).current_dir(&copy).output().expect("run wj build");
    tree(&out_dir)
}

/// Byte equality, except JSON files compare as parsed values: a baseline binary from before metadata.json was
/// written in key order emits its maps in random order (same content).
fn same_content(path: &str, a: Option<&Vec<u8>>, b: Option<&Vec<u8>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) if path.ends_with(".json") => {
            match (serde_json::from_slice::<serde_json::Value>(a), serde_json::from_slice::<serde_json::Value>(b)) {
                (Ok(x), Ok(y)) => x == y,
                _ => a == b,
            }
        }
        _ => a == b,
    }
}

#[test]
fn library_build_output_matches_the_baseline_binary() {
    let Some(baseline) = std::env::var_os("WJ_BASELINE_WJ").map(PathBuf::from) else {
        eprintln!("skipped: set WJ_BASELINE_WJ to a wj built from the base commit to compare generated output");
        return;
    };
    let current = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let work = tempfile::tempdir().unwrap();
    let mut cases: Vec<(String, PathBuf, Vec<&str>)> = vec![(
        "synthetic library".into(),
        generate(&work.path().join("synthetic"), 40),
        vec!["build", "src/mod.wj", "--output", "{out}", "--library", "--no-cargo", "--module-file"],
    )];
    let corpus = root().join("tests/project_corpus");
    let mut projects: Vec<PathBuf> =
        std::fs::read_dir(&corpus).unwrap().flatten().map(|e| e.path()).filter(|p| p.join("main.wj").is_file()).collect();
    projects.sort();
    for p in projects {
        cases.push((p.file_name().unwrap().to_string_lossy().into_owned(), p, vec!["build", ".", "--no-cargo", "-o", "{out}"]));
    }
    let mut diffs = Vec::new();
    for (i, (name, project, args)) in cases.iter().enumerate() {
        let place = work.path().join(format!("case-{i}"));
        let a = generated(&baseline, project, args, &place);
        let b = generated(&current, project, args, &place);
        if a.is_empty() {
            diffs.push(format!("{name}: the baseline generated nothing"));
        }
        for path in a.keys().chain(b.keys()).collect::<std::collections::BTreeSet<_>>() {
            if !same_content(path, a.get(path), b.get(path)) {
                diffs.push(format!("{name}: {path} differs"));
            }
        }
    }
    eprintln!("compared {} builds against {}", cases.len(), baseline.display());
    assert!(diffs.is_empty(), "generated output changed:\n  {}", diffs.join("\n  "));
}
