//! Code mode (ADR-0021 section 12, stage E5): run an agent's Windjammer program in a capability-scoped interpreter and report what
//! happened. This is the library behind `windjammer-mcp`'s `execute` tool; `wj run --sandbox --bind --limits` is the same machinery
//! with the process boundary moved to the caller.

use crate::interpreter::caps::gate_for_run;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::limits::LimitSpec;
use crate::interpreter::runner::{exit_code_for, with_big_stack};
use serde_json::{json, Value as Json};

#[derive(Clone, Debug, Default)]
pub struct Request {
    /// The program (`fn main()` is the entry point).
    pub code: String,
    /// Grants in `--allow` syntax: `fs.read=./data/**`, `net.connect=api.example.com`, `effects=mcp.github.list_issues`.
    pub allow: Vec<String>,
    /// `NAME=ENDPOINT`, see `interpreter::mcp_host`.
    pub bindings: Vec<String>,
    pub limits: LimitSpec,
}

#[derive(Clone, Debug, Default)]
pub struct Response {
    pub exit: i32,
    pub stdout: String,
    pub stderr: String,
    /// Compiler diagnostics when the program was refused before running (typed errors for the self-repair loop).
    pub diagnostics: String,
    /// The effects the program actually used, by name (`mcp.github.list_issues`, `fs.read`, ...).
    pub effects_used: Vec<String>,
    pub steps: u64,
    pub effect_calls: usize,
}

impl Response {
    pub fn to_json(&self) -> Json {
        json!({
            "exit": self.exit,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "diagnostics": self.diagnostics,
            "effects_used": self.effects_used,
            "steps": self.steps,
            "effect_calls": self.effect_calls,
        })
    }
}

/// Run `req` in a fresh temporary project. Never panics on a bad program: the failure is the response.
pub fn execute(req: &Request) -> Response {
    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return Response { exit: 1, stderr: format!("cannot create a scratch directory: {e}"), ..Response::default() },
    };
    let entry = dir.path().join("main.wj");
    if let Err(e) = std::fs::write(&entry, &req.code) {
        return Response { exit: 1, stderr: format!("cannot write the program: {e}"), ..Response::default() };
    }
    let req = req.clone();
    let path = entry.clone();
    // the front end and the interpreter both want a deep stack, and a server's worker threads have a small one
    let resp = with_big_stack(move || {
        // typed errors first: the same front end as `wj check`
        let checked = match crate::check::check_path(&path, &crate::check::CheckOptions { use_cache: false }) {
            Ok(c) => c,
            Err(e) => return Response { exit: 1, diagnostics: e.to_string(), ..Response::default() },
        };
        // lints (W####) are advice; every other error stops the program before anything runs
        let blocking: Vec<_> = checked.report.diagnostics.iter().filter(|d| d.severity == crate::diagnostics::Severity::Error && !(d.code.len() == 5 && d.code.starts_with('W'))).collect();
        if !blocking.is_empty() {
            let source_of = |f: &str| checked.sources.get(f).cloned();
            let text = blocking.iter().map(|d| crate::diagnostics::render::render(d, &source_of)).collect::<Vec<_>>().join("\n");
            return Response { exit: 1, diagnostics: text, ..Response::default() };
        }
        run_checked(&req, &path)
    });
    drop(dir);
    resp
}

fn run_checked(req: &Request, entry: &std::path::Path) -> Response {
    let mut it = Interpreter::new_capturing();
    it.guard_stack();
    it.set_engine_resolved(false); // limits tick in the tree walker
    match gate_for_run(entry, true, &req.allow, false, "") {
        Ok(gate) => it.set_caps(gate),
        Err(m) => return Response { exit: 1, stderr: m, ..Response::default() },
    }
    it.set_limits(req.limits.clone());
    for b in &req.bindings {
        let Some((name, spec)) = b.split_once('=') else {
            return Response { exit: 1, stderr: format!("a binding is NAME=ENDPOINT, found `{b}`"), ..Response::default() };
        };
        if let Err(m) = it.bind_mcp(name, spec) {
            return Response { exit: 1, stderr: m, ..Response::default() };
        }
    }
    if let Err(e) = it.load_program_at(entry) {
        return Response { exit: exit_code_for(&e), stderr: e.render(), ..Response::default() };
    }
    let (exit, stderr) = match it.call_main() {
        Ok(_) => (0, String::new()),
        Err(e) => (exit_code_for(&e), if matches!(e.kind, crate::interpreter::error::ErrorKind::Exit(_)) { String::new() } else { e.render() }),
    };
    let (steps, effect_calls, _) = it.usage();
    Response { exit, stdout: it.get_output(), stderr, diagnostics: String::new(), effects_used: it.effects_used(), steps, effect_calls }
}

/// What the operator of an MCP server allows `execute` to do. The arguments of a tool call are written by the model, so authority
/// never comes from them: a request may select from the operator's grants and servers and lower the limits, nothing more.
#[derive(Clone, Debug)]
pub struct Policy {
    /// Grants in `--allow` syntax; a request may use any subset, by exact text.
    pub allow: Vec<String>,
    /// `(name, endpoint)` of the MCP servers the operator configured.
    pub servers: Vec<(String, String)>,
    /// Ceiling for every limit.
    pub limits: LimitSpec,
}

impl Default for Policy {
    fn default() -> Policy {
        Policy {
            allow: Vec::new(),
            servers: Vec::new(),
            limits: LimitSpec { fuel: Some(100_000_000), wall: Some(std::time::Duration::from_secs(10)), output: Some(1 << 20), calls: Some(100), mem_mb: Some(512) },
        }
    }
}

impl Policy {
    /// `WJ_MCP_EXECUTE_ALLOW` (grants separated by commas), `WJ_MCP_EXECUTE_SERVERS` (`name=endpoint` separated by semicolons),
    /// `WJ_MCP_EXECUTE_LIMITS` (as `--limits`).
    pub fn from_env() -> Policy {
        let mut p = Policy::default();
        if let Ok(a) = std::env::var("WJ_MCP_EXECUTE_ALLOW") {
            p.allow = a.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
        }
        if let Ok(s) = std::env::var("WJ_MCP_EXECUTE_SERVERS") {
            p.servers = s.split(';').filter_map(|e| e.split_once('=')).map(|(n, e)| (n.trim().to_string(), e.trim().to_string())).collect();
        }
        if let Ok(l) = std::env::var("WJ_MCP_EXECUTE_LIMITS") {
            if let Ok(spec) = LimitSpec::parse(&l) {
                p.limits = spec;
            }
        }
        p
    }

    /// The request to run for what a caller asked, or why it is refused.
    pub fn plan(&self, code: &str, allow: &[String], servers: &[String], limits: Option<&str>) -> Result<Request, String> {
        for a in allow {
            if !self.allow.contains(a) {
                return Err(format!("`{a}` is not in this server's policy (allowed: {})", if self.allow.is_empty() { "nothing".to_string() } else { self.allow.join(", ") }));
            }
        }
        let mut bindings = Vec::new();
        for s in servers {
            let (n, e) = self.servers.iter().find(|(n, _)| n == s).ok_or_else(|| format!("no MCP server `{s}` is configured here (configured: {})", self.servers.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")))?;
            bindings.push(format!("{n}={e}"));
        }
        let asked = match limits {
            Some(l) => LimitSpec::parse(l)?,
            None => LimitSpec::default(),
        };
        let lower = |ask: Option<u64>, cap: Option<u64>| match (ask, cap) {
            (Some(a), Some(c)) => Some(a.min(c)),
            (None, c) => c,
            (a, None) => a,
        };
        let limits = LimitSpec {
            fuel: lower(asked.fuel, self.limits.fuel),
            wall: match (asked.wall, self.limits.wall) {
                (Some(a), Some(c)) => Some(a.min(c)),
                (None, c) => c,
                (a, None) => a,
            },
            output: lower(asked.output.map(|x| x as u64), self.limits.output.map(|x| x as u64)).map(|x| x as usize),
            calls: lower(asked.calls.map(|x| x as u64), self.limits.calls.map(|x| x as u64)).map(|x| x as usize),
            mem_mb: lower(asked.mem_mb, self.limits.mem_mb),
        };
        Ok(Request { code: code.to_string(), allow: allow.to_vec(), bindings, limits })
    }
}
