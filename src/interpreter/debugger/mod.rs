//! Time-travel debugging (ADR-0035): stop the tree walker at a statement, answer commands about the stopped state,
//! and go back by re-executing the deterministic run from its start (`session.rs` drives the re-executions).
//!
//! A step is one statement started by the walker, counted from 1 across the run and its tasks. The hook in
//! `exec_stmt` costs one `Option` test when no debugger is attached.

mod commands;
pub mod lines;
mod readonly;
pub mod render;
pub mod script;
pub mod session;

use super::engine::Interpreter;
use super::error::{Ctl, ErrorKind, RuntimeError, R};
use crate::parser::Statement;
use crate::source_map::Location;
use lines::Pos;
use script::Line;
use serde_json::{json, Value as Json};
use std::path::PathBuf;

/// Where the run goes after a stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resume {
    /// Stop before the first statement.
    Start,
    /// Stop after N more statements start.
    Steps(u64),
    /// Run to a breakpoint, a watched change, the failure or the end.
    Continue,
    /// Re-executing after `back` (or `--at-effect`): stop at this step.
    RunTo(u64),
    /// `next`: run until a statement of this task starts at this call depth or a shallower one.
    Over { depth: usize, task: usize },
    /// `finish`: run until a statement starts at a shallower call depth than the frame being finished.
    Finish { depth: usize },
    /// `--at-effect K`: run silently to the statement that answers journal event K (then re-execute to stop there).
    Probe(u64),
}

/// What the command loop decided at a stop.
pub(crate) enum Next {
    Resume,
    /// Re-execute from the start and stop at this step.
    Restart(u64),
    /// The script has no more commands.
    End,
}

struct Breakpoint {
    file: Option<String>,
    line: usize,
    /// `break LINE if EXPR`: stop only when this read-only expression is true.
    cond: Option<String>,
}

/// `watch NAME`: stop when the local changes, in any activation of the function it was set in.
struct Watch {
    name: String,
    function: String,
    depth: usize,
    /// The activation being watched has started and `last` is its previous value.
    active: bool,
    last: Option<String>,
}

/// Why a stop happened, and what the stop event says besides.
pub(crate) struct Why {
    reason: &'static str,
    extra: Json,
}

impl Why {
    fn plain(reason: &'static str) -> Why {
        Why { reason, extra: json!({}) }
    }
}

pub struct Debugger {
    cmds: Vec<Line>,
    next_cmd: usize,
    breakpoints: Vec<Breakpoint>,
    watches: Vec<Watch>,
    pub(crate) events: Vec<Json>,
    resume: Resume,
    /// Statements started in this execution.
    pub(crate) step: u64,
    /// The last located statement started (a breakpoint stops once per entry to its line).
    prev_line: Option<Pos>,
    /// The last statement started, by address: the same statement again is a new entry to its line (a loop body).
    prev_stmt: usize,
    /// Where the run is stopped (the innermost frame of `where`).
    here: Option<Pos>,
    lines: lines::Lines,
    /// Pieces of captured stdout / stderr already reported in this session (output is a deterministic prefix).
    out_seen: (usize, usize),
    /// Set by `back`: re-execute and stop at this step.
    pub(crate) restart: Option<u64>,
    /// The run is being abandoned (`back`, or the script ended): every statement fails at once.
    aborting: bool,
    pub(crate) script_ended: bool,
    /// A failure was reported and is still propagating (do not stop again in every enclosing statement).
    failing: bool,
    /// The program's directory: files in events are shown relative to it.
    root: PathBuf,
    /// `--at-effect K`: the journal event the session starts at, until the probe run has found its statement.
    probe: Option<u64>,
    /// The reason the next `RunTo` stop reports (`back`, or `effect` after a probe).
    run_to_reason: &'static str,
    /// While `finish` runs: the value the frame's last completed statement produced (its tail, or a `return`).
    finish_value: Option<super::value::Value>,
    /// The function `finish` is finishing.
    finishing: Option<String>,
}

pub(crate) const ABORT: &str = "the debugger left this run";

fn abort() -> Ctl {
    Ctl::Err(Box::new(RuntimeError::new(ErrorKind::Limit, ABORT)))
}

impl Debugger {
    pub fn new(cmds: Vec<Line>, root: PathBuf) -> Debugger {
        Debugger {
            cmds,
            next_cmd: 0,
            breakpoints: Vec::new(),
            events: Vec::new(),
            resume: Resume::Start,
            step: 0,
            prev_line: None,
            prev_stmt: 0,
            here: None,
            lines: lines::Lines::default(),
            out_seen: (0, 0),
            restart: None,
            aborting: false,
            script_ended: false,
            failing: false,
            root,
            probe: None,
            run_to_reason: "back",
            finish_value: None,
            finishing: None,
            watches: Vec::new(),
        }
    }

    /// Start the session at the statement that answers journal event `k` (1-based; `--at-effect`).
    pub fn start_at_effect(&mut self, k: u64) {
        self.probe = Some(k);
    }

    /// Get ready for one more execution of the program from its start.
    pub(crate) fn begin_execution(&mut self) {
        self.resume = match (self.restart.take(), self.probe) {
            (Some(t), _) => Resume::RunTo(t),
            (None, Some(k)) => Resume::Probe(k),
            (None, None) => Resume::Start,
        };
        self.step = 0;
        self.prev_line = None;
        self.prev_stmt = 0;
        self.finish_value = None;
        for w in &mut self.watches {
            w.active = false;
            w.last = None;
        }
        self.aborting = false;
        self.failing = false;
    }

    fn file_name(&self, l: &Location) -> String {
        let p = l.file.strip_prefix(&self.root).unwrap_or(&l.file);
        p.to_string_lossy().to_string()
    }

    /// The conditions of the breakpoints on this statement's line (`None` = unconditional).
    fn breakpoints_at(&self, p: &Pos) -> Vec<Option<String>> {
        let file = self.file_name(p.loc);
        let here = |b: &&Breakpoint| b.line == p.line && b.file.as_ref().is_none_or(|f| file == *f || file.ends_with(&format!("/{f}")));
        self.breakpoints.iter().filter(here).map(|b| b.cond.clone()).collect()
    }

    /// Report the program output written since the last report.
    pub(crate) fn flush_output(&mut self, it: &Interpreter) {
        let out = &it.output;
        if out.len() > self.out_seen.0 {
            let text: String = out[self.out_seen.0..].concat();
            self.out_seen.0 = out.len();
            self.events.push(json!({ "event": "output", "stream": "stdout", "text": text }));
        }
        let err: &[String] = it.err_output.as_deref().unwrap_or(&[]);
        if err.len() > self.out_seen.1 {
            let text: String = err[self.out_seen.1..].concat();
            self.out_seen.1 = err.len();
            self.events.push(json!({ "event": "output", "stream": "stderr", "text": text }));
        }
    }

    fn stopped(&mut self, it: &Interpreter, why: Why, step: u64, pos: Option<Pos>) {
        self.flush_output(it);
        self.here = pos;
        let mut e = json!({
            "event": "stopped",
            "reason": why.reason,
            "step": step,
            "task": it.sched.current,
            "function": it.backtrace.last().map_or("<top>".to_string(), |(n, _)| n.to_string()),
        });
        if let Some(p) = pos {
            e["file"] = json!(self.file_name(p.loc));
            e["line"] = json!(p.line);
            e["column"] = json!(p.column);
        }
        if let Some(n) = it.events_answered() {
            e["effect"] = json!(n);
        }
        if let Json::Object(extra) = why.extra {
            for (k, v) in extra {
                e[k] = v;
            }
        }
        self.events.push(e);
    }

    fn error(&mut self, command: &str, message: impl Into<String>) {
        self.events.push(json!({ "event": "error", "command": command, "message": message.into() }));
    }
}

impl Debugger {
    /// `--at-effect`: the first statement that starts after journal event K was answered ends the probe run; the
    /// statement before it is the one that answered it, and the session re-executes to stop there.
    fn probe_check(&mut self, it: &Interpreter, step: u64) -> Option<Ctl> {
        let Resume::Probe(k) = self.resume else { return None };
        if it.events_answered().is_none_or(|n| n < k) {
            return None;
        }
        self.found_effect_step(step.saturating_sub(1).max(1));
        Some(abort())
    }

    fn found_effect_step(&mut self, step: u64) {
        self.probe = None;
        self.restart = Some(step);
        self.run_to_reason = "effect";
        self.aborting = true;
    }

    /// After a probe run that reached the end: the last statement answered event K, or the journal is too short.
    pub(crate) fn probe_finished(&mut self, answered: Option<u64>) -> Result<(), String> {
        let Some(k) = self.probe else { return Ok(()) };
        match answered {
            Some(n) if n >= k && self.step > 0 => {
                self.found_effect_step(self.step);
                Ok(())
            }
            Some(n) => Err(format!("--at-effect {k}: the journal holds only {n} event{}", if n == 1 { "" } else { "s" })),
            None => Err("--at-effect needs a journal: it names an event of a recorded run (`wj replay LOG PROGRAM --at-effect K`)".into()),
        }
    }

    /// Should the run stop at the statement that is starting? The resume mode says where it was going; breakpoints and
    /// watches also stop a free-running (`continue`, `next`, `finish`) run.
    fn decide(&mut self, it: &mut Interpreter, step: u64, loc: Option<Pos>, sid: usize) -> Option<Why> {
        let (depth, task) = (it.backtrace.len(), it.sched.current);
        let primary = match self.resume {
            Resume::Start => Some("start"),
            Resume::Steps(n) if n <= 1 => Some("step"),
            Resume::Steps(n) => {
                self.resume = Resume::Steps(n - 1);
                None
            }
            Resume::RunTo(t) if t == step => Some(self.run_to_reason),
            Resume::Over { depth: d, task: t } if depth <= d && task == t => Some("next"),
            Resume::Finish { depth: d } if depth < d => Some("finish"),
            _ => None,
        };
        let watch = self.observe_watches(it, depth);
        let free = matches!(self.resume, Resume::Continue | Resume::Over { .. } | Resume::Finish { .. });
        let same_line = |l: &Pos| self.prev_line.is_some_and(|p| p.line == l.line && p.loc.file == l.loc.file) && self.prev_stmt != sid;
        let entered = loc.filter(|l| !same_line(l));
        let mut why = primary.map(Why::plain);
        if free && why.is_none() {
            why = entered.and_then(|l| self.breakpoint_stop(it, &l));
        }
        match (&mut why, watch) {
            (None, Some(w)) if free => Some(Why { reason: "watch", extra: w }),
            (Some(y), Some(w)) if free => {
                if let (Json::Object(mine), Json::Object(theirs)) = (&mut y.extra, w) {
                    mine.extend(theirs);
                }
                why
            }
            _ => why,
        }
    }

    /// A breakpoint on this line whose condition (if any) holds.
    fn breakpoint_stop(&mut self, it: &mut Interpreter, at: &Pos) -> Option<Why> {
        for cond in self.breakpoints_at(at) {
            let Some(c) = cond else { return Some(Why::plain("breakpoint")) };
            match readonly::eval_bool(it, &c) {
                Ok(true) => return Some(Why { reason: "breakpoint", extra: json!({ "condition": c }) }),
                Ok(false) => {}
                Err(m) => return Some(Why { reason: "breakpoint", extra: json!({ "message": format!("condition `{c}` could not be evaluated: {m}") }) }),
            }
        }
        None
    }

    /// The text of a local now (`None` when it is not bound in the current frame), the digest `watch` compares.
    fn local_text(it: &mut Interpreter, name: &str) -> Option<String> {
        let slot = it.lookup_var(name)?;
        let v = slot.borrow().clone();
        Some(format_quietly(it, &v).unwrap_or_else(|m| format!("<{m}>")))
    }

    /// Compare every watched local with its value at the previous statement of the same activation. Returns the
    /// first change as stop-event fields. Baselines move on every statement, whatever the resume mode.
    fn observe_watches(&mut self, it: &mut Interpreter, depth: usize) -> Option<Json> {
        if self.watches.is_empty() {
            return None;
        }
        let function = it.backtrace.last().map(|(n, _)| n.to_string());
        let mut hit = None;
        for i in 0..self.watches.len() {
            let w = &mut self.watches[i];
            if depth < w.depth {
                w.active = false; // the watched frame returned; the next call starts a new activation
                continue;
            }
            if depth != w.depth || function.as_deref() != Some(w.function.as_str()) {
                continue;
            }
            let name = w.name.clone();
            let now = Self::local_text(it, &name);
            let w = &mut self.watches[i];
            if !w.active {
                w.active = true;
            } else if now != w.last && hit.is_none() {
                hit = Some(json!({ "watch": name, "old": w.last, "new": now }));
            }
            w.last = now;
        }
        hit
    }

    /// `finish`: remember what the frame's statements produce; the last one completed is the frame's value.
    fn note_value(&mut self, depth: usize, r: &R) {
        if !matches!(self.resume, Resume::Finish { depth: d } if d == depth) {
            return;
        }
        self.finish_value = match r {
            Ok(v) | Err(Ctl::Return(v)) => Some(v.clone()),
            _ => None,
        };
    }

    /// Report a stop. A `finish` stop first says what the finished frame returned.
    fn report_stop(&mut self, it: &mut Interpreter, why: Why, step: u64, loc: Option<Pos>) {
        if why.reason == "finish" {
            self.report_returned(it);
        }
        self.stopped(it, why, step, loc);
    }

    /// The `returned` event of a `finish` (the function that was finished, and its value).
    pub(crate) fn report_returned(&mut self, it: &mut Interpreter) {
        let Resume::Finish { .. } = self.resume else { return };
        let value = self.finish_value.take().map(|v| format_quietly(it, &v).unwrap_or_else(|m| format!("<{m}>")));
        let function = self.finishing.take().unwrap_or_default();
        self.events.push(json!({ "event": "returned", "function": function, "value": value }));
    }
}

/// `{:?}` of a value; anything the formatting writes (a user `Debug` impl that prints) is discarded.
pub(crate) fn format_quietly(it: &mut Interpreter, v: &super::value::Value) -> Result<String, String> {
    let (o, e) = (it.output.len(), it.err_output.as_ref().map(Vec::len));
    let r = it.debug_string(v);
    it.output.truncate(o);
    if let (Some(n), Some(err)) = (e, it.err_output.as_mut()) {
        err.truncate(n);
    }
    r.map_err(|c| match c {
        Ctl::Err(e) => e.message,
        _ => "cannot format this value".to_string(),
    })
}

fn is_failure(r: &R) -> Option<&RuntimeError> {
    match r {
        Err(Ctl::Err(e)) if matches!(e.kind, ErrorKind::Panic | ErrorKind::Semantic | ErrorKind::Unsupported | ErrorKind::CapabilityDenied) => Some(e.as_ref()),
        _ => None,
    }
}

impl Interpreter {
    pub fn attach_debugger(&mut self, d: Box<Debugger>) {
        self.debug = Some(d);
    }

    pub fn detach_debugger(&mut self) -> Option<Box<Debugger>> {
        self.debug.take()
    }

    /// `exec_stmt` with the debugger attached: count the step, stop when asked, and stop at a failure before it unwinds.
    pub(crate) fn exec_stmt_debugged(&mut self, s: &'static Statement<'static>) -> R {
        let Some(mut d) = self.debug.take() else { return self.exec_stmt_plain(s) };
        if d.aborting {
            self.debug = Some(d);
            return Err(abort());
        }
        d.step += 1;
        let step = d.step;
        let loc = d.lines.start(s);
        if let Some(c) = d.probe_check(self, step) {
            self.debug = Some(d);
            return Err(c);
        }
        let sid = s as *const Statement as usize;
        let why = d.decide(self, step, loc, sid);
        if loc.is_some() {
            d.prev_line = loc;
        }
        d.prev_stmt = sid;
        if let Some(why) = why {
            self.mark(loc.map(|p| p.loc));
            d.report_stop(self, why, step, loc);
            if let Err(c) = self.debugger_decide(&mut d, Some(step)) {
                self.debug = Some(d);
                return Err(c);
            }
        }
        let depth = self.backtrace.len();
        self.debug = Some(d);
        let r = self.exec_stmt_plain(s);
        if !matches!(self.debug.as_ref(), Some(d) if !d.aborting) {
            return r;
        }
        let failed = is_failure(&r).map(|e| e.message.clone());
        let mut d = self.debug.take().expect("debugger");
        d.note_value(depth, &r);
        match failed {
            Some(msg) if !d.failing => {
                d.failing = true;
                let why = Why { reason: "failure", extra: json!({ "message": msg }) };
                d.stopped(self, why, step, loc);
                if let Err(c) = self.debugger_decide(&mut d, Some(step)) {
                    self.debug = Some(d);
                    return Err(c);
                }
            }
            Some(_) => {}
            None => d.failing = false,
        }
        self.debug = Some(d);
        r
    }

    /// Run the command loop at a stop; `Err` abandons this execution.
    fn debugger_decide(&mut self, d: &mut Debugger, here: Option<u64>) -> Result<(), Ctl> {
        match d.commands(self, here, d.step) {
            Next::Resume => Ok(()),
            Next::Restart(t) => {
                d.restart = Some(t);
                d.aborting = true;
                Err(abort())
            }
            Next::End => {
                d.aborting = true;
                Err(abort())
            }
        }
    }

    /// Journal events answered so far, under `--replay` (the position `--at-effect` will name).
    pub(crate) fn events_answered(&self) -> Option<u64> {
        let p = self.port.as_ref()?.borrow();
        (p.kind() == super::effect_port::Kind::Replay).then(|| p.seq())
    }
}
