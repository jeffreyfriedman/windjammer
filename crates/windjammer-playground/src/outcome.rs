//! What a playground run returns, and its JSON form (the only thing that crosses into JavaScript).

use serde_json::{json, Value as Json};

/// Limits and seed of one run.
#[derive(Clone, Debug)]
pub struct Options {
    /// Evaluated expressions before the run stops with exit status 124 (the interpreter's `--limits fuel=N`).
    pub fuel: u64,
    /// Bytes of standard output and error before the run stops with exit status 124.
    pub output_bytes: usize,
    /// Seed of the simulated world: random numbers, hash order, task interleavings.
    pub seed: u64,
}

impl Default for Options {
    fn default() -> Self {
        // about 1.3 s of tree-walking in wasm on a laptop (measured: 50M steps in 6.2 s); an infinite loop ends here
        Options {
            fuel: 10_000_000,
            output_bytes: 1 << 20,
            seed: 0,
        }
    }
}

/// Result of a run: what the program printed, the compiler's diagnostics (when it refused to run it) and the
/// exit status `wj run --interpret` would have exited with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    /// `wj check` diagnostics as JSON (the same schema as `wj check --json`), errors and warnings.
    pub diagnostics: Vec<Json>,
    /// 0 ok, 1 refused or failed, 101 panic, 124 a limit, 126 a denied capability, N for `process::exit(N)`.
    pub exit_code: i32,
    /// Expressions evaluated (fuel used).
    pub steps: u64,
}

impl Outcome {
    pub fn to_json(&self) -> String {
        json!({
            "stdout": self.stdout,
            "stderr": self.stderr,
            "diagnostics": self.diagnostics,
            "exit_code": self.exit_code,
            "steps": self.steps,
        })
        .to_string()
    }
}
