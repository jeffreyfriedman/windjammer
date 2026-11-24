//! Runs the real `wj` binary on a program in a fresh cache root and reads the per-stage counters
//! (`WJ_STAGE_COUNTERS=<file>`, `debug_env::Stage`) it appends to a file.

use std::collections::BTreeMap;
use std::process::{Command, Output};

/// Every stage `debug_env::Stage` names.
pub const STAGES: [&str; 9] = ["parse", "analyzer_pass", "analyzer_reused", "numeric", "numeric_reused", "ir_lowering", "registry_copy", "facts_build", "rust_generation"];

pub const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// A small program every consumer of the check profile reads: bindings (local mutability), parameters and methods
/// (move prediction), a struct and a loop.
pub const SMALL: &str = "struct Counter {\n    total: int,\n}\n\nfn bump(c: Counter, by: int) -> Counter {\n    Counter { total: c.total + by }\n}\n\nfn main() {\n    let mut items: Vec<int> = Vec::new()\n    items.push(1)\n    items.push(2)\n    let mut c = Counter { total: 0 }\n    for x in items {\n        c = bump(c, x)\n    }\n    println(\"${c.total}\")\n}\n";

pub struct Ran {
    pub out: Output,
    pub counts: BTreeMap<String, u64>,
}

impl Ran {
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.out.stdout).into_owned()
    }
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.out.stderr).into_owned()
    }
    pub fn n(&self, stage: &str) -> u64 {
        *self.counts.get(stage).unwrap_or_else(|| panic!("no counter {stage}: {:?}", self.counts))
    }
}

/// `wj <args>` in `dir`, cold: its own cache root, so no run cache, analysis memo or snapshot is reused.
pub fn wj(dir: &std::path::Path, args: &[&str], env: &[(&str, &str)]) -> Ran {
    let cache = tempfile::tempdir().unwrap();
    let log = cache.path().join("stages.log");
    let mut c = Command::new(WJ);
    c.args(args)
        .current_dir(dir)
        .env("WJ_CARGO_TARGET_ROOT", cache.path())
        .env("WJ_STAGE_COUNTERS", &log)
        .env("WJ_NO_STALE_WARN", "1")
        .env_remove("WJ_NO_SHARED_ANALYSIS")
        .env_remove("WJ_VERIFY_SHARED_ANALYSIS")
        .env_remove("WJ_ERROR_FORMAT")
        .env_remove("WJ_PROGRESS");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().unwrap();
    let mut counts: BTreeMap<String, u64> = STAGES.iter().map(|s| (s.to_string(), 0)).collect();
    for name in std::fs::read_to_string(&log).unwrap_or_default().lines() {
        *counts.get_mut(name).unwrap_or_else(|| panic!("unknown stage {name}")) += 1;
    }
    Ran { out, counts }
}
