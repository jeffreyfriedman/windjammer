//! Compiled-build verdicts, keyed on the program's content and the compiler build (ADR-0033).
//!
//! Each compiled build that reaches rustc (`wj build`, `wj run --compiled`) records whether the program
//! built, under a key that is a SHA-256 of every input of the program (`check::program_inputs`: sources,
//! manifest, lock file, dependency sources, crate surfaces) and the compiler build. An interpreted run of the
//! same content then says, after the program's own output, that the compiled build of exactly this program
//! failed. Nothing is recorded for a failure that is not a compile error (a dependency that could not be
//! fetched), so a verdict is a property of the content and the compiler alone.
//!
//! Entries live in `<cache root>/verdicts/<key>.json` (one small file each, written atomically); the
//! directory keeps the newest `MAX_ENTRIES`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const FORMAT: u32 = 1;
pub const MAX_ENTRIES: usize = 512;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Outcome {
    Accepted,
    /// rustc rejected the generated code: the number of errors and the first one, as reported to the user.
    Rejected { errors: usize, first: Option<String> },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    format: u32,
    pub key: String,
    pub outcome: Outcome,
    pub release: bool,
}

impl Verdict {
    pub fn new(key: &str, outcome: Outcome, release: bool) -> Self {
        Verdict { format: FORMAT, key: key.to_string(), outcome, release }
    }
}

/// SHA-256 over the compiler identity and every (path, text) input, each length-prefixed.
pub fn key(inputs: &[(String, String)], compiler: &str) -> String {
    let mut h = Sha256::new();
    let mut part = |b: &[u8]| {
        h.update((b.len() as u64).to_le_bytes());
        h.update(b);
    };
    part(b"wj-verdict");
    part(compiler.as_bytes());
    for (f, t) in inputs {
        part(f.as_bytes());
        part(t.as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// The key of the program at `path` for this compiler build; `None` when its inputs cannot be read.
pub fn program_key(path: &Path) -> Option<String> {
    let inputs = crate::check::program_inputs(path).ok()?;
    Some(key(&inputs, &crate::check::compiler_identity()))
}

pub fn dir() -> PathBuf {
    crate::cargo_cache::cache_root().join("verdicts")
}

fn file_in(dir: &Path, key: &str) -> Option<PathBuf> {
    // keys are hex digests; anything else never names a file
    (key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit())).then(|| dir.join(format!("{key}.json")))
}

pub fn load_from(dir: &Path, key: &str) -> Option<Verdict> {
    let text = std::fs::read_to_string(file_in(dir, key)?).ok()?;
    let v: Verdict = serde_json::from_str(&text).ok()?;
    (v.format == FORMAT && v.key == key).then_some(v)
}

/// Write through a temporary file and a rename (a concurrent reader never sees half a file), then keep the
/// newest `max` entries.
pub fn store_in(dir: &Path, v: &Verdict, max: usize) {
    let Some(path) = file_in(dir, &v.key) else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(text) = serde_json::to_string(v) else { return };
    let tmp = dir.join(format!(".{}.tmp{}", v.key, std::process::id()));
    if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    evict(dir, max);
}

fn evict(dir: &Path, max: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    if entries.len() <= max {
        return;
    }
    entries.sort();
    for (_, p) in &entries[..entries.len() - max] {
        let _ = std::fs::remove_file(p);
    }
}

/// Record the compiled verdict of the program at `path` (best effort: a cache that cannot be written is no error).
pub fn record(path: &Path, outcome: Outcome, release: bool) {
    record_in(&dir(), path, outcome, release);
}

pub fn record_in(dir: &Path, path: &Path, outcome: Outcome, release: bool) {
    if let Some(k) = program_key(path) {
        store_in(dir, &Verdict::new(&k, outcome, release), MAX_ENTRIES);
    }
}

/// Record the verdict of a `cargo build --message-format=json*` run (see `cargo_json_outcome`).
pub fn record_cargo_json(path: &Path, success: bool, stdout: &str, release: bool) {
    if let Some(o) = cargo_json_outcome(success, stdout) {
        record(path, o, release);
    }
}

/// The verdict of a cargo build from its JSON messages: success, or a failure with rustc errors. A failure
/// without a compiler error (a fetch, a linker) is no verdict on the program.
pub fn cargo_json_outcome(success: bool, stdout: &str) -> Option<Outcome> {
    if success {
        return Some(Outcome::Accepted);
    }
    let errors: Vec<String> = stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["reason"] == "compiler-message" && v["message"]["level"] == "error")
        .map(|v| {
            let code = v["message"]["code"]["code"].as_str().map(|c| format!("{c} ")).unwrap_or_default();
            let msg: String = v["message"]["message"].as_str().unwrap_or("").chars().take(120).collect();
            format!("rustc {code}{msg}")
        })
        // rustc's closing "aborting due to N previous errors" is an error message too, and not one of them
        .filter(|e| !e.contains("aborting due to"))
        .collect();
    let first = errors.first()?.clone();
    Some(Outcome::Rejected { errors: errors.len(), first: Some(first) })
}

pub fn lookup(path: &Path) -> Option<Verdict> {
    lookup_in(&dir(), path)
}

pub fn lookup_in(dir: &Path, path: &Path) -> Option<Verdict> {
    load_from(dir, &program_key(path)?)
}

/// One line naming a diagnostic: code, message and the place.
pub fn summary(d: &crate::diagnostics::Diagnostic) -> String {
    let at = d.primary_span.as_ref().map(|s| format!(" at {}:{}", Path::new(&s.file).file_name().map_or(s.file.clone(), |n| n.to_string_lossy().to_string()), s.line_start)).unwrap_or_default();
    let msg: String = d.message.chars().take(120).collect();
    format!("{} {msg}{at}", d.code)
}

/// The line an interpreted run prints (on stderr) when the compiled build of the same content failed.
pub fn footer(v: &Verdict) -> Option<String> {
    match &v.outcome {
        Outcome::Accepted => None,
        Outcome::Rejected { errors, first } => {
            let n = if *errors == 1 { "1 error".to_string() } else { format!("{errors} errors") };
            let first = first.as_deref().map(|f| format!("; first: {f}")).unwrap_or_default();
            Some(format!("wj: note: the compiled build of this exact program failed ({n}{first}); `wj build` shows the details"))
        }
    }
}
