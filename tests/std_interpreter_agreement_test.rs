//! Gate: every top-level `fn` declared in `std/<mod>.wj` has an interpreter builtin registered as
//! `"<mod>::<fn>"` (same name), or is listed in `tests/data/std_interpreter_missing.txt`.
//! The list is a ratchet: it may only shrink. A listed name that gains a builtin must be removed;
//! a new std declaration without a builtin fails. (Arity is not checked: builtins take `&[Value]`.)

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rs_files(dir: &Path, out: &mut String) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push_str(&fs::read_to_string(&p).unwrap());
        }
    }
}

fn declared() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for e in fs::read_dir(root().join("std")).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "wj") {
            continue;
        }
        let m = p.file_stem().unwrap().to_string_lossy().to_string();
        if m == "basic_test" || m == "test_simple" {
            continue;
        }
        for line in fs::read_to_string(&p).unwrap().lines() {
            let l = line.strip_prefix("pub ").unwrap_or(line);
            if let Some(rest) = l.strip_prefix("fn ") {
                if let Some(name) = rest.split('(').next() {
                    if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        set.insert(format!("{m}::{name}"));
                    }
                }
            }
        }
    }
    set
}

#[test]
fn std_declarations_and_interpreter_builtins_agree() {
    let mut src = String::new();
    rs_files(&root().join("src/interpreter"), &mut src);
    let missing: BTreeSet<String> = declared().into_iter().filter(|d| !src.contains(&format!("\"{d}\""))).collect();
    let list_path = root().join("tests/data/std_interpreter_missing.txt");
    if std::env::var("WJ_UPDATE_STD_GATE").is_ok() {
        let head = "# std/<mod>.wj top-level fns with no interpreter builtin (`\"mod::fn\"` registration). Ratchet: only shrinks.\n# Regenerate: WJ_UPDATE_STD_GATE=1 cargo test --test all std_interpreter_agreement\n";
        fs::write(&list_path, format!("{head}{}\n", missing.iter().cloned().collect::<Vec<_>>().join("\n"))).unwrap();
        return;
    }
    let allowed: BTreeSet<String> = fs::read_to_string(&list_path)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(str::to_string)
        .collect();
    let new_missing: Vec<_> = missing.difference(&allowed).collect();
    let now_present: Vec<_> = allowed.difference(&missing).collect();
    assert!(new_missing.is_empty(), "std declares fns the interpreter lacks (add builtins, do not grow the list): {new_missing:?}");
    assert!(now_present.is_empty(), "now implemented or removed; delete from tests/data/std_interpreter_missing.txt: {now_present:?}");
}
