#![cfg(feature = "jit")]

//! ADR-0020 round 3: machine-code loops (`resolved/jit/osr.rs`) are a second implementation of the
//! walker's arithmetic, comparisons, casts, `Vec` and field access, so they are checked against the
//! tree walker over generated programs: hot `while` loops over `i32`, `int`, `usize`, `u8`, `f64`
//! and `bool` locals, `Vec<int>`/`Vec<f64>`/`Vec<bool>` elements and fields of a `Vec` of structs,
//! with overflows, out-of-bounds indices, zero divisors, NaN/infinities/`-0.0`, `break`/`continue`
//! and nested loops. Every program runs under the walker, under the resolved engine with every loop
//! entering machine code at its first iteration (`WJ_JIT_LOOP_HOT=0`), with the default threshold,
//! and with `--no-jit`; stdout, stderr and the exit status must be identical.
//!
//! In debug builds, `WJ_JIT_MUTANT` removes one check from the machine code; the mutation gate
//! asserts the generator notices each one.

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len() as u64) as usize]
    }
}

const LEN: u64 = 64;

/// An `int` expression (the loop counter `i`, `b`, elements of `v`, casts of the others).
fn int_expr(r: &mut Rng, d: u32) -> String {
    if d == 0 || r.below(3) == 0 {
        return match r.below(9) {
            0 => (r.below(2000) as i64 - 1000).to_string(),
            1 => format!("v[(i * {}) % {LEN}]", 1 + r.below(13)),
            2 => "(a as int)".into(),
            3 => "(c as int)".into(),
            4 => "(u as int)".into(),
            5 => "ps[i % 4].n".into(),
            6 => format!("({} as int)", r.pick(&["x", "y", "fs[i % 8]"])),
            7 => "b".into(),
            _ => "i".into(),
        };
    }
    let op = r.pick(&["+", "-", "*", "%", "/", "+", "-", "&", "^"]);
    let rhs = if matches!(op, "%" | "/") { (1 + r.below(97)).to_string() } else { int_expr(r, d - 1) };
    format!("({} {op} {rhs})", int_expr(r, d - 1))
}

fn float_expr(r: &mut Rng, d: u32) -> String {
    if d == 0 || r.below(3) == 0 {
        return match r.below(8) {
            0 => r.pick(&["0.5", "1.0", "-2.25", "1e300", "0.0", "-0.0", "3.0"]).into(),
            1 => "x".into(),
            2 => "y".into(),
            3 => "fs[i % 8]".into(),
            4 => "ps[i % 4].x".into(),
            5 => "(i as f64)".into(),
            6 => format!("({}).sqrt()", r.pick(&["x", "y", "-x"])),
            _ => format!("({}).{}()", r.pick(&["x", "y"]), r.pick(&["abs", "floor", "ceil", "trunc", "round"])),
        };
    }
    let op = r.pick(&["+", "-", "*", "/", "%", "*"]);
    format!("({} {op} {})", float_expr(r, d - 1), float_expr(r, d - 1))
}

fn cond(r: &mut Rng) -> String {
    let c = match r.below(6) {
        0 => format!("{} {} {}", float_expr(r, 1), r.pick(&["<", "<=", ">", ">=", "==", "!="]), float_expr(r, 1)),
        1 => format!("bs[i % {LEN}]"),
        2 => format!("c {} {}", r.pick(&["<", ">=", "=="]), r.below(50)),
        3 => "f".into(),
        _ => format!("{} {} {}", int_expr(r, 1), r.pick(&["<", "<=", ">", ">=", "==", "!="]), int_expr(r, 1)),
    };
    match r.below(5) {
        0 => format!("{c} && {}", cond_leaf(r)),
        1 => format!("!({c}) || {}", cond_leaf(r)),
        _ => c,
    }
}

fn cond_leaf(r: &mut Rng) -> String {
    format!("i % {} == 0", 2 + r.below(9))
}

/// A statement of the main loop body (`inner`: inside a nested `while`, where `continue` would skip
/// the inner counter).
fn stmt(r: &mut Rng, depth: u32, iters: u64, inner: bool) -> String {
    let late = iters - 1 - r.below(iters / 4);
    let k = loop {
        let k = r.below(27);
        let ok = match k {
            19 => !inner,
            21 => depth < 2 && !inner,
            16 => depth < 3,
            _ => true,
        };
        if ok {
            break k;
        }
    };
    match k {
        0 => format!("a = ({} % 1000) as i32", int_expr(r, 2)),
        1 => format!("a {}= (i % 7) as i32", r.pick(&["+", "-", "*"])),
        2 => format!("b = {} % 1000003", int_expr(r, 2)),
        3 => format!("b {}= {}", r.pick(&["+", "-", "*"]), int_expr(r, 1)),
        4 => format!("c = c + (i % 5) as usize"),
        5 => format!("c {}= 1", r.pick(&["+", "-"])),
        6 => format!("u = u + {}", r.pick(&["1", "(i % 3) as u8", "2"])),
        7 => format!("x = {}", float_expr(r, 2)),
        8 => format!("y {}= {}", r.pick(&["+", "-", "*", "/"]), float_expr(r, 1)),
        9 => format!("f = {}", cond(r)),
        10 => format!("v[(i * {}) % {LEN}] = {}", 1 + r.below(11), int_expr(r, 1)),
        11 => format!("v[i % {LEN}] {}= {}", r.pick(&["+", "-"]), int_expr(r, 1)),
        12 => format!("fs[i % 8] = {}", float_expr(r, 1)),
        13 => format!("bs[(i + {}) % {LEN}] = {}", r.below(9), cond(r)),
        14 => format!("ps[i % 4].x += {}", float_expr(r, 1)),
        15 => format!("ps[(i + 1) % 4].n = {} % 997", int_expr(r, 1)),
        16 => format!("if {} {{\n{}\n}} else {{\n{}\n}}", cond(r), stmt(r, depth + 1, iters, inner), stmt(r, depth + 1, iters, inner)),
        // planted edges, late enough that the loop is in machine code
        17 => planted(r.below(6), late),
        // a literal-typed value stored into the `i32` local keeps `i32` only while it fits (`retag`)
        22 => format!("a = i * {}", r.pick(&["3", "3000000", "-1000000"])),
        23 => "a = a + 1".into(),
        // statements outside the machine-code subset run as closure code from inside the loop
        24 => format!("b = h(b) + {}", r.below(5)),
        25 => format!("if i % {} == 0 {{ println(\"p ${{b}} ${{x}}\") }}", 97 + r.below(200)),
        26 => format!("let w = h(a as int)\nb = (b + w) % 1000003"),
        18 => format!("let t = {}\nb = (b + t) % 1000003", int_expr(r, 2)),
        19 => format!("if {} {{ continue }}", cond_leaf(r)),
        20 => format!("if i == {late} {{ break }}"),
        21 if r.below(2) == 0 => format!(
            "for j in {}{} {{\n{}\n}}",
            r.pick(&["0..", "1..=", "(c % 3)..", "(i % 2)..="]),
            1 + r.below(4),
            stmt(r, depth + 1, iters, true).replace("i % ", "(i + j as int) % ")
        ),
        _ => format!(
            "let mut j = 0\nwhile j < {} {{\n{}\nj += 1\n}}",
            1 + r.below(4),
            stmt(r, depth + 1, iters, true).replace("i % ", "(i + j) % ")
        ),
    }
}

/// An edge reached at a late iteration (`late`), when the loop runs as machine code: a zero divisor,
/// an index past the end (read and write), `usize` underflow, `i32` and `u8` overflow.
fn planted(kind: u64, late: u64) -> String {
    match kind {
        0 => format!("if i == {late} {{ b = b / (i - {late}) }}"),
        1 => format!("if i == {late} {{ b = v[i + {LEN}] }}"),
        2 => format!("if i == {late} {{ c = c - 1000000 }}"),
        3 => format!("if i == {late} {{ a = a * 100000 * 100000 }}"),
        4 => format!("if i == {late} {{ u = u + 255 }}"),
        5 => format!("if i == {late} {{ v[{late} % {LEN} + {LEN}] = 1 }}"),
        // a panic inside a function a closure-run statement calls (its frame is in the backtrace)
        _ => format!("if i == {late} {{ b = h(-1) }}"),
    }
}

fn program(seed: u64) -> String {
    let mut r = Rng(seed.wrapping_mul(0x9e3779b97f4a7c15) | 1);
    let iters = 400 + r.below(1500);
    let mut body = String::new();
    for _ in 0..2 + r.below(5) {
        body += &stmt(&mut r, 0, iters, false);
        body.push('\n');
    }
    // every third program has one edge of each kind in turn, so each kind is exercised
    if seed % 3 == 0 {
        body += &planted((seed / 3) % 7, iters - 1 - r.below(iters / 4));
        body.push('\n');
    }
    format!(
        "struct P {{\n    x: f64,\n    n: int,\n}}\n\nfn h(x: int) -> int {{\n    if x < 0 {{ return 1000 / (x + 1) }}\n    x % 1000 + 1\n}}\n\nfn main() {{
    let mut v: Vec<int> = Vec::new()
    let mut fs: Vec<f64> = Vec::new()
    let mut bs: Vec<bool> = Vec::new()
    let mut ps: Vec<P> = Vec::new()
    let mut k = 0
    while k < {LEN} {{
        v.push(k * {m} % 101)
        fs.push(k as f64 * {fl})
        bs.push(k % 3 == 0)
        k += 1
    }}
    let mut q = 0
    while q < 4 {{
        ps.push(P {{ x: q as f64 * 0.5, n: q }})
        q += 1
    }}
    let mut a: i32 = {a0}
    let mut b: int = {b0}
    let mut c: usize = {c0}
    let mut u: u8 = {u0}
    let mut x: f64 = {x0}
    let mut y = {y0}
    let mut f = false
    let mut n = 0
{head}
        n += 1
{body}    }}
    println(\"${{a}} ${{b}} ${{c}} ${{u}} ${{x}} ${{y}} ${{f}} ${{n}}\")
    println(\"${{v[0]}} ${{v[{last}]}} ${{fs[1]}} ${{bs[2]}} ${{ps[0].x}} ${{ps[3].n}}\")
}}
",
        m = 1 + r.below(50),
        fl = r.pick(&["0.5", "1.25", "-3.0"]),
        a0 = r.below(100),
        b0 = r.below(1000),
        c0 = r.below(20),
        u0 = r.below(200),
        x0 = r.pick(&["0.0", "1.5", "-2.0", "1e10"]),
        y0 = r.pick(&["0.25", "2.0", "-1.0"]),
        last = LEN - 1,
        head = if r.below(3) == 0 {
            format!("    for i in 1..={iters} {{")
        } else {
            format!("    let mut i = 0\n    while i < {iters} {{\n        i += 1")
        },
    )
}

type Outcome = (Vec<u8>, Vec<u8>, Option<i32>);

fn run(path: &Path, engine: &str, extra: &[&str], env: &[(&str, &str)]) -> Outcome {
    let mut c = Command::new(WJ);
    c.args(["run", "--interpret", "--no-lint", "--engine", engine]).args(extra).arg(path).stdin(std::process::Stdio::null());
    c.env_remove("WJ_JIT").env_remove("WJ_JIT_LOOP_HOT").env_remove("WJ_JIT_MUTANT").env_remove("WJ_JIT_DEBUG");
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output().unwrap();
    (o.stdout, o.stderr, o.status.code())
}

fn show(o: &Outcome) -> String {
    format!("exit {:?}\n{}{}", o.2, String::from_utf8_lossy(&o.0), String::from_utf8_lossy(&o.1))
}

/// Seeds `seeds`: the programs where some configuration differs from the walker.
fn differences(dir: &Path, seeds: std::ops::RangeInclusive<u64>, configs: &[(&str, &[&str], &[(&str, &str)])], outcomes: &mut Vec<Option<i32>>) -> Vec<String> {
    let mut diffs = Vec::new();
    for seed in seeds {
        let p = dir.join(format!("loops_{seed}.wj"));
        std::fs::write(&p, program(seed)).unwrap();
        let base = run(&p, "walker", &[], &[]);
        outcomes.push(base.2);
        for (label, extra, env) in configs {
            let o = run(&p, "resolved", extra, env);
            if o != base {
                diffs.push(format!("seed {seed} ({label}):\n{}\n--- walker {}\n--- {label} {}", program(seed), show(&base), show(&o)));
            }
        }
    }
    diffs
}

#[test]
fn machine_code_loops_match_the_walker() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut outcomes = Vec::new();
    let configs: &[(&str, &[&str], &[(&str, &str)])] =
        &[("eager jit", &[], &[("WJ_JIT_LOOP_HOT", "0")]), ("default jit", &[], &[]), ("--no-jit", &["--no-jit"], &[])];
    let diffs = differences(tmp.path(), 1..=150, configs, &mut outcomes);
    let clean = outcomes.iter().filter(|o| **o == Some(0)).count();
    let panics = outcomes.iter().filter(|o| **o == Some(101)).count();
    // the generator must exercise both the machine-code path and the exits that end in a panic
    assert!(clean >= 20 && panics >= 20, "clean {clean}, panicking {panics}: {outcomes:?}");
    assert!(diffs.is_empty(), "{} programs differ:\n{}", diffs.len(), diffs.join("\n=====\n"));
}

/// The generated loops must actually run as machine code (not be refused by the lowering).
#[test]
fn generated_loops_enter_machine_code() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut compiled = 0;
    let n = 40;
    for seed in 1..=n {
        let p = tmp.path().join(format!("cov_{seed}.wj"));
        std::fs::write(&p, program(seed)).unwrap();
        let src = program(seed);
        let line = 1 + src.lines().position(|l| l.starts_with("    while i < ") || l.starts_with("    for i in ")).unwrap();
        let o = run(&p, "resolved", &[], &[("WJ_JIT_LOOP_HOT", "0"), ("WJ_JIT_DEBUG", "1")]);
        let err = String::from_utf8_lossy(&o.1);
        if err.lines().any(|l| l.starts_with("[jit] loop") && l.ends_with(&format!("cov_{seed}.wj:{line}: compiled"))) {
            compiled += 1;
        }
    }
    assert!(compiled * 10 >= n * 7, "only {compiled} of {n} generated programs ran a loop in machine code");
}

/// Mutation gate (debug builds): each check left out of the machine code makes some program differ.
#[test]
#[cfg(debug_assertions)]
fn every_machine_code_check_is_load_bearing() {
    let tmp = tempfile::TempDir::new().unwrap();
    for m in ["overflow", "range", "divzero", "bounds", "nan", "retag"] {
        let env = [("WJ_JIT_LOOP_HOT", "0"), ("WJ_JIT_MUTANT", m)];
        let configs: &[(&str, &[&str], &[(&str, &str)])] = &[("mutant", &[], &env)];
        let diffs = differences(tmp.path(), 1..=60, configs, &mut Vec::new());
        assert!(!diffs.is_empty(), "mutant `{m}` survived 60 generated programs");
    }
}
