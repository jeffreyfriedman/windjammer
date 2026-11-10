//! The Windjammer interpreter for the browser playground (ADR-0034).
//!
//! `run(source)` is `wj run --interpret --sandbox --simulate --seed N --limits fuel=F,output=B` on one in-memory
//! file: the compiler's checker first (a program `wj check` rejects is refused with its diagnostics, ADR-0030), then
//! the tree-walking interpreter with no grants beyond console, a virtual clock and seeded random numbers, so nothing
//! the program does can reach the reader's machine and every run of a program prints the same thing.
//! `wasm.rs` exposes it to JavaScript; the same function runs natively in this crate's tests.

pub mod outcome;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use outcome::{Options, Outcome};

use std::path::{Path, PathBuf};
use windjammer::diagnostics::{render, Severity};
use windjammer::interpreter::caps::{CapGate, CapMode};
use windjammer::interpreter::effect_port::sim::SimConfig;
use windjammer::interpreter::engine::Interpreter;
use windjammer::interpreter::error::ErrorKind;
use windjammer::interpreter::limits::LimitSpec;
use windjammer::interpreter::project::LoadedFile;
use windjammer::interpreter::{loader, runner};
use windjammer::ir::effect_facts::Grant;

/// The program's file name in diagnostics and panic locations.
pub const FILE: &str = "main.wj";
/// The directory the run pretends to be in (nothing is read from it: the grant has no files).
const ROOT: &str = "/playground";
/// Interpreter recursion budget on the wasm stack (`.cargo/config.toml` links a 32 MiB stack).
#[cfg(target_arch = "wasm32")]
const STACK_BUDGET: usize = 24 << 20;

pub fn run(source: &str, opts: &Options) -> Outcome {
    let mut out = Outcome::default();
    if refused(source, &mut out) {
        return out;
    }
    let program = match loader::parse_source(source, FILE) {
        Ok(p) => p,
        Err(e) => {
            out.stderr = format!("parse error in {FILE}: {e}\n");
            out.exit_code = 1;
            return out;
        }
    };
    let mut it = configured(opts);
    let file = LoadedFile {
        module_path: Vec::new(),
        file: PathBuf::from(FILE),
        program: Box::leak(Box::new(program)),
    };
    let (code, error) = match it.load_sources(vec![file], Vec::new(), Vec::new()) {
        Err(e) => (runner::exit_code_for(&e), Some(e.render())),
        Ok(()) => match it.call_main() {
            Ok(v) => (runner::main_result_code(&mut it, &v), None),
            Err(e) => {
                let shown = (!matches!(e.kind, ErrorKind::Exit(_))).then(|| e.render());
                (runner::exit_code_for(&e), shown)
            }
        },
    };
    it.flush();
    out.stdout = it.get_output();
    out.stderr = it.get_err_output();
    if let Some(e) = error {
        out.stderr.push_str(&e);
        out.stderr.push('\n');
    }
    out.exit_code = code;
    out.steps = it.usage().0;
    out
}

/// `wj check` on the source, as `wj run` does before interpreting: true (and the diagnostics rendered on stderr)
/// when it reports an error.
fn refused(source: &str, out: &mut Outcome) -> bool {
    let diags = windjammer::check::check_sources(
        Path::new(ROOT),
        &[(FILE.to_string(), source.to_string())],
    );
    out.diagnostics = diags
        .iter()
        .filter_map(|d| serde_json::to_value(d).ok())
        .collect();
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    if errors.is_empty() {
        return false;
    }
    let source_of = |f: &str| (f == FILE).then(|| source.to_string());
    for d in &errors {
        out.stderr.push_str(&render::render(d, &source_of));
        out.stderr.push('\n');
    }
    out.stderr.push_str(&format!(
        "not running {FILE}: `wj check` reports {} error(s); the interpreter runs only programs the compiler accepts\n",
        errors.len()
    ));
    out.exit_code = 1;
    true
}

/// An interpreter in the playground's world: captured output, no grants, the seeded simulation, fuel and output limits.
fn configured(opts: &Options) -> Interpreter {
    let mut it = Interpreter::new_capturing();
    it.set_capture_stderr(true);
    #[cfg(target_arch = "wasm32")]
    it.guard_stack_bytes(STACK_BUDGET);
    it.set_jit(false);
    it.set_engine_resolved(false); // fuel is counted by the tree walker (limits.rs)
    it.set_args(vec![FILE.to_string()]);
    it.set_caps(Some(CapGate::new(
        Grant::sandbox(Path::new(ROOT)),
        CapMode::Enforce,
    )));
    let sim = SimConfig {
        seed: opts.seed,
        faults: 0.0,
    };
    runner::install_simulation(&mut it, &sim, Path::new(ROOT), &Path::new(ROOT).join("tmp"));
    it.set_real_sleep(false);
    it.set_limits(LimitSpec {
        fuel: Some(opts.fuel),
        output: Some(opts.output_bytes),
        ..LimitSpec::default()
    });
    it
}

#[cfg(test)]
mod tests;
