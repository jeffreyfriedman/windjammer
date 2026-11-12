//! `wj fix` migrates `vec![..]` to `[..]` wherever a DECLARED type makes the place a `Vec`
//! (call formal, return type, struct field, typed `let`, assigned place), and keeps it, with the
//! reason, where the place is a declared array or has no declared type.
//!
//! Oracles per position: `wj fix` leaves no `vec!`, the fixed program passes `wj check` and prints the
//! same output as the original in the interpreter AND as compiled Rust (rustc), and `wj fix` is
//! idempotent. A mutation check shows the verdicts, not the corpus, do the work.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::process::Command;
use tempfile::TempDir;
use test_utils::compile_single;
use windjammer::intrinsics::surface_fix::{fix_source, fix_source_with};
use windjammer::intrinsics::vec_context::{verdicts, DeclIndex, Verdict};

const PRELUDE: &str = "pub struct Item {\n    pub name: string,\n    pub qty: int,\n}\n\npub struct Store {\n    pub items: Vec<Item>,\n    pub tags: Vec<string>,\n    pub note: Option<Vec<string>>,\n}\n\nimpl Store {\n    pub fn new() -> Store {\n        Store { items: [], tags: [], note: None }\n    }\n    pub fn add_tags(self, more: Vec<string>) -> int {\n        more.len()\n    }\n    pub fn with_tags(more: Vec<string>) -> Store {\n        Store { items: [], tags: more, note: None }\n    }\n}\n\nfn total(xs: Vec<int>) -> int {\n    let mut t = 0\n    for x in xs {\n        t = t + x\n    }\n    t\n}\n\nfn item(name: string, qty: int) -> Item {\n    Item { name: name, qty: qty }\n}\n";

/// (position, program appended to the prelude). Every program is valid with `vec![..]` as written.
fn corpus() -> Vec<(&'static str, String)> {
    let cases: Vec<(&str, &str)> = vec![
        ("free function argument", "fn main() {\n    let n = total(vec![1, 2, 3])\n    println(\"${n}\")\n}\n"),
        (
            "method argument",
            "fn main() {\n    let s = Store::new()\n    let n = s.add_tags(vec![\"a\", \"b\"])\n    println(\"${n}\")\n}\n",
        ),
        (
            "associated function argument",
            "fn main() {\n    let s = Store::with_tags(vec![\"x\"])\n    println(\"${s.tags.len()}\")\n}\n",
        ),
        (
            "argument over several lines",
            "fn main() {\n    let n = total(\n        vec![\n            4,\n            5,\n        ],\n    )\n    println(\"${n}\")\n}\n",
        ),
        (
            "nested list argument",
            "fn flat(xs: Vec<Vec<int>>) -> int {\n    let mut t = 0\n    for row in xs {\n        t = t + total(row)\n    }\n    t\n}\n\nfn main() {\n    let n = flat(vec![vec![1], vec![2, 3]])\n    println(\"${n}\")\n}\n",
        ),
        (
            "Ok tail and return",
            "fn make(ok: bool) -> Result<Vec<int>, string> {\n    if ok {\n        return Ok(vec![7, 8])\n    }\n    Ok(vec![9])\n}\n\nfn main() {\n    match make(true) {\n        Ok(v) => println(\"${v.len()}\"),\n        Err(e) => println(e),\n    }\n}\n",
        ),
        (
            "tail of a Vec function",
            "fn items() -> Vec<Item> {\n    vec![item(\"a\", 1), item(\"b\", 2)]\n}\n\nfn main() {\n    println(\"${items().len()}\")\n}\n",
        ),
        (
            "return and branch tails",
            "fn pick(a: bool) -> Vec<int> {\n    if a {\n        return vec![1]\n    }\n    match a {\n        true => vec![2],\n        false => vec![3, 4],\n    }\n}\n\nfn main() {\n    println(\"${pick(true).len()} ${pick(false).len()}\")\n}\n",
        ),
        (
            "Option tail",
            "fn maybe(a: bool) -> Option<Vec<int>> {\n    if a {\n        Some(vec![5, 6])\n    } else {\n        None\n    }\n}\n\nfn main() {\n    match maybe(true) {\n        Some(v) => println(\"${v.len()}\"),\n        None => println(\"none\"),\n    }\n}\n",
        ),
        (
            "field assignment",
            "fn main() {\n    let mut s = Store::new()\n    s.tags = vec![\"p\", \"q\"]\n    println(\"${s.tags.len()}\")\n}\n",
        ),
        (
            "self field assignment",
            "impl Store {\n    pub fn reset(self) -> Store {\n        let mut s = self\n        s.tags = vec![\"r\"]\n        s\n    }\n}\n\nfn main() {\n    let s = Store::new().reset()\n    println(\"${s.tags.len()}\")\n}\n",
        ),
        (
            "local assignment",
            "fn main() {\n    let mut v: Vec<int> = []\n    v = vec![4, 5, 6]\n    println(\"${v.len()}\")\n}\n",
        ),
        (
            "typed let option",
            "fn main() {\n    let o: Option<Vec<int>> = Some(vec![1, 2, 3])\n    println(\"${o.unwrap_or([]).len()}\")\n}\n",
        ),
        (
            "two list arguments",
            "fn join(a: Vec<int>, b: Vec<int>) -> int {\n    total(a) + total(b)\n}\n\nfn main() {\n    let n = join(vec![1], vec![2, 3])\n    println(\"${n}\")\n}\n",
        ),
        (
            "tail of a method",
            "impl Store {\n    pub fn names(self) -> Vec<string> {\n        vec![\"a\", \"b\"]\n    }\n}\n\nfn main() {\n    let n = Store::new().names().len()\n    println(\"${n}\")\n}\n",
        ),
        (
            "argument of a call result receiver",
            "fn main() {\n    let n = Store::new().add_tags(vec![\"z\"])\n    println(\"${n}\")\n}\n",
        ),
        (
            "field assignment on a parameter",
            "fn fill(s: Store) -> Store {\n    let mut t = s\n    t.items = vec![item(\"q\", 1)]\n    t\n}\n\nfn main() {\n    let t = fill(Store::new())\n    println(\"${t.items.len()}\")\n}\n",
        ),
        (
            "nested lists in a tail",
            "fn grid() -> Vec<Vec<int>> {\n    vec![vec![1], vec![2, 3]]\n}\n\nfn main() {\n    println(\"${grid().len()}\")\n}\n",
        ),
        (
            "struct elements as argument",
            "fn count(items: Vec<Item>) -> int {\n    let mut t = 0\n    for i in items {\n        t = t + i.qty\n    }\n    t\n}\n\nfn main() {\n    let n = count(vec![item(\"a\", 4), item(\"b\", 5)])\n    println(\"${n}\")\n}\n",
        ),
    ];
    cases.into_iter().map(|(n, b)| (n, format!("{PRELUDE}\n{b}"))).collect()
}

fn wj(args: &[&str], dir: &std::path::Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(dir).output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr))
}

fn run_interpreted(dir: &std::path::Path) -> String {
    let (ok, out) = wj(&["run", "--interpret", "main.wj"], dir);
    assert!(ok, "interpreter failed:\n{out}");
    out.lines()
        .filter(|l| !l.starts_with("warn") && !l.contains("-->") && !l.trim().is_empty() && !l.contains('|') && !l.trim_start().starts_with('='))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_compiled(src: &str) -> String {
    let rust = compile_single(src);
    let tmp = TempDir::new().unwrap();
    let rs = tmp.path().join("prog.rs");
    std::fs::write(&rs, rust).unwrap();
    let bin = tmp.path().join("prog");
    let out = Command::new("rustc").args(["--edition=2021", "-A", "warnings", "-o"]).arg(&bin).arg(&rs).output().unwrap();
    assert!(out.status.success(), "rustc failed:\n{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&Command::new(&bin).output().unwrap().stdout).to_string()
}

fn verdicts_of(src: &str) -> (Vec<Verdict>, windjammer::intrinsics::vec_context::Verdicts) {
    let tokens = windjammer::lexer::Lexer::new(src).tokenize_with_locations();
    let mut parser = windjammer::parser::Parser::new_with_source(tokens, "t.wj".into(), src.into());
    let program = parser.parse().expect("parses");
    let mut index = DeclIndex::default();
    index.add_program(&program);
    let map = verdicts(&program, &index);
    let mut v: Vec<_> = map.iter().map(|(k, v)| (*k, v.clone())).collect();
    v.sort_by_key(|(k, _)| *k);
    (v.into_iter().map(|(_, v)| v).collect(), map)
}

#[test]
fn fix_rewrites_every_declared_vec_position_and_preserves_behaviour() {
    let mut rewritten = 0usize;
    let mut total = 0usize;
    for (name, src) in corpus() {
        assert!(src.contains("vec!"), "{name}: corpus case without a vec!");
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("main.wj");
        std::fs::write(&f, &src).unwrap();
        let want_interp = run_interpreted(dir.path());
        let want_compiled = run_compiled(&src);

        let (ok, out) = wj(&["fix", "main.wj"], dir.path());
        assert!(ok, "{name}: wj fix failed:\n{out}");
        let fixed = std::fs::read_to_string(&f).unwrap();
        let before = src.matches("vec!").count();
        total += before;
        rewritten += before - fixed.matches("vec!").count();
        assert!(!fixed.contains("vec!"), "{name}: a declared-Vec position kept its macro:\n{out}\n{fixed}");

        let (ok, out) = wj(&["check", "main.wj"], dir.path());
        assert!(ok && !out.contains("W0007"), "{name}: wj check after fix:\n{out}\n{fixed}");
        assert_eq!(run_interpreted(dir.path()), want_interp, "{name}: interpreter output changed");
        assert_eq!(run_compiled(&fixed), want_compiled, "{name}: compiled output changed");

        let (ok, out) = wj(&["fix", "--check", "main.wj"], dir.path());
        assert!(ok, "{name}: wj fix is not idempotent:\n{out}");
    }
    assert!(total >= 25, "corpus too small: {total}");
    assert_eq!(rewritten, total, "rewritten {rewritten} of {total}");
}

/// Places with no declared `Vec` type that the compiler lowers: the macro stays, with the reason.
const UNDECLARED: [(&str, &str); 7] = [
    ("generic formal", "fn id<T>(x: T) -> T {\n    x\n}\n\nfn main() {\n    let n = id(vec![1, 2]).len()\n    println(\"${n}\")\n}\n"),
    ("receiver method", "fn main() {\n    let n = 1 + vec![1, 2, 3].len()\n    println(\"${n}\")\n}\n"),
    ("closure result", "fn main() {\n    let f = |a: int| vec![a, 2]\n    let n = f(1).len()\n    println(\"${n}\")\n}\n"),
    ("unknown callee", "fn main() {\n    let n = missing(vec![1])\n    println(\"${n}\")\n}\n"),
    (
        "tuple slot",
        "fn total(xs: Vec<int>) -> int {\n    xs.len()\n}\n\nfn pair(p: (Vec<int>, int)) -> int {\n    total(p.0) + p.1\n}\n\nfn main() {\n    let n = pair((vec![1, 2], 3))\n    println(\"${n}\")\n}\n",
    ),
    (
        "Err payload",
        "fn check(ok: bool) -> Result<int, Vec<string>> {\n    if ok {\n        return Ok(1)\n    }\n    Err(vec![\"bad\"])\n}\n\nfn main() {\n    match check(false) {\n        Ok(n) => println(\"${n}\"),\n        Err(e) => println(\"${e.len()}\"),\n    }\n}\n",
    ),
    (
        "wrapped in a field initializer",
        "pub struct S {\n    pub note: Option<Vec<int>>,\n}\n\nfn main() {\n    let s = S { note: Some(vec![1, 2]) }\n    println(\"${s.note.unwrap_or([]).len()}\")\n}\n",
    ),
];

#[test]
fn fix_keeps_vec_without_a_declared_type_and_says_why() {
    for (name, src) in UNDECLARED {
        let (vs, map) = verdicts_of(src);
        assert_eq!(vs.last(), Some(&Verdict::Unknown), "{name}");
        let out = fix_source_with(src, &map);
        assert!(out.text.contains("vec!"), "{name}: rewrote an undeclared position\n{}", out.text);
        let msg = out.findings.iter().map(|f| f.message.clone()).collect::<Vec<_>>().join("\n");
        assert!(msg.contains("no declared `Vec<T>` type reaches this position"), "{name}: {msg}");
        assert!(msg.contains("wj fix"), "{name}: the message says how to resolve: {msg}");
    }
}

#[test]
fn fix_keeps_vec_where_the_declared_place_is_a_fixed_array() {
    let cases = [
        ("array formal", "fn sum3(xs: [int; 3]) -> int {\n    xs[0]\n}\n\nfn main() {\n    let n = sum3(vec![1, 2, 3])\n    println(\"${n}\")\n}\n"),
        ("array return", "fn three() -> [int; 3] {\n    vec![1, 2, 3]\n}\n\nfn main() {\n    println(\"${three()[0]}\")\n}\n"),
        ("array field", "pub struct Box3 {\n    pub xs: [int; 3],\n}\n\nfn main() {\n    let mut b = Box3 { xs: [0, 0, 0] }\n    b.xs = vec![1, 2, 3]\n    println(\"${b.xs[0]}\")\n}\n"),
    ];
    for (name, src) in cases {
        let (vs, map) = verdicts_of(src);
        assert_eq!(vs.last(), Some(&Verdict::Array), "{name}");
        let out = fix_source_with(src, &map);
        assert!(out.text.contains("vec![1, 2, 3]"), "{name}: rewrote an array place\n{}", out.text);
        let msg = out.findings.iter().map(|f| f.message.clone()).collect::<Vec<_>>().join("\n");
        assert!(msg.contains("declared a fixed array"), "{name}: {msg}");
    }
}

/// Mutation check: without the declared-type verdicts (or with every verdict degraded to Unknown) the
/// corpus keeps its macros, so the positive test above can fail; the verdicts are what rewrite them.
#[test]
fn mutation_without_verdicts_keeps_the_macros() {
    let (mut kept_plain, mut kept_mutated, mut with) = (0, 0, 0);
    for (name, src) in corpus() {
        let (_, real) = verdicts_of(&src);
        assert!(!real.is_empty(), "{name}: no verdicts");
        let degraded: std::collections::HashMap<_, _> = real.keys().map(|k| (*k, Verdict::Unknown)).collect();
        kept_plain += fix_source(&src).text.matches("vec!").count();
        kept_mutated += fix_source_with(&src, &degraded).text.matches("vec!").count();
        with += fix_source_with(&src, &real).text.matches("vec!").count();
    }
    assert_eq!(with, 0);
    assert!(kept_plain >= 20, "context-free fix kept only {kept_plain}");
    assert_eq!(kept_plain, kept_mutated);
}
