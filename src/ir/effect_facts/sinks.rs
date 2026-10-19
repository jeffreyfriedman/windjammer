//! The sink table: which standard-library function performs which effect on which argument.
//!
//! One table, read by the effect pass (`analysis.rs`) and by the interpreter's capability gate. No
//! other code classifies effects (ADR-0021 section 1). Paths are written as the interpreter registers them
//! (`module::function`); lookup drops leading segments, so `std::fs::read` finds `fs::read`.

use super::kind::EffectKind;
use super::kind::EffectKind::*;

/// Where the resource (path, host, variable name, program) of an effect comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Res {
    /// The effect has no refinement.
    None,
    /// Argument `n` (path, URL or address, variable name, program).
    Arg(usize),
    /// Argument `n` is a command line; the resource is its first word.
    CmdArg(usize),
    /// A fixed resource (`HOME`, `RUST_LOG`).
    Fixed(&'static str),
    /// Minting a capability value (`cap::net()`): the effect must be granted for *some* resource; the value then
    /// carries that scope. The runtime check is the mint itself (`Interpreter::cap_mint`).
    Scope,
}

#[derive(Debug, Clone)]
pub struct SinkUse {
    pub effect: EffectKind,
    pub res: Res,
}

const fn u(effect: EffectKind, res: Res) -> SinkUse {
    SinkUse { effect, res }
}

const P0: Res = Res::Arg(0);

/// Table rows: (path, uses). Several uses = the call performs several effects (`fs::copy` reads and writes).
static SINKS: &[(&str, &[SinkUse])] = &[
    // filesystem
    ("fs::read", &[u(FsRead, P0)]),
    ("fs::read_to_string", &[u(FsRead, P0)]),
    ("fs::read_dir", &[u(FsRead, P0)]),
    ("fs::metadata", &[u(FsRead, P0)]),
    ("fs::exists", &[u(FsRead, P0)]),
    ("fs::is_file", &[u(FsRead, P0)]),
    ("fs::is_dir", &[u(FsRead, P0)]),
    ("fs::canonicalize", &[u(FsRead, P0)]),
    ("fs::write", &[u(FsWrite, P0)]),
    ("fs::write_bytes", &[u(FsWrite, P0)]),
    ("fs::append", &[u(FsWrite, P0)]),
    ("fs::create_dir", &[u(FsWrite, P0)]),
    ("fs::create_dir_all", &[u(FsWrite, P0)]),
    ("fs::remove_file", &[u(FsWrite, P0)]),
    ("fs::remove_dir", &[u(FsWrite, P0)]),
    ("fs::remove_dir_all", &[u(FsWrite, P0)]),
    ("fs::copy", &[u(FsRead, P0), u(FsWrite, Res::Arg(1))]),
    ("fs::rename", &[u(FsWrite, P0), u(FsWrite, Res::Arg(1))]),
    // changing the working directory changes what every relative path means
    ("fs::set_current_dir", &[u(EnvWrite, Res::Fixed("cwd"))]),
    ("db::connect", &[u(FsWrite, P0)]),
    // environment
    ("env::var", &[u(EnvRead, P0)]),
    ("env::get", &[u(EnvRead, P0)]),
    ("env::get_or", &[u(EnvRead, P0)]),
    ("env::set", &[u(EnvWrite, P0)]),
    ("env::remove", &[u(EnvWrite, P0)]),
    ("env::home_dir", &[u(EnvRead, Res::Fixed("HOME"))]),
    ("log::init", &[u(EnvRead, Res::Fixed("RUST_LOG"))]),
    ("log::init_with_level", &[u(EnvRead, Res::Fixed("RUST_LOG"))]),
    // processes
    ("process::run", &[u(Process, Res::CmdArg(0))]),
    ("process::run_with_args", &[u(Process, P0)]),
    ("subprocess::spawn", &[u(Process, P0)]),
    // network
    ("http::get", &[u(NetConnect, P0)]),
    ("http::post", &[u(NetConnect, P0)]),
    ("http::put", &[u(NetConnect, P0)]),
    ("http::patch", &[u(NetConnect, P0)]),
    ("http::delete", &[u(NetConnect, P0)]),
    ("http::head", &[u(NetConnect, P0)]),
    ("http::serve", &[u(NetListen, P0)]),
    ("http::serve_fn", &[u(NetListen, P0)]),
    ("http::server_serve", &[u(NetListen, P0)]),
    // clocks and timers
    ("time::now", &[u(Time, Res::None)]),
    ("time::utc_now", &[u(Time, Res::None)]),
    ("time::now_millis", &[u(Time, Res::None)]),
    ("time::now_unix", &[u(Time, Res::None)]),
    ("time::now_unix_nano", &[u(Time, Res::None)]),
    ("time::sleep_millis", &[u(Time, Res::None)]),
    ("Instant::now", &[u(Time, Res::None)]),
    ("SystemTime::now", &[u(Time, Res::None)]),
    ("thread::sleep", &[u(Time, Res::None)]),
    // randomness
    ("random::alphanumeric", &[u(Random, Res::None)]),
    ("random::ascii", &[u(Random, Res::None)]),
    ("random::bool", &[u(Random, Res::None)]),
    ("random::bool_with_probability", &[u(Random, Res::None)]),
    ("random::bytes", &[u(Random, Res::None)]),
    ("random::choice", &[u(Random, Res::None)]),
    ("random::float", &[u(Random, Res::None)]),
    ("random::float_range", &[u(Random, Res::None)]),
    ("random::range", &[u(Random, Res::None)]),
    ("random::range_inclusive", &[u(Random, Res::None)]),
    ("random::sample", &[u(Random, Res::None)]),
    ("random::shuffle", &[u(Random, Res::None)]),
    ("uuid::v4", &[u(Random, Res::None)]),
    ("uuid::v7", &[u(Random, Res::None), u(Time, Res::None)]),
    // capability values (std::cap): minting needs the effect, the value is then checked on every use
    ("cap::net", &[u(NetConnect, Res::Scope)]),
    ("cap::fs", &[u(FsRead, Res::Scope), u(FsWrite, Res::Scope)]),
    ("cap::env", &[u(EnvRead, Res::Scope)]),
    ("cap::process", &[u(Process, Res::Scope)]),
    ("cap::clock", &[u(Time, Res::None)]),
    ("cap::rng", &[u(Random, Res::None)]),
    // console input and output
    ("io::stdin", &[u(Console, Res::None)]),
    ("io::read_line", &[u(Console, Res::None)]),
    ("io::read_to_string", &[u(Console, Res::None)]),
    ("print", &[u(Console, Res::None)]),
    ("println", &[u(Console, Res::None)]),
    ("eprint", &[u(Console, Res::None)]),
    ("eprintln", &[u(Console, Res::None)]),
    ("log::debug", &[u(Console, Res::None)]),
    ("log::debug_with", &[u(Console, Res::None)]),
    ("log::error", &[u(Console, Res::None)]),
    ("log::error_with", &[u(Console, Res::None)]),
    ("log::info", &[u(Console, Res::None)]),
    ("log::info_with", &[u(Console, Res::None)]),
    ("log::trace", &[u(Console, Res::None)]),
    ("log::trace_with", &[u(Console, Res::None)]),
    ("log::warn", &[u(Console, Res::None)]),
    ("log::warn_with", &[u(Console, Res::None)]),
];

/// Library functions in effect-relevant modules that are deliberately NOT sinks (pure string/value
/// work, handle operations on an already-authorized handle, deterministic generators).
/// `interpreter_sinks_are_classified` (tests) requires every interpreter builtin of an authority module
/// to be either in `SINKS` or here, so a new builtin cannot silently bypass the gate.
pub static NOT_SINKS: &[&str] = &[
    "fs::join", "fs::extension", "fs::file_name", "fs::file_stem", "fs::parent", "fs::is_absolute", "fs::is_relative",
    "fs::current_dir", "env::args", "env::current_dir", "env::temp_dir", "path::join", "path::file_name", "path::glob_match",
    "process::exit", "process::pid", "random::seeded", "uuid::v7_from_timestamp", "thread::spawn", "thread::scope",
    "thread::current", "thread::panicking", "thread::yield_now", "thread::Builder::new", "time::format", "time::parse",
    "time::parse_rfc3339", "time::to_rfc3339", "time::parse_duration_ms", "time::format_duration_ms", "time::duration_days",
    "time::duration_hours", "time::duration_minutes", "time::duration_seconds", "subprocess::close",
    "subprocess::frames_in_agent_command", "subprocess::invalid_handle", "subprocess::is_valid", "subprocess::kill",
    "subprocess::read_agent_obs_n", "subprocess::read_line", "subprocess::read_line_until_prefix", "subprocess::wait",
    "subprocess::write_line", "cli::app", "cli::args", "log::debug_enabled", "log::error_enabled", "log::info_enabled", "log::trace_enabled", "log::warn_enabled", 
];

/// Modules whose builtins carry authority and must therefore be classified one way or the other.
pub static AUTHORITY_MODULES: &[&str] =
    &["fs", "env", "process", "subprocess", "http", "net", "db", "time", "random", "uuid", "io", "log", "path", "Instant", "SystemTime"];

/// Cheap pre-filter for hot paths: can `path` possibly be a sink? (a segment names a sink module, or the path is a
/// console intrinsic). No hashing, no allocation; `lookup` decides.
pub fn may_be_sink(path: &str) -> bool {
    path.split("::").any(|seg| {
        matches!(
            seg,
            "fs" | "env" | "process" | "subprocess" | "http" | "db" | "time" | "Instant" | "SystemTime" | "thread" | "random" | "uuid" | "io" | "log"
                | "print" | "println" | "eprint" | "eprintln" | "cap"
        )
    })
}

/// Find the sink row for a call path, dropping leading segments (`std::fs::read` finds `fs::read`).
pub fn lookup(path: &str) -> Option<&'static [SinkUse]> {
    let mut p = path;
    loop {
        if let Some((_, uses)) = SINKS.iter().find(|(n, _)| *n == p) {
            return Some(uses);
        }
        match p.split_once("::") {
            Some((_, rest)) => p = rest,
            None => return None,
        }
    }
}

pub fn is_not_sink(path: &str) -> bool {
    let mut p = path;
    loop {
        if NOT_SINKS.contains(&p) {
            return true;
        }
        match p.split_once("::") {
            Some((_, rest)) => p = rest,
            None => return false,
        }
    }
}

pub fn all_paths() -> impl Iterator<Item = &'static str> {
    SINKS.iter().map(|(n, _)| *n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_drops_leading_segments() {
        assert!(lookup("std::fs::read_to_string").is_some());
        assert!(lookup("fs::read_to_string").is_some());
        assert!(lookup("read_to_string").is_none());
        assert!(lookup("mymod::fs::read_to_string").is_some(), "leading segments are ignored");
    }

    #[test]
    fn prefilter_never_hides_a_sink() {
        for p in all_paths() {
            assert!(may_be_sink(p), "{p} would be skipped by the gate");
            assert!(may_be_sink(&format!("std::{p}")));
        }
        assert!(!may_be_sink("HashMap::new"));
    }

    #[test]
    fn copy_reads_and_writes() {
        let uses = lookup("fs::copy").unwrap();
        let kinds: Vec<_> = uses.iter().map(|x| x.effect.name()).collect();
        assert_eq!(kinds, ["fs.read", "fs.write"]);
    }

    #[test]
    fn no_path_is_both_sink_and_not_sink() {
        for p in all_paths() {
            assert!(!is_not_sink(p), "{p} is listed as both");
        }
    }

    #[test]
    fn every_authority_effect_has_a_sink() {
        for k in EffectKind::BUILTIN.iter().filter(|k| !k.is_determinism() && **k != Ffi) {
            assert!(
                SINKS.iter().any(|(_, us)| us.iter().any(|x| &x.effect == k)),
                "no sink performs {k}"
            );
        }
    }
}
