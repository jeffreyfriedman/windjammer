//! Agent mode (ADR-0037): `wj run|check|build|test` re-run themselves in a child process whose stdout and stderr are ONE
//! pipe, so the agent reads both in the order they were written, bounded, and ends with `exit status: N`. An agent then has no
//! reason to append `2>&1 | head -40` or `$?` / `${PIPESTATUS}` (a harness allow-list that matches `wj run:*` refuses
//! them). Off unless agent mode is selected (`WJ_AGENT=1`, `WJ_ERROR_FORMAT=agent` or `--error-format agent`), so a human
//! at a terminal is never affected.
//!
//! The first half of the line budget is written as it arrives, the last half at the end, and one marker line in
//! between says how many lines and bytes were left out. A single line longer than `MAX_LINE` bytes is cut with a
//! note of its length. With output inside the budget, the bytes pass through unchanged.

use std::collections::VecDeque;
use std::io::Write;

/// Longest line kept whole, in bytes.
pub const MAX_LINE: usize = 400;

/// A streaming filter: `write` program output into it, then `finish`.
pub struct Budget<W: Write> {
    sink: W,
    head: usize,
    tail: usize,
    budget: usize,
    written: usize,
    partial: Vec<u8>,
    ring: VecDeque<Vec<u8>>,
    omitted_lines: usize,
    omitted_bytes: usize,
}

impl<W: Write> Budget<W> {
    /// At most `lines` lines (at least 2) of output reach `sink`, plus the marker line.
    pub fn new(sink: W, lines: usize) -> Self {
        let lines = lines.max(2);
        let head = lines.div_ceil(2);
        Budget { sink, head, tail: lines - head, budget: lines, written: 0, partial: Vec::new(), ring: VecDeque::new(), omitted_lines: 0, omitted_bytes: 0 }
    }

    fn line(&mut self, mut l: Vec<u8>) {
        if l.len() > MAX_LINE + 1 {
            let extra = l.len() - MAX_LINE - usize::from(l.ends_with(b"\n"));
            l.truncate(MAX_LINE);
            while !l.is_empty() && std::str::from_utf8(&l).is_err() {
                l.pop();
            }
            l.extend_from_slice(format!(" [wj: line cut, {extra} more byte(s)]\n").as_bytes());
        }
        if self.written < self.head {
            self.written += 1;
            let _ = self.sink.write_all(&l);
            let _ = self.sink.flush();
            return;
        }
        self.ring.push_back(l);
        if self.ring.len() > self.tail {
            let dropped = self.ring.pop_front().unwrap_or_default();
            self.omitted_lines += 1;
            self.omitted_bytes += dropped.len();
        }
    }

    /// Lines and bytes left out so far.
    pub fn omitted(&self) -> (usize, usize) {
        (self.omitted_lines, self.omitted_bytes)
    }

    /// Write the kept tail (after the marker, when anything was left out) and return the sink.
    pub fn finish(mut self) -> W {
        if !self.partial.is_empty() {
            let rest = std::mem::take(&mut self.partial);
            self.line(rest);
        }
        if self.omitted_lines > 0 {
            let _ = writeln!(
                self.sink,
                "[wj: {} line(s), {} byte(s) of output omitted (WJ_MAX_OUTPUT={})]",
                self.omitted_lines, self.omitted_bytes, self.budget
            );
        }
        while let Some(l) = self.ring.pop_front() {
            let _ = self.sink.write_all(&l);
        }
        let _ = self.sink.flush();
        self.sink
    }
}

impl<W: Write> Write for Budget<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &b in buf {
            self.partial.push(b);
            if b == b'\n' {
                let l = std::mem::take(&mut self.partial);
                self.line(l);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.sink.flush()
    }
}

/// `--stdin-file FILE` (before the program's own arguments): the file and the command line without the option.
pub fn stdin_file(args: &[String]) -> Option<(String, Vec<String>)> {
    let i = args.iter().position(|a| a == "--stdin-file")?;
    let end = args.iter().position(|a| a == "--").unwrap_or(args.len());
    if i >= end {
        return None;
    }
    let file = args.get(i + 1)?.clone();
    let mut rest = args.to_vec();
    rest.drain(i..=i + 1);
    Some((file, rest))
}

/// Run this command line again with the file as stdin (exit status passed through).
pub fn run_with_stdin_file(file: &str, rest: &[String]) -> i32 {
    let Ok(exe) = std::env::current_exe() else { return 1 };
    let input = match std::fs::File::open(file) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: --stdin-file {file}: {e}");
            return 1;
        }
    };
    match std::process::Command::new(exe).args(rest).stdin(input).status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("error: cannot run wj: {e}");
            1
        }
    }
}

/// After a successful `wj build FILE` in agent mode: how to run what was built (agents guess `./build/main`, which is not there).
fn run_hint(args: &[String], code: i32) -> Option<String> {
    let verb = args.iter().find(|a| !a.starts_with('-'))?;
    let file = args.iter().find(|a| a.ends_with(".wj"))?;
    (verb == "build" && code == 0).then(|| format!("run: wj run --compiled {file}"))
}

/// Default line budget of agent mode (half from the start, half from the end); `WJ_MAX_OUTPUT=N` changes it, `0` lifts it.
pub const DEFAULT_LINES: usize = 160;

/// The wrapped verbs.
const VERBS: [&str; 4] = ["run", "check", "build", "test"];
/// Options that make the output a protocol or a long-lived session: not wrapped.
const RAW: [&str; 8] = ["--json", "--watch", "--hot", "--record", "--replay", "--help", "-h", "--fix"];

/// Agent mode is on: `WJ_AGENT` set, or the agent diagnostics format selected by flag or `WJ_ERROR_FORMAT`.
pub fn agent_mode(args: &[String], env: &dyn Fn(&str) -> Option<String>) -> bool {
    let on = |v: Option<String>| v.is_some_and(|v| !v.is_empty() && v != "0");
    if on(env("WJ_AGENT")) {
        return true;
    }
    let flag = args.windows(2).find(|w| w[0] == "--error-format").map(|w| w[1].as_str()).or_else(|| args.iter().find_map(|a| a.strip_prefix("--error-format=")));
    flag.map(str::to_string).or_else(|| env("WJ_ERROR_FORMAT")).is_some_and(|f| f == "agent")
}

/// Whether this command line is wrapped, and the line budget (`None`: unbounded).
pub fn plan(args: &[String], env: &dyn Fn(&str) -> Option<String>) -> Option<Option<usize>> {
    if env("WJ_AGENT_CHILD").is_some() || !agent_mode(args, env) {
        return None;
    }
    // the verb is the first word that is not an option
    let mut words = args.iter().enumerate().filter(|(i, a)| !a.starts_with('-') && !(*i > 0 && args[*i - 1] == "--error-format"));
    let verb = words.next()?.1;
    if !VERBS.contains(&verb.as_str()) || args.iter().any(|a| RAW.contains(&a.as_str())) {
        return None;
    }
    let lines = match env("WJ_MAX_OUTPUT").and_then(|v| v.parse::<usize>().ok()) {
        Some(0) => None,
        Some(n) => Some(n.max(2)),
        None => Some(DEFAULT_LINES),
    };
    Some(lines)
}

/// Run this command line again with stdout and stderr merged into one pipe; stream it through the budget and end with
/// `exit status: N`. Stdin is the child's own. Returns the child's exit status.
pub fn run_wrapped(lines: Option<usize>) -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Ok(exe) = std::env::current_exe() else { return 1 };
    let Ok((mut reader, writer)) = std::io::pipe() else { return 1 };
    let Ok(writer2) = writer.try_clone() else { return 1 };
    let mut cmd = std::process::Command::new(exe);
    cmd.args(&args).env("WJ_AGENT_CHILD", "1").stdout(writer).stderr(writer2);
    let spawned = cmd.spawn();
    drop(cmd); // closes the parent's copies of the write end, so the read below ends with the child
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot run wj: {e}");
            return 1;
        }
    };
    let mut out = std::io::stdout();
    match lines {
        Some(n) => {
            let mut budget = Budget::new(&mut out, n);
            let _ = std::io::copy(&mut reader, &mut budget);
            budget.finish();
        }
        None => {
            let _ = std::io::copy(&mut reader, &mut out);
        }
    }
    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    if let Some(h) = run_hint(&args, code) {
        let _ = writeln!(out, "{h}");
    }
    let _ = writeln!(out, "exit status: {code}");
    let _ = out.flush();
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str, lines: usize) -> String {
        let mut b = Budget::new(Vec::new(), lines);
        for chunk in input.as_bytes().chunks(3) {
            b.write_all(chunk).unwrap();
        }
        String::from_utf8(b.finish()).unwrap()
    }
    fn v(s: &str) -> Vec<String> {
        s.split(' ').map(String::from).collect()
    }
    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, x)| x.to_string())
    }

    #[test]
    fn short_output_passes_through_unchanged() {
        assert_eq!(run("a\nb\nc\nd", 4), "a\nb\nc\nd");
    }

    #[test]
    fn long_output_keeps_head_and_tail_with_a_marker() {
        let text: String = (1..=100).map(|i| format!("line {i}\n")).collect();
        let out = run(&text, 6);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[..3], ["line 1", "line 2", "line 3"]);
        assert_eq!(lines[4..], ["line 98", "line 99", "line 100"]);
        assert!(lines[3].starts_with("[wj: 94 line(s),") && lines[3].ends_with("(WJ_MAX_OUTPUT=6)]"), "{}", lines[3]);
    }

    #[test]
    fn stdin_file_is_taken_out_of_the_command_line() {
        assert_eq!(stdin_file(&v("run --verify --stdin-file in.txt m.wj")), Some(("in.txt".into(), v("run --verify m.wj"))));
        assert_eq!(stdin_file(&v("run m.wj -- --stdin-file x")), None, "after `--` it belongs to the program");
        assert_eq!(stdin_file(&v("run m.wj")), None);
    }

    #[test]
    fn a_successful_agent_build_says_how_to_run_it() {
        assert_eq!(run_hint(&v("build m.wj"), 0).as_deref(), Some("run: wj run --compiled m.wj"));
        assert_eq!(run_hint(&v("build m.wj"), 1), None);
        assert_eq!(run_hint(&v("check m.wj"), 0), None);
    }

    #[test]
    fn plan_wraps_only_agent_mode_verbs() {
        let none = env(&[]);
        assert_eq!(plan(&v("run --verify m.wj"), &none), None, "humans are never wrapped");
        let agent = env(&[("WJ_ERROR_FORMAT", "agent")]);
        assert_eq!(plan(&v("run --verify m.wj"), &agent), Some(Some(DEFAULT_LINES)));
        assert_eq!(plan(&v("check m.wj"), &env(&[("WJ_AGENT", "1")])), Some(Some(DEFAULT_LINES)));
        assert_eq!(plan(&v("run --error-format agent m.wj"), &none), Some(Some(DEFAULT_LINES)));
        assert_eq!(plan(&v("--error-format agent run m.wj"), &none), Some(Some(DEFAULT_LINES)));
        for raw in ["check --json m.wj", "run --watch m.wj", "fmt m.wj", "docs split", "check --fix m.wj"] {
            assert_eq!(plan(&v(raw), &agent), None, "{raw}");
        }
        assert_eq!(plan(&v("run m.wj"), &env(&[("WJ_AGENT", "1"), ("WJ_AGENT_CHILD", "1")])), None, "the child is not wrapped again");
        assert_eq!(plan(&v("run m.wj"), &env(&[("WJ_AGENT", "1"), ("WJ_MAX_OUTPUT", "40")])), Some(Some(40)));
        assert_eq!(plan(&v("run m.wj"), &env(&[("WJ_AGENT", "1"), ("WJ_MAX_OUTPUT", "0")])), Some(None));
    }
}
