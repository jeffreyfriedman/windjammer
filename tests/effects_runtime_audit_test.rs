// consumes: crates/windjammer-runtime/src/{fs,http,env,process,subprocess,db}.rs
//! Audit of the Rust runtime (ADR-0021): every public function that touches the operating system must reach a
//! `caps::` check, so a new std wrapper cannot silently bypass the capability model.
//!
//! Independent oracle: the OS-touching tokens below come from the Rust standard library and the crates the runtime
//! uses, not from the sink table. A function passes when its body contains `caps::`, or when it calls another public
//! function of the same module that does (e.g. `write_string` -> `write`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const OS_TOKENS: &[&str] = &[
    "std::fs::", "std::env::var", "std::env::set_var", "std::env::remove_var", "std::env::vars", "Command::new", "reqwest::", "TcpListener::bind",
    "SqlitePoolOptions::new", "tokio::net::",
];

/// Public functions that read or act on the OS but are deliberately not capabilities (information about the
/// process itself, or operations on a handle/connection that was already authorized when it was created).
const ALLOWED: &[&str] = &["fs::DirEntry::path", "env::current_dir", "env::current_exe", "env::args", "process::exit"];

struct Chunk {
    name: String,
    public: bool,
    text: String,
}

fn chunks(src: &str) -> Vec<Chunk> {
    let body = src.split("#[cfg(test)]").next().unwrap_or(src);
    let lines: Vec<&str> = body.lines().collect();
    let mut starts = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        let rest = t.strip_prefix("pub async fn ").or_else(|| t.strip_prefix("pub fn ")).or_else(|| t.strip_prefix("async fn ")).or_else(|| t.strip_prefix("fn "));
        if let Some(rest) = rest {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            starts.push((i, name, t.starts_with("pub ")));
        }
    }
    starts
        .iter()
        .enumerate()
        .map(|(k, (i, name, public))| {
            let end = starts.get(k + 1).map(|s| s.0).unwrap_or(lines.len());
            Chunk { name: name.clone(), public: *public, text: lines[*i..end].join("\n") }
        })
        .collect()
}

#[test]
fn every_os_touching_runtime_function_reaches_a_capability_check() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime/src");
    let mut problems = Vec::new();
    for module in ["fs", "http", "env", "process", "subprocess", "db"] {
        let src = std::fs::read_to_string(base.join(format!("{module}.rs"))).unwrap();
        let cs = chunks(&src);
        let os_names: BTreeSet<&str> = cs.iter().filter(|c| OS_TOKENS.iter().any(|t| c.text.contains(t))).map(|c| c.name.as_str()).collect();
        let calls = |c: &Chunk, name: &str| c.name != name && c.text.contains(&format!("{name}("));
        let mut guarded: BTreeSet<&str> = cs.iter().filter(|c| c.text.contains("caps::")).map(|c| c.name.as_str()).collect();
        loop {
            let before = guarded.len();
            for c in cs.iter().filter(|c| c.public) {
                if guarded.iter().any(|g| calls(c, g)) {
                    guarded.insert(c.name.as_str());
                }
            }
            if guarded.len() == before {
                break;
            }
        }
        let mut by_name: BTreeMap<&str, bool> = BTreeMap::new();
        for c in cs.iter().filter(|c| c.public) {
            let touches = OS_TOKENS.iter().any(|t| c.text.contains(t)) || os_names.iter().any(|o| calls(c, o));
            let ok = guarded.contains(c.name.as_str()) || ALLOWED.contains(&format!("{module}::{}", c.name).as_str());
            if touches && !ok {
                by_name.insert(c.name.as_str(), false);
            }
        }
        for (name, _) in by_name {
            problems.push(format!("{module}::{name}"));
        }
    }
    assert!(problems.is_empty(), "runtime functions that touch the OS without a `caps::` check (add one, or allow-list with a reason): {problems:?}");
}

#[test]
fn the_scanner_sees_the_guards_it_is_meant_to_see() {
    // a mutation check on the oracle itself: removing the guard from `fs::read_to_string` must make the scan fail
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime/src");
    let src = std::fs::read_to_string(base.join("fs.rs")).unwrap();
    let cs = chunks(&src);
    let read = cs.iter().find(|c| c.name == "read_to_string").expect("fs::read_to_string");
    assert!(read.text.contains("caps::fs_read"), "{}", read.text);
    let stripped = read.text.replace("caps::fs_read", "removed");
    assert!(OS_TOKENS.iter().any(|t| stripped.contains(t)) && !stripped.contains("caps::"));
}
