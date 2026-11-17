//! Every method the interpreter implements for chars, strings, lists, numbers,
//! Option, Result, maps and sets must be listed in `agent_index/builtins.txt`
//! (and so in `wj docs` / the agent index), or sit in the shrink-only ledger
//! `tests/ledgers/agent_index_unlisted_methods.txt`. The ledger may only lose
//! lines: a listed method left in the ledger fails, and so does a new
//! unlisted method.

use std::collections::BTreeSet;
use std::path::PathBuf;

const MODULES: &[(&str, &str)] = &[
    ("Char", "char"),
    ("Str", "string"),
    ("Vec", "Vec"),
    ("Int", "int"),
    ("Float", "float"),
    ("Option", "Option"),
    ("Result", "Result"),
    ("Map", "HashMap"),
    ("Set", "HashSet"),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `(module, method)` pairs registered by the interpreter.
fn registered() -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let dir = root().join("src/interpreter/builtins");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("char_pred!(\"") {
                if let Some(name) = rest.split('"').next() {
                    out.insert(("char".to_string(), name.to_string()));
                }
                continue;
            }
            if !line.starts_with("r.") {
                continue;
            }
            let Some(kind_at) = line.find("(Kind::") else { continue };
            let after = &line[kind_at + "(Kind::".len()..];
            let Some((kind, rest)) = after.split_once(", \"") else { continue };
            let Some(name) = rest.split('"').next() else { continue };
            if let Some((_, module)) = MODULES.iter().find(|(k, _)| *k == kind) {
                out.insert((module.to_string(), name.to_string()));
            }
        }
    }
    out
}

fn listed() -> BTreeSet<(String, String)> {
    let text = std::fs::read_to_string(root().join("agent_index/builtins.txt")).unwrap();
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let p: Vec<&str> = l.split('|').map(str::trim).collect();
            (p.len() >= 3).then(|| (p[0].to_string(), p[1].to_string()))
        })
        .collect()
}

fn ledger() -> BTreeSet<(String, String)> {
    let text = std::fs::read_to_string(root().join("tests/ledgers/agent_index_unlisted_methods.txt")).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once(' ').map(|(m, n)| (m.to_string(), n.trim().to_string())))
        .collect()
}

#[test]
fn interpreter_methods_are_documented_or_ledgered() {
    let listed = listed();
    let ledger = ledger();
    let unlisted: BTreeSet<_> = registered().difference(&listed).cloned().collect();
    let new: Vec<_> = unlisted.difference(&ledger).collect();
    assert!(
        new.is_empty(),
        "methods the interpreter implements but `wj docs` does not list; add them to agent_index/builtins.txt (then `wj agent-index`): {:?}",
        new
    );
    let stale: Vec<_> = ledger.difference(&unlisted).collect();
    assert!(
        stale.is_empty(),
        "now listed (or gone): delete these lines from tests/ledgers/agent_index_unlisted_methods.txt: {:?}",
        stale
    );
}

#[test]
fn char_predicates_are_all_listed() {
    let listed = listed();
    for name in ["is_uppercase", "is_lowercase", "is_alphanumeric", "is_whitespace", "is_ascii_digit"] {
        assert!(listed.contains(&("char".to_string(), name.to_string())), "char.{name} missing from builtins.txt");
    }
}
