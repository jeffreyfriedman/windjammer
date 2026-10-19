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

//! Generic regression gates for compiler findings from compiling a large multi-module program.
//!
//! One test per finding class. Numeric-width classes (F4, F5) are acceptance tests owned by the
//! numeric solver; cross-module ownership classes (F7-F9) are owned by the convergence stream.
//! Open ones are listed in `.config/known-test-failures.txt`; notes in `docs/known-issues/dogfood-findings.md`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn interpret(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let out = Command::new(WJ).args(["run", "--interpret"]).arg(&f).output().unwrap();
    if !out.status.success() {
        return Err(format!("interpreter failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn compile_and_run(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ)
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(&f)
        .current_dir(dir.path())
        .output()
        .unwrap();
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
    let run = Command::new(&bin).output().unwrap();
    Ok(String::from_utf8_lossy(&run.stdout).to_string())
}

/// Compiled output, interpreter output and the hand-derived expectation must all agree.
fn agree(src: &str, expect: &str) {
    let interp = interpret(src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(interp, expect, "interpreter output differs from the expectation");
    let compiled = compile_and_run(src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(compiled, expect, "compiled output differs from the expectation");
}

fn write_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    dir
}

/// Build `src/mod.wj` as a library project and require the generated crate to type-check.
fn project_compiles(files: &[(&str, &str)]) {
    let dir = write_project(files);
    let out = dir.path().join("gen");
    let o = Command::new(WJ)
        .arg("build")
        .arg(dir.path().join("src/mod.wj"))
        .arg("--output")
        .arg(&out)
        .args(["--library", "--no-cargo", "--module-file"])
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "wj build failed:\n{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let rustc = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(out.join("lib.rs"))
        .output()
        .unwrap();
    assert!(rustc.status.success(), "generated library does not compile:\n{}", String::from_utf8_lossy(&rustc.stderr));
}

/// Run the entry `main.wj` of a multi-file directory interpreted, and compiled as a directory build
/// (`wj build <dir>` + `rustc main.rs`); both must print `expect`.
fn project_runs_both_ways(files: &[(&str, &str)], expect: &str) {
    let dir = write_project(files);
    let o = Command::new(WJ)
        .args(["run", "--interpret"])
        .arg(dir.path().join("main.wj"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "interpreted run failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout), expect, "interpreted output differs from the expectation");

    let out = dir.path().join("gen");
    let b = Command::new(WJ).arg("build").arg(dir.path()).arg("--output").arg(&out).arg("--no-cargo").output().unwrap();
    assert!(b.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&b.stdout), String::from_utf8_lossy(&b.stderr));
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(out.join("main.rs")).arg("-o").arg(&bin).args(["--edition", "2021"]).output().unwrap();
    assert!(rustc.status.success(), "rustc failed:\n{}", String::from_utf8_lossy(&rustc.stderr));
    let run = Command::new(&bin).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout), expect, "compiled output differs from the expectation");
}

// ------------------------------------------------------------------ F2: std idiom lowering

/// `map.get(k).unwrap_or(&0)` on `HashMap<string, u32>` must yield a `u32` that type-checks.
#[test]
fn map_get_unwrap_or_ref_literal_yields_value() {
    agree(
        r#"use std::collections::HashMap

pub struct Counter {
    counts: HashMap<string, u32>,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { counts: HashMap::new() }
    }

    pub fn add(self, key: string, n: u32) {
        self.counts.insert(key, n)
    }

    pub fn get(self, key: string) -> u32 {
        let c = self.counts.get(key).unwrap_or(&0)
        c
    }
}

fn main() {
    let c = Counter::new()
    c.add("a", 7)
    println(c.get("a"))
    println(c.get("b"))
}
"#,
        "7\n0\n",
    );
}

// ------------------------------------------------------------------ F4, F5: numeric solver (C4)

/// Acceptance test (numeric solver): an untyped float local combined with an `f32` field stays `f32`
/// through `.min(1.0).max(0.0)`.
#[test]
fn untyped_float_local_min_max_chain_stays_f32() {
    agree(
        r#"pub struct Mods {
    pub a: f32,
}

impl Mods {
    pub fn combine(self, flag: bool) -> f32 {
        let mut v = 1.0
        if flag {
            v = v * self.a
        }
        v.min(1.0).max(0.0)
    }
}

fn main() {
    let m = Mods { a: 0.5 }
    println(m.combine(true))
    println(m.combine(false))
}
"#,
        "0.5\n1\n",
    );
}

/// Acceptance test (numeric solver): a typed `u32` literal pushed to `Vec<u32>` is not re-cast to
/// `i32`, and does not influence a later push to another vector.
#[test]
fn push_typed_literal_width_does_not_leak() {
    agree(
        r#"pub fn build() -> Vec<i32> {
    let mut a: Vec<u32> = Vec::new()
    let mut b: Vec<i32> = Vec::new()
    a.push(0u32)
    b.push(0)
    b.push(a.len() as i32)
    b
}

fn main() {
    let v = build()
    println(v.len())
    println(v[0] + v[1])
}
"#,
        "2\n1\n",
    );
}

// ------------------------------------------------------------------ F6: module aliases in calls

const LOWLEVEL: &str = "pub fn make(n: u32) -> u32 {\n    n + 1\n}\n";

#[test]
fn module_alias_call_resolves_same_crate_callee() {
    project_compiles(&[
        ("src/lowlevel.wj", LOWLEVEL),
        ("src/caller.wj", "use crate::lowlevel as low\n\npub fn go() -> u32 {\n    low::make(4)\n}\n"),
        ("src/mod.wj", "pub mod lowlevel\npub mod caller\n"),
    ]);
}

#[test]
fn module_alias_call_with_borrowed_param_resolves_converged_signature() {
    project_compiles(&[
        ("src/lowlevel.wj", "pub fn size(s: string) -> int {\n    s.len() as int\n}\n"),
        (
            "src/caller.wj",
            "use crate::lowlevel as low\n\npub fn go(name: string) -> int {\n    low::size(name) + low::size(name)\n}\n",
        ),
        ("src/mod.wj", "pub mod lowlevel\npub mod caller\n"),
    ]);
}

#[test]
fn super_qualified_call_resolves_same_crate_callee() {
    project_compiles(&[
        ("src/lowlevel.wj", LOWLEVEL),
        ("src/outer/mod.wj", "pub mod inner\n"),
        ("src/outer/inner.wj", "pub fn go() -> u32 {\n    super::super::lowlevel::make(4)\n}\n"),
        ("src/mod.wj", "pub mod lowlevel\npub mod outer\n"),
    ]);
}

#[test]
fn self_qualified_call_resolves_same_crate_callee() {
    project_compiles(&[
        ("src/lowlevel.wj", "pub fn make(n: u32) -> u32 {\n    n + 1\n}\n\npub fn twice(n: u32) -> u32 {\n    self::make(self::make(n))\n}\n"),
        ("src/mod.wj", "pub mod lowlevel\n"),
    ]);
}

// ------------------------------------------------------------------ interpreter test runner

fn run_wj(dir: &std::path::Path, args: &[&str]) -> (bool, String) {
    let o = Command::new(WJ).args(args).current_dir(dir).output().unwrap();
    (
        o.status.success(),
        format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)),
    )
}

const CRATE_FILES: &[(&str, &str)] = &[
    ("wj.toml", "[package]\nname = \"pair\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/lib.wj\"\n"),
    ("src/lib.wj", "pub mod a\npub mod b\npub mod sub\n"),
    (
        "src/a.wj",
        "/// Build a point.\n///\n/// ```wj\n/// let p = P::new()\n/// assert_eq(p.x, 3)\n/// ```\npub struct P {\n    pub x: int,\n}\n\nimpl P {\n    pub fn new() -> P {\n        P { x: 3 }\n    }\n}\n",
    ),
    (
        "src/b.wj",
        "use crate::a::P\n\n/// Double the x of a point.\n///\n/// ```wj\n/// use crate::a::P\n/// let d = double_x(P::new())\n/// assert_eq(d, 6)\n/// ```\npub fn double_x(p: P) -> int {\n    p.x * 2\n}\n",
    ),
    ("src/sub/mod.wj", "pub mod c\n"),
    ("src/sub/c.wj", "pub fn one() -> int {\n    1\n}\n"),
    (
        "src/sub/c_test.wj",
        "use crate::a::P\nuse crate::sub::c::one\n\n@test\nfn test_nested_module_reaches_siblings() {\n    assert_eq(P::new().x + one(), 4)\n}\n",
    ),
    (
        "src/a_test.wj",
        "use crate::a::P\n\n@test\nfn test_new_has_x() {\n    let p = P::new()\n    assert_eq(p.x, 3)\n}\n",
    ),
];

#[test]
fn interpreted_test_file_loads_sibling_crate_modules() {
    let dir = write_project(CRATE_FILES);
    let (ok, out) = run_wj(dir.path(), &["test", "--interpret", "src/a_test.wj"]);
    assert!(ok, "interpreted test of a single file failed:\n{out}");
    assert!(out.contains("1 passed; 0 failed"), "no pass reported:\n{out}");
}

#[test]
fn interpreted_test_file_in_subdirectory_loads_crate_modules() {
    let dir = write_project(CRATE_FILES);
    let (ok, out) = run_wj(dir.path(), &["test", "--interpret", "src/sub/c_test.wj"]);
    assert!(ok, "interpreted test of a nested file failed:\n{out}");
    assert!(out.contains("1 passed; 0 failed"), "no pass reported:\n{out}");
}

#[test]
fn interpreted_test_on_project_root_finds_tests() {
    let dir = write_project(CRATE_FILES);
    let (ok, out) = run_wj(dir.path(), &["test", "--interpret", "."]);
    assert!(ok, "interpreted test of project root failed:\n{out}");
    assert!(out.contains("2 passed; 0 failed"), "project root found no tests:\n{out}");
}

#[test]
fn compiled_test_on_crate_with_crate_use_passes() {
    let dir = write_project(CRATE_FILES);
    let (ok, out) = run_wj(dir.path(), &["test", "."]);
    assert!(ok, "compiled wj test failed:\n{out}");
}

#[test]
fn interpreted_doctests_in_library_module_run() {
    let dir = write_project(CRATE_FILES);
    let (ok, out) = run_wj(dir.path(), &["test", "--interpret", "--doc", "."]);
    assert!(ok, "interpreted doctests failed:\n{out}");
    assert!(out.contains("doctests: 2 passed, 0 failed"), "doctests did not run:\n{out}");
}

// ------------------------------------------------------------------ F7-F9: cross-module ownership
//
// Owned by the convergence stream (ownership decisions); these are acceptance tests, not codegen
// patches. Each is a minimal multi-file project.

/// F7: a trait declared in one module with `self` receivers, implemented by a no-op type and by a
/// mutating type in two other modules. The trait receiver must be the widest one any impl needs
/// (`&mut self`); the single-file form of this program already gets that right.
#[test]
fn trait_receiver_widens_to_mutating_impl_across_modules() {
    project_compiles(&[
        ("src/ports.wj", "pub trait Sink {\n    fn get(self) -> int\n    fn set(self, v: int)\n}\n"),
        (
            "src/null_sink.wj",
            "use crate::ports::Sink\n\npub struct Null {}\n\nimpl Sink for Null {\n    fn get(self) -> int {\n        0\n    }\n    fn set(self, v: int) {\n    }\n}\n",
        ),
        (
            "src/cell.wj",
            "use crate::ports::Sink\n\npub struct Cell {\n    pub value: int,\n}\n\nimpl Sink for Cell {\n    fn get(self) -> int {\n        self.value\n    }\n    fn set(self, v: int) {\n        self.value = v\n    }\n}\n",
        ),
        ("src/mod.wj", "pub mod ports\npub mod null_sink\npub mod cell\n"),
    ]);
}

/// F8 (not reproduced on this tip): a by-value `f32` argument forwarded through `self.items[i].m(v)` into
/// another module must stay by value. Guards the shape; the original failure needs a larger program.
#[test]
fn by_value_scalar_forwarded_through_indexed_receiver_across_modules() {
    project_compiles(&[
        (
            "src/channel.wj",
            "pub struct Channel {\n    pub volume: f32,\n}\n\nimpl Channel {\n    pub fn set_volume(self, volume: f32) {\n        volume = volume * 0.5\n        self.volume = volume\n    }\n}\n",
        ),
        (
            "src/mixer.wj",
            "use crate::channel::Channel\n\npub struct Mixer {\n    pub items: Vec<Channel>,\n}\n\nimpl Mixer {\n    pub fn set(self, i: int, volume: f32) {\n        self.items[i].set_volume(volume)\n    }\n}\n",
        ),
        ("src/mod.wj", "pub mod mixer\npub mod channel\n"),
    ]);
}

/// F9 (not reproduced on this tip): `&T` vs `T` arguments for by-value `Vec`/struct formals defined in
/// another module, with the argument reused after the call.
#[test]
fn owned_vec_and_struct_args_across_modules_reused_after_call() {
    project_compiles(&[
        (
            "src/bag.wj",
            "pub struct Item {\n    pub n: int,\n}\n\npub struct Bag {\n    pub items: Vec<Item>,\n    pub best: Item,\n}\n\nimpl Bag {\n    pub fn new(items: Vec<Item>, best: Item) -> Bag {\n        Bag { items: items, best: best }\n    }\n\n    pub fn replace(self, items: Vec<Item>, best: Item) {\n        self.items = items\n        self.best = best\n    }\n\n    pub fn count(self) -> int {\n        self.items.len() as int\n    }\n}\n\npub fn make(items: Vec<Item>) -> Bag {\n    let best = Item { n: items.len() as int }\n    Bag::new(items, best)\n}\n",
        ),
        (
            "src/user.wj",
            "use crate::bag::Bag\nuse crate::bag::Item\nuse crate::bag::make\n\npub struct Holder {\n    pub bag: Bag,\n    pub spare: Vec<Item>,\n}\n\nimpl Holder {\n    pub fn rebuild(self, extra: Vec<Item>, pick: Item) -> int {\n        self.bag.replace(extra, pick)\n        let n = self.bag.count()\n        let again = make(self.spare)\n        n + again.count() + extra.len() as int + pick.n\n    }\n}\n\npub fn run(list: Vec<Item>) -> int {\n    let bag = make(list)\n    bag.count() + list.len() as int\n}\n",
        ),
        ("src/mod.wj", "pub mod bag\npub mod user\n"),
    ]);
}

// ------------------------------------------------------------------ found while reducing (single file)

/// A by-value `Vec` parameter that is mutated and then moved into a field is emitted as `&mut Vec`
/// (E0308 at the field assignment). Ownership decision: convergence stream.
#[test]
fn mutated_by_value_param_moved_into_field_stays_owned() {
    agree(
        r#"pub struct Bus {
    pub levels: Vec<f32>,
}

impl Bus {
    pub fn set_levels(self, volume: Vec<f32>) {
        volume.push(1.0)
        self.levels = volume
    }
}

fn main() {
    let b = Bus { levels: vec![] }
    b.set_levels(vec![0.5])
    println(b.levels.len())
}
"#,
        "2\n",
    );
}
