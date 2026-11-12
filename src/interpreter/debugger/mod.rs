//! Time-travel debugging (ADR-0035): stop the tree walker at a statement, answer commands about the stopped state,
//! and go back by re-executing the deterministic run from its start (`session.rs` drives the re-executions).
//!
//! A step is one statement started by the walker, counted from 1 across the run and its tasks. The hook in
//! `exec_stmt` costs one `Option` test when no debugger is attached.

pub mod lines;
pub mod render;
pub mod script;
pub mod session;

use super::engine::Interpreter;
use super::error::{Ctl, ErrorKind, RuntimeError, R};
use crate::parser::Statement;
use crate::source_map::Location;
use lines::Pos;
use script::{Cmd, Line};
use serde_json::{json, Value as Json};
use std::path::PathBuf;

/// Where the run goes after a stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resume {
    /// Stop before the first statement.
    Start,
    /// Stop after N more statements start.
    Steps(u64),
    /// Run to a breakpoint, the failure or the end.
    Continue,
    /// Re-executing after `back`: stop at this step.
    RunTo(u64),
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
}

pub struct Debugger {
    cmds: Vec<Line>,
    next_cmd: usize,
    breakpoints: Vec<Breakpoint>,
    pub(crate) events: Vec<Json>,
    resume: Resume,
    /// Statements started in this execution.
    pub(crate) step: u64,
    /// The last located statement started (a breakpoint stops once per entry to its line).
    prev_line: Option<Pos>,
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
            here: None,
            lines: lines::Lines::default(),
            out_seen: (0, 0),
            restart: None,
            aborting: false,
            script_ended: false,
            failing: false,
            root,
        }
    }

    /// Get ready for one more execution of the program from its start.
    pub(crate) fn begin_execution(&mut self) {
        self.resume = match self.restart.take() {
            Some(t) => Resume::RunTo(t),
            None => Resume::Start,
        };
        self.step = 0;
        self.prev_line = None;
        self.aborting = false;
        self.failing = false;
    }

    fn file_name(&self, l: &Location) -> String {
        let p = l.file.strip_prefix(&self.root).unwrap_or(&l.file);
        p.to_string_lossy().to_string()
    }

    fn bp_hit(&self, p: &Pos) -> bool {
        let file = self.file_name(p.loc);
        self.breakpoints.iter().any(|b| b.line == p.line && b.file.as_ref().is_none_or(|f| file == *f || file.ends_with(&format!("/{f}"))))
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

    fn stopped(&mut self, it: &Interpreter, reason: &str, step: u64, pos: Option<Pos>, message: Option<&str>) {
        self.flush_output(it);
        self.here = pos;
        let mut e = json!({
            "event": "stopped",
            "reason": reason,
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
        if let Some(m) = message {
            e["message"] = json!(m);
        }
        self.events.push(e);
    }

    fn error(&mut self, command: &str, message: impl Into<String>) {
        self.events.push(json!({ "event": "error", "command": command, "message": message.into() }));
    }

    /// Carry out commands until one resumes the run. `here` is the stopped step; `None` after the program ended
    /// (`steps` statements ran), when only `break` and `back` still apply.
    pub(crate) fn commands(&mut self, it: &mut Interpreter, here: Option<u64>, steps: u64) -> Next {
        loop {
            let Some(line) = self.cmds.get(self.next_cmd).cloned() else {
                self.script_ended = true;
                return Next::End;
            };
            self.next_cmd += 1;
            let ended = "the program has exited; `back` goes back into it";
            match line.cmd {
                Cmd::Break { file, line } => self.breakpoints.push(Breakpoint { file, line }),
                Cmd::Continue | Cmd::Step(_) if here.is_none() => self.error(&line.text, ended),
                Cmd::Continue => {
                    self.resume = Resume::Continue;
                    return Next::Resume;
                }
                Cmd::Step(n) => {
                    self.resume = Resume::Steps(n);
                    return Next::Resume;
                }
                Cmd::Back(n) => {
                    let current = here.unwrap_or(steps + 1);
                    if n >= current {
                        self.error(&line.text, format!("cannot go back {n} from step {current}: the run starts at step 1"));
                        continue;
                    }
                    return Next::Restart(current - n);
                }
                Cmd::Print(_) | Cmd::Locals | Cmd::Where if here.is_none() => self.error(&line.text, ended),
                Cmd::Print(name) => match it.lookup_var(&name) {
                    None => self.error(&line.text, format!("no binding `{name}` in the current frame")),
                    Some(slot) => {
                        let v = slot.borrow().clone();
                        match format_quietly(it, &v) {
                            Ok(text) => self.events.push(json!({ "event": "value", "name": name, "value": text })),
                            Err(m) => self.error(&line.text, m),
                        }
                    }
                },
                Cmd::Locals => {
                    let vars: Vec<(&'static str, super::value::Slot)> = it.scope.vars.clone();
                    let mut bindings: Vec<Json> = Vec::new();
                    for (i, (name, slot)) in vars.iter().enumerate() {
                        // a shadowed binding is not visible: keep the last of each name
                        if vars[i + 1..].iter().any(|(n, _)| n == name) {
                            continue;
                        }
                        let v = slot.borrow().clone();
                        let text = format_quietly(it, &v).unwrap_or_else(|m| format!("<{m}>"));
                        bindings.push(json!({ "name": name, "value": text }));
                    }
                    let function = it.backtrace.last().map_or("<top>".to_string(), |(n, _)| n.to_string());
                    self.events.push(json!({ "event": "locals", "function": function, "bindings": bindings }));
                }
                Cmd::Where => {
                    let mut frames = Vec::new();
                    let mut at = self.here.map(|p| (p.loc, p.line, p.column));
                    for (name, site) in it.backtrace.iter().rev() {
                        let mut f = json!({ "function": name.to_string() });
                        if let Some((l, line, column)) = at {
                            f["file"] = json!(self.file_name(l));
                            f["line"] = json!(line);
                            f["column"] = json!(column);
                        }
                        frames.push(f);
                        at = site.map(|l| (l, l.line, l.display_column()));
                    }
                    self.events.push(json!({ "event": "stack", "frames": frames }));
                }
            }
        }
    }
}

/// `{:?}` of a value; anything the formatting writes (a user `Debug` impl that prints) is discarded.
fn format_quietly(it: &mut Interpreter, v: &super::value::Value) -> Result<String, String> {
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
        let reason = match d.resume {
            Resume::Start => Some("start"),
            Resume::Steps(n) if n <= 1 => Some("step"),
            Resume::Steps(n) => {
                d.resume = Resume::Steps(n - 1);
                None
            }
            Resume::RunTo(t) if t == step => Some("back"),
            Resume::RunTo(_) => None,
            Resume::Continue => loc.filter(|l| d.bp_hit(l) && !d.prev_line.is_some_and(|p| p.line == l.line && p.loc.file == l.loc.file)).map(|_| "breakpoint"),
        };
        if loc.is_some() {
            d.prev_line = loc;
        }
        if let Some(reason) = reason {
            self.mark(loc.map(|p| p.loc));
            d.stopped(self, reason, step, loc, None);
            if let Err(c) = self.debugger_decide(&mut d, Some(step)) {
                self.debug = Some(d);
                return Err(c);
            }
        }
        self.debug = Some(d);
        let r = self.exec_stmt_plain(s);
        if !matches!(self.debug.as_ref(), Some(d) if !d.aborting) {
            return r;
        }
        let failed = is_failure(&r).map(|e| e.message.clone());
        let mut d = self.debug.take().expect("debugger");
        match failed {
            Some(msg) if !d.failing => {
                d.failing = true;
                d.stopped(self, "failure", step, loc, Some(&msg));
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
