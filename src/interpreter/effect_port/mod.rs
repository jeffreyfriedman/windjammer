//! The effect port (ADR-0021 section 10, stage E3): every effect that can differ between two runs of one program
//! goes through here. `Record` performs it live and appends it to a log; `Replay` answers it from the log and
//! performs nothing; `Simulate` answers it from a seeded world. A run with no port (`Interpreter::port == None`)
//! pays one pointer test on the sink path.
//!
//! What goes through the port: sink calls (the same table the capability gate uses), the real-clock readings under
//! `Instant`/`SystemTime`/`DateTime`, stdin, and the host observations that are not sinks but differ between
//! runs (`process::pid`, `env::temp_dir`, ...). Random numbers and task interleavings are not events: they are
//! functions of `rng_seed` and `sched_seed` in the header. Sleeping is a scheduler operation on a virtual clock,
//! never a live effect, so it is not an event either.

pub mod codec;
pub mod log;
pub mod sim;
pub mod sim_fs;

use super::engine::Interpreter;
use super::error::{Ctl, ErrorKind, RuntimeError};
use super::value::Value;
use crate::ir::effect_facts::sinks;
use log::{Event, Header};
use serde_json::{json, Value as Json};
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;

type R = Result<Value, Ctl>;

/// Host reads that are not sinks (never denied) but still differ between runs, so replay must answer them.
pub static OBSERVED: &[&str] = &[
    "process::pid", "env::temp_dir", "env::current_dir", "fs::current_dir", "path::current_dir",
    // password hashing draws a salt from the operating system, outside the seeded generator
    "crypto::hash_password", "crypto::hash_password_with_cost", "crypto::hash_password_argon2id",
];

/// Sinks that are not events: time is read through the clock sources, sleeping is virtual, random is seeded,
/// console output is the program's own product, and `cap::*` minting returns a value the grant already decided
/// (the use of that value reaches the real sink, which is an event).
fn not_an_event(path: &str, uses: &[sinks::SinkUse]) -> bool {
    use crate::ir::effect_facts::EffectKind::{Console, Random, Time};
    let module = path.rsplit_once("::").map_or(path, |(m, _)| m.rsplit("::").next().unwrap_or(m));
    if module == "cap" || path.ends_with("io::stdin") {
        return true;
    }
    if path.ends_with("io::read_line") || path.ends_with("io::read_to_string") {
        return false;
    }
    uses.iter().any(|u| matches!(u.effect, Time | Random | Console))
}

pub use crate::ir::effect_facts::sinks::HANDLE_SINKS;

fn is_handle_sink(path: &str) -> bool {
    HANDLE_SINKS.iter().any(|h| path.ends_with(h))
}

/// Does `path` go through the port?
pub fn routed(path: &str) -> bool {
    if path.ends_with("mcp::call") {
        return true;
    }
    if is_handle_sink(path) {
        return true;
    }
    if !sinks::may_be_sink(path) {
        return OBSERVED.iter().any(|o| path.ends_with(o));
    }
    match sinks::lookup(path) {
        Some(uses) => !not_an_event(path, uses),
        None => OBSERVED.iter().any(|o| path.ends_with(o)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Record,
    Replay,
    Simulate,
}

enum Mode {
    Record { out: std::io::BufWriter<std::fs::File>, path: PathBuf },
    Replay { events: VecDeque<Event> },
    Simulate(Box<sim::World>),
}

pub struct EffectPort {
    mode: Mode,
    pub header: Header,
    seq: u64,
    depth: u32,
    /// A divergence found where no error can be returned (a clock reading); raised at the next sink call or at the end.
    pending: Option<RuntimeError>,
    recorded_digest: String,
}

fn fail(kind: ErrorKind, msg: impl Into<String>) -> Ctl {
    Ctl::Err(Box::new(RuntimeError::new(kind, msg)))
}

impl EffectPort {
    pub fn record(path: &std::path::Path, header: Header) -> std::io::Result<EffectPort> {
        let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
        writeln!(out, "{}", log::header_line(&header))?;
        let _ = std::fs::remove_dir_all(log::blob_dir(path));
        Ok(EffectPort { mode: Mode::Record { out, path: path.to_path_buf() }, recorded_digest: header.program.clone(), header, seq: 0, depth: 0, pending: None })
    }

    pub fn replay(path: &std::path::Path, current_program: &str) -> Result<EffectPort, String> {
        let (header, events) = log::read_log(path)?;
        let recorded_digest = header.program.clone();
        let mut header = header;
        header.program = current_program.to_string();
        Ok(EffectPort { mode: Mode::Replay { events: events.into() }, header, seq: 0, depth: 0, pending: None, recorded_digest })
    }

    pub fn simulate(world: sim::World, header: Header) -> EffectPort {
        EffectPort { mode: Mode::Simulate(Box::new(world)), recorded_digest: header.program.clone(), header, seq: 0, depth: 0, pending: None }
    }

    pub fn kind(&self) -> Kind {
        match self.mode {
            Mode::Record { .. } => Kind::Record,
            Mode::Replay { .. } => Kind::Replay,
            Mode::Simulate(_) => Kind::Simulate,
        }
    }

    pub(crate) fn world(&mut self) -> Option<&mut sim::World> {
        match &mut self.mode {
            Mode::Simulate(w) => Some(w),
            _ => None,
        }
    }

    fn describe(effect: &str, op: &str, args: &Json) -> String {
        format!("{effect} {op}({})", args.as_array().map(|a| a.iter().map(log::canon).collect::<Vec<_>>().join(", ")).unwrap_or_default())
    }

    fn divergence(&self, seq: u64, want: Option<&Event>, effect: &str, op: &str, args: &Json) -> RuntimeError {
        let logged = want.map_or("(the log has no more events)".to_string(), |e| Self::describe(&e.effect, &e.op, &e.args));
        let mut msg = format!(
            "WJ0049: replay divergence at event {seq}\n  logged: {logged}\n  actual: {}",
            Self::describe(effect, op, args)
        );
        if self.recorded_digest != self.header.program {
            msg.push_str(&format!("\n  note: the log was recorded from program {:.12}, this is {:.12}", self.recorded_digest, self.header.program));
        }
        msg.push_str("\n  help: re-record with `wj run --interpret --record LOG FILE`");
        RuntimeError::new(ErrorKind::Semantic, msg)
    }

    /// Start of a recorded sink call: false when it is nested inside another one (then it is not an event).
    fn enter(&mut self) -> bool {
        self.depth += 1;
        self.depth == 1
    }

    fn leave_and_append(&mut self, effect: &str, op: &str, args: Json, result: Json, task: u64, vt_ns: u64) -> Result<(), RuntimeError> {
        self.depth -= 1;
        let seq = self.seq;
        self.seq += 1;
        let Mode::Record { out, path } = &mut self.mode else { return Ok(()) };
        let e = Event { seq, task, effect: effect.into(), op: op.into(), args, result, vt_ns };
        log::event_line(&e, path)
            .and_then(|l| writeln!(out, "{l}"))
            .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("cannot write the effect log: {e}")))
    }

    /// One event: record it, or take the next logged one and check it is the request we are making.
    fn step(&mut self, effect: &str, op: &str, args: Json, task: u64, vt_ns: u64, live: &mut dyn FnMut() -> Json) -> Result<Json, RuntimeError> {
        if let Some(p) = self.pending.take() {
            return Err(p);
        }
        let seq = self.seq;
        self.seq += 1;
        match &mut self.mode {
            Mode::Record { out, path } => {
                let result = live();
                let e = Event { seq, task, effect: effect.into(), op: op.into(), args, result: result.clone(), vt_ns };
                let line = log::event_line(&e, path).and_then(|l| writeln!(out, "{l}"));
                line.map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("cannot write the effect log: {e}")))?;
                Ok(result)
            }
            Mode::Replay { events } => {
                let next = events.pop_front();
                match next {
                    Some(e) if e.effect == effect && e.op == op && log::canon(&e.args) == log::canon(&args) => Ok(e.result),
                    other => Err(self.divergence(seq, other.as_ref(), effect, op, &args)),
                }
            }
            Mode::Simulate(_) => Ok(live()),
        }
    }

    pub(crate) fn is_simulating(&self) -> bool {
        matches!(self.mode, Mode::Simulate(_))
    }

    /// End of the run: a replay must have consumed the whole log; a recording is flushed.
    pub fn finish(&mut self) -> Result<(), RuntimeError> {
        if let Some(p) = self.pending.take() {
            return Err(p);
        }
        match &mut self.mode {
            Mode::Record { out, .. } => out.flush().map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("cannot write the effect log: {e}"))),
            Mode::Replay { events } if !events.is_empty() => {
                let e = events.front().cloned();
                let seq = self.seq;
                Err(self.divergence(seq, e.as_ref(), "(the program ended)", "", &json!([])))
            }
            _ => Ok(()),
        }
    }
}

fn effect_name(path: &str) -> String {
    sinks::lookup(path).and_then(|u| u.first()).map_or("host".to_string(), |u| u.effect.name())
}

fn rt_error_json(e: &RuntimeError) -> Json {
    let (k, n) = match &e.kind {
        ErrorKind::Panic => ("panic", 0),
        ErrorKind::Unsupported => ("unsupported", 0),
        ErrorKind::Semantic => ("semantic", 0),
        ErrorKind::CapabilityDenied => ("denied", 0),
        ErrorKind::Limit => ("limit", 0),
        ErrorKind::Exit(n) => ("exit", *n),
    };
    json!({ "rt_error": { "kind": k, "code": n, "message": e.message } })
}

fn rt_error_from(j: &Json) -> Option<RuntimeError> {
    let e = j.get("rt_error")?;
    let kind = match e["kind"].as_str()? {
        "panic" => ErrorKind::Panic,
        "unsupported" => ErrorKind::Unsupported,
        "denied" => ErrorKind::CapabilityDenied,
        "limit" => ErrorKind::Limit,
        "exit" => ErrorKind::Exit(e["code"].as_i64().unwrap_or(1) as i32),
        _ => ErrorKind::Semantic,
    };
    Some(RuntimeError::new(kind, e["message"].as_str().unwrap_or("").to_string()))
}

impl Interpreter {
    pub fn set_port(&mut self, port: EffectPort) {
        self.port = Some(std::cell::RefCell::new(port));
    }

    pub fn port_header(&self) -> Option<Header> {
        self.port.as_ref().map(|p| p.borrow().header.clone())
    }

    /// Raise a divergence found after the last sink call, and flush a recording.
    pub fn finish_port(&mut self) -> Result<(), RuntimeError> {
        match &self.port {
            Some(p) => p.borrow_mut().finish(),
            None => Ok(()),
        }
    }

    pub(crate) fn port_kind(&self) -> Option<Kind> {
        self.port.as_ref().map(|p| p.borrow().kind())
    }

    /// A sink call under the port. The capability gate has already run.
    pub(crate) fn port_call(&mut self, path: &str, args: Vec<Value>) -> R {
        let kind = self.port_kind().expect("port_call without a port");
        if is_handle_sink(path) {
            let mode = match kind {
                Kind::Record => "--record",
                Kind::Replay => "--replay",
                Kind::Simulate => "--simulate",
            };
            return Err(fail(
                ErrorKind::Unsupported,
                format!("`{}` returns a live handle, which {mode} cannot reproduce (E3 covers effects that return data)", sinks::canonical(path)),
            ));
        }
        if kind == Kind::Simulate {
            return sim::answer(self, path, args);
        }
        let key = sinks::canonical(path);
        let effect = effect_name(path);
        let args_j = Json::Array(args.iter().map(codec::encode_lossy).collect());
        let (task, vt) = (self.sched.current as u64, self.sched.clock);
        if kind == Kind::Record {
            // a sink that calls other sinks (a server handler, say) is one event: the nested calls run live, unlogged
            let nested = !self.port.as_ref().unwrap().borrow_mut().enter();
            if nested {
                return self.call_builtin_live(path, args);
            }
            let out = self.call_builtin_live(path, args);
            let (j, bad) = match &out {
                Ok(v) => match codec::encode(v) {
                    Ok(j) => (j, None),
                    Err(what) => (Json::Null, Some(what)),
                },
                Err(Ctl::Err(e)) => (rt_error_json(e), None),
                Err(_) => (Json::Null, None),
            };
            let logged = self.port.as_ref().unwrap().borrow_mut().leave_and_append(&effect, &key, args_j, j, task, vt);
            if let Err(e) = logged {
                return Err(Ctl::Err(Box::new(e)));
            }
            if let Some(what) = bad {
                return Err(fail(
                    ErrorKind::Unsupported,
                    format!("`{key}` returns {what}, which --record cannot put in an effect log (E3 records effects that return data)"),
                ));
            }
            return out;
        }
        // replay: the log answers
        let r = self.port.as_ref().unwrap().borrow_mut().step(&effect, &key, args_j, task, vt, &mut || Json::Null);
        match r {
            Err(e) => Err(Ctl::Err(Box::new(e))),
            Ok(j) => match rt_error_from(&j) {
                Some(e) => Err(Ctl::Err(Box::new(e))),
                None => codec::decode(&j).map_err(|m| fail(ErrorKind::Semantic, m)),
            },
        }
    }

    /// One reading of a host quantity that is not a sink call (the real clock, a line of stdin): live, logged or replayed.
    pub(crate) fn port_observe(&self, effect: &str, op: &str, live: &mut dyn FnMut() -> Json) -> Json {
        let Some(port) = &self.port else { return live() };
        let (task, vt) = (self.sched.current as u64, self.sched.clock);
        let mut p = port.borrow_mut();
        match p.step(effect, op, json!([]), task, vt, live) {
            Ok(j) => j,
            Err(e) => {
                p.pending = Some(e);
                Json::Null
            }
        }
    }

    /// One line of standard input, newline included (`""` at end of input). A simulated world has no terminal: end of input.
    pub(crate) fn stdin_line(&self) -> Result<String, String> {
        let mut live = || {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(_) => json!({ "ok": line }),
                Err(e) => json!({ "err": e.to_string() }),
            }
        };
        let j = match &self.port {
            Some(p) if p.borrow().is_simulating() => json!({ "ok": "" }),
            _ => self.port_observe("console", "Stdin.read_line", &mut live),
        };
        match j.get("err").and_then(|e| e.as_str()) {
            Some(e) => Err(e.to_string()),
            None => Ok(j["ok"].as_str().unwrap_or("").to_string()),
        }
    }

    /// A clock reading in nanoseconds (the real component of `Instant`/`SystemTime`).
    pub(crate) fn clock_reading(&self, op: &str, live: impl Fn() -> i128) -> i128 {
        if self.port.is_none() {
            return live();
        }
        let j = self.port_observe("time", op, &mut || json!({ "ns": live().to_string() }));
        j["ns"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Primitives that read something that can differ between two runs of one program.
    const SOURCES: &[&str] = &[
        "SystemTime::now()", "Instant::now()", "from_entropy", "OsRng", "thread_rng", "io::stdin()", "env::var(", "env::vars(", "env::current_dir",
        "env::temp_dir", "process::id()", "bcrypt::hash(", "getrandom", "std::net::", "TcpStream", "TcpListener", "UdpSocket",
    ];

    /// Every place the interpreter reads one, and what makes the effect port see it. A new place must be added here with its
    /// reason (or routed through the port): this is the audit behind the E3 gate "replay is bit-exact".
    const COVERED: &[(&str, &str, &str)] = &[
        ("builtins/time.rs", "SystemTime::now()", "real wall base, read through clock_reading (logged, replayed)"),
        ("engine.rs", "Instant::now()", "origin of the monotonic clock; readings are elapsed() through clock_reading"),
        ("builtins/std_sys.rs", "from_entropy", "default random stream; record and replay seed it from the header, simulate from the seed"),
        ("builtins/std_sys.rs", "env::current_dir", "OBSERVED path::/env::current_dir"),
        ("builtins/std_sys.rs", "process::id()", "OBSERVED process::pid"),
        ("builtins/std_runtime_gaps.rs", "env::current_dir", "OBSERVED path::current_dir"),
        ("builtins/std_cap.rs", "env::current_dir", "absolutizing a grant scope; the result is a check, never a program value"),
        ("builtins/std_fns.rs", "env::var(", "host_var, reached only through env:: sinks (routed)"),
        ("builtins/std_fns.rs", "env::current_dir", "OBSERVED env::current_dir"),
        ("builtins/std_fns.rs", "io::stdin()", "io::read_line / io::read_to_string are routed sinks; Stdin.read_line is stdin_line()"),
        ("builtins/std_fns.rs", "SystemTime::now()", "dead helper `now`, unused"),
        ("builtins/std_extra.rs", "OsRng", "OBSERVED crypto::hash_password_argon2id (salt)"),
        ("builtins/std_extra.rs", "bcrypt::hash(", "OBSERVED crypto::hash_password, hash_password_with_cost (salt)"),
        ("caps.rs", "SystemTime::now()", "scratch directory name; env::temp_dir() is OBSERVED"),
        ("caps.rs", "env::temp_dir", "scratch directory location; env::temp_dir() is OBSERVED"),
        ("caps.rs", "process::id()", "scratch directory name"),
        ("repl.rs", "env::current_dir", "the REPL is interactive and has no port"),
        ("repl.rs", "io::stdin()", "the REPL is interactive and has no port"),
        ("builtins/std_fns.rs", "env::temp_dir", "OBSERVED env::temp_dir"),
        ("builtins/std_http.rs", "std::net::", "the http client behind the routed http:: sinks (the response is the event)"),
        ("builtins/std_http.rs", "TcpStream", "the http client behind the routed http:: sinks (the response is the event)"),
        ("builtins/std_http_server.rs", "std::net::", "http::serve* are HANDLE_SINKS: refused under the port"),
        ("builtins/std_http_server.rs", "TcpListener", "http::serve* are HANDLE_SINKS: refused under the port"),
        ("builtins/std_http_server.rs", "TcpStream", "http::serve* are HANDLE_SINKS: refused under the port"),
        ("caps.rs", "env::current_dir", "resolving the project directory for the grant"),
        ("runner.rs", "env::current_dir", "locating the entry file"),
        ("runner.rs", "env::temp_dir", "the simulated world's temp directory when no scratch directory could be made; the program sees the world's"),
        ("runner.rs", "OsRng", "choosing the seed that --record puts in the header"),
    ];

    fn scan(dir: &Path, root: &Path, out: &mut BTreeSet<(String, String)>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "effect_port") {
                    continue;
                }
                scan(&p, root, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                // only code before the file's unit tests
                let code = text.split("#[cfg(test)]").next().unwrap_or(&text);
                for s in SOURCES {
                    if code.contains(s) {
                        out.insert((p.strip_prefix(root).unwrap().display().to_string(), s.to_string()));
                    }
                }
            }
        }
    }

    #[test]
    fn every_host_read_in_the_interpreter_is_routed_or_justified() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/interpreter");
        let mut found = BTreeSet::new();
        scan(&root, &root, &mut found);
        let covered: BTreeSet<(String, String)> = COVERED.iter().map(|(f, p, _)| (f.to_string(), p.to_string())).collect();
        let new: Vec<_> = found.difference(&covered).collect();
        let stale: Vec<_> = covered.difference(&found).collect();
        assert!(new.is_empty(), "host reads the effect port does not see (route them through effect_port or justify them in COVERED): {new:?}");
        assert!(stale.is_empty(), "COVERED lists reads that no longer exist: {stale:?}");
    }

    #[test]
    fn handle_sinks_are_sink_rows() {
        for h in super::HANDLE_SINKS {
            assert!(crate::ir::effect_facts::sinks::lookup(h).is_some(), "{h} is not a sink");
        }
    }
}
