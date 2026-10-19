//! Gates: the standard library the compiler offers and the interpreter's builtins agree, in the module-qualified
//! form programs use (`mime.APPLICATION_JSON`, `encoding.url_decode(..)`, `regex.find_all(..)`).
//!
//! 1. Every top-level `fn` / `const` declared in `std/<mod>.wj` or `std/<mod>/mod.wj` is registered in the
//!    interpreter registry as `"<mod>::<name>"` (a function or a constant), or is listed in
//!    `tests/data/std_interpreter_missing.txt`.
//! 2. The same for every public free function / constant of `crates/windjammer-runtime/src/<mod>.rs`, which is what
//!    compiled programs actually call (`std/regex.wj` does not even declare `find_all`), against
//!    `tests/data/runtime_interpreter_missing.txt`.
//! 3. Qualified use works end to end: a constant read as `mod.NAME` and `mod::NAME` through the real interpreter,
//!    for every registered std module constant.
//!
//! The lists are ratchets: they may only shrink. A listed name that gains a builtin must be removed; a new
//! declaration without a builtin fails. (Arity is not checked: builtins take `&[Value]`.)

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use windjammer::interpreter::builtins::Registry;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Top-level `fn name` / `const NAME` of a source file (column 0, before any `#[cfg(test)]`).
fn top_level_names(src: &str, require_pub: bool) -> Vec<String> {
    let src = src.split("#[cfg(test)]").next().unwrap_or(src);
    let mut out = Vec::new();
    for line in src.lines() {
        let l = match line.strip_prefix("pub ") {
            Some(rest) => rest,
            None if require_pub => continue,
            None => line,
        };
        let l = l.strip_prefix("async ").unwrap_or(l);
        for kw in ["fn ", "const "] {
            if let Some(rest) = l.strip_prefix(kw) {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if is_ident(&name) {
                    out.push(name);
                }
            }
        }
    }
    out
}

fn std_module(stem: &str) -> &str {
    stem.strip_suffix("_mod").or_else(|| stem.strip_suffix("_runtime")).unwrap_or(stem)
}

/// `module::name` for everything `std/*.wj` and `std/*/mod.wj` declare.
fn declared_std() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for e in fs::read_dir(root().join("std")).unwrap().flatten() {
        let p = e.path();
        let (module, file) = if p.is_dir() {
            (p.file_name().unwrap().to_string_lossy().to_string(), p.join("mod.wj"))
        } else if p.extension().is_some_and(|x| x == "wj") {
            (p.file_stem().unwrap().to_string_lossy().to_string(), p.clone())
        } else {
            continue;
        };
        if matches!(module.as_str(), "basic_test" | "test_simple") || !file.is_file() {
            continue;
        }
        for name in top_level_names(&fs::read_to_string(&file).unwrap(), false) {
            set.insert(format!("{module}::{name}"));
        }
    }
    set
}

/// Runtime modules that are test-framework, FFI-glue or developer-tooling internals rather than the
/// standard library programs import (the interpreter has its own `assert*` intrinsics and `@test` runner).
const NOT_PROGRAM_STD: &[&str] =
    &["bench", "contracts", "doc_test", "ffi", "fixtures", "mock", "mock_function", "mock_interface", "property", "setup_teardown", "test", "test_output", "testing", "timeout"];

/// `module::name` for the public free functions / constants of the runtime crate.
fn declared_runtime() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for e in fs::read_dir(root().join("crates/windjammer-runtime/src")).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        if stem == "lib" || NOT_PROGRAM_STD.contains(&std_module(&stem)) {
            continue;
        }
        for name in top_level_names(&fs::read_to_string(&p).unwrap(), true) {
            set.insert(format!("{}::{}", std_module(&stem), name));
        }
    }
    set
}

fn read_list(path: &Path) -> BTreeSet<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn missing_from(registry: &Registry, declared: &BTreeSet<String>) -> BTreeSet<String> {
    declared.iter().filter(|d| !registry.has_function(d) && registry.constant(d).is_none()).cloned().collect()
}

fn check_ratchet(declared: BTreeSet<String>, list: &str, header: &str, what: &str) {
    let registry = Registry::standard();
    let missing = missing_from(&registry, &declared);
    let list_path = root().join("tests/data").join(list);
    if std::env::var("WJ_UPDATE_STD_GATE").is_ok() {
        fs::write(&list_path, format!("{header}{}\n", missing.iter().cloned().collect::<Vec<_>>().join("\n"))).unwrap();
        return;
    }
    let allowed = read_list(&list_path);
    let new_missing: Vec<_> = missing.difference(&allowed).collect();
    let now_present: Vec<_> = allowed.difference(&missing).collect();
    assert!(new_missing.is_empty(), "{what} the interpreter lacks (add builtins, do not grow {list}): {new_missing:?}");
    assert!(now_present.is_empty(), "now implemented or no longer declared; delete from tests/data/{list}: {now_present:?}");
}

#[test]
fn std_declarations_and_interpreter_builtins_agree() {
    check_ratchet(
        declared_std(),
        "std_interpreter_missing.txt",
        "# std/<mod>.wj and std/<mod>/mod.wj top-level fns and consts with no interpreter builtin. Ratchet: only shrinks.\n# Regenerate: WJ_UPDATE_STD_GATE=1 cargo test --test all std_interpreter_agreement\n",
        "std declares names",
    );
}

#[test]
fn runtime_public_functions_and_interpreter_builtins_agree() {
    check_ratchet(
        declared_runtime(),
        "runtime_interpreter_missing.txt",
        "# windjammer-runtime public free fns and consts (as `module::name`) with no interpreter builtin. Ratchet: only shrinks.\n# Regenerate: WJ_UPDATE_STD_GATE=1 cargo test --test all std_interpreter_agreement\n",
        "the runtime exports names",
    );
}

/// Every standard-library module constant reads the same through `mod.NAME` and `mod::NAME` in a program.
#[test]
fn module_constants_are_readable_in_qualified_form() {
    let registry = Registry::standard();
    let names: Vec<String> = declared_std()
        .into_iter()
        .filter(|d| d.rsplit_once("::").unwrap().1.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
        .filter(|d| registry.constant(d).is_some())
        .collect();
    assert!(names.len() > 20, "expected the mime and math constants, found {}", names.len());
    let modules: BTreeSet<&str> = names.iter().map(|n| n.split_once("::").unwrap().0).collect();
    let mut src = String::new();
    for m in &modules {
        src.push_str(&format!("use std::{m}\n"));
    }
    src.push_str("\nfn main() {\n");
    for n in &names {
        let (m, c) = n.split_once("::").unwrap();
        src.push_str(&format!("    assert_eq(\"${{{m}.{c}}}\", \"${{{m}::{c}}}\")\n"));
    }
    src.push_str("    println(\"ok\")\n}\n");
    let dir = tempfile::TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(&f).current_dir(dir.path()).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "ok\n", "{}", String::from_utf8_lossy(&o.stderr));
}
