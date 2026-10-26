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
//! Generic regression gates, third batch: name collisions between user methods and std/auto-generated
//! ones, slice conversions, literal widths and ownership shapes found compiling a large multi-crate
//! program. Every test emits Rust for a small program and requires `rustc` to accept it.
//!
//! Numeric-width classes belong to the numeric solver (convergence C4) and ownership classes to the
//! convergence stream: those tests are ratcheted in `.config/known-test-failures.txt`.

use std::fs;
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

fn rustc_accepts(dir: &TempDir, rs: &std::path::Path) -> Result<(), String> {
    let o = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(rs)
        .output()
        .unwrap();
    if o.status.success() {
        Ok(())
    } else {
        Err(format!("{}\n--- generated\n{}", String::from_utf8_lossy(&o.stderr), fs::read_to_string(rs).unwrap_or_default()))
    }
}

/// Emit Rust for one program and require rustc to accept it.
fn type_checks(src: &str) {
    let dir = write_project(&[("m.wj", src)]);
    let o = Command::new(WJ).args(["build", "m.wj", "--no-cargo", "--output", "out"]).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if let Err(e) = rustc_accepts(&dir, &dir.path().join("out/m.rs")) {
        panic!("generated Rust does not type-check:\n{e}");
    }
}

/// Library project (`src/mod.wj`) whose generated crate must type-check.
fn project_type_checks(files: &[(&str, &str)]) {
    let dir = write_project(files);
    let o = Command::new(WJ)
        .arg("build")
        .arg(dir.path().join("src/mod.wj"))
        .arg("--output")
        .arg(dir.path().join("gen"))
        .args(["--library", "--no-cargo", "--module-file"])
        .output()
        .unwrap();
    assert!(o.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if let Err(e) = rustc_accepts(&dir, &dir.path().join("gen/lib.rs")) {
        panic!("generated crate does not type-check:\n{e}");
    }
}

/// A user-defined `to_bytes` on a plain-field struct wins over the auto-generated GPU serialiser (no duplicate definition).
#[test]
fn user_to_bytes_method_replaces_the_generated_one() {
    type_checks(r#"pub struct Pair {
    pub lo: u64,
    pub hi: u64,
}

impl Pair {
    pub fn to_bytes(self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new()
        out.push((self.lo & 255) as u8)
        out
    }
}
"#);
}

/// `name[a..].to_string()` yields an owned `String`, not `&String`, and can be pushed into a `Vec<string>`; a field assigned from an owned parameter stays owned.
#[test]
fn slice_to_string_is_not_borrowed_after_conversion() {
    type_checks(r#"pub struct Store {
    pub items: Vec<Vec<u8>>,
    pub names: Vec<string>,
}

impl Store {
    pub fn set(self, k: usize, data: Vec<u8>) {
        self.items[k] = data
    }

    pub fn tails(self, prefix: string) -> Vec<string> {
        let mut out: Vec<string> = Vec::new()
        let mut i: usize = 0
        while i < self.names.len() {
            let name = self.names[i]
            if name.starts_with(prefix) {
                let rest = name[prefix.len()..].to_string()
                out.push(rest)
            }
            i = i + 1
        }
        out
    }
}
"#);
}

/// A user method named like a std lookup (`contains`) is not given the std key borrow: `self.contains(r.k)` passes the field by value.
#[test]
fn user_method_named_contains_keeps_its_owned_formal() {
    type_checks(r#"pub struct Rec {
    pub k: i64,
}

pub struct Reg {
    pub ids: Vec<i64>,
}

impl Reg {
    pub fn contains(self, id: i64) -> bool {
        let mut i = 0
        while i < self.ids.len() {
            if self.ids[i] == id {
                return true
            }
            i = i + 1
        }
        false
    }

    pub fn has_rec(self, r: Rec) -> bool {
        self.contains(r.k)
    }
}
"#);
}

/// `f64::from_bits(x.to_bits())` resolves (std numeric associated function).
#[test]
fn f64_from_bits_has_a_boundary_signature() {
    type_checks(r#"pub fn roundtrip(x: f64) -> f64 {
    f64::from_bits(x.to_bits())
}
"#);
}

/// Decimal and hex `u64` literals at or above 2^63 lex, and emit as valid `u64` literals.
#[test]
fn u64_literals_above_i64_max_lex_and_emit() {
    type_checks(r#"pub fn a() -> u64 {
    0xff51afd7ed558ccd
}
pub fn b() -> u64 {
    18397679294719823053
}
pub fn c(x: u64) -> u64 {
    x.wrapping_mul(0x9e3779b97f4a7c15)
}
"#);
}

/// `let a = if i < v.len() { v[i] } else { 0 }` with `v: Vec<u64>` types the literal arm `u64` (guard).
#[test]
fn if_else_literal_arm_takes_u64_from_vec_element() {
    type_checks(r#"pub struct Ctr {
    pub cs: Vec<u64>,
}

impl Ctr {
    pub fn merge(self, other: Ctr) -> Ctr {
        let mut merged: Vec<u64> = Vec::new()
        let mut i = 0
        while i < self.cs.len() {
            let a = if i < self.cs.len() {
                self.cs[i]
            } else {
                0
            }
            let b = if i < other.cs.len() {
                other.cs[i]
            } else {
                0
            }
            if a > b {
                merged.push(a)
            } else {
                merged.push(b)
            }
            i = i + 1
        }
        Ctr { cs: merged }
    }
}
"#);
}

/// Nested `vec![vec![..]]` literals take `i64` from the formal, as a binding and as a direct argument (guard).
#[test]
fn nested_vec_literals_follow_i64_formal() {
    type_checks(r#"pub fn total(rows: Vec<Vec<i64>>) -> i64 {
    let mut t: i64 = 0
    for r in rows {
        for v in r {
            t = t + v
        }
    }
    t
}

pub fn run() -> i64 {
    let rows = vec![vec![1, 2], vec![3]]
    total(rows)
}

pub fn run2() -> i64 {
    total(vec![vec![4, 5], vec![6]])
}
"#);
}

/// A struct parameter that is only read through methods defined in another module is borrowed, so the caller can reuse it (guard).
#[test]
fn cross_module_readonly_param_stays_borrowed() {
    project_type_checks(&[
        ("src/store.wj", r#"pub struct Store {
    pub items: Vec<u64>,
    pub last: u64,
}

impl Store {
    pub fn get(self, i: usize) -> u64 {
        self.items[i]
    }
}
"#),
        ("src/store_test.wj", r#"use crate::store::Store

fn check(db: Store, n: usize) {
    assert_eq(db.last, 3)
    let mut i: usize = 0
    while i < n {
        assert_eq(db.get(i), 1)
        i = i + 1
    }
}

pub fn test_twice() {
    let mut db = Store { items: vec![1, 1], last: 3 }
    check(db, 2)
    check(db, 2)
}
"#),
        ("src/mod.wj", r#"pub mod store
pub mod store_test
"#),
    ]);
}

/// A user method named `get` with owned formals, called from another module, is not given the std collection borrow (guard).
#[test]
fn user_get_method_called_across_modules_keeps_owned_formals() {
    project_type_checks(&[
        ("src/mgr.wj", r#"pub struct Mgr {
    pub rows: Vec<Vec<u8>>,
    pub reads: u64,
}

impl Mgr {
    pub fn get(self, t: usize, key: Vec<u8>) -> u64 {
        self.reads = self.reads + 1
        (self.rows[t].len() + key.len()) as u64
    }
}
"#),
        ("src/user.wj", r#"use crate::mgr::Mgr

pub fn read_it(m: Mgr, t: usize, key: Vec<u8>) -> u64 {
    m.get(t, key)
}
"#),
        ("src/mod.wj", r#"pub mod mgr
pub mod user
"#),
    ]);
}

/// Numeric solver: `let kind = seed % 5` (u64) then `if kind == 0` with struct-literal branches compares with `0_u64`.
#[test]
fn comparison_of_u64_local_with_literal_inside_struct_if() {
    type_checks(r#"pub struct Case {
    pub seed: u64,
    pub name: string,
}

pub fn case_for(seed: u64) -> Case {
    let kind = seed % 5
    if kind == 0 {
        Case { seed: seed, name: "a" }
    } else {
        Case { seed: seed, name: "b" }
    }
}
"#);
}

/// Numeric solver: `Val::Int(-1)` in an argument whose function returns `Vec<u8>` takes `i64` from the variant field, not `u8` from the unrelated result.
#[test]
fn negative_literal_in_enum_payload_is_typed_by_the_variant() {
    type_checks(r#"pub enum Val {
    Int(i64),
    Flag(bool),
}

pub fn encode(items: Vec<Val>) -> Vec<u8> {
    let mut out = Vec::new()
    for item in items {
        match item {
            Val::Int(n) => out.push((n % 256) as u8),
            Val::Flag(f) => out.push(if f { 1 } else { 0 }),
        }
    }
    out
}

pub fn test_ordering() {
    let neg = encode(vec![Val::Int(-1)])
    let pos = encode(vec![Val::Int(1)])
    assert(neg != pos, "differ")
}
"#);
}

/// Numeric solver: `let n = if a.len() < b.len() { a.len() } else { b.len() }` in a function returning `i32` stays `usize` for `while i < n`.
#[test]
fn len_if_expression_in_i32_function_stays_usize() {
    type_checks(r#"pub fn cmp(a: Vec<u8>, b: Vec<u8>) -> i32 {
    let n = if a.len() < b.len() { a.len() } else { b.len() }
    let mut i: usize = 0
    while i < n {
        if a[i] < b[i] {
            return -1
        }
        i = i + 1
    }
    0
}
"#);
}

/// Numeric solver: `data.len() < 8 + 4 + 16` compares `usize` with `usize`.
#[test]
fn len_compared_with_literal_sum_stays_usize() {
    type_checks(r#"pub fn short(data: Vec<u8>) -> bool {
    if data.len() < 8 + 4 + 16 {
        return true
    }
    false
}
"#);
}

/// Numeric solver: `let mut i = 0` compared with `.len()` in a function that returns the vector keeps `i` and `i + 1` `usize` (the compare must not cast `len()` to `i32`).
#[test]
fn counter_compared_with_len_in_swap_loop_stays_usize() {
    type_checks(r#"pub struct Entry {
    pub id: i64,
    pub score: f64,
}

pub fn sort_desc(entries: Vec<Entry>) -> Vec<Entry> {
    let mut out = entries
    let mut i = 0
    while i < out.len() {
        let mut j = i + 1
        while j < out.len() {
            if out[j].score > out[i].score {
                let tmp = out[i]
                out[i] = out[j]
                out[j] = tmp
            }
            j = j + 1
        }
        i = i + 1
    }
    out
}

pub struct State {
    pub entries: Vec<Entry>,
}

impl State {
    pub fn len(self) -> u32 {
        self.entries.len() as u32
    }
}
"#);
}

/// Ownership (convergence): a `Vec` field passed bare to a function that only borrows its parameter must not make the method take `self` by value.
#[test]
fn field_moved_into_function_keeps_mut_self_callers_valid() {
    type_checks(r#"pub struct E {
    pub key: Vec<u8>,
    pub v: u64,
}

pub fn lb(entries: Vec<E>, key: Vec<u8>) -> usize {
    let mut lo: usize = 0
    while lo < entries.len() {
        if entries[lo].v > 3 {
            break
        }
        lo = lo + 1
    }
    lo + key.len()
}

pub struct M {
    pub mem: Vec<E>,
    pub bytes: usize,
}

impl M {
    pub fn insert(self, entry: E) {
        let at = lb(self.mem, entry.key.clone())
        self.bytes = self.bytes + 1
        if at < self.mem.len() {
            self.mem[at] = entry
        } else {
            self.mem.insert(at, entry)
        }
    }
}

pub fn test_twice() {
    let mut m = M { mem: Vec::new(), bytes: 0 }
    m.insert(E { key: Vec::new(), v: 1 })
    m.insert(E { key: Vec::new(), v: 2 })
}
"#);
}

/// Ownership (convergence): a field passed by value to an `extern fn` must not make the enclosing method
/// take `self` by value, or `&mut self` callers cannot call it.
#[test]
fn field_passed_to_extern_fn_keeps_borrowed_receiver() {
    let dir = write_project(&[(
        "m.wj",
        r#"extern fn store_ffi(path: string, bytes: Vec<u8>) -> bool

pub struct Segment {
    pub bytes: Vec<u8>,
}

pub struct Writer {
    pub path: string,
    pub segment: Segment,
    pub n: u64,
}

impl Writer {
    pub fn flush(self) {
        store_ffi(self.path, self.segment.bytes)
    }

    pub fn append(self, b: u8) -> u64 {
        self.segment.bytes.push(b)
        self.n = self.n + 1
        self.flush()
        self.n
    }
}
"#,
    )]);
    let o = Command::new(WJ).args(["build", "m.wj", "--no-cargo", "--output", "out"]).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success());
    let rs = fs::read_to_string(dir.path().join("out/m.rs")).unwrap();
    assert!(!rs.contains("fn flush(self)"), "flush takes self by value:\n{rs}");
}
