//! Build-speed gates (docs/design/build-speed.md).
//!
//! The tests use the real shared cache (a gate that recompiled the runtime each run would be minutes
//! long); the first run on a fresh machine is cold and is not budgeted.
//!
//! 1. CPU-time budgets (user+sys of the whole process tree, so machine load cannot flake them) for a
//!    small program: warm `wj run --compiled` and `wj build` into a fresh output directory.
//! 2. `wj build` and `wj run --compiled` share one target tree: a second, different output directory
//!    reuses the dependency artifacts instead of recompiling them.
//! 3. Output equality: the fast dev profile changes neither the generated Rust nor program behaviour,
//!    and the binary stays at the documented `<output>/target/debug/<name>`.
//!
//! The cache-invalidation gate and its mutation check live next to the key function
//! (`src/embedded_assets.rs`, `cache_key_tests`).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Warm run budget in CPU seconds (measured 0.4 to 0.9 under load 20; see the design doc).
const RUN_WARM_CPU_BUDGET: f64 = 5.0;
/// Warm build budget in CPU seconds into a fresh output dir with a warm shared cache.
const BUILD_WARM_CPU_BUDGET: f64 = 8.0;

const PROGRAM: &str = "fn main() {\n    let mut s = 0\n    for i in 0..10 { s = s + i }\n    println(\"sum {}\", s)\n}\n";

fn wj() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

/// Run `cmd` under `/usr/bin/time -p` and return (stdout, user+sys CPU seconds).
fn timed(mut args: Vec<String>, cache: &Path) -> (String, f64, bool) {
    args.insert(0, "-p".into());
    let out = Command::new("/usr/bin/time")
        .args(&args)
        .output()
        .expect("run /usr/bin/time");
    let err = String::from_utf8_lossy(&out.stderr);
    let mut cpu = 0.0;
    for l in err.lines() {
        if let Some(v) = l.strip_prefix("user ").or_else(|| l.strip_prefix("sys ")) {
            cpu += v.trim().parse::<f64>().unwrap_or(0.0);
        }
    }
    (String::from_utf8_lossy(&out.stdout).into_owned(), cpu, out.status.success())
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let src = t.path().join("small");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("main.wj"), PROGRAM).unwrap();
    (t, src.join("main.wj"))
}

#[test]
fn warm_builds_fit_the_cpu_budget_share_the_cache_and_keep_outputs() {
    let (t, main) = fixture();
    let cache = PathBuf::from("");
    let wj = wj().display().to_string();
    let main_s = main.display().to_string();

    // Cold: populates the shared tree (not budgeted).
    let (cold_out, _, ok) = timed(argv(&[&wj, "run", "--compiled", &main_s]), &cache);
    assert!(ok, "cold run failed");
    assert!(cold_out.contains("sum 45"));

    // Warm run.
    let (out, cpu, ok) = timed(argv(&[&wj, "run", "--compiled", &main_s]), &cache);
    assert!(ok && out.contains("sum 45"));
    assert!(cpu < RUN_WARM_CPU_BUDGET, "warm `wj run --compiled` used {cpu:.2}s CPU (budget {RUN_WARM_CPU_BUDGET}s)");

    // Warm build into a fresh output dir: dependencies come from the shared tree.
    let b1 = t.path().join("b1");
    let (_, cpu, ok) = timed(argv(&[&wj, "build", &main_s, "-o", &b1.display().to_string()]), &cache);
    assert!(ok, "wj build failed");
    assert!(cpu < BUILD_WARM_CPU_BUDGET, "warm `wj build` used {cpu:.2}s CPU (budget {BUILD_WARM_CPU_BUDGET}s)");

    // Output equality: documented binary location, same behaviour, same generated Rust as --no-cargo.
    let bin = b1.join("target/debug/small");
    assert!(bin.is_file(), "binary missing at {}", bin.display());
    let ran = Command::new(&bin).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout), cold_out);
    let b2 = t.path().join("b2");
    let st = Command::new(&wj)
        .args(["build", &main_s, "-o", &b2.display().to_string(), "--no-cargo"])
        .output()
        .unwrap();
    assert!(st.status.success());
    for f in ["main.rs", "Cargo.toml"] {
        assert_eq!(std::fs::read(b1.join(f)).unwrap(), std::fs::read(b2.join(f)).unwrap(), "{f} differs");
    }
}
