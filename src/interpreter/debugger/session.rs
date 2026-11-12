//! The `wj replay` session (ADR-0035): run the program under the debugger in the tree walker, answering effects from a
//! journal (`--replay`) or the seeded world (`--simulate`); `back` starts a fresh interpreter and runs it to the
//! earlier step. Deterministic replay is what makes the re-execution land in the same state.

use super::super::engine::Interpreter;
use super::super::error::ErrorKind;
use super::super::runner::{exit_code_for, install_simulation, main_result_code, setup_effect_port, with_big_stack, RunOptions};
use super::{Debugger, Next, ABORT};
use serde_json::{json, Value as Json};
use std::path::{Path, PathBuf};

/// Where the effects of the debugged run come from.
#[derive(Clone, Debug)]
pub enum Source {
    /// A journal written by `wj run --record`.
    Log(PathBuf),
    /// The simulated world of `wj run --simulate`.
    Simulate { seed: u64, faults: f64 },
}

#[derive(Clone, Debug)]
pub struct SessionSpec {
    pub entry: PathBuf,
    pub source: Source,
    /// `--allow` grants, as for the recording.
    pub allow: Vec<String>,
    pub script: Vec<super::script::Line>,
    /// `--at-effect K`: start at the statement that answers the K-th event of the journal (1-based).
    pub at_effect: Option<u64>,
}

/// One fresh interpreter, set up as `wj run` would set it up for this source, in the walker with machine code off.
fn prepare(spec: &SessionSpec) -> Result<Interpreter, String> {
    let mut it = Interpreter::new_capturing();
    it.set_capture_stderr(true);
    it.guard_stack();
    it.set_engine_resolved(false);
    it.set_jit(false);
    it.set_caps(super::super::caps::gate_for_run(&spec.entry, false, &spec.allow, false, "enforce")?);
    match &spec.source {
        Source::Log(log) => {
            let opts = RunOptions { replay: Some(log.clone()), ..RunOptions::default() };
            setup_effect_port(&mut it, &opts, &spec.entry)?;
        }
        Source::Simulate { seed, faults } => {
            let cfg = super::super::effect_port::sim::SimConfig { seed: *seed, faults: *faults };
            let root = super::super::caps::project_root(&spec.entry);
            install_simulation(&mut it, &cfg, &root, &root.join(".wj-sim").join("tmp"));
        }
    }
    it.set_real_sleep(false);
    Ok(it)
}

/// Run the session; the events in order. `Err` is a setup failure (an unreadable journal, say).
pub fn run(mut spec: SessionSpec) -> Result<Vec<Json>, String> {
    // a replay re-applies the grants the recording had, as well as any `--allow` given now
    if let Source::Log(log) = &spec.source {
        for grant in super::super::effect_port::log::read_header(log)?.grants {
            if !spec.allow.contains(&grant) {
                spec.allow.push(grant);
            }
        }
    }
    with_big_stack(move || run_here(&spec))
}

fn run_here(spec: &SessionSpec) -> Result<Vec<Json>, String> {
    let root = spec.entry.parent().map(Path::to_path_buf).unwrap_or_default();
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let mut d = Box::new(Debugger::new(spec.script.clone(), root));
    if let Some(k) = spec.at_effect {
        d.start_at_effect(k);
    }
    loop {
        d.begin_execution();
        let mut it = prepare(spec)?;
        if let Err(e) = it.load_program_at(&spec.entry) {
            d.events.push(json!({ "event": "exited", "code": exit_code_for(&e), "steps": 0, "message": e.render() }));
            return Ok(d.events);
        }
        it.attach_debugger(d);
        let r = it.call_main();
        d = it.detach_debugger().expect("the debugger stays attached");
        if d.restart.is_some() {
            continue;
        }
        if d.script_ended {
            return Ok(d.events);
        }
        d.probe_finished(it.events_answered())?;
        if d.restart.is_some() {
            continue;
        }
        let ok = r.is_ok();
        let (code, message) = match r {
            Ok(v) => {
                let code = main_result_code(&mut it, &v);
                (code, None)
            }
            Err(e) if e.kind == ErrorKind::Limit && e.message == ABORT => return Ok(d.events),
            Err(e) => (exit_code_for(&e), (!matches!(e.kind, ErrorKind::Exit(_))).then(|| e.render())),
        };
        d.flush_output(&it);
        if ok {
            d.report_returned(&mut it);
        }
        let mut ev = json!({ "event": "exited", "code": code, "steps": d.step });
        if let Some(m) = message {
            ev["message"] = json!(m);
        }
        d.events.push(ev);
        let steps = d.step;
        match d.commands(&mut it, None, steps) {
            Next::Restart(t) => d.restart = Some(t),
            Next::Resume | Next::End => return Ok(d.events),
        }
    }
}
