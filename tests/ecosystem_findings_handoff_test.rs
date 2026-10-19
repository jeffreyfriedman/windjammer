#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! Ratchet gates for ecosystem porting findings that are ownership / integer-width decisions owned by the
//! convergence stream (`docs/known-issues/ownership-same-name-free-functions-share-parameter-modes.md`,
//! `compiled-int-width-index-variable-pushed-to-vec-int.md`, `compiled-param-moved-into-mutex-new-is-borrowed.md`).
//! Each test states the required behaviour; open ones are listed in `.config/known-test-failures.txt` and leave the
//! list when the convergence work lands. No codegen decisions are made here.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn compile_and_run(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).current_dir(dir.path()).output().unwrap();
    if !wj.status.success() {
        return Err(format!("wj build failed: {}{}", String::from_utf8_lossy(&wj.stdout), String::from_utf8_lossy(&wj.stderr)));
    }
    let rs = dir.path().join("build/test.rs");
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(&rs).arg("-o").arg(&bin).args(["--edition", "2021"]).output().unwrap();
    if !rustc.status.success() {
        let generated = fs::read_to_string(&rs).unwrap_or_default();
        return Err(format!("rustc failed: {}\n--- generated\n{}", String::from_utf8_lossy(&rustc.stderr), generated));
    }
    Ok(String::from_utf8_lossy(&Command::new(&bin).output().unwrap().stdout).to_string())
}

/// C4: an index variable (`v[i]`, `i < v.len()`) whose `i + 1` flows into a `Vec<int>` / `int` is one integer width.
#[test]
fn index_counter_pushed_to_vec_int_keeps_one_width() {
    let out = compile_and_run(
        r#"fn line_numbers(lines: Vec<string>) -> Vec<int> {
    let mut out = Vec::new()
    let mut i = 0
    while i < lines.len() {
        out.push(i + 1)
        i = i + 1
    }
    out
}

fn total(a: Vec<string>) -> int {
    let mut n = 0
    for s in a {
        n = n + s.len()
    }
    n
}

fn main() {
    let v = ["a", "bb"]
    println("${line_numbers(v).len()} ${total(v)}")
}
"#,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, "2 3\n");
}

/// C4: `.len()` stored into an `int` struct field.
#[test]
fn len_into_int_field_converts() {
    let out = compile_and_run(
        r#"struct Delivery {
    conn: int,
}

fn count(items: Vec<string>) -> Delivery {
    Delivery { conn: items.len() }
}

fn main() {
    let items = ["a", "b", "c"]
    println("${count(items).conn}")
}
"#,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, "3\n");
}

/// C5: a parameter moved into `Mutex::new` stays owned (`fn share(c: Counter)`, not `c: &Counter`).
#[test]
fn param_moved_into_mutex_new_stays_owned() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(
        &f,
        r#"use std::sync::{Arc, Mutex}

struct Counter {
    n: int,
    label: string,
}

fn share(c: Counter) -> Arc<Mutex<Counter>> {
    Arc::new(Mutex::new(c))
}

fn main() {
    let s = share(Counter { n: 1, label: "a" })
    println("done")
}
"#,
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).current_dir(dir.path()).output().unwrap();
    assert!(wj.status.success(), "wj build failed: {}", String::from_utf8_lossy(&wj.stderr));
    let rs = fs::read_to_string(dir.path().join("build/test.rs")).unwrap();
    assert!(rs.contains("fn share(c: Counter)"), "parameter moved into Mutex::new must stay owned:\n{rs}");
}

/// C3: free functions with the same name in different modules keep their own parameter modes.
#[test]
fn same_name_free_functions_keep_own_parameter_modes() {
    let files: [(&str, &str); 6] = [
        ("wj.toml", "[package]\nname = \"samename\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[dependencies]\n"),
        ("src/mod.wj", "pub mod domain\n"),
        ("src/domain/mod.wj", "pub mod a\npub mod b\npub mod c\n"),
        (
            "src/domain/a.wj",
            "pub struct User {\n    pub id: int,\n}\n\npub fn show(user: User, n: int) -> int {\n    user.id + n\n}\n",
        ),
        (
            "src/domain/b.wj",
            "use crate::domain::a::User\n\npub fn show(user: User, sink: Vec<User>) -> int {\n    sink.push(user)\n    sink.len()\n}\n",
        ),
        (
            "src/domain/c.wj",
            "use crate::domain::a::User\nuse crate::domain::a::show as show_user\nuse crate::domain::b::show as show_sink\n\npub fn both() -> int {\n    let mut sink = Vec::new()\n    let a = show_user(User { id: 1 }, 2)\n    let b = show_sink(User { id: 3 }, sink)\n    a + b\n}\n",
        ),
    ];
    let dir = TempDir::new().unwrap();
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    let out = dir.path().join("gen");
    let o = Command::new(WJ)
        .arg("build")
        .arg(dir.path().join("src/mod.wj"))
        .arg("--output")
        .arg(&out)
        .args(["--library", "--no-cargo", "--module-file"])
        .output()
        .unwrap();
    assert!(o.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rustc = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(out.join("lib.rs"))
        .output()
        .unwrap();
    assert!(rustc.status.success(), "generated library does not compile:\n{}", String::from_utf8_lossy(&rustc.stderr));
}

/// C6 (found porting `wj-todo-cli`): a by-value `Store` threaded through `run_args` -> `apply` in another module
/// stays owned at both functions. Today `apply`/`run_args` are emitted with `&mut Store`, and the callers then
/// pass `&Store` (E0308 types differ in mutability).
#[test]
fn store_threaded_through_adapter_functions_stays_owned() {
    let dir = TempDir::new().unwrap();
    let files: [(&str, &str); 6] = [
        ("wj.toml", "[package]\nname = \"todop\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[dependencies]\n"),
        ("src/main.wj", "mod domain\nmod adapters\n\nfn main() {\n    println(\"x\")\n}\n"),
        ("src/domain/mod.wj", "pub mod store\npub use store::Store\n"),
        ("src/domain/store.wj", STORE),
        ("src/adapters/mod.wj", "pub mod run\n"),
        ("src/adapters/run.wj", RUN),
    ];
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    let o = Command::new(WJ).args(["build", "--no-cargo", "src"]).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success(), "wj build failed: {}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rs = fs::read_to_string(dir.path().join("build/adapters/run.rs")).unwrap();
    assert!(rs.contains("fn apply(store: Store,"), "apply must take the store by value:\n{rs}");
    assert!(rs.contains("fn run_args(store: Store,"), "run_args must take the store by value:\n{rs}");
}

const STORE: &str = r####"pub struct Store {
    items: Vec<string>,
}

impl Store {
    pub fn new() -> Store {
        Store { items: Vec::new() }
    }

    pub fn add(self, title: string) -> Result<(Store, int), string> {
        if title == "" {
            return Err("empty")
        }
        let mut next = self.items
        next.push(title)
        let n = next.len() as int
        Ok((Store { items: next }, n))
    }

    pub fn into_items(self) -> Vec<string> {
        self.items
    }

    pub fn merge_from(self, imported: Vec<string>) -> (Store, int) {
        let mut next = self.items
        let mut added = 0
        for x in imported {
            if !next.contains(x) {
                next.push(x)
                added = added + 1
            }
        }
        (Store { items: next }, added)
    }

    pub fn clear_all(self) -> Store {
        Store { items: Vec::new() }
    }
}

"####;

const RUN: &str = r####"use crate::domain::Store

fn encode(store: Store) -> string {
    store.items.join(",")
}

fn decode(text: string) -> Result<Store, string> {
    Ok(Store { items: text.split(",").map(|x| x.to_string()).collect() })
}

fn apply(store: Store, kind: string, title: string) -> Result<(Store, string), string> {
    if kind == "add" {
        match store.add(title) {
            Ok(pair) => Ok((pair.0, "added ${pair.1}")),
            Err(e) => Err(e),
        }
    } else if kind == "list" {
        let raw = store.into_items()
        Ok((Store { items: raw }, "listed"))
    } else if kind == "export" {
        let snapshot = encode(store)
        match decode(snapshot) {
            Ok(copy) => {
                let raw = copy.into_items()
                match decode(snapshot) {
                    Ok(restored) => Ok((restored, "x ${raw.len()}")),
                    Err(e) => Err(e),
                }
            },
            Err(e) => Err(e),
        }
    } else if kind == "import" {
        let imported = title.split(",").map(|x| x.to_string()).collect()
        if title == "merge" {
            let result = store.merge_from(imported)
            Ok((result.0, "imported ${result.1}"))
        } else {
            Ok((Store { items: imported }, "imported"))
        }
    } else {
        Ok((store.clear_all(), "cleared"))
    }
}

pub fn run_args(store: Store, kind: string, title: string) -> Result<(Store, string), string> {
    apply(store, kind, title)
}

"####;
