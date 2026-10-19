#![cfg(feature = "jit")]

//! ADR-0020 step 7 kill criterion: the Cranelift tier is a second implementation of integer
//! arithmetic, so it is checked against the tree walker over generated programs. Every program has
//! hot `int` functions (called far more often than the jit threshold) with constants large enough
//! that some calls overflow, divide by zero or recurse past the jit's depth budget; the walker and
//! `--engine resolved` (which jits) must print the same output, panic message and exit status.

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
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[(self.next() % xs.len() as u64) as usize]
    }
    fn int(&mut self) -> i64 {
        match self.next() % 6 {
            0 => (self.next() % 2_000_000_000) as i64,
            1 => -((self.next() % 1000) as i64),
            _ => (self.next() % 20) as i64,
        }
    }
}

fn expr(r: &mut Rng, vars: &[&str], depth: u32) -> String {
    if depth == 0 || r.next() % 3 == 0 {
        return match r.next() % 3 {
            0 => r.int().to_string(),
            _ => r.pick(vars).to_string(),
        };
    }
    let op = r.pick(&["+", "-", "*", "/", "%", "+", "-", "*"]);
    format!("({} {} {})", expr(r, vars, depth - 1), op, expr(r, vars, depth - 1))
}

fn cond(r: &mut Rng, vars: &[&str]) -> String {
    let op = r.pick(&["<", "<=", ">", ">=", "==", "!="]);
    format!("{} {} {}", expr(r, vars, 1), op, expr(r, vars, 1))
}

fn program(seed: u64) -> String {
    let mut r = Rng(seed.wrapping_mul(0x9e3779b97f4a7c15) | 1);
    let mut s = String::new();
    // f2: a leaf; f1 calls f2; f0 recurses on its first argument and calls f1
    // `b * b` overflows once `b` (scaled by the loop counter in `main`) is large enough: usually
    // after the functions are hot, so the jitted code must bail out there
    s += &format!("fn f2(a: int, b: int) -> int {{\n    let x = {}\n    if {} {{ return x }}\n    x - (b * b) / 7 + {}\n}}\n", expr(&mut r, &["a", "b"], 2), cond(&mut r, &["a", "b", "x"]), expr(&mut r, &["a", "b", "x"], 2));
    s += &format!(
        "fn f1(a: int, b: int) -> int {{\n    let mut t = {}\n    let mut k = 0\n    while k < 3 {{\n        t = t + f2(k, b + {})\n        k += 1\n    }}\n    t\n}}\n",
        expr(&mut r, &["a", "b"], 2),
        expr(&mut r, &["a", "b", "k"], 1)
    );
    let depth = if r.next() % 4 == 0 { 3000 } else { 6 };
    s += &format!(
        "fn f0(n: int, m: int) -> int {{\n    if n <= 0 {{ return {} }}\n    let y = f1(n, m) % 1000003\n    y + f0(n - 1, {})\n}}\n",
        expr(&mut r, &["m"], 1),
        expr(&mut r, &["n", "m", "y"], 1)
    );
    let scale = 5_000_000 + r.next() % 25_000_000;
    s += &format!(
        "fn main() {{\n    let mut total = 0\n    let mut i = 0\n    while i < 300 {{\n        total = (total + f0({}, i * {scale}) % 1000000007) % 1000000007\n        i += 1\n    }}\n    println(\"${{total}}\")\n}}\n",
        if depth > 100 { "i * 10".to_string() } else { "i % 7".to_string() }
    );
    s
}

fn run(path: &Path, engine: &str) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let o = Command::new(WJ)
        .args(["run", "--interpret", "--no-lint", "--engine", engine])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    (o.stdout, o.stderr, o.status.code())
}

#[test]
fn jitted_integer_functions_match_the_walker() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut diffs = Vec::new();
    let mut outcomes = std::collections::BTreeMap::new();
    for seed in 1..=120u64 {
        let p = tmp.path().join(format!("jit_{seed}.wj"));
        std::fs::write(&p, program(seed)).unwrap();
        let (a, b) = (run(&p, "walker"), run(&p, "resolved"));
        *outcomes.entry(a.2).or_insert(0) += 1;
        if a != b {
            diffs.push(format!("seed {seed}:\n{}\n--- walker {:?}\n{}{}\n--- resolved {:?}\n{}{}", program(seed), a.2, String::from_utf8_lossy(&a.0), String::from_utf8_lossy(&a.1), b.2, String::from_utf8_lossy(&b.0), String::from_utf8_lossy(&b.1)));
        }
    }
    // the generator must exercise both the jitted path (exit 0) and the bail-out path (panics)
    assert!(outcomes.get(&Some(0)).copied().unwrap_or(0) >= 10, "too few clean programs: {outcomes:?}");
    assert!(outcomes.get(&Some(101)).copied().unwrap_or(0) >= 10, "too few panicking programs: {outcomes:?}");
    assert!(diffs.is_empty(), "{} programs differ:\n{}", diffs.len(), diffs.join("\n=====\n"));
}
