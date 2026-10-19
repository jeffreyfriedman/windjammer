//! Cost of one run-time capability check in a compiled program (ADR-0021).
//!
//!   cargo run --release -p windjammer-runtime --example caps_bench
//!
//! Prints nanoseconds per `caps::fs_read` for: no grant installed (what every proven program pays: one atomic load),
//! and an installed `./data/**` grant for a path inside it (what a guarded program pays per call). Quote these only
//! from a quiet machine; the 1-minute load average is printed next to the numbers.

use std::hint::black_box;
use std::time::Instant;
use windjammer_runtime::caps;

fn per_call(label: &str, n: u32, f: impl Fn()) {
    for _ in 0..1000 {
        f();
    }
    let t = Instant::now();
    for _ in 0..n {
        f();
    }
    println!("{label}: {:.1} ns/check", t.elapsed().as_nanos() as f64 / n as f64);
}

fn main() {
    let load = std::fs::read_to_string("/proc/loadavg").ok().or_else(|| std::process::Command::new("sysctl").args(["-n", "vm.loadavg"]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()));
    println!("load: {}", load.unwrap_or_default().trim());
    per_call("no grant installed", 2_000_000, || caps::fs_read(black_box("data/a.txt")));
    caps::install(&caps::Spec { root: ".", allow: &[("fs.read", &["./data/**"])], deny: &[] });
    per_call("grant installed, path inside ./data/**", 200_000, || caps::fs_read(black_box("data/a.txt")));
}
