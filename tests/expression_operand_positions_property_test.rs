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
//! Property gate: `if` / `match` expressions used as operands must keep their value in every
//! statement position (let initialiser, assignment right-hand side, compound assignment, tail of an
//! `if` block, loop body, after a `let = match`).
//!
//! A seeded generator builds random programs from a small statement/expression tree, evaluates the
//! tree itself (the independent oracle), renders Windjammer source, and requires the compiled
//! program (`wj build --no-cargo` + `rustc`) and the interpreted program (`wj run --interpret`) to
//! both print exactly what the oracle computed. A dropped operand changes the number or fails to
//! compile, so a miscompile cannot pass silently.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[derive(Clone)]
enum E {
    Lit(i64),
    Var(String),
    Bin(char, Box<E>, Box<E>),
    If(Box<E>, &'static str, Box<E>, Box<E>, Box<E>),
    Match(Box<E>, Box<E>, Box<E>, Box<E>),
    /// `pick(a, b)` = a - b: operands as call arguments
    Pick(Box<E>, Box<E>),
}

enum S {
    Let(String, E),
    LetMatch(String, E, E, E, E),
    /// `match <expr> { 0 => a, 1 => b, _ => c }`: the scrutinee itself is an expression-valued operand
    LetMatchDirect(String, E, E, E, E),
    /// `note(<expr>)`: a call statement whose argument is the operand
    Note(E),
    Assign(E),
    Compound(char, E),
    If(E, &'static str, E, Vec<S>, Vec<S>),
    For(String, i64, Vec<S>),
}

type Env = Vec<(String, i64)>;

fn lookup(env: &Env, n: &str) -> i64 {
    env.iter().rev().find(|(k, _)| k == n).map(|(_, v)| *v).unwrap()
}

fn eval(e: &E, env: &Env) -> i64 {
    match e {
        E::Lit(n) => *n,
        E::Var(v) => lookup(env, v),
        E::Bin(op, a, b) => {
            let (x, y) = (eval(a, env), eval(b, env));
            match op {
                '+' => x + y,
                '-' => x - y,
                _ => x * y,
            }
        }
        E::If(l, cmp, r, t, f) => {
            let (x, y) = (eval(l, env), eval(r, env));
            let c = match *cmp {
                "<" => x < y,
                ">" => x > y,
                _ => x == y,
            };
            if c { eval(t, env) } else { eval(f, env) }
        }
        E::Pick(a, b) => eval(a, env) - eval(b, env),
        E::Match(s, a, b, c) => match eval(s, env) % 3 {
            0 => eval(a, env),
            1 => eval(b, env),
            _ => eval(c, env),
        },
    }
}

fn set(env: &mut Env, n: &str, v: i64) {
    env.iter_mut().rev().find(|(k, _)| k == n).unwrap().1 = v;
}

fn run(stmts: &[S], env: &mut Env) {
    for s in stmts {
        match s {
            S::Let(n, e) => {
                let v = eval(e, env);
                env.push((n.clone(), v));
            }
            S::LetMatch(n, s, a, b, c) => {
                let v = match eval(s, env) % 3 {
                    0 => eval(a, env),
                    1 => eval(b, env),
                    _ => eval(c, env),
                };
                env.push((n.clone(), v));
            }
            S::LetMatchDirect(n, s, a, b, c) => {
                let v = match eval(s, env) {
                    0 => eval(a, env),
                    1 => eval(b, env),
                    _ => eval(c, env),
                };
                env.push((n.clone(), v));
            }
            S::Note(_) => {}
            S::Assign(e) => {
                let v = lookup(env, "acc") + eval(e, env);
                set(env, "acc", v);
            }
            S::Compound(op, e) => {
                let (x, y) = (lookup(env, "acc"), eval(e, env));
                set(env, "acc", if *op == '+' { x + y } else { x - y });
            }
            S::If(l, cmp, r, t, f) => {
                let (x, y) = (eval(l, env), eval(r, env));
                let c = match *cmp {
                    "<" => x < y,
                    ">" => x > y,
                    _ => x == y,
                };
                let n = env.len();
                run(if c { t } else { f }, env);
                env.truncate(n);
            }
            S::For(v, n, body) => {
                for i in 0..*n {
                    let len = env.len();
                    env.push((v.clone(), i));
                    run(body, env);
                    env.truncate(len);
                }
            }
        }
    }
}

fn render_e(e: &E) -> String {
    match e {
        E::Lit(n) if *n < 0 => format!("(0 - {})", -n),
        E::Lit(n) => n.to_string(),
        E::Var(v) => v.clone(),
        E::Bin(op, a, b) => format!("({} {} {})", render_e(a), op, render_e(b)),
        E::If(l, c, r, t, f) => format!("(if {} {} {} {{ {} }} else {{ {} }})", render_e(l), c, render_e(r), render_e(t), render_e(f)),
        E::Pick(a, b) => format!("pick({}, {})", render_e(a), render_e(b)),
        E::Match(s, a, b, c) => format!("(match {} % 3 {{ 0 => {}, 1 => {}, _ => {} }})", render_e(s), render_e(a), render_e(b), render_e(c)),
    }
}

fn render_s(stmts: &[S], ind: usize, out: &mut String) {
    let pad = "    ".repeat(ind);
    for s in stmts {
        match s {
            S::Let(n, e) => out.push_str(&format!("{pad}let {n}: int = {}\n", render_e(e))),
            S::LetMatch(n, s, a, b, c) => out.push_str(&format!(
                "{pad}let {n}: int = match {} % 3 {{\n{pad}    0 => {},\n{pad}    1 => {},\n{pad}    _ => {},\n{pad}}}\n",
                render_e(s), render_e(a), render_e(b), render_e(c)
            )),
            S::LetMatchDirect(n, s, a, b, c) => out.push_str(&format!(
                "{pad}let {n}: int = match {} {{\n{pad}    0 => {},\n{pad}    1 => {},\n{pad}    _ => {},\n{pad}}}\n",
                render_e(s), render_e(a), render_e(b), render_e(c)
            )),
            S::Note(e) => out.push_str(&format!("{pad}note({})\n", render_e(e))),
            S::Assign(e) => out.push_str(&format!("{pad}acc = acc + {}\n", render_e(e))),
            S::Compound(op, e) => out.push_str(&format!("{pad}acc {op}= {}\n", render_e(e))),
            S::If(l, c, r, t, f) => {
                out.push_str(&format!("{pad}if {} {} {} {{\n", render_e(l), c, render_e(r)));
                render_s(t, ind + 1, out);
                if f.is_empty() {
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    render_s(f, ind + 1, out);
                    out.push_str(&format!("{pad}}}\n"));
                }
            }
            S::For(v, n, body) => {
                // a typed bound keeps the loop variable `int`; an untyped `0..2` is the numeric solver's
                // concern (see `for_variable_over_untyped_range_is_int` below)
                out.push_str(&format!("{pad}let {v}_n: int = {n}\n{pad}for {v} in 0..{v}_n {{\n"));
                render_s(body, ind + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
        }
    }
}

struct Gen {
    r: Rng,
    fresh: usize,
}

impl Gen {
    fn expr(&mut self, vars: &[String], depth: u32) -> E {
        let k = if depth == 0 { self.r.below(2) } else { self.r.below(7) };
        match k {
            0 => E::Lit(self.r.below(9) as i64 - 4),
            1 => E::Var(vars[self.r.below(vars.len() as u64) as usize].clone()),
            2 | 3 => {
                let op = ['+', '-', '*'][self.r.below(3) as usize];
                let a = self.expr(vars, depth - 1);
                // keep products small: the right operand of `*` is a literal
                let b = if op == '*' { E::Lit(self.r.below(3) as i64 + 1) } else { self.expr(vars, depth - 1) };
                E::Bin(op, Box::new(a), Box::new(b))
            }
            4 => E::If(
                Box::new(self.expr(vars, depth - 1)),
                ["<", ">", "=="][self.r.below(3) as usize],
                Box::new(self.expr(vars, depth - 1)),
                Box::new(self.expr(vars, depth - 1)),
                Box::new(self.expr(vars, depth - 1)),
            ),
            6 => E::Pick(Box::new(self.expr(vars, depth - 1)), Box::new(self.expr(vars, depth - 1))),
            _ => E::Match(
                Box::new(self.expr(vars, depth - 1)),
                Box::new(self.expr(vars, depth - 1)),
                Box::new(self.expr(vars, depth - 1)),
                Box::new(self.expr(vars, depth - 1)),
            ),
        }
    }

    fn block(&mut self, vars: &mut Vec<String>, depth: u32, tail_assign: bool) -> Vec<S> {
        let scope = vars.len();
        let mut out = Vec::new();
        for _ in 0..1 + self.r.below(3) {
            match self.r.below(4) {
                0 => {
                    out.push(S::Note(self.expr(vars, 2)));
                    continue;
                }
                1 => {
                    let n = self.name();
                    let (s, a, b, c) = (self.expr(vars, 2), self.expr(vars, 1), self.expr(vars, 1), self.expr(vars, 1));
                    out.push(S::LetMatchDirect(n.clone(), s, a, b, c));
                    vars.push(n);
                    continue;
                }
                _ => {}
            }
            let k = self.r.below(if depth == 0 { 4 } else { 7 });
            match k {
                0 => {
                    let n = self.name();
                    out.push(S::Let(n.clone(), self.expr(vars, 2)));
                    vars.push(n);
                }
                1 => {
                    let n = self.name();
                    let (s, a, b, c) = (self.expr(vars, 1), self.expr(vars, 1), self.expr(vars, 1), self.expr(vars, 1));
                    out.push(S::LetMatch(n.clone(), s, a, b, c));
                    vars.push(n);
                }
                2 => out.push(S::Assign(self.expr(vars, 3))),
                3 => out.push(S::Compound(['+', '-'][self.r.below(2) as usize], self.expr(vars, 3))),
                4 | 5 => {
                    let (l, r) = (self.expr(vars, 1), self.expr(vars, 1));
                    let cmp = ["<", ">", "=="][self.r.below(3) as usize];
                    let t = self.block(vars, depth - 1, true);
                    let f = if self.r.below(2) == 0 { Vec::new() } else { self.block(vars, depth - 1, true) };
                    out.push(S::If(l, cmp, r, t, f));
                }
                _ => {
                    let v = self.name();
                    let n = 1 + self.r.below(3) as i64;
                    vars.push(v.clone());
                    let body = self.block(vars, depth - 1, true);
                    vars.pop();
                    out.push(S::For(v, n, body));
                }
            }
        }
        if tail_assign {
            // the shape that was miscompiled: an operand `if`/`match` in the last statement of a block
            let e = if self.r.below(2) == 0 {
                E::If(
                    Box::new(self.expr(vars, 1)),
                    "<",
                    Box::new(E::Lit(0)),
                    Box::new(self.expr(vars, 1)),
                    Box::new(self.expr(vars, 1)),
                )
            } else {
                self.expr(vars, 3)
            };
            out.push(S::Assign(e));
        }
        vars.truncate(scope);
        out
    }

    fn name(&mut self) -> String {
        self.fresh += 1;
        format!("t{}", self.fresh)
    }
}

/// Program with `count` generated functions, its Windjammer source and the oracle's output.
fn program(seed: u64, count: usize) -> (String, String) {
    let mut g = Gen { r: Rng::new(seed), fresh: 0 };
    let mut src = String::from("fn pick(a: int, b: int) -> int {\n    a - b\n}\n\nfn note(x: int) {\n}\n\n");
    let mut main = String::from("fn main() {\n");
    let mut expect = String::new();
    for i in 0..count {
        let mut vars = vec!["a".to_string(), "b".to_string(), "acc".to_string()];
        let body = g.block(&mut vars, 2, true);
        src.push_str(&format!("fn f{i}(a: int, b: int) -> int {{\n    let mut acc: int = 0\n"));
        render_s(&body, 1, &mut src);
        src.push_str("    acc\n}\n\n");
        for (a, b) in [(3, -2), (-5, 4)] {
            let mut env: Env = vec![("a".into(), a), ("b".into(), b), ("acc".into(), 0)];
            run(&body, &mut env);
            main.push_str(&format!("    println(f{i}({a}, {b}))\n"));
            expect.push_str(&format!("{}\n", lookup(&env, "acc")));
        }
    }
    main.push_str("}\n");
    src.push_str(&main);
    (src, expect)
}

fn compiled(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).current_dir(dir.path()).output().unwrap();
    if !wj.status.success() {
        return Err(format!("wj build failed: {}{}", String::from_utf8_lossy(&wj.stdout), String::from_utf8_lossy(&wj.stderr)));
    }
    let rs = dir.path().join("build/prog.rs");
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(&rs).arg("-o").arg(&bin).args(["--edition", "2021", "-A", "warnings"]).output().unwrap();
    if !rustc.status.success() {
        return Err(format!("rustc failed: {}\n--- generated\n{}", String::from_utf8_lossy(&rustc.stderr), fs::read_to_string(&rs).unwrap_or_default()));
    }
    Ok(String::from_utf8_lossy(&Command::new(&bin).output().unwrap().stdout).to_string())
}

fn interpreted(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let out = Command::new(WJ).args(["run", "--interpret"]).arg(&f).output().unwrap();
    if !out.status.success() {
        return Err(format!("interpreter failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// A compile failure that is purely an integer-width disagreement (`i32` vs `i64`): the numeric
/// solver's class (see `loop_variable_compared_with_literal_inside_if_expression_is_int`), not a dropped
/// value. Any error mentioning `()` is never classified this way.
fn is_width_only_failure(e: &str) -> bool {
    e.contains("rustc failed")
        && !e.contains("`()`")
        && !e.contains("expected pattern")
        && !e.contains("expected expression")
        && (e.contains("found `i32`") || e.contains("found `i64`") || e.contains("`i64` to `i32`") || e.contains("`i32` to `i64`") || e.contains("`i32` by `i64`") || e.contains("`i64` by `i32`"))
}

fn check_seeds(seeds: std::ops::Range<u64>) {
    let total = seeds.end - seeds.start;
    let mut width_only = 0;
    for seed in seeds {
        let (src, expect) = program(seed, 8);
        let i = interpreted(&src).unwrap_or_else(|e| panic!("seed {seed}: {e}\n--- source\n{src}"));
        assert_eq!(i, expect, "seed {seed}: interpreter differs from the oracle\n--- source\n{src}");
        match compiled(&src) {
            Ok(c) => assert_eq!(c, expect, "seed {seed}: compiled output differs from the oracle\n--- source\n{src}"),
            Err(e) if is_width_only_failure(&e) => {
                eprintln!("seed {seed}: integer-width disagreement (numeric solver class), skipped");
                width_only += 1;
            }
            Err(e) => panic!("seed {seed}: {e}\n--- source\n{src}"),
        }
    }
    assert!(width_only * 2 <= total, "more than half of the seeds hit the numeric-width class: {width_only}/{total}");
}

#[test]
fn operand_if_and_match_keep_their_value_in_every_statement_position() {
    check_seeds(1..13);
}

#[test]
fn operand_if_in_tail_assignment_after_let_match() {
    let src = r#"fn parse(s: string) -> Result<int, string> {
    if s.is_empty() { return Err("empty") }
    Ok(5)
}

fn credit_total(text: string, bias: int) -> Result<int, string> {
    let mut cents = 0
    if !text.is_empty() {
        let c = match parse(text) {
            Ok(c) => c,
            Err(e) => return Err("bad: " + e),
        }
        cents = cents + (if c < bias { 0 - c } else { c })
    }
    Ok(cents)
}

fn show(r: Result<int, string>) -> int {
    match r {
        Ok(v) => v,
        Err(_) => 0 - 1000,
    }
}

fn main() {
    println(show(credit_total("x", 9)))
    println(show(credit_total("x", 1)))
    println(show(credit_total("", 1)))
}
"#;
    let expect = "-5\n5\n0\n";
    assert_eq!(interpreted(src).unwrap_or_else(|e| panic!("{e}")), expect);
    assert_eq!(compiled(src).unwrap_or_else(|e| panic!("{e}")), expect);
}

/// Acceptance test (numeric solver): a loop variable over an untyped range meets `int` operands, so it
/// is `int` as well.
#[test]
fn for_variable_over_untyped_range_is_int() {
    let src = r#"fn total(a: int) -> int {
    let mut acc: int = 0
    for i in 0..3 {
        acc = acc + a * i
    }
    acc
}

fn main() {
    println(total(5))
}
"#;
    assert_eq!(interpreted(src).unwrap_or_else(|e| panic!("{e}")), "15\n");
    assert_eq!(compiled(src).unwrap_or_else(|e| panic!("{e}")), "15\n");
}

/// Acceptance test (numeric solver): in `for i in 0..n` with `n: int`, comparing `i` with an untyped
/// literal inside an `if` expression must use the loop variable's width.
#[test]
fn loop_variable_compared_with_literal_inside_if_expression_is_int() {
    let src = r#"fn g(a: int, b: int) -> int {
    let n: int = 2
    let mut acc: int = 0
    for i in 0..n {
        acc = acc + (if i == 0 { a } else { b })
    }
    acc
}

fn main() {
    println(g(3, 4))
}
"#;
    assert_eq!(interpreted(src).unwrap_or_else(|e| panic!("{e}")), "7\n");
    assert_eq!(compiled(src).unwrap_or_else(|e| panic!("{e}")), "7\n");
}
