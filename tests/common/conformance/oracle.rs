//! Independent oracles. None of these runs through a Windjammer backend.
//!
//! * `Known`   - published/known-answer lines typed in the program header (`// expect:`).
//! * `Reference` - `<name>.ref.py`: an independent model; committed vectors in `<name>.vectors/<seed>.out`
//!   are DERIVED from it by `python3 <name>.ref.py <seed>` and re-checked whenever python3 exists.
//! * the Windjammer interpreter is handled in `backends.rs` as an explicitly weaker 4th voice.

use super::compare::Trace;
use super::program::Program;
use super::toolchain::{Tool, ToolProbe};
use std::path::PathBuf;
use std::process::Command;

pub fn vector_path(p: &Program, seed: u64) -> PathBuf {
    p.dir().join(format!("{}.vectors", p.name)).join(format!("{seed}.out"))
}

/// Run the reference model directly (requires python3).
pub fn run_reference(p: &Program, seed: u64) -> Option<String> {
    let script = p.path.with_extension("ref.py");
    let out = Command::new("python3").arg("-I").arg(&script).arg(seed.to_string()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// Expected trace for a (program, seed): committed vector, else known-answer header lines.
pub fn expected(p: &Program, seed: u64) -> Result<Trace, String> {
    if p.has_ref {
        let path = vector_path(p, seed);
        let text = std::fs::read_to_string(&path)
            .map_err(|_| format!("missing committed vector {} (run python3 {}.ref.py {seed})", path.display(), p.name))?;
        return Ok(Trace::from_expect(text.lines().map(|l| l.to_string()).collect(), p.expect_exit));
    }
    if p.known_expect.is_empty() && p.expect_exit == 0 {
        return Err(format!("{}: no oracle (need `// expect:` lines or a .ref.py)", p.name));
    }
    Ok(Trace::from_expect(p.known_expect.clone(), p.expect_exit))
}

/// Re-derive every committed vector from its reference model; list mismatches.
pub fn verify_vectors(programs: &[Program], probe: &ToolProbe) -> Vec<String> {
    let mut bad = Vec::new();
    if !probe.has(Tool::Python3) {
        return bad;
    }
    for p in programs.iter().filter(|p| p.has_ref) {
        for &seed in &p.seeds {
            let derived = run_reference(p, seed);
            let committed = std::fs::read_to_string(vector_path(p, seed)).ok();
            if derived.is_none() || derived != committed {
                bad.push(format!("{} seed {seed}: committed vector differs from reference model", p.name));
            }
        }
    }
    bad
}
