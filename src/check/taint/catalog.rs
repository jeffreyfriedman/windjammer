//! What counts as a source, a sink and a sanitizer (ADR-0009). Sinks come from the effect table
//! (`ir::effect_facts::sinks`), the one place that says which std function performs which effect on which argument;
//! sources and std sanitizers are the short tables below, each row checked against that table by
//! `tests/taint_flow_test.rs` so a renamed std function cannot silently stop being a source.

use crate::ir::effect_facts::kind::EffectKind;
use crate::ir::effect_facts::sinks::{self, Res};

/// A source: a std function whose result is data the program did not produce.
pub struct Source {
    pub path: &'static str,
    /// Reads as "data from <what>".
    pub what: &'static str,
}

pub static SOURCES: &[Source] = &[
    // The environment and the command line are NOT sources: they belong to the operator who starts the program and who
    // already holds all of its authority (a command or a path they name is not an injection). ADR-0009, amended.
    Source { path: "fs::read_to_string", what: "file contents" },
    Source { path: "fs::read", what: "file contents" },
    Source { path: "fs::read_bytes", what: "file contents" },
    Source { path: "io::read_line", what: "standard input" },
    Source { path: "io::read_to_string", what: "standard input" },
];

/// Std functions that make text safe for one kind of sink. `protects` names the sink role it makes safe; a value that has
/// passed through one is clean for every sink (the analysis does not track which sink it was cleaned for).
pub struct StdSanitizer {
    pub path: &'static str,
    pub protects: &'static str,
}

pub static STD_SANITIZERS: &[StdSanitizer] = &[
    StdSanitizer { path: "regex::escape", protects: "a regular expression" },
    // Only the final path component survives (`None` for `..`), so the result cannot climb out of a directory.
    StdSanitizer { path: "path::file_name", protects: "a file path" },
    StdSanitizer { path: "fs::file_name", protects: "a file path" },
];

/// Methods whose result is a number, a boolean or a count of its receiver: it carries no text of the receiver, so it is clean.
/// Each row is a method `std/rust_core.wji` declares with a non-text return type (`tests/taint_flow_test.rs` checks the declaration).
pub static NARROWING_METHODS: &[&str] = &[
    "len", "count", "is_empty", "contains", "contains_key", "starts_with", "ends_with", "is_some", "is_none", "is_ok", "is_err",
    "eq", "ne", "parse",
];

/// Methods that store their arguments in the receiver (`list.push(x)`): the receiver then holds the argument's taint.
pub static STORING_METHODS: &[&str] = &["push", "push_str", "insert", "extend", "append", "add", "set", "push_back", "push_front"];

pub fn source(path: &str) -> Option<&'static Source> {
    let mut p = path;
    loop {
        if let Some(s) = SOURCES.iter().find(|s| s.path == p) {
            return Some(s);
        }
        p = p.split_once("::")?.1;
    }
}

pub fn std_sanitizer(path: &str) -> Option<&'static StdSanitizer> {
    let mut p = path;
    loop {
        if let Some(s) = STD_SANITIZERS.iter().find(|s| s.path == p) {
            return Some(s);
        }
        p = p.split_once("::")?.1;
    }
}

/// The tainted argument of a sink call: which argument, what the sink does, and what to do instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SinkRole {
    pub arg: usize,
    /// "a shell command", "a file path", "SQL text".
    pub desc: &'static str,
    /// Label on the sink's argument.
    pub role: &'static str,
    /// What a sanitizer must achieve for this sink (the help line).
    pub advice: &'static str,
}

const SHELL: (&str, &str, &str) = (
    "a command",
    "used as the command",
    "pass the program and its arguments separately with `process::run_with_args(program, args)` (no shell is involved), or validate the value in a function documented `/// wj-taint: sanitizer`",
);
const WRITE_PATH: (&str, &str, &str) = (
    "a file path to write",
    "used as the path",
    "keep only the file name with `path::file_name(..)` and join it to a fixed directory, or validate the value in a function documented `/// wj-taint: sanitizer`",
);
const SQL: (&str, &str, &str) = (
    "SQL text",
    "used as the SQL text",
    "write the statement with `$1`/`?` placeholders and pass the value in `params`, or validate it in a function documented `/// wj-taint: sanitizer`",
);

/// The arguments of a call to the std function at `path` that must not be tainted. Read from the effect table: a call that
/// starts a process, and a call that writes a file by name, hold their resource in the argument the table names.
pub fn sink_roles(path: &str) -> Vec<SinkRole> {
    let Some(uses) = sinks::lookup(path) else { return Vec::new() };
    let mut out = Vec::new();
    for u in uses {
        let (arg, (desc, role, advice)) = match (&u.effect, u.res) {
            (EffectKind::Process, Res::Arg(n)) | (EffectKind::Process, Res::CmdArg(n)) => (n, SHELL),
            (EffectKind::FsWrite, Res::Arg(n)) if !path.ends_with("db::connect") => (n, WRITE_PATH),
            _ => continue,
        };
        out.push(SinkRole { arg, desc, role, advice });
    }
    out
}

/// `conn.query(sql, params)` and `conn.execute(sql, params)` of `std::db::Connection`: the SQL text is argument 0.
pub fn db_method_role(method: &str) -> Option<SinkRole> {
    matches!(method, "query" | "execute").then(|| SinkRole { arg: 0, desc: SQL.0, role: SQL.1, advice: SQL.2 })
}
