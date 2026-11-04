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
//! Receiver and parameter ownership decided by one owner (convergence C5, burn-down O1-O6).
//!
//! Each test is a neutral minimal program for a shape where the analyzer, the solver and codegen used to
//! disagree. Oracle: `rustc` must accept the generated Rust (independent of the compiler), and where the
//! signature matters to callers, the emitted receiver or formal is asserted directly.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    dir
}

fn rustc_accepts(dir: &TempDir, rs: &Path) -> Result<(), String> {
    let o = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(rs)
        .output()
        .unwrap();
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).into_owned())
    }
}

/// Build one file; return the generated Rust and require rustc to accept it.
fn single_file(src: &str) -> String {
    let dir = write_project(&[("m.wj", src)]);
    let o = Command::new(WJ).args(["build", "m.wj", "--no-cargo", "--output", "out"]).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rs = fs::read_to_string(dir.path().join("out/m.rs")).unwrap();
    if let Err(e) = rustc_accepts(&dir, &dir.path().join("out/m.rs")) {
        panic!("generated Rust does not type-check:\n{e}\n--- generated\n{rs}");
    }
    rs
}

/// Build a library project (`src/mod.wj`); return (all generated .rs text, wj stdout+stderr) without a rustc check.
fn library_project(files: &[(&str, &str)]) -> (TempDir, String, String) {
    let dir = write_project(files);
    let o = Command::new(WJ)
        .arg("build")
        .arg(dir.path().join("src/mod.wj"))
        .arg("--output")
        .arg(dir.path().join("gen"))
        .args(["--library", "--no-cargo", "--module-file"])
        .output()
        .unwrap();
    let log = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "wj build failed:\n{log}");
    let mut all = String::new();
    let mut names: Vec<_> = fs::read_dir(dir.path().join("gen")).unwrap().flatten().map(|e| e.path()).collect();
    names.sort();
    for p in names {
        if p.extension().is_some_and(|e| e == "rs") {
            all.push_str(&format!("// ---- {}\n", p.file_name().unwrap().to_string_lossy()));
            all.push_str(&fs::read_to_string(&p).unwrap());
        }
    }
    (dir, all, log)
}

fn library_type_checks(files: &[(&str, &str)]) -> String {
    let (dir, all, _) = library_project(files);
    if let Err(e) = rustc_accepts(&dir, &dir.path().join("gen/lib.rs")) {
        panic!("generated crate does not type-check:\n{e}\n--- generated\n{all}");
    }
    all
}

/// A Vec field of `self` passed bare to a function whose formal is only read must not make the method
/// take `self` by value: the method mutates another field, so it is `&mut self`, and two calls in a row work.
#[test]
fn field_passed_to_read_only_formal_keeps_mut_self() {
    let rs = single_file(
        r#"pub struct Note {
    pub tag: Vec<u8>,
    pub weight: u64,
}

pub fn first_heavy(notes: Vec<Note>, probe: Vec<u8>) -> usize {
    let mut at: usize = 0
    while at < notes.len() {
        if notes[at].weight > 3 {
            break
        }
        at = at + 1
    }
    at + probe.len()
}

pub struct Board {
    pub notes: Vec<Note>,
    pub edits: usize,
}

impl Board {
    pub fn add(self, note: Note) {
        let at = first_heavy(self.notes, note.tag.clone())
        self.edits = self.edits + 1
        if at < self.notes.len() {
            self.notes[at] = note
        } else {
            self.notes.insert(at, note)
        }
    }
}

pub fn add_two() {
    let mut b = Board { notes: Vec::new(), edits: 0 }
    b.add(Note { tag: Vec::new(), weight: 1 })
    b.add(Note { tag: Vec::new(), weight: 2 })
}
"#,
    );
    assert!(rs.contains("fn add(&mut self"), "expected `&mut self`:\n{rs}");
}

/// A read-only parameter whose type is declared in another module of the same crate borrows (`&Ledger`), so the
/// caller can pass the same value twice.
#[test]
fn read_only_parameter_of_a_type_from_another_module_borrows() {
    let all = library_type_checks(&[
        ("src/mod.wj", "pub mod ledger\npub mod audit\n"),
        (
            "src/ledger.wj",
            "pub struct Ledger {\n    pub rows: Vec<u64>,\n    pub tail: u64,\n}\n\nimpl Ledger {\n    pub fn row(self, i: usize) -> u64 {\n        self.rows[i]\n    }\n}\n",
        ),
        (
            "src/audit.wj",
            "use crate::ledger::Ledger\n\nfn verify(l: Ledger, n: usize) {\n    assert_eq(l.tail, 3)\n    let mut i: usize = 0\n    while i < n {\n        assert_eq(l.row(i), 1)\n        i = i + 1\n    }\n}\n\npub fn verify_twice() {\n    let l = Ledger { rows: vec![1, 1], tail: 3 }\n    verify(l, 2)\n    verify(l, 2)\n}\n",
        ),
    ]);
    assert!(all.contains("fn verify(l: &Ledger"), "expected `l: &Ledger`:\n{all}");
}

/// A read-only `match self` whose arms use their bindings borrows the receiver (`&self`): a caller holding the
/// enum in a field of `&mut self` can call it.
#[test]
fn read_only_match_on_self_with_bindings_borrows_the_receiver() {
    let rs = single_file(
        r#"pub enum Badge {
    Titled { title: string },
    Blank,
}

impl Badge {
    pub fn caption(self) -> string {
        match self {
            Badge::Titled { title } => "titled:" + title,
            Badge::Blank => "blank",
        }
    }
}

pub struct Desk {
    pub badge: Badge,
    pub looks: u64,
}

impl Desk {
    pub fn show(self) -> string {
        self.looks = self.looks + 1
        self.badge.caption()
    }
}
"#,
    );
    assert!(rs.contains("fn caption(&self"), "expected `&self`:\n{rs}");
}

/// `o = self.parts[n].advance(x)` calls a `&mut self` method on an element of a field: that mutates `self`.
#[test]
fn mutating_call_on_indexed_field_element_in_assignment_is_a_mutation() {
    let rs = single_file(
        r#"pub struct Gear {
    pub turns: u64,
}

impl Gear {
    pub fn advance(self, x: u64) -> u64 {
        self.turns = self.turns + x
        self.turns
    }
}

pub struct Train {
    pub gears: Vec<Gear>,
}

impl Train {
    pub fn run(self, xs: Vec<u64>) -> Vec<u64> {
        let mut out: Vec<u64> = Vec::new()
        let mut n: usize = 0
        while n < self.gears.len() {
            let mut o: u64 = 0
            if n == 0 {
                o = xs[0]
            } else {
                o = self.gears[n].advance(xs[n])
            }
            out.push(o)
            n = n + 1
        }
        out
    }
}
"#,
    );
    assert!(rs.contains("fn run(&mut self"), "expected `&mut self`:\n{rs}");
}

const COUNTER_MODULE: &str = "pub struct Counter {\n    pub hits: u64,\n}\n\nimpl Counter {\n    pub fn touch(self, k: u64) -> u64 {\n        self.hits = self.hits + 1\n        k\n    }\n}\n";

const VAULT_IMPL: &str = "use crate::store::Store\nuse crate::counter::Counter\n\npub struct Vault {\n    pub c: Counter,\n}\n\nimpl Store for Vault {\n    fn get(self, k: u64) -> u64 {\n        self.c.touch(k)\n    }\n    fn put(self, k: u64, v: u64) {\n        self.c.touch(k + v)\n    }\n}\n";

/// A trait method's receiver is what the trait declares: `&mut self` in the declaration is emitted in the trait
/// and in an impl in another module, whatever the method is called.
#[test]
fn declared_trait_receiver_is_authoritative_across_modules() {
    let all = library_type_checks(&[
        ("src/mod.wj", "pub mod counter\npub mod store\npub mod vault\n"),
        ("src/counter.wj", COUNTER_MODULE),
        ("src/store.wj", "pub trait Store {\n    fn get(&mut self, k: u64) -> u64\n    fn put(&mut self, k: u64, v: u64)\n}\n"),
        ("src/vault.wj", VAULT_IMPL),
    ]);
    assert!(all.contains("fn get(&mut self, k: u64) -> u64;"), "{all}");
    assert!(all.contains("fn get(&mut self, k: u64) -> u64 {"), "{all}");
}

/// A bodyless trait method that does not declare its receiver mode (`self` with no `&`/`&mut`) gets a
/// diagnostic naming the method and the fix, instead of a silent guess.
#[test]
fn bodyless_trait_method_without_declared_receiver_is_diagnosed() {
    let (_dir, _all, log) = library_project(&[
        ("src/mod.wj", "pub mod counter\npub mod store\npub mod vault\n"),
        ("src/counter.wj", COUNTER_MODULE),
        ("src/store.wj", "pub trait Store {\n    fn get(self, k: u64) -> u64\n    fn put(self, k: u64, v: u64)\n}\n"),
        ("src/vault.wj", VAULT_IMPL),
    ]);
    assert!(log.contains("Store::get") && log.contains("&mut self"), "expected a receiver diagnostic for `Store::get`:\n{log}");
}

/// The receiver of a bodyless trait method never comes from its name: an undeclared `get` whose impl mutates
/// (same file) is `&mut self`, like any other name.
#[test]
fn undeclared_trait_receiver_follows_the_impls_not_the_name() {
    let rs = single_file(
        r#"pub trait Bin {
    fn get(self, k: u64) -> u64
    fn delete(self, k: u64)
}

pub struct Tray {
    pub taken: u64,
}

impl Bin for Tray {
    fn get(self, k: u64) -> u64 {
        self.taken = self.taken + k
        self.taken
    }
    fn delete(self, k: u64) {
        self.taken = self.taken - k
    }
}
"#,
    );
    assert!(rs.contains("fn get(&mut self, k: u64) -> u64;"), "{rs}");
    assert!(rs.contains("fn delete(&mut self, k: u64);"), "{rs}");
}

/// A mutating method that reads a non-Copy field into a local (`let allowed = self.cfg.allowed`) keeps `&mut self`
/// (the read is cloned), so a caller holding the value behind `&mut` (a lock guard, a field) can call it.
#[test]
fn mutating_method_reading_a_non_copy_field_keeps_mut_self() {
    let rs = single_file(
        r#"pub struct Cfg {
    pub allowed: Vec<string>,
}

pub struct Gate {
    pub cfg: Cfg,
    pub hits: int,
}

impl Gate {
    fn bump(self) {
        self.hits = self.hits + 1
    }

    pub fn admit(self, who: string) -> bool {
        let allowed = self.cfg.allowed
        self.bump()
        allowed.contains(who)
    }
}

pub struct Server {
    pub gate: Gate,
}

impl Server {
    pub fn serve(self, who: string) -> bool {
        self.gate.admit(who) && self.gate.admit("x")
    }
}
"#,
    );
    assert!(rs.contains("fn admit(&mut self"), "expected `&mut self`:\n{rs}");
}
