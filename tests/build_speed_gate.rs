//! Build-speed gates (docs/design/build-speed.md).
//!
//! 1. CPU-time budgets (user+sys of the whole process tree, so machine load cannot flake them): warm
//!    `wj run --compiled`, warm `wj build` into a fresh output directory, and the first build on a machine
//!    (empty cache) of a program that uses no network code.
//! 2. `wj build` and `wj run --compiled` share one target tree: a second output directory reuses the
//!    dependency artifacts instead of recompiling them.
//! 3. Output equality: the dev profile changes neither the generated Rust nor program behaviour, and the
//!    binary stays at the documented `<output>/target/debug/<name>`.
//! 4. Concurrent `wj run --compiled` of one source all succeed with the same output.
//! 5. Mutation check (`--ignored`): with the shared target tree disabled (`WJ_USE_LOCAL_TARGET=1`, the
//!    old behaviour) the warm-build budget is exceeded, so gate 1 would fail.
//!
//! Warm tests use the real shared cache (a gate that recompiled the runtime each run would be minutes
//! long); the first run on a fresh machine is cold and is not budgeted there. The cold test uses a private
//! cache root.
//!
//! The cache-invalidation gate and its mutation check live next to the key function
//! (`src/embedded_assets.rs`, `cache_key_tests`).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Warm run budget in CPU seconds (measured 0.4 to 0.9 under load 20; see the design doc).
const RUN_WARM_CPU_BUDGET: f64 = 5.0;
/// Warm build budget in CPU seconds into a fresh output dir with a warm shared cache.
const BUILD_WARM_CPU_BUDGET: f64 = 8.0;
/// First build on a machine for a program with no network/async/cli code (see the design doc).
const FIRST_BUILD_CPU_BUDGET: f64 = 60.0;

const PROGRAM: &str = "fn main() {\n    let mut s = 0\n    for i in 0..10 { s = s + i }\n    println(\"sum {}\", s)\n}\n";

fn wj() -> String {
    PathBuf::from(env!("CARGO_BIN_EXE_wj")).display().to_string()
}

struct Timed {
    stdout: String,
    cpu: f64,
    ok: bool,
}

/// Run a command under `/usr/bin/time -p`; CPU = user + sys of the process tree.
fn timed(args: &[&str], env: &[(&str, &Path)], cwd: Option<&Path>) -> Timed {
    let mut cmd = Command::new("/usr/bin/time");
    cmd.arg("-p").args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    let out = cmd.output().expect("run /usr/bin/time");
    let err = String::from_utf8_lossy(&out.stderr);
    let cpu = err
        .lines()
        .filter_map(|l| l.strip_prefix("user ").or_else(|| l.strip_prefix("sys ")))
        .map(|v| v.trim().parse::<f64>().unwrap_or(0.0))
        .sum();
    Timed { stdout: String::from_utf8_lossy(&out.stdout).into_owned(), cpu, ok: out.status.success() }
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let src = t.path().join("small");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.wj"), PROGRAM).unwrap();
    let main = src.join("main.wj");
    (t, main)
}

#[test]
fn warm_builds_fit_the_cpu_budget_share_the_cache_and_keep_outputs() {
    let (t, main) = fixture();
    let (wj, main_s) = (wj(), main.display().to_string());

    // Cold if the machine has never built: populates the shared tree (not budgeted here).
    let cold = timed(&[&wj, "run", "--compiled", &main_s], &[], None);
    assert!(cold.ok && cold.stdout.contains("sum 45"), "cold run failed");

    let warm = timed(&[&wj, "run", "--compiled", &main_s], &[], None);
    assert!(warm.ok && warm.stdout.contains("sum 45"));
    assert!(warm.cpu < RUN_WARM_CPU_BUDGET, "warm `wj run --compiled` used {:.2}s CPU (budget {RUN_WARM_CPU_BUDGET}s)", warm.cpu);

    let b1 = t.path().join("b1");
    let b1_s = b1.display().to_string();
    let build = timed(&[&wj, "build", &main_s, "-o", &b1_s], &[], None);
    assert!(build.ok, "wj build failed");
    assert!(build.cpu < BUILD_WARM_CPU_BUDGET, "warm `wj build` used {:.2}s CPU (budget {BUILD_WARM_CPU_BUDGET}s)", build.cpu);

    // Output equality: documented binary location, same behaviour, same generated Rust as --no-cargo.
    let bin = b1.join("target/debug/small");
    assert!(bin.is_file(), "binary missing at {}", bin.display());
    let ran = Command::new(&bin).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout), cold.stdout);
    let b2 = t.path().join("b2");
    let st = Command::new(&wj).args(["build", &main_s, "-o", &b2.display().to_string(), "--no-cargo"]).output().unwrap();
    assert!(st.status.success());
    for f in ["main.rs", "Cargo.toml"] {
        assert_eq!(std::fs::read(b1.join(f)).unwrap(), std::fs::read(b2.join(f)).unwrap(), "{f} differs");
    }
}

#[test]
fn concurrent_compiled_runs_of_one_source_agree() {
    let (_t, main) = fixture();
    let (wj, main_s) = (wj(), main.display().to_string());
    // Warm the cache once so the concurrent runs only contend on the shared work directory.
    assert!(timed(&[&wj, "run", "--compiled", &main_s], &[], None).ok);
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let (wj, main_s) = (wj.clone(), main_s.clone());
            std::thread::spawn(move || timed(&[&wj, "run", "--compiled", &main_s], &[], None))
        })
        .collect();
    for h in handles {
        let r = h.join().unwrap();
        assert!(r.ok, "a concurrent run failed");
        assert_eq!(r.stdout.trim(), "sum 45");
    }
}

#[test]
fn first_build_on_a_fresh_machine_fits_the_cpu_budget() {
    let (t, main) = fixture();
    let cache = t.path().join("fresh-cache");
    let out = t.path().join("out");
    let r = timed(
        &[&wj(), "build", &main.display().to_string(), "-o", &out.display().to_string()],
        &[("WJ_CARGO_TARGET_ROOT", &cache)],
        None,
    );
    assert!(r.ok, "first build failed");
    assert!(r.cpu < FIRST_BUILD_CPU_BUDGET, "first build used {:.1}s CPU (budget {FIRST_BUILD_CPU_BUDGET}s)", r.cpu);
    let manifest = std::fs::read_to_string(out.join("Cargo.toml")).unwrap();
    assert!(!manifest.contains("\"net\"") && !manifest.contains("\"full\""), "{manifest}");
}

/// Mutation check for the shared-cache change: with the shared tree disabled the "warm" build is a cold
/// build and must blow the budget. Run with `cargo test --test build_speed_gate -- --ignored`.
#[test]
#[ignore = "mutation check: compiles the runtime from scratch (minutes of CPU)"]
fn mutation_local_target_dir_breaks_the_warm_build_budget() {
    let (t, main) = fixture();
    let work = t.path().join("cwd");
    std::fs::create_dir_all(&work).unwrap();
    let out = t.path().join("out");
    let r = timed(
        &[&wj(), "build", &main.display().to_string(), "-o", &out.display().to_string()],
        &[("WJ_USE_LOCAL_TARGET", Path::new("1"))],
        Some(&work),
    );
    assert!(r.ok);
    assert!(r.cpu > BUILD_WARM_CPU_BUDGET, "mutant stayed under the budget ({:.1}s): the gate cannot detect a lost shared cache", r.cpu);
}
