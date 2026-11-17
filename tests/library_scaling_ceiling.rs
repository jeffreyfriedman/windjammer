//! Library-scaling ceiling: the memory fix of the library build is permanent.
//!
//! A synthetic library (`tests/mem_meter/gen_library.wj`) of N = 50, 100 and 200 modules is built the way large
//! libraries are built (`wj build src/mod.wj --output gen --library --no-cargo --module-file --mem-report`), and
//! the build is held to absolute ceilings and to a growth bound:
//!
//! * (a) peak resident set at N=200 under the machine's ceiling (`tests/mem_meter/ceiling.toml`: 1.5x the value
//!   measured with the fix; 126 MiB on the PC, so 189 MiB);
//! * (b) wall time at N=200 under its ceiling (loose: a blow-up, not a drift);
//! * (c) growth from N=100 to N=200 of at most 2.3x in peak (3.2x in wall time): memory that grows with the square
//!   of the module count doubles to about 4x and fails here;
//! * (d) two builds of the same library are byte-identical.
//!
//! Peak memory comes from the build's own `[wj-mem]` report (`src/mem_report.rs`); a binary without the report
//! (older than `--mem-report`) is measured with `/usr/bin/time` instead, so the mutation check works on it.
//! Mutation check: `WJ_MEM_METER_BIN=<wj built before the memory fix> cargo test --release --test
//! library_scaling_ceiling` must fail (the PC builds `f55f8b1b6`, line-a).
//!
//! Standalone: `cargo test --release --test library_scaling_ceiling -- --nocapture`. Debug builds skip it.
//! `WJ_CEILING_RECORD=1` prints a table to record for this machine. Wall time is noisy on a loaded machine: the
//! ceiling is deliberately far above the measured value.
//!
//! consumes: tests/mem_meter/**

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const SIZES: [usize; 3] = [50, 100, 200];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wj_under_test() -> PathBuf {
    std::env::var_os("WJ_MEM_METER_BIN").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_wj")))
}

/// `uname -sm` ("Linux x86_64", "Darwin arm64"): the key of a machine's ceilings.
fn machine() -> String {
    let o = Command::new("uname").arg("-sm").output().ok();
    o.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "unknown".into())
}

#[derive(Debug, Clone, PartialEq)]
struct Ceilings {
    peak_mib_n200: f64,
    wall_s_n200: f64,
    max_peak_growth: f64,
    max_wall_growth: f64,
}

fn read_ceilings(machine: &str) -> Ceilings {
    let text = std::fs::read_to_string(root().join("tests/mem_meter/ceiling.toml")).expect("tests/mem_meter/ceiling.toml");
    let doc: toml::Value = text.parse().expect("ceiling.toml parses");
    let table = doc.get("machines").and_then(|m| m.get(machine)).or_else(|| doc.get("default")).expect("[default] in ceiling.toml");
    let num = |k: &str| table.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or_else(|| panic!("{k} in ceiling.toml"));
    Ceilings { peak_mib_n200: num("peak_mib_n200"), wall_s_n200: num("wall_s_n200"), max_peak_growth: num("max_peak_growth"), max_wall_growth: num("max_wall_growth") }
}

#[derive(Debug, Clone, Copy)]
struct Measured {
    peak_mib: f64,
    wall_s: f64,
}

/// Ceiling violations of the N=100 and N=200 measurements (empty when the build holds the ceilings).
fn judge(c: &Ceilings, n100: Measured, n200: Measured) -> Vec<String> {
    let mut fail = Vec::new();
    if n200.peak_mib > c.peak_mib_n200 {
        fail.push(format!("(a) peak {:.0} MiB at N=200 exceeds the ceiling {:.0} MiB", n200.peak_mib, c.peak_mib_n200));
    }
    if n200.wall_s > c.wall_s_n200 {
        fail.push(format!("(b) wall {:.1} s at N=200 exceeds the ceiling {:.0} s", n200.wall_s, c.wall_s_n200));
    }
    let peak_growth = n200.peak_mib / n100.peak_mib.max(1.0);
    if peak_growth > c.max_peak_growth {
        fail.push(format!("(c) peak grows {peak_growth:.2}x from N=100 to N=200 (limit {:.2}x): memory is not linear in the module count", c.max_peak_growth));
    }
    let wall_growth = n200.wall_s / n100.wall_s.max(0.001);
    if wall_growth > c.max_wall_growth {
        fail.push(format!("(c) wall time grows {wall_growth:.2}x from N=100 to N=200 (limit {:.2}x)", c.max_wall_growth));
    }
    fail
}

fn generate(dir: &Path, n: usize) -> PathBuf {
    let gen_dir = dir.join("gen-tool");
    std::fs::create_dir_all(&gen_dir).unwrap();
    for f in ["gen_library.wj", "wj.toml"] {
        std::fs::copy(root().join("tests/mem_meter").join(f), gen_dir.join(f)).unwrap();
    }
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "gen_library.wj", &n.to_string(), "lib"]).current_dir(&gen_dir).output().expect("run the generator");
    let lib = gen_dir.join("lib");
    assert!(lib.join("src/mod.wj").is_file(), "generator wrote no library: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    lib
}

const BUILD_ARGS: [&str; 7] = ["build", "src/mod.wj", "--library", "--no-cargo", "--module-file", "--output", ""];

/// Peak MiB from `/usr/bin/time` output (`maximum resident set size` in bytes on macOS, `Maximum resident set size
/// (kbytes)` on Linux).
fn peak_from_time(stderr: &str) -> Option<f64> {
    for l in stderr.lines() {
        let t = l.trim();
        if let Some(v) = t.strip_suffix("maximum resident set size") {
            return v.trim().parse::<f64>().ok().map(|b| b / (1024.0 * 1024.0));
        }
        if let Some(v) = t.strip_prefix("Maximum resident set size (kbytes):") {
            return v.trim().parse::<f64>().ok().map(|k| k / 1024.0);
        }
    }
    None
}

fn build(wj: &Path, lib: &Path, output: &str) -> Measured {
    let mut args: Vec<&str> = BUILD_ARGS.to_vec();
    *args.last_mut().unwrap() = output;
    let start = std::time::Instant::now();
    let out = Command::new(wj).args(&args).arg("--mem-report").current_dir(lib).output().expect("run wj build");
    let wall_s = start.elapsed().as_secs_f64();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let reported: Option<u64> = stderr.lines().filter_map(windjammer::mem_report::parse_line).map(|(_, peak, _)| peak).max().filter(|p| *p > 0);
    let peak_mib = match (out.status.success(), reported) {
        (true, Some(p)) => p as f64,
        // no report (a binary older than `--mem-report` rejects the flag): measure it from outside
        _ => {
            let (time, flag) = if cfg!(target_os = "macos") { ("/usr/bin/time", "-l") } else { ("/usr/bin/time", "-v") };
            let mut timed = Command::new(time);
            timed.arg(flag).arg(wj).args(&args).current_dir(lib);
            let o = timed.output().expect("run /usr/bin/time");
            assert!(o.status.success(), "wj build failed in {}:\n{}", lib.display(), String::from_utf8_lossy(&o.stderr));
            let peak = peak_from_time(&String::from_utf8_lossy(&o.stderr)).expect("peak resident set from /usr/bin/time");
            return Measured { peak_mib: peak, wall_s };
        }
    };
    Measured { peak_mib, wall_s }
}

/// Every file of a generated crate (relative path -> bytes), dotfiles and caches left out.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(p.strip_prefix(root).unwrap().to_string_lossy().to_string(), std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn the_judge_fails_each_ceiling_and_passes_a_linear_build() {
    let c = Ceilings { peak_mib_n200: 189.0, wall_s_n200: 300.0, max_peak_growth: 2.3, max_wall_growth: 3.2 };
    let m = |peak_mib, wall_s| Measured { peak_mib, wall_s };
    assert!(judge(&c, m(70.0, 10.0), m(126.0, 22.0)).is_empty(), "the memory-fix numbers pass");
    assert_eq!(judge(&c, m(70.0, 10.0), m(190.0, 22.0)).len(), 2, "a high peak fails (a) and (c) when it also grew too fast");
    assert!(judge(&c, m(70.0, 10.0), m(126.0, 301.0)).iter().any(|f| f.starts_with("(b)")));
    assert!(judge(&c, m(60.0, 10.0), m(160.0, 22.0)).iter().any(|f| f.starts_with("(c) peak")), "quadratic growth: 2.7x");
    assert!(judge(&c, m(70.0, 5.0), m(126.0, 22.0)).iter().any(|f| f.starts_with("(c) wall")));
}

#[test]
fn time_output_is_parsed_on_both_platforms() {
    assert_eq!(peak_from_time("  123456789  maximum resident set size\n").map(|m| m.round()), Some(118.0));
    assert_eq!(peak_from_time("\tMaximum resident set size (kbytes): 204800\n"), Some(200.0));
    assert_eq!(peak_from_time("nothing"), None);
}

#[test]
fn library_build_holds_its_scaling_ceilings() {
    if cfg!(debug_assertions) {
        eprintln!("skipped: the scaling ceiling runs on release builds (cargo test --release)");
        return;
    }
    let wj = wj_under_test();
    let work = tempfile::tempdir().unwrap();
    let mut measured: BTreeMap<usize, Measured> = BTreeMap::new();
    let mut libs: BTreeMap<usize, PathBuf> = BTreeMap::new();
    for n in SIZES {
        let lib = generate(&work.path().join(format!("n{n}")), n);
        let m = build(&wj, &lib, "gen");
        eprintln!("N={n}: peak {:.0} MiB, wall {:.1} s", m.peak_mib, m.wall_s);
        measured.insert(n, m);
        libs.insert(n, lib);
    }
    let machine = machine();
    let ceilings = read_ceilings(&machine);
    if std::env::var("WJ_CEILING_RECORD").is_ok_and(|v| v == "1") {
        let (a, b) = (measured[&100], measured[&200]);
        eprintln!("\n[machines.\"{machine}\"]  # measured: peak {:.0} MiB, wall {:.1} s at N=200; growth {:.2}x / {:.2}x", b.peak_mib, b.wall_s, b.peak_mib / a.peak_mib.max(1.0), b.wall_s / a.wall_s.max(0.001));
        eprintln!("peak_mib_n200 = {:.0}\nwall_s_n200 = {:.0}\nmax_peak_growth = 2.3\nmax_wall_growth = 3.2", (b.peak_mib * 1.5).ceil(), (b.wall_s * 3.0).ceil().max(60.0));
    }
    let failures = judge(&ceilings, measured[&100], measured[&200]);
    assert!(failures.is_empty(), "library build scaling ({machine}, {}):\n  {}", wj.display(), failures.join("\n  "));

    // (d) the same library built twice gives the same bytes
    let lib = &libs[&50];
    build(&wj, lib, "gen-again");
    let (a, b) = (tree(&lib.join("gen")), tree(&lib.join("gen-again")));
    let differing: Vec<&String> = a.keys().chain(b.keys()).filter(|k| a.get(*k) != b.get(*k)).collect();
    assert!(differing.is_empty(), "two builds of one library differ in: {differing:?}");
    assert!(!a.is_empty(), "the build produced no files");
}
